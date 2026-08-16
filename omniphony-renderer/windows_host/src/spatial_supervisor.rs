#![cfg(target_os = "windows")]

mod session_mute;

use anyhow::{Context, bail};
use session_mute::DrySessionSilencer;
use std::ffi::OsStr;
use std::fs::{OpenOptions, create_dir_all};
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::io::AsRawHandle;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::{
    CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, LRESULT, POINT, WPARAM,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
    SetInformationJobObject,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::{CREATE_NO_WINDOW, CreateMutexW};
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, IDC_ARROW, IDI_APPLICATION, LoadCursorW,
    LoadIconW, MF_CHECKED, MF_GRAYED, MF_SEPARATOR, MF_STRING, MSG, PostMessageW,
    PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, SetTimer,
    TPM_LEFTALIGN, TPM_RIGHTBUTTON, TrackPopupMenu, TranslateMessage, WM_APP, WM_CLOSE,
    WM_COMMAND, WM_CREATE, WM_DESTROY, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WM_TIMER, WNDCLASSW,
    WS_OVERLAPPED,
};

const TRAY_ID: u32 = 1;
const WM_TRAY: u32 = WM_APP + 1;
const TIMER_ID: usize = 1;
const ID_STATUS: usize = 2001;
const ID_TOGGLE: usize = 2002;
const ID_RESTART: usize = 2003;
const ID_AUTOSTART: usize = 2004;
const ID_EXIT: usize = 2005;

const RESTART_DELAY: Duration = Duration::from_secs(2);
const AUTOSTART_VALUE: &str = "Spatial";
const LEGACY_AUTOSTART_VALUE: &str = "Omniphony";
const AUTOSTART_KEY: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

struct AppState {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    /// User intent. When false, there must be no audio-engine child process and
    /// no Windows sessions muted on Spatial's behalf.
    enabled: bool,
    quitting: bool,
    next_restart: Option<Instant>,
    restart_count: u32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            child: None,
            stdin: None,
            enabled: true,
            quitting: false,
            next_restart: None,
            restart_count: 0,
        }
    }
}

struct HandleGuard(HANDLE);

impl Drop for HandleGuard {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

static STATE: OnceLock<Mutex<AppState>> = OnceLock::new();
static TASKBAR_CREATED: OnceLock<u32> = OnceLock::new();
static DRY_SILENCER: OnceLock<Mutex<DrySessionSilencer>> = OnceLock::new();
// Stored as an integer so the static remains trivially Send + Sync. The actual
// HANDLE is owned by the HandleGuard that lives for the supervisor run.
static CHILD_JOB: OnceLock<usize> = OnceLock::new();

fn state() -> &'static Mutex<AppState> {
    STATE.get_or_init(|| Mutex::new(AppState::default()))
}

fn dry_silencer() -> &'static Mutex<DrySessionSilencer> {
    DRY_SILENCER.get_or_init(|| {
        Mutex::new(DrySessionSilencer::new(
            settings_root().join("spatial-dry-mutes.txt"),
        ))
    })
}

fn wide(text: &str) -> Vec<u16> {
    OsStr::new(text).encode_wide().chain(Some(0)).collect()
}

fn copy_wide<const N: usize>(target: &mut [u16; N], text: &str) {
    target.fill(0);
    let encoded: Vec<u16> = OsStr::new(text).encode_wide().collect();
    let count = encoded.len().min(N.saturating_sub(1));
    target[..count].copy_from_slice(&encoded[..count]);
}

fn taskbar_created_message() -> u32 {
    *TASKBAR_CREATED.get_or_init(|| {
        let name = wide("TaskbarCreated");
        unsafe { RegisterWindowMessageW(name.as_ptr()) }
    })
}

fn claim_single_instance() -> anyhow::Result<Option<HandleGuard>> {
    // Keep the legacy mutex name during the private-name transition so an old
    // Omniphony supervisor and a new Spatial supervisor cannot run together.
    let name = wide("Local\\OmniphonyForHeadphones.Singleton");
    let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
    if handle.is_null() {
        bail!("CreateMutexW failed for Spatial single-instance guard");
    }
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        unsafe {
            let _ = CloseHandle(handle);
        }
        return Ok(None);
    }
    Ok(Some(HandleGuard(handle)))
}

fn install_child_job() -> Option<HandleGuard> {
    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return None;
        }

        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            std::ptr::addr_of!(info).cast(),
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        ) == 0
        {
            let _ = CloseHandle(job);
            return None;
        }

        if CHILD_JOB.set(job as usize).is_err() {
            let _ = CloseHandle(job);
            return None;
        }
        Some(HandleGuard(job))
    }
}

fn assign_child_to_job(child: &Child) -> anyhow::Result<()> {
    let Some(raw_job) = CHILD_JOB.get().copied() else {
        return Ok(());
    };
    let job = raw_job as HANDLE;
    let process = child.as_raw_handle() as HANDLE;
    if unsafe { AssignProcessToJobObject(job, process) } == 0 {
        bail!("failed to bind Spatial audio child to kill-on-close job object");
    }
    Ok(())
}

fn executable_root() -> anyhow::Result<PathBuf> {
    let exe = std::env::current_exe().context("failed to resolve Spatial executable path")?;
    Ok(exe
        .parent()
        .context("Spatial executable has no parent directory")?
        .to_path_buf())
}

fn settings_root() -> PathBuf {
    // Preserve the existing private preference location so an already-disabled
    // autostart preference does not silently turn itself back on after renaming.
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("Omniphony")
}

fn append_log(message: &str) {
    let Ok(root) = executable_root() else {
        return;
    };
    let path = root.join("spatial.log");
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "[supervisor] {message}");
    }
}

fn autostart_marker() -> PathBuf {
    settings_root().join("autostart.disabled")
}

fn autostart_preferred() -> bool {
    !autostart_marker().is_file()
}

fn delete_run_value(name: &str) {
    let mut command = Command::new("reg.exe");
    command
        .creation_flags(CREATE_NO_WINDOW)
        .arg("DELETE")
        .arg(AUTOSTART_KEY)
        .arg("/v")
        .arg(name)
        .arg("/f");
    let _ = command.status();
}

fn set_run_entry(enabled: bool) -> anyhow::Result<()> {
    if enabled {
        let exe = std::env::current_exe().context("failed to resolve Spatial autostart executable")?;
        let value = format!("\"{}\"", exe.display());
        let mut command = Command::new("reg.exe");
        command
            .creation_flags(CREATE_NO_WINDOW)
            .arg("ADD")
            .arg(AUTOSTART_KEY)
            .arg("/v")
            .arg(AUTOSTART_VALUE)
            .arg("/t")
            .arg("REG_SZ")
            .arg("/d")
            .arg(value)
            .arg("/f");
        let status = command.status().context("failed to launch reg.exe")?;
        if !status.success() {
            bail!("reg.exe could not register Spatial autostart");
        }
        delete_run_value(LEGACY_AUTOSTART_VALUE);
    } else {
        delete_run_value(AUTOSTART_VALUE);
        delete_run_value(LEGACY_AUTOSTART_VALUE);
    }
    Ok(())
}

fn set_autostart(enabled: bool) -> anyhow::Result<()> {
    let marker = autostart_marker();
    if enabled {
        set_run_entry(true)?;
        let _ = std::fs::remove_file(marker);
    } else {
        let _ = set_run_entry(false);
        if let Some(parent) = marker.parent() {
            create_dir_all(parent).context("failed to create Spatial settings directory")?;
        }
        std::fs::write(marker, b"disabled\n")
            .context("failed to persist disabled autostart preference")?;
    }
    Ok(())
}

fn ensure_autostart() {
    if autostart_preferred() {
        if let Err(err) = set_run_entry(true) {
            append_log(&format!("autostart registration failed: {err:#}"));
        }
    } else {
        let _ = set_run_entry(false);
    }
}

fn restore_stale_dry_audio() {
    let mut silencer = dry_silencer()
        .lock()
        .expect("Spatial dry-session state poisoned");
    match silencer.restore_stale_snapshot() {
        Ok(count) if count > 0 => append_log(&format!(
            "restored {count} dry audio session(s) from previous Spatial run"
        )),
        Ok(_) => {}
        Err(err) => append_log(&format!("stale dry-session restore failed: {err:#}")),
    }
}

fn restore_dry_audio() {
    let mut silencer = dry_silencer()
        .lock()
        .expect("Spatial dry-session state poisoned");
    match silencer.restore() {
        Ok(count) if count > 0 => {
            append_log(&format!("restored {count} dry audio session(s)"));
        }
        Ok(_) => {}
        Err(err) => append_log(&format!("dry-session restore failed: {err:#}")),
    }
}

fn silence_dry_audio(skip_pids: &[u32]) -> anyhow::Result<()> {
    let mut silencer = dry_silencer()
        .lock()
        .expect("Spatial dry-session state poisoned");
    match silencer.silence_external_sessions(skip_pids) {
        Ok(count) => {
            if count > 0 {
                append_log(&format!("silenced {count} dry audio session(s)"));
            }
            Ok(())
        }
        Err(err) => {
            // If persistence or enumeration fails after any successful mute,
            // immediately roll back everything we still own rather than leave
            // Windows in a partially muted, untracked state.
            let _ = silencer.restore();
            Err(err).context("could not establish temporary single-stream routing")
        }
    }
}

fn child_pid() -> Option<u32> {
    state()
        .lock()
        .expect("Spatial supervisor state poisoned")
        .child
        .as_ref()
        .map(Child::id)
}

fn refresh_dry_audio(hwnd: HWND) {
    let (enabled, quitting, pid) = {
        let app = state().lock().expect("Spatial supervisor state poisoned");
        (
            app.enabled,
            app.quitting,
            app.child.as_ref().map(Child::id),
        )
    };
    let Some(pid) = pid else {
        return;
    };
    if !enabled || quitting {
        return;
    }

    if let Err(err) = silence_dry_audio(&[std::process::id(), pid]) {
        append_log(&format!(
            "dry-session refresh failed; stopping Spatial rather than allowing doubled audio: {err:#}"
        ));
        {
            let mut app = state().lock().expect("Spatial supervisor state poisoned");
            app.enabled = false;
            app.next_restart = None;
        }
        stop_worker();
        restore_dry_audio();
        update_tray_tip(hwnd);
    }
}

fn spawn_worker() -> anyhow::Result<()> {
    {
        let app = state().lock().expect("Spatial supervisor state poisoned");
        if !app.enabled || app.quitting || app.child.is_some() {
            return Ok(());
        }
    }

    // Current process-loopback is a copy rather than an intercept. For this
    // temporary private host, silence existing external render sessions before
    // the audio child starts so the listener hears only Spatial's output.
    silence_dry_audio(&[std::process::id()])?;

    let root = executable_root()?;
    let executable = std::env::current_exe().context("failed to resolve Spatial engine executable")?;

    let log_path = root.join("spatial.log");
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("failed to open {}", log_path.display()))?;
    let log_err = log.try_clone().context("failed to clone Spatial log handle")?;

    let spawn_result = Command::new(&executable)
        .current_dir(&root)
        .env("OMNIPHONY_INTERNAL_ENGINE", "1")
        .env("OMNIPHONY_PROFILE", "external")
        .stdin(Stdio::piped())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();

    let mut child = match spawn_result {
        Ok(child) => child,
        Err(err) => {
            restore_dry_audio();
            return Err(err).with_context(|| {
                format!(
                    "failed to launch internal audio engine from {}",
                    executable.display()
                )
            });
        }
    };

    if let Err(err) = assign_child_to_job(&child) {
        let _ = child.kill();
        let _ = child.wait();
        restore_dry_audio();
        return Err(err);
    }

    let stdin = match child.stdin.take() {
        Some(stdin) => stdin,
        None => {
            let _ = child.kill();
            let _ = child.wait();
            restore_dry_audio();
            bail!("audio engine stdin was not piped");
        }
    };

    let pid = child.id();
    let mut app = state().lock().expect("Spatial supervisor state poisoned");
    if !app.enabled || app.quitting {
        drop(app);
        let _ = child.kill();
        let _ = child.wait();
        restore_dry_audio();
        return Ok(());
    }
    app.child = Some(child);
    app.stdin = Some(stdin);
    app.next_restart = None;
    drop(app);

    // Catch any sessions that appeared during child startup while explicitly
    // excluding both supervisor and Spatial renderer output.
    if let Err(err) = silence_dry_audio(&[std::process::id(), pid]) {
        stop_worker();
        restore_dry_audio();
        return Err(err);
    }

    append_log("Spatial audio engine started with Current model");
    Ok(())
}

fn stop_worker() {
    let (mut child, mut stdin) = {
        let mut app = state().lock().expect("Spatial supervisor state poisoned");
        app.next_restart = None;
        (app.child.take(), app.stdin.take())
    };

    if let Some(stdin) = stdin.as_mut() {
        let _ = stdin.write_all(b"q\n");
        let _ = stdin.flush();
    }

    if let Some(child) = child.as_mut() {
        let mut exited = false;
        for _ in 0..12 {
            match child.try_wait() {
                Ok(Some(_)) => {
                    exited = true;
                    break;
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(_) => break,
            }
        }
        if !exited {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

fn schedule_restart(detail: &str) {
    append_log(detail);
    let mut app = state().lock().expect("Spatial supervisor state poisoned");
    if app.enabled && !app.quitting {
        app.next_restart = Some(Instant::now() + RESTART_DELAY);
        app.restart_count = app.restart_count.saturating_add(1);
    } else {
        app.next_restart = None;
    }
}

fn set_enabled(hwnd: HWND, enabled: bool) {
    {
        let mut app = state().lock().expect("Spatial supervisor state poisoned");
        app.enabled = enabled;
        app.next_restart = None;
    }

    if enabled {
        append_log("Spatial ON requested");
        if let Err(err) = spawn_worker() {
            restore_dry_audio();
            schedule_restart(&format!("audio engine start failed: {err:#}"));
        }
    } else {
        // OFF is a hard lifecycle state, not clean bypass. The child is gone,
        // capture/output handles are released, and source sessions are restored.
        append_log("Spatial OFF requested; stopping audio engine");
        stop_worker();
        restore_dry_audio();
    }
    update_tray_tip(hwnd);
}

fn restart_worker(hwnd: HWND) {
    let enabled = state()
        .lock()
        .expect("Spatial supervisor state poisoned")
        .enabled;
    if !enabled {
        update_tray_tip(hwnd);
        return;
    }

    {
        let mut app = state().lock().expect("Spatial supervisor state poisoned");
        app.next_restart = None;
    }
    stop_worker();
    restore_dry_audio();
    if let Err(err) = spawn_worker() {
        restore_dry_audio();
        schedule_restart(&format!("audio engine restart failed: {err:#}"));
    }
    update_tray_tip(hwnd);
}

fn poll_worker(hwnd: HWND) {
    let mut retry = false;
    let mut exited: Option<String> = None;
    {
        let now = Instant::now();
        let mut app = state().lock().expect("Spatial supervisor state poisoned");
        if let Some(child) = app.child.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    exited = Some(format!("audio engine exited: {status}"));
                    app.child = None;
                    app.stdin = None;
                    if app.enabled && !app.quitting {
                        app.next_restart = Some(now + RESTART_DELAY);
                        app.restart_count = app.restart_count.saturating_add(1);
                    } else {
                        app.next_restart = None;
                    }
                }
                Ok(None) => {}
                Err(err) => {
                    exited = Some(format!("audio engine status failed: {err}"));
                    app.child = None;
                    app.stdin = None;
                    if app.enabled && !app.quitting {
                        app.next_restart = Some(now + RESTART_DELAY);
                        app.restart_count = app.restart_count.saturating_add(1);
                    } else {
                        app.next_restart = None;
                    }
                }
            }
        } else if app.enabled
            && !app.quitting
            && app
                .next_restart
                .map(|deadline| now >= deadline)
                .unwrap_or(false)
        {
            app.next_restart = None;
            retry = true;
        }
    }

    if let Some(detail) = exited {
        append_log(&detail);
        // Do not leave the source muted during the recovery delay.
        restore_dry_audio();
    }
    if retry {
        if let Err(err) = spawn_worker() {
            restore_dry_audio();
            schedule_restart(&format!("automatic audio recovery failed: {err:#}"));
        }
    }

    refresh_dry_audio(hwnd);
    update_tray_tip(hwnd);
}

fn tray_status() -> String {
    let app = state().lock().expect("Spatial supervisor state poisoned");
    if app.quitting {
        "Spatial - stopping".to_string()
    } else if !app.enabled {
        "Spatial - OFF".to_string()
    } else if app.child.is_some() {
        "Spatial - ON - Current model".to_string()
    } else {
        format!("Spatial - recovering audio ({})", app.restart_count)
    }
}

fn add_tray_icon(hwnd: HWND) {
    let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = TRAY_ID;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = WM_TRAY;
    data.hIcon = unsafe { LoadIconW(std::ptr::null_mut(), IDI_APPLICATION) };
    copy_wide(&mut data.szTip, &tray_status());
    unsafe {
        let _ = Shell_NotifyIconW(NIM_ADD, &data);
    }
}

fn update_tray_tip(hwnd: HWND) {
    let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = TRAY_ID;
    data.uFlags = NIF_TIP;
    copy_wide(&mut data.szTip, &tray_status());
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
    }
}

fn remove_tray_icon(hwnd: HWND) {
    let mut data: NOTIFYICONDATAW = unsafe { std::mem::zeroed() };
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = TRAY_ID;
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &data);
    }
}

fn append_menu_item(menu: *mut core::ffi::c_void, flags: u32, id: usize, text: &str) {
    let text = wide(text);
    unsafe {
        let _ = AppendMenuW(menu, flags, id, text.as_ptr());
    }
}

fn show_tray_menu(hwnd: HWND) {
    let menu = unsafe { CreatePopupMenu() };
    if menu.is_null() {
        return;
    }

    let (running, enabled, restarts) = {
        let app = state().lock().expect("Spatial supervisor state poisoned");
        (app.child.is_some(), app.enabled, app.restart_count)
    };
    let status = if !enabled {
        "Spatial: OFF".to_string()
    } else if running {
        "Spatial: ON | Current model".to_string()
    } else {
        format!("Spatial: recovering ({restarts})")
    };

    append_menu_item(menu, MF_STRING | MF_GRAYED, ID_STATUS, &status);
    append_menu_item(
        menu,
        MF_STRING,
        ID_TOGGLE,
        if enabled {
            "Turn Spatial off"
        } else {
            "Turn Spatial on"
        },
    );
    append_menu_item(
        menu,
        MF_STRING | if enabled { 0 } else { MF_GRAYED },
        ID_RESTART,
        "Restart audio engine",
    );
    append_menu_item(
        menu,
        MF_STRING | if autostart_preferred() { MF_CHECKED } else { 0 },
        ID_AUTOSTART,
        "Start with Windows",
    );
    unsafe {
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
    }
    append_menu_item(menu, MF_STRING, ID_EXIT, "Exit Spatial");

    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        let _ = GetCursorPos(&mut point);
        let _ = SetForegroundWindow(hwnd);
        let _ = TrackPopupMenu(
            menu,
            TPM_LEFTALIGN | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        let _ = PostMessageW(hwnd, WM_NULL, 0, 0);
        let _ = DestroyMenu(menu);
    }
}

fn shutdown(hwnd: HWND) {
    {
        let mut app = state().lock().expect("Spatial supervisor state poisoned");
        app.quitting = true;
        app.enabled = false;
        app.next_restart = None;
    }
    stop_worker();
    restore_dry_audio();
    remove_tray_icon(hwnd);
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == taskbar_created_message() {
        add_tray_icon(hwnd);
        update_tray_tip(hwnd);
        return 0;
    }

    match message {
        WM_CREATE => {
            add_tray_icon(hwnd);
            unsafe {
                SetTimer(hwnd, TIMER_ID, 250, None);
            }
            if let Err(err) = spawn_worker() {
                restore_dry_audio();
                schedule_restart(&format!("initial audio engine start failed: {err:#}"));
                update_tray_tip(hwnd);
            }
            0
        }
        WM_TRAY => {
            let event = lparam as u32;
            if event == WM_RBUTTONUP || event == WM_LBUTTONUP {
                show_tray_menu(hwnd);
            }
            0
        }
        WM_COMMAND => {
            let command_id = wparam & 0xffff;
            match command_id {
                ID_TOGGLE => {
                    let enabled = state()
                        .lock()
                        .expect("Spatial supervisor state poisoned")
                        .enabled;
                    set_enabled(hwnd, !enabled);
                }
                ID_RESTART => restart_worker(hwnd),
                ID_AUTOSTART => {
                    let desired = !autostart_preferred();
                    if let Err(err) = set_autostart(desired) {
                        append_log(&format!("could not change autostart preference: {err:#}"));
                    }
                }
                ID_EXIT => {
                    shutdown(hwnd);
                    unsafe {
                        DestroyWindow(hwnd);
                    }
                }
                _ => {}
            }
            0
        }
        WM_TIMER => {
            if wparam == TIMER_ID {
                poll_worker(hwnd);
            }
            0
        }
        WM_CLOSE => {
            shutdown(hwnd);
            unsafe {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            // Defensive idempotent cleanup for any destruction path that did
            // not arrive through our normal Exit/WM_CLOSE handlers.
            stop_worker();
            restore_dry_audio();
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

pub fn run() -> anyhow::Result<()> {
    wasapi::initialize_mta()
        .ok()
        .context("failed to initialize COM MTA for Spatial supervisor")?;

    let Some(_instance_guard) = claim_single_instance()? else {
        return Ok(());
    };

    // A force-killed previous private build can leave source sessions muted.
    // Recover those first, before autostart or a new engine is allowed to run.
    restore_stale_dry_audio();

    if std::env::args().any(|arg| arg == "--restore-dry-audio") {
        append_log("manual dry-audio restore completed");
        return Ok(());
    }

    // If the supervisor itself disappears, closing this job handle makes
    // Windows terminate the renderer child. That prevents a ghost audio engine
    // from surviving after the tray/application process is gone.
    let _child_job_guard = install_child_job();
    if _child_job_guard.is_none() {
        append_log("warning: kill-on-close child job unavailable");
    }

    ensure_autostart();
    let _ = taskbar_created_message();

    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    if instance.is_null() {
        bail!("GetModuleHandleW failed");
    }

    let class_name = wide("SpatialAudioSupervisor");
    let title = wide("Spatial");
    let mut class: WNDCLASSW = unsafe { std::mem::zeroed() };
    class.lpfnWndProc = Some(window_proc);
    class.hInstance = instance;
    class.hCursor = unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) };
    class.lpszClassName = class_name.as_ptr();

    if unsafe { RegisterClassW(&class) } == 0 {
        bail!("RegisterClassW failed");
    }

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        restore_dry_audio();
        bail!("CreateWindowExW failed");
    }

    let mut message: MSG = unsafe { std::mem::zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) };
        if result == -1 {
            stop_worker();
            restore_dry_audio();
            bail!("GetMessageW failed");
        }
        if result == 0 {
            break;
        }
        unsafe {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }

    stop_worker();
    restore_dry_audio();
    Ok(())
}

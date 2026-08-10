#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
use anyhow::{Context, bail};
#[cfg(target_os = "windows")]
use std::ffi::OsStr;
#[cfg(target_os = "windows")]
use std::fs::OpenOptions;
#[cfg(target_os = "windows")]
use std::io::Write;
#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;
#[cfg(target_os = "windows")]
use std::process::{Child, ChildStdin, Command, Stdio};
#[cfg(target_os = "windows")]
use std::sync::{Mutex, OnceLock};
#[cfg(target_os = "windows")]
use std::time::Duration;
#[cfg(target_os = "windows")]
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;
#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::WindowsAndMessaging::{
    BS_PUSHBUTTON, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, CreateWindowExW, DefWindowProcW,
    DestroyWindow, DispatchMessageW, GetDlgItem, GetMessageW, IDC_ARROW, LoadCursorW,
    MB_ICONERROR, MB_OK, MSG, MessageBoxW, PostQuitMessage, RegisterClassW, SW_SHOW, SetTimer,
    SetWindowTextW, ShowWindow, TranslateMessage, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY,
    WM_TIMER, WNDCLASSW, WS_CAPTION, WS_CHILD, WS_MINIMIZEBOX, WS_OVERLAPPED, WS_SYSMENU,
    WS_TABSTOP, WS_VISIBLE,
};

#[cfg(target_os = "windows")]
const ID_TOGGLE: i32 = 1001;
#[cfg(target_os = "windows")]
const ID_QUIT: i32 = 1002;
#[cfg(target_os = "windows")]
const ID_STATUS: i32 = 1003;
#[cfg(target_os = "windows")]
const ID_ROUTE: i32 = 1004;
#[cfg(target_os = "windows")]
const TIMER_ID: usize = 1;

#[cfg(target_os = "windows")]
#[derive(Default)]
struct AppState {
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    enabled: bool,
}

#[cfg(target_os = "windows")]
static STATE: OnceLock<Mutex<AppState>> = OnceLock::new();

fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        return run_windows_app();
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("Omniphony.exe is only available on Windows");
    }
}

#[cfg(target_os = "windows")]
fn wide(text: &str) -> Vec<u16> {
    OsStr::new(text).encode_wide().chain(Some(0)).collect()
}

#[cfg(target_os = "windows")]
fn state() -> &'static Mutex<AppState> {
    STATE.get_or_init(|| Mutex::new(AppState::default()))
}

#[cfg(target_os = "windows")]
fn set_control_text(hwnd: HWND, id: i32, text: &str) {
    let text = wide(text);
    unsafe {
        let control = GetDlgItem(hwnd, id);
        if !control.is_null() {
            SetWindowTextW(control, text.as_ptr());
        }
    }
}

#[cfg(target_os = "windows")]
fn set_running_ui(hwnd: HWND, enabled: bool) {
    set_control_text(hwnd, ID_TOGGLE, if enabled { "ON" } else { "OFF" });
    set_control_text(
        hwnd,
        ID_STATUS,
        if enabled {
            "Audio engine running - Omniphony enabled"
        } else {
            "Audio engine running - clean bypass comparison"
        },
    );
}

#[cfg(target_os = "windows")]
fn set_failed_ui(hwnd: HWND, detail: &str) {
    set_control_text(hwnd, ID_TOGGLE, "RESTART");
    set_control_text(hwnd, ID_STATUS, detail);
}

#[cfg(target_os = "windows")]
fn show_start_error(hwnd: HWND, err: &anyhow::Error) {
    set_failed_ui(hwnd, "Audio engine failed to start - see omniphony.log");
    let body = wide(&format!("Could not start the Omniphony audio engine.\n\n{err:#}"));
    let title = wide("Omniphony for Headphones");
    unsafe {
        MessageBoxW(hwnd, body.as_ptr(), title.as_ptr(), MB_OK | MB_ICONERROR);
    }
}

#[cfg(target_os = "windows")]
fn spawn_worker(hwnd: HWND, enabled: bool) -> anyhow::Result<()> {
    let exe = std::env::current_exe().context("failed to resolve Omniphony.exe path")?;
    let root = exe.parent().context("Omniphony.exe has no parent directory")?;
    let worker = root.join("omniphony_worker.exe");
    if !worker.is_file() {
        bail!("missing audio worker: {}", worker.display());
    }

    let log_path = root.join("omniphony.log");
    let log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("failed to open {}", log_path.display()))?;
    let log_err = log.try_clone().context("failed to clone Omniphony log handle")?;

    let mut command = Command::new(&worker);
    command
        .current_dir(root)
        .stdin(Stdio::piped())
        .stdout(Stdio::from(log))
        .stderr(Stdio::from(log_err))
        .creation_flags(CREATE_NO_WINDOW);
    if !enabled {
        command.arg("--start-off");
    }

    let mut child = command
        .spawn()
        .with_context(|| format!("failed to launch {}", worker.display()))?;
    let stdin = child.stdin.take().context("audio worker stdin was not piped")?;

    {
        let mut app = state().lock().expect("Omniphony app state poisoned");
        app.child = Some(child);
        app.stdin = Some(stdin);
        app.enabled = enabled;
    }

    set_running_ui(hwnd, enabled);
    Ok(())
}

#[cfg(target_os = "windows")]
fn stop_worker() {
    let (mut child, mut stdin) = {
        let mut app = state().lock().expect("Omniphony app state poisoned");
        (app.child.take(), app.stdin.take())
    };

    if let Some(stdin) = stdin.as_mut() {
        let _ = stdin.write_all(b"q\n");
        let _ = stdin.flush();
    }

    if let Some(child) = child.as_mut() {
        for _ in 0..10 {
            match child.try_wait() {
                Ok(Some(_)) => return,
                Ok(None) => std::thread::sleep(Duration::from_millis(20)),
                Err(_) => break,
            }
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(target_os = "windows")]
fn toggle_or_restart(hwnd: HWND) {
    let (running, enabled) = {
        let app = state().lock().expect("Omniphony app state poisoned");
        (app.child.is_some(), app.enabled)
    };

    if !running {
        if let Err(err) = spawn_worker(hwnd, true) {
            show_start_error(hwnd, &err);
        }
        return;
    }

    let next = !enabled;
    set_control_text(
        hwnd,
        ID_STATUS,
        if next {
            "Restarting clean route - enabling Omniphony..."
        } else {
            "Restarting clean route - bypassing Omniphony..."
        },
    );

    // Prototype safety rule: destroy the old output/capture queues completely
    // before changing wet/dry selection. This intentionally permits a short gap
    // so no already-queued wet block can leak after OFF. A later implementation
    // can replace this with sample-aligned paired wet/dry buffers at output time.
    stop_worker();

    if let Err(err) = spawn_worker(hwnd, next) {
        show_start_error(hwnd, &err);
    }
}

#[cfg(target_os = "windows")]
fn poll_worker(hwnd: HWND) {
    let exit_code = {
        let mut app = state().lock().expect("Omniphony app state poisoned");
        let Some(child) = app.child.as_mut() else {
            return;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                let code = status.code();
                app.child = None;
                app.stdin = None;
                app.enabled = false;
                Some(code)
            }
            Ok(None) => None,
            Err(_) => {
                app.child = None;
                app.stdin = None;
                app.enabled = false;
                Some(None)
            }
        }
    };

    if let Some(code) = exit_code {
        let detail = match code {
            Some(0) => "Audio engine stopped - click RESTART".to_string(),
            Some(code) => format!("Audio engine failed (code {code}) - see omniphony.log"),
            None => "Audio engine status failed - see omniphony.log".to_string(),
        };
        set_failed_ui(hwnd, &detail);
    }
}

#[cfg(target_os = "windows")]
fn shutdown_worker() {
    stop_worker();
}

#[cfg(target_os = "windows")]
unsafe fn create_control(
    parent: HWND,
    class: &str,
    text: &str,
    style: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    id: i32,
) -> HWND {
    let class = wide(class);
    let text = wide(text);
    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    unsafe {
        CreateWindowExW(
            0,
            class.as_ptr(),
            text.as_ptr(),
            style,
            x,
            y,
            width,
            height,
            parent,
            id as usize as _,
            instance,
            std::ptr::null(),
        )
    }
}

#[cfg(target_os = "windows")]
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_CREATE => {
            unsafe {
                create_control(
                    hwnd,
                    "STATIC",
                    "Omniphony for Headphones",
                    WS_CHILD | WS_VISIBLE,
                    30,
                    24,
                    360,
                    28,
                    0,
                );
                create_control(
                    hwnd,
                    "STATIC",
                    "Starting audio engine...",
                    WS_CHILD | WS_VISIBLE,
                    30,
                    64,
                    360,
                    24,
                    ID_STATUS,
                );
                create_control(
                    hwnd,
                    "BUTTON",
                    "ON",
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
                    125,
                    102,
                    170,
                    58,
                    ID_TOGGLE,
                );
                create_control(
                    hwnd,
                    "STATIC",
                    "Automatic route - physical FiiO preferred",
                    WS_CHILD | WS_VISIBLE,
                    30,
                    174,
                    360,
                    22,
                    ID_ROUTE,
                );
                create_control(
                    hwnd,
                    "BUTTON",
                    "Quit",
                    WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_PUSHBUTTON as u32,
                    160,
                    208,
                    100,
                    30,
                    ID_QUIT,
                );
                SetTimer(hwnd, TIMER_ID, 500, None);
            }

            if let Err(err) = spawn_worker(hwnd, true) {
                show_start_error(hwnd, &err);
            }
            0
        }
        WM_COMMAND => {
            let id = (wparam & 0xffff) as i32;
            match id {
                ID_TOGGLE => toggle_or_restart(hwnd),
                ID_QUIT => {
                    shutdown_worker();
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
            shutdown_worker();
            unsafe {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
    }
}

#[cfg(target_os = "windows")]
fn run_windows_app() -> anyhow::Result<()> {
    state();

    let instance = unsafe { GetModuleHandleW(std::ptr::null()) };
    if instance.is_null() {
        bail!("GetModuleHandleW failed");
    }

    let class_name = wide("OmniphonyForHeadphonesWindow");
    let title = wide("Omniphony for Headphones");

    let mut class: WNDCLASSW = unsafe { std::mem::zeroed() };
    class.style = CS_HREDRAW | CS_VREDRAW;
    class.lpfnWndProc = Some(window_proc);
    class.hInstance = instance;
    class.hCursor = unsafe { LoadCursorW(std::ptr::null_mut(), IDC_ARROW) };
    class.hbrBackground = std::ptr::null_mut();
    class.lpszClassName = class_name.as_ptr();

    if unsafe { RegisterClassW(&class) } == 0 {
        bail!("RegisterClassW failed");
    }

    let hwnd = unsafe {
        CreateWindowExW(
            0,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            440,
            300,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            instance,
            std::ptr::null(),
        )
    };
    if hwnd.is_null() {
        bail!("CreateWindowExW failed");
    }

    unsafe {
        ShowWindow(hwnd, SW_SHOW);
    }

    let mut message: MSG = unsafe { std::mem::zeroed() };
    loop {
        let result = unsafe { GetMessageW(&mut message, std::ptr::null_mut(), 0, 0) };
        if result == -1 {
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

    Ok(())
}

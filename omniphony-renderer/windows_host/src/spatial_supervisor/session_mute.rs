use anyhow::{Context, Result};
use std::collections::HashSet;
use std::fs::{create_dir_all, read_to_string};
use std::path::PathBuf;
use windows::Win32::Media::Audio::{
    eRender, IAudioSessionControl, IAudioSessionControl2, IAudioSessionManager2,
    IMMDeviceEnumerator, ISimpleAudioVolume, MMDeviceEnumerator, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL};
use windows::core::{GUID, Interface, PWSTR};

// Unique only to Spatial's temporary private session-silencing layer. It lets
// other session observers distinguish our mute changes from user changes.
const EVENT_CONTEXT: GUID = GUID::from_u128(0xd6d28f74_3c41_4b57_88a4_e5850ce34011);

/// Temporary interception bridge for the process-loopback host.
///
/// Windows loopback is pre-volume/pre-mute by default. While the Spatial child
/// is actively rendering, this object mutes the *dry* render sessions after the
/// loopback tap. The child process itself is excluded so its binaural K7 output
/// remains audible. This belongs to the private Windows host only and disappears
/// once the real Spatial endpoint/APO owns routing.
///
/// We remember only sessions that were unmuted and that Spatial changed. OFF,
/// Exit, engine failure, and the next launch all try to restore those sessions.
pub struct DrySessionSilencer {
    changed_sessions: HashSet<String>,
    snapshot_path: PathBuf,
}

impl DrySessionSilencer {
    pub fn new(snapshot_path: PathBuf) -> Self {
        Self {
            changed_sessions: HashSet::new(),
            snapshot_path,
        }
    }

    /// Recover a mute snapshot left by an unclean Spatial termination.
    pub fn restore_stale_snapshot(&mut self) -> Result<usize> {
        self.load_snapshot();
        self.restore()
    }

    /// Mute every external shared-mode render session currently present.
    /// Sessions created later are picked up by repeated calls from the tray
    /// watchdog. `skip_pids` must contain both supervisor and audio-child PIDs.
    ///
    /// Per-session failures are deliberately non-fatal. A session that cannot
    /// be inspected or muted is left alone, while every successful change is
    /// still persisted before this call returns.
    pub fn silence_external_sessions(&mut self, skip_pids: &[u32]) -> Result<usize> {
        let sessions = enumerate_render_sessions()?;
        let mut changed = 0usize;

        for session in sessions {
            if session.pid == 0 || skip_pids.contains(&session.pid) {
                continue;
            }

            if self.changed_sessions.contains(&session.instance_id) {
                // A mixer UI or source app may have unmuted itself. Spatial is
                // authoritative while ON, so best-effort reassert our own mute.
                let _ = unsafe { session.volume.SetMute(true, &EVENT_CONTEXT) };
                continue;
            }

            let Ok(already_muted) = (unsafe { session.volume.GetMute() }) else {
                continue;
            };
            if already_muted.as_bool() {
                // User/application state, not ours. Never claim ownership of it.
                continue;
            }

            if unsafe { session.volume.SetMute(true, &EVENT_CONTEXT) }.is_err() {
                continue;
            }
            self.changed_sessions.insert(session.instance_id);
            changed += 1;
        }

        // Persist after the complete pass even if one individual session did
        // not cooperate, so a later recovery can always undo successful mutes.
        self.persist_snapshot()?;
        Ok(changed)
    }

    /// Restore only sessions that Spatial itself changed from unmuted to muted.
    /// Failed restorations remain in the snapshot for a later retry.
    pub fn restore(&mut self) -> Result<usize> {
        if self.changed_sessions.is_empty() {
            let _ = std::fs::remove_file(&self.snapshot_path);
            return Ok(0);
        }

        let sessions = enumerate_render_sessions()?;
        let mut live_owned = HashSet::new();
        let mut restored_ids = Vec::new();
        let mut restored = 0usize;

        for session in sessions {
            if !self.changed_sessions.contains(&session.instance_id) {
                continue;
            }
            live_owned.insert(session.instance_id.clone());

            if unsafe { session.volume.SetMute(false, &EVENT_CONTEXT) }.is_ok() {
                restored_ids.push(session.instance_id);
                restored += 1;
            }
        }

        for id in restored_ids {
            self.changed_sessions.remove(&id);
        }

        // A session instance that no longer exists cannot remain muted. Drop
        // dead instance IDs, but retain live IDs whose restore failed so the
        // snapshot can recover them on the next timer tick or launch.
        self.changed_sessions.retain(|id| live_owned.contains(id));
        self.persist_snapshot()?;
        Ok(restored)
    }

    fn load_snapshot(&mut self) {
        let Ok(text) = read_to_string(&self.snapshot_path) else {
            return;
        };
        self.changed_sessions.extend(
            text.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned),
        );
    }

    fn persist_snapshot(&self) -> Result<()> {
        if self.changed_sessions.is_empty() {
            let _ = std::fs::remove_file(&self.snapshot_path);
            return Ok(());
        }
        if let Some(parent) = self.snapshot_path.parent() {
            create_dir_all(parent).context("failed to create Spatial settings directory")?;
        }
        let mut ids: Vec<_> = self.changed_sessions.iter().cloned().collect();
        ids.sort();
        std::fs::write(&self.snapshot_path, ids.join("\n") + "\n")
            .context("failed to persist Spatial dry-session recovery snapshot")?;
        Ok(())
    }
}

struct RenderSession {
    instance_id: String,
    pid: u32,
    volume: ISimpleAudioVolume,
}

fn enumerate_render_sessions() -> Result<Vec<RenderSession>> {
    let device_enumerator: IMMDeviceEnumerator = unsafe {
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
            .context("failed to create Windows audio device enumerator")?
    };
    let devices = unsafe { device_enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE) }
        .context("failed to enumerate active Windows render endpoints")?;
    let device_count = unsafe { devices.GetCount() }
        .context("failed to count Windows render endpoints")?;

    let mut result = Vec::new();
    for device_index in 0..device_count {
        let Ok(device) = (unsafe { devices.Item(device_index) }) else {
            continue;
        };
        let Ok(manager) = (unsafe { device.Activate::<IAudioSessionManager2>(CLSCTX_ALL, None) }) else {
            continue;
        };
        let Ok(enumerator) = (unsafe { manager.GetSessionEnumerator() }) else {
            continue;
        };
        let Ok(count) = (unsafe { enumerator.GetCount() }) else {
            continue;
        };

        for session_index in 0..count {
            let Ok(control): Result<IAudioSessionControl, _> =
                (unsafe { enumerator.GetSession(session_index) })
            else {
                continue;
            };
            let Ok(control2): Result<IAudioSessionControl2, _> = control.cast() else {
                continue;
            };
            let Ok(volume): Result<ISimpleAudioVolume, _> = control2.cast() else {
                continue;
            };
            let Ok(pid) = (unsafe { control2.GetProcessId() }) else {
                continue;
            };
            let Ok(instance_ptr) = (unsafe { control2.GetSessionInstanceIdentifier() }) else {
                continue;
            };
            let Ok(instance_id) = take_pwstr(instance_ptr) else {
                continue;
            };
            if instance_id.is_empty() {
                continue;
            }

            result.push(RenderSession {
                instance_id,
                pid,
                volume,
            });
        }
    }
    Ok(result)
}

fn take_pwstr(value: PWSTR) -> Result<String> {
    let converted = unsafe { value.to_string() };
    unsafe {
        CoTaskMemFree(Some(value.0.cast()));
    }
    converted.context("invalid Windows audio session identifier")
}

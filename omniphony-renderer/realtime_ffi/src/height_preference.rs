//! Windows listening preference for a little more vertical extent in Current.
//!
//! Height+ does not create another wet copy or change the protected stereo
//! master. It moves a bounded fraction of already-derived horizontal support
//! into the matching top lanes before the 22-direction HRTF renderer. The
//! baseline is unchanged when the setting is absent/off.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const FIELD_CHANNELS: usize = 12;
const SETTING_FILE_NAME: &str = "height-plus.txt";
const SETTING_POLL_MS: u64 = 500;
const FRONT_TRANSFER: f32 = 0.12;
const REAR_TRANSFER: f32 = 0.05;

pub(crate) struct HeightPreference {
    enabled: bool,
    setting_path: PathBuf,
    last_setting_check: Instant,
}

impl HeightPreference {
    pub(crate) fn new() -> Self {
        let root = env::var_os("ProgramData")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
        let setting_path = root.join("Omniphony").join(SETTING_FILE_NAME);
        let enabled = read_enabled(&setting_path);
        Self {
            enabled,
            setting_path,
            last_setting_check: Instant::now(),
        }
    }

    fn refresh(&mut self) {
        if self.last_setting_check.elapsed() < Duration::from_millis(SETTING_POLL_MS) {
            return;
        }
        self.last_setting_check = Instant::now();
        self.enabled = read_enabled(&self.setting_path);
    }

    pub(crate) fn apply(&mut self, field: &mut [f32]) {
        self.refresh();
        if !self.enabled || field.len() % FIELD_CHANNELS != 0 {
            return;
        }

        for frame in field.chunks_exact_mut(FIELD_CHANNELS) {
            transfer(&mut frame[0], &mut frame[8], FRONT_TRANSFER);
            transfer(&mut frame[1], &mut frame[9], FRONT_TRANSFER);
            transfer(&mut frame[6], &mut frame[10], REAR_TRANSFER);
            transfer(&mut frame[7], &mut frame[11], REAR_TRANSFER);
        }
    }
}

fn transfer(horizontal: &mut f32, elevated: &mut f32, fraction: f32) {
    let amount = *horizontal * fraction.clamp(0.0, 0.25);
    *horizontal -= amount;
    *elevated += amount;
}

fn read_enabled(path: &PathBuf) -> bool {
    fs::read_to_string(path)
        .map(|text| matches!(text.trim().to_ascii_lowercase().as_str(), "1" | "on" | "true" | "enabled"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transfer_preserves_sample_sum() {
        let mut horizontal = 0.75_f32;
        let mut elevated = 0.20_f32;
        let before = horizontal + elevated;
        transfer(&mut horizontal, &mut elevated, FRONT_TRANSFER);
        assert!((horizontal + elevated - before).abs() < 1.0e-6);
        assert!(horizontal < 0.75);
        assert!(elevated > 0.20);
    }

    #[test]
    fn height_plus_is_front_weighted() {
        assert!(FRONT_TRANSFER > REAR_TRANSFER);
        assert!(FRONT_TRANSFER <= 0.15);
        assert!(REAR_TRANSFER <= 0.08);
    }
}

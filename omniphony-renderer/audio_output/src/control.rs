use crate::AdaptiveResamplingConfig;
use std::sync::Arc;
use std::sync::{
    Mutex,
    atomic::{AtomicBool, AtomicU32, Ordering},
};

/// Default audio-meter update rate. Polled by `AudioMeter::poll()` each call
/// so it can be updated live via OSC without restarting the renderer.
pub const DEFAULT_METER_RATE_HZ: f32 = 50.0;

/// Default diag-publication rate. Independent of the audio-meter rate so the
/// diag plot can be sampled faster (or slower) than the level meters.
pub const DEFAULT_DIAG_RATE_HZ: f32 = 50.0;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct OutputDeviceOption {
    pub value: String,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct RequestedAudioOutputConfig {
    pub output_device: Option<String>,
    pub output_sample_rate_hz: Option<u32>,
    pub latency_target_ms: Option<u32>,
    pub adaptive_enabled: bool,
    pub adaptive: AdaptiveResamplingConfig,
}

impl Default for RequestedAudioOutputConfig {
    fn default() -> Self {
        Self {
            output_device: None,
            output_sample_rate_hz: None,
            latency_target_ms: None,
            adaptive_enabled: false,
            adaptive: AdaptiveResamplingConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AppliedAudioOutputState {
    pub output_device: Option<String>,
    pub output_sample_rate_hz: Option<u32>,
    pub sample_format: String,
    pub audio_error: Option<String>,
}

pub struct AudioControl {
    requested: Mutex<RequestedAudioOutputConfig>,
    applied: Mutex<AppliedAudioOutputState>,
    available_output_devices: Mutex<Vec<OutputDeviceOption>>,
    device_list_fetcher: Mutex<Option<Box<dyn Fn() -> Vec<OutputDeviceOption> + Send + Sync>>>,
    reset_ratio_pending: AtomicBool,
    /// Audio-meter update rate in Hz, stored as `f32::to_bits`. Shared with
    /// `AudioMeter`, which re-reads it each `poll()` so changes take effect
    /// immediately without a restart.
    meter_rate_hz_bits: Arc<AtomicU32>,
    /// Diag-publication rate in Hz, stored as `f32::to_bits`. Independent of
    /// `meter_rate_hz_bits` so diag (output_*, latency_*, bridge_*) and audio
    /// levels can be sampled at separate cadences.
    diag_publish_rate_hz_bits: Arc<AtomicU32>,
}

impl Default for AudioControl {
    fn default() -> Self {
        Self::new(RequestedAudioOutputConfig::default())
    }
}

impl AudioControl {
    pub fn new(requested: RequestedAudioOutputConfig) -> Self {
        Self {
            requested: Mutex::new(requested),
            applied: Mutex::new(AppliedAudioOutputState::default()),
            available_output_devices: Mutex::new(Vec::new()),
            device_list_fetcher: Mutex::new(None),
            reset_ratio_pending: AtomicBool::new(false),
            meter_rate_hz_bits: Arc::new(AtomicU32::new(DEFAULT_METER_RATE_HZ.to_bits())),
            diag_publish_rate_hz_bits: Arc::new(AtomicU32::new(
                DEFAULT_DIAG_RATE_HZ.to_bits(),
            )),
        }
    }

    /// Shared atomic holding the audio-meter update rate (Hz, encoded as
    /// `f32::to_bits`). `AudioMeter` clones this and reads it each `poll()`.
    pub fn meter_rate_atomic(&self) -> Arc<AtomicU32> {
        Arc::clone(&self.meter_rate_hz_bits)
    }

    pub fn meter_rate_hz(&self) -> f32 {
        f32::from_bits(self.meter_rate_hz_bits.load(Ordering::Relaxed))
    }

    /// Update the audio-meter rate. Clamped to a sane range so a typo on the
    /// OSC side can't lock the writer thread (rate 0 would never tick) or
    /// flood the network (rate 10_000 would).
    pub fn set_meter_rate_hz(&self, hz: f32) {
        let clamped = hz.clamp(1.0, 1000.0);
        self.meter_rate_hz_bits
            .store(clamped.to_bits(), Ordering::Relaxed);
    }

    /// Shared atomic holding the diag-publication rate (Hz, encoded as
    /// `f32::to_bits`). The decoder-side diag publisher clones this and
    /// recomputes its interval whenever the value changes.
    pub fn diag_publish_rate_atomic(&self) -> Arc<AtomicU32> {
        Arc::clone(&self.diag_publish_rate_hz_bits)
    }

    pub fn diag_publish_rate_hz(&self) -> f32 {
        f32::from_bits(self.diag_publish_rate_hz_bits.load(Ordering::Relaxed))
    }

    /// Update the diag-publication rate, with the same `[1, 1000]` clamp
    /// as `set_meter_rate_hz` for the same reasons.
    pub fn set_diag_publish_rate_hz(&self, hz: f32) {
        let clamped = hz.clamp(1.0, 1000.0);
        self.diag_publish_rate_hz_bits
            .store(clamped.to_bits(), Ordering::Relaxed);
    }

    pub fn requested_snapshot(&self) -> RequestedAudioOutputConfig {
        self.requested.lock().unwrap().clone()
    }

    pub fn update_requested(&self, f: impl FnOnce(&mut RequestedAudioOutputConfig)) {
        let mut requested = self.requested.lock().unwrap();
        f(&mut requested);
    }

    pub fn applied_snapshot(&self) -> AppliedAudioOutputState {
        self.applied.lock().unwrap().clone()
    }

    pub fn update_applied(&self, f: impl FnOnce(&mut AppliedAudioOutputState)) {
        let mut applied = self.applied.lock().unwrap();
        f(&mut applied);
    }

    pub fn set_requested_output_device(&self, output_device: Option<String>) {
        self.update_requested(|requested| requested.output_device = output_device);
    }

    pub fn requested_output_device(&self) -> Option<String> {
        self.requested_snapshot().output_device
    }

    pub fn set_requested_output_sample_rate(&self, rate_hz: Option<u32>) {
        self.update_requested(|requested| requested.output_sample_rate_hz = rate_hz);
    }

    pub fn requested_output_sample_rate(&self) -> Option<u32> {
        self.requested_snapshot().output_sample_rate_hz
    }

    pub fn set_requested_latency_target_ms(&self, value: Option<u32>) {
        self.update_requested(|requested| requested.latency_target_ms = value);
    }

    pub fn requested_latency_target_ms(&self) -> Option<u32> {
        self.requested_snapshot().latency_target_ms
    }

    pub fn set_requested_adaptive_resampling(&self, enabled: bool) {
        self.update_requested(|requested| requested.adaptive_enabled = enabled);
    }

    pub fn requested_adaptive_resampling(&self) -> bool {
        self.requested_snapshot().adaptive_enabled
    }

    pub fn set_requested_adaptive_resampling_enable_far_mode(&self, enabled: bool) {
        self.update_requested(|requested| requested.adaptive.enable_far_mode = enabled);
    }

    pub fn requested_adaptive_resampling_enable_far_mode(&self) -> bool {
        self.requested_snapshot().adaptive.enable_far_mode
    }

    pub fn set_requested_adaptive_resampling_force_silence_in_far_mode(&self, enabled: bool) {
        self.update_requested(|requested| requested.adaptive.force_silence_in_far_mode = enabled);
    }

    pub fn requested_adaptive_resampling_force_silence_in_far_mode(&self) -> bool {
        self.requested_snapshot().adaptive.force_silence_in_far_mode
    }

    pub fn set_requested_adaptive_resampling_hard_recover_high_in_far_mode(&self, enabled: bool) {
        self.update_requested(|requested| {
            requested.adaptive.hard_recover_high_in_far_mode = enabled
        });
    }

    pub fn requested_adaptive_resampling_hard_recover_high_in_far_mode(&self) -> bool {
        self.requested_snapshot()
            .adaptive
            .hard_recover_high_in_far_mode
    }

    pub fn set_requested_adaptive_resampling_hard_recover_low_in_far_mode(&self, enabled: bool) {
        self.update_requested(|requested| {
            requested.adaptive.hard_recover_low_in_far_mode = enabled
        });
    }

    pub fn requested_adaptive_resampling_hard_recover_low_in_far_mode(&self) -> bool {
        self.requested_snapshot()
            .adaptive
            .hard_recover_low_in_far_mode
    }

    pub fn set_requested_adaptive_resampling_far_mode_return_fade_in_ms(&self, value: u32) {
        self.update_requested(|requested| requested.adaptive.far_mode_return_fade_in_ms = value);
    }

    pub fn requested_adaptive_resampling_far_mode_return_fade_in_ms(&self) -> u32 {
        self.requested_snapshot()
            .adaptive
            .far_mode_return_fade_in_ms
    }

    pub fn set_requested_adaptive_resampling_kp_near(&self, value: f32) {
        self.update_requested(|requested| requested.adaptive.kp_near = value as f64);
    }

    pub fn requested_adaptive_resampling_kp_near(&self) -> f64 {
        self.requested_snapshot().adaptive.kp_near
    }

    pub fn set_requested_adaptive_resampling_ki(&self, value: f32) {
        self.update_requested(|requested| requested.adaptive.ki = value as f64);
    }

    pub fn requested_adaptive_resampling_ki(&self) -> f64 {
        self.requested_snapshot().adaptive.ki
    }

    pub fn set_requested_adaptive_resampling_integral_discharge_ratio(&self, value: f32) {
        self.update_requested(|requested| {
            requested.adaptive.integral_discharge_ratio = value as f64;
        });
    }

    pub fn requested_adaptive_resampling_integral_discharge_ratio(&self) -> f64 {
        self.requested_snapshot().adaptive.integral_discharge_ratio
    }

    pub fn set_requested_adaptive_resampling_max_adjust(&self, value: f32) {
        self.update_requested(|requested| requested.adaptive.max_adjust = value as f64);
    }

    pub fn requested_adaptive_resampling_max_adjust(&self) -> f64 {
        self.requested_snapshot().adaptive.max_adjust
    }

    pub fn set_requested_adaptive_resampling_update_interval_callbacks(&self, value: u32) {
        self.update_requested(|requested| requested.adaptive.update_interval_callbacks = value);
    }

    pub fn requested_adaptive_resampling_update_interval_callbacks(&self) -> u32 {
        self.requested_snapshot().adaptive.update_interval_callbacks
    }

    pub fn set_requested_adaptive_resampling_near_far_threshold_ms(&self, value: u32) {
        self.update_requested(|requested| requested.adaptive.near_far_threshold_ms = value);
    }

    pub fn requested_adaptive_resampling_near_far_threshold_ms(&self) -> u32 {
        self.requested_snapshot().adaptive.near_far_threshold_ms
    }

    pub fn set_requested_adaptive_resampling_low_recover_settle_stable_ms(&self, value: f32) {
        self.update_requested(|requested| {
            requested.adaptive.low_recover_settle_stable_ms = value;
        });
    }

    pub fn requested_adaptive_resampling_low_recover_settle_stable_ms(&self) -> f32 {
        self.requested_snapshot()
            .adaptive
            .low_recover_settle_stable_ms
    }

    pub fn set_requested_adaptive_resampling_low_recover_entry_margin_ms(&self, value: f32) {
        self.update_requested(|requested| {
            requested.adaptive.low_recover_entry_margin_ms = value;
        });
    }

    pub fn requested_adaptive_resampling_low_recover_entry_margin_ms(&self) -> f32 {
        self.requested_snapshot()
            .adaptive
            .low_recover_entry_margin_ms
    }

    pub fn set_requested_adaptive_resampling_low_recover_exit_margin_ms(&self, value: f32) {
        self.update_requested(|requested| {
            requested.adaptive.low_recover_exit_margin_ms = value;
        });
    }

    pub fn requested_adaptive_resampling_low_recover_exit_margin_ms(&self) -> f32 {
        self.requested_snapshot()
            .adaptive
            .low_recover_exit_margin_ms
    }

    pub fn set_requested_adaptive_resampling_low_recover_settle_margin_ms(&self, value: f32) {
        self.update_requested(|requested| {
            requested.adaptive.low_recover_settle_margin_ms = value;
        });
    }

    pub fn requested_adaptive_resampling_low_recover_settle_margin_ms(&self) -> f32 {
        self.requested_snapshot()
            .adaptive
            .low_recover_settle_margin_ms
    }

    pub fn set_requested_adaptive_resampling_low_recover_refill_delta_alpha(&self, value: f32) {
        self.update_requested(|requested| {
            requested.adaptive.low_recover_refill_delta_alpha = value;
        });
    }

    pub fn requested_adaptive_resampling_low_recover_refill_delta_alpha(&self) -> f32 {
        self.requested_snapshot()
            .adaptive
            .low_recover_refill_delta_alpha
    }

    pub fn set_requested_adaptive_resampling_control_smoothing_alpha(&self, value: f32) {
        self.update_requested(|requested| {
            requested.adaptive.control_smoothing_alpha = value as f64;
        });
    }

    pub fn requested_adaptive_resampling_control_smoothing_alpha(&self) -> f64 {
        self.requested_snapshot().adaptive.control_smoothing_alpha
    }

    pub fn set_requested_adaptive_resampling_paused(&self, paused: bool) {
        self.update_requested(|requested| requested.adaptive.paused = paused);
    }

    pub fn requested_adaptive_resampling_paused(&self) -> bool {
        self.requested_snapshot().adaptive.paused
    }

    pub fn set_requested_adaptive_resampling_use_pre_bridge_clock(&self, enabled: bool) {
        self.update_requested(|requested| {
            requested.adaptive.use_pre_bridge_clock = enabled
        });
    }

    pub fn requested_adaptive_resampling_use_pre_bridge_clock(&self) -> bool {
        self.requested_snapshot().adaptive.use_pre_bridge_clock
    }

    /// Request a one-shot ratio reset. Consumed by the sync loop via `take_ratio_reset`.
    pub fn request_ratio_reset(&self) {
        self.reset_ratio_pending.store(true, Ordering::Relaxed);
    }

    /// Returns true and clears the pending flag if a reset was requested.
    pub fn take_ratio_reset(&self) -> bool {
        self.reset_ratio_pending
            .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
    }

    pub fn set_available_output_devices(&self, devices: Vec<OutputDeviceOption>) {
        *self.available_output_devices.lock().unwrap() = devices;
    }

    pub fn available_output_devices(&self) -> Vec<OutputDeviceOption> {
        self.available_output_devices.lock().unwrap().clone()
    }

    pub fn set_device_list_fetcher(
        &self,
        fetcher: impl Fn() -> Vec<OutputDeviceOption> + Send + Sync + 'static,
    ) {
        *self.device_list_fetcher.lock().unwrap() = Some(Box::new(fetcher));
    }

    pub fn refresh_available_output_devices(&self) -> Option<Vec<OutputDeviceOption>> {
        let fetcher = self.device_list_fetcher.lock().unwrap();
        fetcher.as_ref().map(|f| {
            let devices = f();
            *self.available_output_devices.lock().unwrap() = devices.clone();
            devices
        })
    }

    pub fn set_audio_state(&self, sample_rate_hz: u32, sample_format: impl Into<String>) {
        self.update_applied(|applied| {
            applied.output_sample_rate_hz = Some(sample_rate_hz);
            applied.sample_format = sample_format.into();
        });
    }

    pub fn set_effective_output_device(&self, output_device: Option<String>) {
        self.update_applied(|applied| applied.output_device = output_device);
    }

    pub fn set_audio_error(&self, error: Option<String>) {
        self.update_applied(|applied| applied.audio_error = error);
    }

    pub fn audio_state(&self) -> (Option<u32>, String) {
        let applied = self.applied_snapshot();
        (applied.output_sample_rate_hz, applied.sample_format)
    }

    pub fn audio_error(&self) -> Option<String> {
        self.applied_snapshot().audio_error
    }

    pub fn effective_output_device(&self) -> Option<String> {
        self.applied_snapshot().output_device
    }
}

use rosc::{OscMessage, OscType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::context::RuntimeControlContext;
use audio_input::{
    InputBackend, InputClockMode, InputLfeMode, InputMapMode, InputMode, InputSampleFormat,
};
use renderer::crossover::compute_bands;
use renderer::live_params::LiveEvaluationMode;
use renderer::render_backend::RenderBackendKind;
use renderer::render_backend::{CartesianSpeakerHeatmapSlices, CartesianSpeakerHeatmapVolume};

#[derive(Debug, Clone, Default)]
pub struct SpeakerPatch {
    pub az: Option<f32>,
    pub el: Option<f32>,
    pub distance: Option<f32>,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub z: Option<f32>,
    pub coord_mode: Option<String>,
    pub spatialize: Option<bool>,
    pub freq_low: Option<Option<f32>>,
    pub freq_high: Option<Option<f32>>,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum BroadcastValue {
    Int(i32),
    Float(f32),
    Fff(f32, f32, f32),
    String(String),
}

#[derive(Debug, Clone)]
pub struct BroadcastUpdate {
    pub addr: String,
    pub value: BroadcastValue,
}

#[derive(Debug, Clone, Default)]
pub struct ControlEffects {
    pub mark_dirty: bool,
    pub trigger_layout_recompute: bool,
    pub broadcasts: Vec<BroadcastUpdate>,
    pub log_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpeakerHeatmapRequest {
    request_id: u64,
    speaker_index: usize,
    #[serde(default)]
    band_index: usize,
    mode: String,
    max_samples: Option<usize>,
}

#[derive(Debug, Serialize)]
struct SpeakerHeatmapMetaPayload {
    request_id: u64,
    speaker_index: usize,
    band_index: usize,
    speaker_position: [f32; 3],
}

#[derive(Debug, Serialize)]
struct SpeakerHeatmapSlicePayload {
    request_id: u64,
    speaker_index: usize,
    band_index: usize,
    fixed_axis_value: f32,
    axis_a: Vec<f32>,
    axis_b: Vec<f32>,
    values: Vec<f32>,
}

#[derive(Debug, Serialize)]
struct SpeakerHeatmapUnavailablePayload {
    request_id: u64,
    speaker_index: usize,
    band_index: usize,
    reason: &'static str,
}

#[derive(Debug, Serialize)]
struct SpeakerHeatmapVolumeChunkPayload {
    request_id: u64,
    speaker_index: usize,
    band_index: usize,
    chunk_index: usize,
    chunk_count: usize,
    samples: Vec<f32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AdaptiveResamplingPatch {
    enabled: Option<bool>,
    enable_far_mode: Option<bool>,
    force_silence_in_far_mode: Option<bool>,
    hard_recover_high_in_far_mode: Option<bool>,
    hard_recover_low_in_far_mode: Option<bool>,
    far_mode_return_fade_in_ms: Option<u32>,
    kp_near: Option<f64>,
    ki: Option<f64>,
    integral_discharge_ratio: Option<f64>,
    max_adjust: Option<f64>,
    near_far_threshold_ms: Option<u32>,
    update_interval_callbacks: Option<u32>,
    paused: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct AudioConfigPatch {
    output_device: Option<Option<String>>,
    sample_rate: Option<Option<u32>>,
    latency_target_ms: Option<Option<u32>>,
    adaptive_resampling: Option<AdaptiveResamplingPatch>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LiveInputPatch {
    backend: Option<Option<InputBackend>>,
    node: Option<Option<String>>,
    description: Option<Option<String>>,
    layout: Option<Option<String>>,
    clock_mode: Option<InputClockMode>,
    channels: Option<Option<u16>>,
    sample_rate: Option<Option<u32>>,
    format: Option<Option<InputSampleFormat>>,
    map: Option<InputMapMode>,
    lfe_mode: Option<InputLfeMode>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct InputConfigPatch {
    mode: Option<InputMode>,
    live_input: Option<LiveInputPatch>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LayoutSpeakerPatch {
    id: usize,
    name: Option<String>,
    coord_mode: Option<String>,
    x: Option<f32>,
    y: Option<f32>,
    z: Option<f32>,
    azimuth: Option<f32>,
    elevation: Option<f32>,
    distance: Option<f32>,
    spatialize: Option<bool>,
    #[serde(default)]
    freq_low: Option<Option<f32>>,
    #[serde(default)]
    freq_high: Option<Option<f32>>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LayoutAddSpeakerPatch {
    name: Option<String>,
    coord_mode: Option<String>,
    x: Option<f32>,
    y: Option<f32>,
    z: Option<f32>,
    azimuth: Option<f32>,
    elevation: Option<f32>,
    distance: Option<f32>,
    spatialize: Option<bool>,
    delay_ms: Option<f32>,
    #[serde(default)]
    freq_low: Option<Option<f32>>,
    #[serde(default)]
    freq_high: Option<Option<f32>>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LayoutMoveSpeakerPatch {
    from: usize,
    to: usize,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LayoutConfigPatch {
    radius_m: Option<f32>,
    speaker_edits: Option<Vec<LayoutSpeakerPatch>>,
    add_speaker: Option<LayoutAddSpeakerPatch>,
    remove_speaker: Option<usize>,
    move_speaker: Option<LayoutMoveSpeakerPatch>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SpeakersRuntimePatch {
    id: usize,
    muted: Option<bool>,
    delay_ms: Option<f32>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SpeakersConfigPatch {
    speaker_edits: Option<Vec<SpeakersRuntimePatch>>,
}

fn build_constant_slices_from_reference(
    reference: CartesianSpeakerHeatmapSlices,
    value: f32,
) -> CartesianSpeakerHeatmapSlices {
    let xy_len = reference.x_positions.len() * reference.y_positions.len();
    let xz_len = reference.x_positions.len() * reference.z_positions.len();
    let yz_len = reference.y_positions.len() * reference.z_positions.len();
    CartesianSpeakerHeatmapSlices {
        speaker_index: reference.speaker_index,
        speaker_position: reference.speaker_position,
        x_positions: reference.x_positions,
        y_positions: reference.y_positions,
        z_positions: reference.z_positions,
        xy_values: vec![value; xy_len],
        xz_values: vec![value; xz_len],
        yz_values: vec![value; yz_len],
    }
}

fn build_constant_volume_samples(
    reference: &CartesianSpeakerHeatmapSlices,
    value: f32,
    max_samples: usize,
) -> Vec<f32> {
    if value <= 0.0 {
        return Vec::new();
    }
    let total =
        reference.x_positions.len() * reference.y_positions.len() * reference.z_positions.len();
    if total == 0 {
        return Vec::new();
    }
    let sample_count = if max_samples > 0 {
        total.min(max_samples)
    } else {
        total
    };
    let mut samples = Vec::with_capacity(sample_count * 4);
    for sample_index in 0..sample_count {
        let flat_index = if sample_count == total {
            sample_index
        } else {
            ((sample_index as f64 * total as f64) / sample_count as f64).floor() as usize
        };
        let x_len = reference.x_positions.len();
        let y_len = reference.y_positions.len();
        let xy_len = x_len * y_len;
        let z_index = flat_index / xy_len;
        let rem = flat_index % xy_len;
        let y_index = rem / x_len;
        let x_index = rem % x_len;
        samples.extend_from_slice(&[
            reference.x_positions[x_index],
            reference.y_positions[y_index],
            reference.z_positions[z_index],
            value,
        ]);
    }
    samples
}

fn parse_bool_arg(arg: Option<&OscType>) -> Option<bool> {
    match arg {
        Some(OscType::Int(i)) => Some(*i != 0),
        Some(OscType::Float(f)) => Some(*f != 0.0),
        _ => None,
    }
}

fn parse_positive_u32_arg(arg: Option<&OscType>) -> Option<u32> {
    match arg {
        Some(OscType::Int(i)) if *i > 0 => Some(*i as u32),
        Some(OscType::Float(f)) if *f > 0.0 => Some(*f as u32),
        _ => None,
    }
}

fn parse_nonnegative_u32_arg(arg: Option<&OscType>) -> Option<u32> {
    match arg {
        Some(OscType::Int(i)) if *i >= 0 => Some(*i as u32),
        Some(OscType::Float(f)) if *f >= 0.0 => Some(*f as u32),
        _ => None,
    }
}

fn parse_positive_f32_arg(arg: Option<&OscType>) -> Option<f32> {
    match arg {
        Some(OscType::Float(f)) if *f > 0.0 => Some(*f),
        Some(OscType::Int(i)) if *i > 0 => Some(*i as f32),
        _ => None,
    }
}

fn parse_nonnegative_f32_arg(arg: Option<&OscType>) -> Option<f32> {
    match arg {
        Some(OscType::Float(f)) if *f >= 0.0 => Some(*f),
        Some(OscType::Int(i)) if *i >= 0 => Some(*i as f32),
        _ => None,
    }
}

fn parse_f32_arg(arg: Option<&OscType>) -> Option<f32> {
    match arg {
        Some(OscType::Float(f)) => Some(*f),
        Some(OscType::Int(i)) => Some(*i as f32),
        _ => None,
    }
}

fn parse_string_arg(arg: Option<&OscType>) -> Option<String> {
    match arg {
        Some(OscType::String(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        _ => None,
    }
}

fn parse_input_layout_arg(
    arg: Option<&OscType>,
) -> Option<renderer::speaker_layout::SpeakerLayout> {
    let raw = parse_string_arg(arg)?;
    serde_yaml_ng::from_str::<renderer::speaker_layout::SpeakerLayout>(&raw).ok()
}

fn spherical_to_cartesian(azimuth: f32, elevation: f32, distance: f32) -> (f32, f32, f32) {
    let az = azimuth.to_radians();
    let el = elevation.to_radians();
    let horizontal = distance * el.cos();
    let x = horizontal * az.sin();
    let y = horizontal * az.cos();
    let z = distance * el.sin();
    (x.clamp(-1.0, 1.0), y.clamp(-1.0, 1.0), z.clamp(-1.0, 1.0))
}

fn cartesian_to_spherical(x: f32, y: f32, z: f32) -> (f32, f32, f32) {
    let dist = (x * x + y * y + z * z).sqrt();
    let az = x.atan2(y).to_degrees();
    let el = if dist > 0.0 {
        z.atan2((x * x + y * y).sqrt()).to_degrees()
    } else {
        0.0
    };
    (az, el, dist.max(0.01))
}

fn remap_live_speakers_remove(
    speakers: &mut std::collections::HashMap<usize, renderer::live_params::SpeakerLiveParams>,
    remove_idx: usize,
) {
    let mut next = std::collections::HashMap::new();
    for (idx, params) in speakers.drain() {
        if idx == remove_idx {
            continue;
        }
        let mapped = if idx > remove_idx { idx - 1 } else { idx };
        next.insert(mapped, params);
    }
    *speakers = next;
}

fn parse_json_string_arg<T: for<'de> Deserialize<'de>>(arg: Option<&OscType>) -> Option<T> {
    let OscType::String(value) = arg? else {
        return None;
    };
    serde_json::from_str(value).ok()
}

fn build_audio_state_json(audio: &audio_output::AudioControl) -> String {
    let requested = audio.requested_snapshot();
    let (_, sample_format) = audio.audio_state();
    serde_json::json!({
        "outputDevices": audio.available_output_devices(),
        "outputDevice": requested.output_device,
        "outputDeviceEffective": audio.effective_output_device(),
        "sampleRate": requested.output_sample_rate_hz,
        "sampleFormat": sample_format,
        "error": audio.audio_error(),
        "adaptiveResampling": {
            "enabled": requested.adaptive_enabled,
            "enableFarMode": requested.adaptive.enable_far_mode,
            "forceSilenceInFarMode": requested.adaptive.force_silence_in_far_mode,
            "hardRecoverHighInFarMode": requested.adaptive.hard_recover_high_in_far_mode,
            "hardRecoverLowInFarMode": requested.adaptive.hard_recover_low_in_far_mode,
            "farModeReturnFadeInMs": requested.adaptive.far_mode_return_fade_in_ms,
            "kpNear": requested.adaptive.kp_near,
            "ki": requested.adaptive.ki,
            "integralDischargeRatio": requested.adaptive.integral_discharge_ratio,
            "maxAdjust": requested.adaptive.max_adjust,
            "updateIntervalCallbacks": requested.adaptive.update_interval_callbacks,
            "nearFarThresholdMs": requested.adaptive.near_far_threshold_ms,
            "paused": requested.adaptive.paused
        },
        "latencyTargetMs": requested.latency_target_ms
    })
    .to_string()
}

fn build_input_state_json(input: &audio_input::InputControl) -> String {
    let requested = input.requested_snapshot();
    let applied = input.applied_snapshot();
    serde_json::json!({
        "mode": requested.mode,
        "activeMode": applied.active_mode,
        "applyPending": input.is_apply_pending(),
        "requested": {
            "backend": requested.backend,
            "node": requested.node_name,
            "description": requested.node_description,
            "layout": requested.layout_path.as_ref().map(|path| path.display().to_string()),
            "clockMode": requested.clock_mode,
            "channels": requested.channels,
            "sampleRate": requested.sample_rate_hz,
            "format": requested.sample_format,
            "map": requested.map_mode,
            "lfeMode": requested.lfe_mode
        },
        "applied": {
            "backend": applied.backend,
            "channels": applied.channels,
            "sampleRate": applied.sample_rate_hz,
            "node": applied.node_name,
            "description": applied.node_description,
            "streamFormat": applied.stream_format,
            "error": applied.input_error
        }
    })
    .to_string()
}

fn push_audio_domain_broadcasts(
    effects: &mut ControlEffects,
    audio: &audio_output::AudioControl,
    include_logical_apply: bool,
) {
    effects.broadcasts.push(BroadcastUpdate {
        addr: "/omniphony/state/audio".to_string(),
        value: BroadcastValue::String(build_audio_state_json(audio)),
    });
    if include_logical_apply {
        effects.log_message = Some("OSC: audio config staged".to_string());
    }
}

fn push_input_domain_broadcasts(
    effects: &mut ControlEffects,
    input: &audio_input::InputControl,
    include_logical_apply: bool,
) {
    effects.broadcasts.push(BroadcastUpdate {
        addr: "/omniphony/state/input".to_string(),
        value: BroadcastValue::String(build_input_state_json(input)),
    });
    if include_logical_apply {
        effects.log_message = Some("OSC: input config staged".to_string());
    }
}

fn remap_live_speakers_move(
    speakers: &mut std::collections::HashMap<usize, renderer::live_params::SpeakerLiveParams>,
    from: usize,
    to: usize,
) {
    if from == to {
        return;
    }
    let moved = speakers.remove(&from);
    let mut next = std::collections::HashMap::new();
    for (idx, params) in speakers.drain() {
        let mapped = if from < to {
            if idx > from && idx <= to {
                idx - 1
            } else {
                idx
            }
        } else if idx >= to && idx < from {
            idx + 1
        } else {
            idx
        };
        next.insert(mapped, params);
    }
    if let Some(params) = moved {
        next.insert(to, params);
    }
    *speakers = next;
}

fn apply_pending_speakers(
    pending: &mut HashMap<usize, SpeakerPatch>,
    ctx: &RuntimeControlContext,
) -> renderer::speaker_layout::SpeakerLayout {
    let layout = ctx.renderer.with_editable_layout(|layout| {
        for (idx, patch) in pending.iter() {
            if let Some(speaker) = layout.speakers.get_mut(*idx) {
                if let Some(az) = patch.az {
                    speaker.azimuth = az;
                }
                if let Some(el) = patch.el {
                    speaker.elevation = el;
                }
                if let Some(dist) = patch.distance {
                    speaker.distance = dist;
                }
                if let Some(x) = patch.x {
                    speaker.x = x.clamp(-1.0, 1.0);
                }
                if let Some(y) = patch.y {
                    speaker.y = y.clamp(-1.0, 1.0);
                }
                if let Some(z) = patch.z {
                    speaker.z = z.clamp(-1.0, 1.0);
                }
                if let Some(coord_mode) = &patch.coord_mode {
                    speaker.coord_mode = if coord_mode.eq_ignore_ascii_case("cartesian") {
                        "cartesian".to_string()
                    } else {
                        "polar".to_string()
                    };
                }
                if let Some(spatialize) = patch.spatialize {
                    speaker.spatialize = spatialize;
                }
                if let Some(freq_low) = patch.freq_low {
                    speaker.freq_low = freq_low.map(|value| value.max(0.0));
                }
                if let Some(freq_high) = patch.freq_high {
                    speaker.freq_high = freq_high.map(|value| value.max(0.0));
                }
                if let Some(name) = &patch.name {
                    speaker.name = name.clone();
                }
            }
        }
        layout.clone()
    });
    pending.clear();
    layout
}

fn normalize_coord_mode(mode: Option<&str>) -> &'static str {
    if mode.is_some_and(|value| value.eq_ignore_ascii_case("cartesian")) {
        "cartesian"
    } else {
        "polar"
    }
}

fn apply_layout_speaker_patch(
    speaker: &mut renderer::speaker_layout::Speaker,
    patch: &LayoutSpeakerPatch,
) -> bool {
    let mut changed = false;
    if let Some(name) = patch
        .name
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        if speaker.name != name {
            speaker.name = name.to_string();
            changed = true;
        }
    }
    if let Some(spatialize) = patch.spatialize {
        if speaker.spatialize != spatialize {
            speaker.spatialize = spatialize;
            changed = true;
        }
    }
    if let Some(freq_low) = patch.freq_low {
        let next = freq_low.filter(|value| *value > 0.0);
        if speaker.freq_low != next {
            speaker.freq_low = next;
            changed = true;
        }
    }
    if let Some(freq_high) = patch.freq_high {
        let next = freq_high.filter(|value| *value > 0.0);
        if speaker.freq_high != next {
            speaker.freq_high = next;
            changed = true;
        }
    }
    if let Some(coord_mode) = patch.coord_mode.as_deref() {
        let normalized = normalize_coord_mode(Some(coord_mode)).to_string();
        if speaker.coord_mode != normalized {
            speaker.coord_mode = normalized;
            changed = true;
        }
    }

    let has_cartesian = patch.x.is_some() || patch.y.is_some() || patch.z.is_some();
    if has_cartesian {
        let x = patch.x.unwrap_or(speaker.x).clamp(-1.0, 1.0);
        let y = patch.y.unwrap_or(speaker.y).clamp(-1.0, 1.0);
        let z = patch.z.unwrap_or(speaker.z).clamp(-1.0, 1.0);
        let (azimuth, elevation, distance) = cartesian_to_spherical(x, y, z);
        if speaker.x != x
            || speaker.y != y
            || speaker.z != z
            || speaker.azimuth != azimuth
            || speaker.elevation != elevation
            || speaker.distance != distance
        {
            speaker.x = x;
            speaker.y = y;
            speaker.z = z;
            speaker.azimuth = azimuth;
            speaker.elevation = elevation;
            speaker.distance = distance;
            changed = true;
        }
    }

    let has_polar =
        patch.azimuth.is_some() || patch.elevation.is_some() || patch.distance.is_some();
    if has_polar {
        let azimuth = patch
            .azimuth
            .unwrap_or(speaker.azimuth)
            .clamp(-180.0, 180.0);
        let elevation = patch
            .elevation
            .unwrap_or(speaker.elevation)
            .clamp(-90.0, 90.0);
        let distance = patch.distance.unwrap_or(speaker.distance).max(0.01);
        let (x, y, z) = spherical_to_cartesian(azimuth, elevation, distance);
        if speaker.azimuth != azimuth
            || speaker.elevation != elevation
            || speaker.distance != distance
            || speaker.x != x
            || speaker.y != y
            || speaker.z != z
        {
            speaker.azimuth = azimuth;
            speaker.elevation = elevation;
            speaker.distance = distance;
            speaker.x = x;
            speaker.y = y;
            speaker.z = z;
            changed = true;
        }
    }
    changed
}

fn build_layout_speaker_from_patch(
    patch: LayoutAddSpeakerPatch,
    default_name: String,
) -> renderer::speaker_layout::Speaker {
    let name = patch
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(default_name.as_str())
        .to_string();
    let spatialize = patch.spatialize.unwrap_or(true);
    let delay_ms = patch.delay_ms.unwrap_or(0.0).max(0.0);
    let mut speaker = renderer::speaker_layout::Speaker::from_polar(
        name,
        patch.azimuth.unwrap_or(0.0).clamp(-180.0, 180.0),
        patch.elevation.unwrap_or(0.0).clamp(-90.0, 90.0),
        patch.distance.unwrap_or(1.0).max(0.01),
        spatialize,
        delay_ms,
    );
    if let Some(freq_low) = patch.freq_low {
        speaker.freq_low = freq_low.filter(|value| *value > 0.0);
    }
    if let Some(freq_high) = patch.freq_high {
        speaker.freq_high = freq_high.filter(|value| *value > 0.0);
    }
    if patch.coord_mode.as_deref().is_some() {
        speaker.coord_mode = normalize_coord_mode(patch.coord_mode.as_deref()).to_string();
    }
    if patch.x.is_some() || patch.y.is_some() || patch.z.is_some() {
        let x = patch.x.unwrap_or(speaker.x).clamp(-1.0, 1.0);
        let y = patch.y.unwrap_or(speaker.y).clamp(-1.0, 1.0);
        let z = patch.z.unwrap_or(speaker.z).clamp(-1.0, 1.0);
        let (azimuth, elevation, distance) = cartesian_to_spherical(x, y, z);
        speaker.x = x;
        speaker.y = y;
        speaker.z = z;
        speaker.azimuth = azimuth;
        speaker.elevation = elevation;
        speaker.distance = distance;
    }
    speaker
}

pub fn apply_simple_osc_control(
    msg: &OscMessage,
    ctx: &RuntimeControlContext,
) -> Option<ControlEffects> {
    let addr = msg.addr.as_str();
    let mut effects = ControlEffects::default();

    if addr == "/omniphony/control/config/audio" {
        let patch = parse_json_string_arg::<AudioConfigPatch>(msg.args.first());
        if let (Some(audio), Some(patch)) = (ctx.audio.as_ref(), patch) {
            if let Some(output_device) = patch.output_device {
                audio.set_requested_output_device(output_device.and_then(|value| {
                    let trimmed = value.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                }));
            }
            if let Some(sample_rate) = patch.sample_rate {
                audio.set_requested_output_sample_rate(sample_rate.filter(|value| *value > 0));
            }
            if let Some(latency_target_ms) = patch.latency_target_ms {
                audio.set_requested_latency_target_ms(latency_target_ms.filter(|value| *value > 0));
            }
            if let Some(adaptive) = patch.adaptive_resampling {
                if let Some(enabled) = adaptive.enabled {
                    audio.set_requested_adaptive_resampling(enabled);
                }
                if let Some(enabled) = adaptive.enable_far_mode {
                    audio.set_requested_adaptive_resampling_enable_far_mode(enabled);
                }
                if let Some(enabled) = adaptive.force_silence_in_far_mode {
                    audio.set_requested_adaptive_resampling_force_silence_in_far_mode(enabled);
                }
                if let Some(enabled) = adaptive.hard_recover_high_in_far_mode {
                    audio.set_requested_adaptive_resampling_hard_recover_high_in_far_mode(enabled);
                }
                if let Some(enabled) = adaptive.hard_recover_low_in_far_mode {
                    audio.set_requested_adaptive_resampling_hard_recover_low_in_far_mode(enabled);
                }
                if let Some(value) = adaptive.far_mode_return_fade_in_ms {
                    audio.set_requested_adaptive_resampling_far_mode_return_fade_in_ms(value);
                }
                if let Some(value) = adaptive.kp_near.filter(|value| *value > 0.0) {
                    audio.set_requested_adaptive_resampling_kp_near(value as f32);
                }
                if let Some(value) = adaptive.ki.filter(|value| *value > 0.0) {
                    audio.set_requested_adaptive_resampling_ki(value as f32);
                }
                if let Some(value) = adaptive
                    .integral_discharge_ratio
                    .map(|value| value.clamp(0.0, 1.0))
                {
                    audio.set_requested_adaptive_resampling_integral_discharge_ratio(value as f32);
                }
                if let Some(value) = adaptive.max_adjust.filter(|value| *value > 0.0) {
                    audio.set_requested_adaptive_resampling_max_adjust(value as f32);
                }
                if let Some(value) = adaptive.near_far_threshold_ms.filter(|value| *value > 0) {
                    audio.set_requested_adaptive_resampling_near_far_threshold_ms(value);
                }
                if let Some(value) = adaptive
                    .update_interval_callbacks
                    .filter(|value| *value > 0)
                {
                    audio.set_requested_adaptive_resampling_update_interval_callbacks(value);
                }
                if let Some(paused) = adaptive.paused {
                    audio.set_requested_adaptive_resampling_paused(paused);
                }
            }
            effects.mark_dirty = true;
            push_audio_domain_broadcasts(&mut effects, audio, true);
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/config/audio/apply" {
        if let Some(audio) = ctx.audio.as_ref() {
            push_audio_domain_broadcasts(&mut effects, audio, false);
            effects.log_message = Some("OSC: audio config apply".to_string());
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/config/input" {
        let patch = parse_json_string_arg::<InputConfigPatch>(msg.args.first());
        if let (Some(input), Some(patch)) = (ctx.input.as_ref(), patch) {
            if let Some(mode) = patch.mode {
                input.set_requested_mode(mode);
            }
            if let Some(live_input) = patch.live_input {
                if let Some(backend) = live_input.backend {
                    input.set_requested_backend(backend);
                }
                if let Some(node) = live_input.node {
                    input.set_requested_node_name(node.and_then(|value| {
                        let trimmed = value.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    }));
                }
                if let Some(description) = live_input.description {
                    input.set_requested_node_description(description.and_then(|value| {
                        let trimmed = value.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(trimmed.to_string())
                        }
                    }));
                }
                if let Some(layout) = live_input.layout {
                    input.set_requested_layout_path(layout.and_then(|value| {
                        let trimmed = value.trim();
                        if trimmed.is_empty() {
                            None
                        } else {
                            Some(std::path::PathBuf::from(trimmed))
                        }
                    }));
                    input.set_requested_current_layout(None);
                }
                if let Some(clock_mode) = live_input.clock_mode {
                    input.set_requested_clock_mode(clock_mode);
                }
                if let Some(channels) = live_input.channels {
                    input.set_requested_channels(channels.filter(|value| *value > 0));
                }
                if let Some(sample_rate) = live_input.sample_rate {
                    input.set_requested_sample_rate_hz(sample_rate.filter(|value| *value > 0));
                }
                if let Some(sample_format) = live_input.format {
                    input.set_requested_sample_format(sample_format);
                }
                if let Some(map_mode) = live_input.map {
                    input.set_requested_map_mode(map_mode);
                }
                if let Some(lfe_mode) = live_input.lfe_mode {
                    input.set_requested_lfe_mode(lfe_mode);
                }
            }
            effects.mark_dirty = true;
            push_input_domain_broadcasts(&mut effects, input, true);
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/config/input/apply" {
        if let Some(input) = ctx.input.as_ref() {
            input.request_apply();
            effects.mark_dirty = true;
            push_input_domain_broadcasts(&mut effects, input, false);
            effects.log_message = Some("OSC: input config apply requested".to_string());
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/config/layout" {
        let patch = parse_json_string_arg::<LayoutConfigPatch>(msg.args.first());
        if let Some(patch) = patch {
            let mut changed = false;

            if let Some(radius_m) = patch.radius_m {
                let radius_m = radius_m.max(0.01);
                changed |= ctx.renderer.with_editable_layout(|layout| {
                    if (layout.radius_m - radius_m).abs() > f32::EPSILON {
                        layout.radius_m = radius_m;
                        true
                    } else {
                        false
                    }
                });
            }

            if let Some(add_speaker) = patch.add_speaker {
                let idx = ctx.renderer.editable_layout().speakers.len();
                let speaker = build_layout_speaker_from_patch(add_speaker, format!("spk-{idx}"));
                let delay_ms = speaker.delay_ms;
                ctx.renderer.with_editable_layout(|layout| {
                    layout.speakers.push(speaker);
                });
                if delay_ms > 0.0 {
                    ctx.renderer
                        .live
                        .write()
                        .unwrap()
                        .speakers
                        .entry(idx)
                        .or_default()
                        .delay_ms = delay_ms;
                    ctx.renderer.mark_speaker_params_dirty();
                }
                changed = true;
            }

            if let Some(remove_idx) = patch.remove_speaker {
                let removed = ctx.renderer.with_editable_layout(|layout| {
                    if remove_idx >= layout.speakers.len() {
                        false
                    } else {
                        layout.speakers.remove(remove_idx);
                        true
                    }
                });
                if removed {
                    {
                        let mut live = ctx.renderer.live.write().unwrap();
                        remap_live_speakers_remove(&mut live.speakers, remove_idx);
                    }
                    ctx.renderer.mark_speaker_params_dirty();
                    changed = true;
                }
            }

            if let Some(move_speaker) = patch.move_speaker {
                let moved = ctx.renderer.with_editable_layout(|layout| {
                    let len = layout.speakers.len();
                    if move_speaker.from >= len
                        || move_speaker.to >= len
                        || move_speaker.from == move_speaker.to
                    {
                        false
                    } else {
                        let speaker = layout.speakers.remove(move_speaker.from);
                        layout.speakers.insert(move_speaker.to, speaker);
                        true
                    }
                });
                if moved {
                    {
                        let mut live = ctx.renderer.live.write().unwrap();
                        remap_live_speakers_move(
                            &mut live.speakers,
                            move_speaker.from,
                            move_speaker.to,
                        );
                    }
                    ctx.renderer.mark_speaker_params_dirty();
                    changed = true;
                }
            }

            if let Some(speaker_edits) = patch.speaker_edits {
                changed |= ctx.renderer.with_editable_layout(|layout| {
                    let mut any = false;
                    for speaker_patch in &speaker_edits {
                        if let Some(speaker) = layout.speakers.get_mut(speaker_patch.id) {
                            any |= apply_layout_speaker_patch(speaker, speaker_patch);
                        }
                    }
                    any
                });
            }

            if changed {
                effects.mark_dirty = true;
            }
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/config/layout/apply" {
        effects.mark_dirty = true;
        effects.trigger_layout_recompute = true;
        effects.log_message = Some("OSC: layout config apply".to_string());
        return Some(effects);
    }

    if addr == "/omniphony/control/config/speakers" {
        let patch = parse_json_string_arg::<SpeakersConfigPatch>(msg.args.first());
        if let Some(patch) = patch {
            let mut changed = false;
            if let Some(speaker_edits) = patch.speaker_edits {
                for speaker_patch in speaker_edits {
                    if let Some(delay_ms) = speaker_patch.delay_ms.map(|value| value.max(0.0)) {
                        ctx.renderer
                            .live
                            .write()
                            .unwrap()
                            .speakers
                            .entry(speaker_patch.id)
                            .or_default()
                            .delay_ms = delay_ms;
                        ctx.renderer.with_editable_layout(|layout| {
                            if let Some(speaker) = layout.speakers.get_mut(speaker_patch.id) {
                                speaker.delay_ms = delay_ms;
                            }
                        });
                        ctx.renderer.mark_speaker_params_dirty();
                        changed = true;
                    }
                    if let Some(muted) = speaker_patch.muted {
                        ctx.renderer
                            .live
                            .write()
                            .unwrap()
                            .speakers
                            .entry(speaker_patch.id)
                            .or_default()
                            .muted = muted;
                        ctx.renderer.mark_speaker_params_dirty();
                        changed = true;
                    }
                }
            }
            if changed {
                effects.mark_dirty = true;
            }
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/audio/output_devices/refresh" {
        if let Some(audio) = ctx.audio.as_ref() {
            if let Some(devices) = audio.refresh_available_output_devices() {
                effects.broadcasts.push(BroadcastUpdate {
                    addr: "/omniphony/state/audio".to_string(),
                    value: BroadcastValue::String(build_audio_state_json(audio)),
                });
                effects.log_message = Some(format!(
                    "OSC: output_devices/refresh → {} device(s)",
                    devices.len()
                ));
            }
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/audio/output_device" {
        let requested = msg.args.first().and_then(|arg| match arg {
            OscType::String(s) => {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            _ => None,
        });
        if let Some(audio) = ctx.audio.as_ref() {
            audio.set_requested_output_device(requested.clone());
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/render_backend" {
        let requested = parse_string_arg(msg.args.first())
            .and_then(|value| RenderBackendKind::from_str(&value));
        if let Some(requested) = requested {
            let mut live = ctx.renderer.live.write().unwrap();
            if live.backend_id() != requested.as_str() {
                live.backend_id = requested.as_str().to_string();
                effects.mark_dirty = true;
                effects.trigger_layout_recompute = true;
                effects.log_message =
                    Some(format!("OSC: render_backend -> {}", requested.as_str()));
            }
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/render_backend/restore" {
        effects.log_message = Some(
            "OSC: render_backend/restore is no longer supported after removing from_file"
                .to_string(),
        );
        return Some(effects);
    }

    if addr == "/omniphony/control/render_evaluation_mode" {
        let requested = parse_string_arg(msg.args.first())
            .and_then(|value| LiveEvaluationMode::from_str(&value));
        if let Some(requested) = requested {
            let mut live = ctx.renderer.live.write().unwrap();
            if live.evaluation.mode != requested {
                live.set_evaluation_mode(requested);
                effects.mark_dirty = true;
                effects.trigger_layout_recompute = true;
            }
            {
                if live.backend_kind() == Some(RenderBackendKind::Vbap) {
                    effects.mark_dirty = true;
                }
                effects.log_message = Some(format!(
                    "OSC: render_evaluation_mode -> {}",
                    live.requested_evaluation_mode().as_str()
                ));
            }
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/render_evaluation_mode/from_file" {
        effects.log_message =
            Some("OSC: render_evaluation_mode/from_file is no longer supported".to_string());
        return Some(effects);
    }

    if addr == "/omniphony/control/debug/speaker_heatmap/request" {
        let request = parse_string_arg(msg.args.first())
            .and_then(|value| serde_json::from_str::<SpeakerHeatmapRequest>(&value).ok());
        if let Some(request) = request {
            let mode = request.mode.trim().to_ascii_lowercase();
            let max_samples = request.max_samples.unwrap_or(3072).clamp(128, 20000);
            let topology = ctx.renderer.active_topology();
            let speaker = topology.speaker_layout.speakers.get(request.speaker_index);
            let unavailable_reason = match speaker {
                None => Some("speaker_not_found"),
                Some(_)
                    if topology.backend.evaluation_mode()
                        != renderer::render_backend::EffectiveEvaluationMode::PrecomputedCartesian =>
                {
                    Some("evaluation_mode_not_precomputed_cartesian")
                }
                Some(speaker)
                    if topology
                        .backend_speaker_index_for_layout_speaker(request.speaker_index)
                        .is_none() =>
                {
                    let _ = speaker;
                    Some("speaker_not_spatializable")
                }
                _ => None,
            };

            if let Some(reason) = unavailable_reason {
                let json = serde_json::to_string(&SpeakerHeatmapUnavailablePayload {
                    request_id: request.request_id,
                    speaker_index: request.speaker_index,
                    band_index: request.band_index,
                    reason,
                })
                .unwrap_or_else(|_| "{}".to_string());
                effects.broadcasts.push(BroadcastUpdate {
                    addr: "/omniphony/state/debug/speaker_heatmap/unavailable".to_string(),
                    value: BroadcastValue::String(json),
                });
                return Some(effects);
            }

            if let Some(speaker) = speaker {
                if let Some(backend_speaker_index) =
                    topology.backend_speaker_index_for_layout_speaker(request.speaker_index)
                {
                    let bands = compute_bands(&topology.speaker_layout);
                    let selected_band = bands.get(request.band_index);
                    if selected_band.is_none() {
                        let json = serde_json::to_string(&SpeakerHeatmapUnavailablePayload {
                            request_id: request.request_id,
                            speaker_index: request.speaker_index,
                            band_index: request.band_index,
                            reason: "band_not_found",
                        })
                        .unwrap_or_else(|_| "{}".to_string());
                        effects.broadcasts.push(BroadcastUpdate {
                            addr: "/omniphony/state/debug/speaker_heatmap/unavailable".to_string(),
                            value: BroadcastValue::String(json),
                        });
                        return Some(effects);
                    }
                    let selected_band = selected_band.unwrap();
                    let speaker_position = [speaker.x, speaker.y, speaker.z];
                    let band_layout_index = selected_band
                        .speaker_indices
                        .iter()
                        .position(|&index| index == request.speaker_index);

                    let reference_slices = topology
                        .backend
                        .cartesian_slices_for_speaker(backend_speaker_index, speaker_position);
                    let band_slices = if selected_band.speaker_indices.len() >= 3 {
                        if let Some(layout_index) = band_layout_index {
                            let band_layout = renderer::speaker_layout::SpeakerLayout {
                                radius_m: topology.speaker_layout.radius_m,
                                speakers: selected_band
                                    .speaker_indices
                                    .iter()
                                    .map(|&index| topology.speaker_layout.speakers[index].clone())
                                    .collect(),
                            };
                            ctx.renderer
                                .prepare_topology_rebuild_for_layout(band_layout)
                                .and_then(|plan| plan.build_topology().ok())
                                .and_then(|band_topology| {
                                    band_topology
                                        .backend_speaker_index_for_layout_speaker(layout_index)
                                        .and_then(|band_backend_index| {
                                            band_topology.backend.cartesian_slices_for_speaker(
                                                band_backend_index,
                                                speaker_position,
                                            )
                                        })
                                })
                        } else {
                            reference_slices.clone().map(|reference| {
                                build_constant_slices_from_reference(reference, 0.0)
                            })
                        }
                    } else {
                        let fallback_value = if band_layout_index.is_some()
                            && !selected_band.speaker_indices.is_empty()
                        {
                            1.0 / (selected_band.speaker_indices.len() as f32).sqrt()
                        } else {
                            0.0
                        };
                        reference_slices.clone().map(|reference| {
                            build_constant_slices_from_reference(reference, fallback_value)
                        })
                    };

                    let meta = serde_json::to_string(&SpeakerHeatmapMetaPayload {
                        request_id: request.request_id,
                        speaker_index: request.speaker_index,
                        band_index: request.band_index,
                        speaker_position,
                    })
                    .unwrap_or_else(|_| "{}".to_string());
                    effects.broadcasts.push(BroadcastUpdate {
                        addr: "/omniphony/state/debug/speaker_heatmap/meta".to_string(),
                        value: BroadcastValue::String(meta),
                    });

                    if mode == "volume" {
                        let volume = if selected_band.speaker_indices.len() >= 3 {
                            if let Some(layout_index) = band_layout_index {
                                let band_layout = renderer::speaker_layout::SpeakerLayout {
                                    radius_m: topology.speaker_layout.radius_m,
                                    speakers: selected_band
                                        .speaker_indices
                                        .iter()
                                        .map(|&index| {
                                            topology.speaker_layout.speakers[index].clone()
                                        })
                                        .collect(),
                                };
                                ctx.renderer
                                    .prepare_topology_rebuild_for_layout(band_layout)
                                    .and_then(|plan| plan.build_topology().ok())
                                    .and_then(|band_topology| {
                                        band_topology
                                            .backend_speaker_index_for_layout_speaker(layout_index)
                                            .and_then(|band_backend_index| {
                                                band_topology.backend.cartesian_volume_for_speaker(
                                                    band_backend_index,
                                                    0.0,
                                                    max_samples,
                                                )
                                            })
                                    })
                            } else {
                                Some(CartesianSpeakerHeatmapVolume {
                                    speaker_index: request.speaker_index,
                                    samples: Vec::new(),
                                })
                            }
                        } else {
                            let fallback_value = if band_layout_index.is_some()
                                && !selected_band.speaker_indices.is_empty()
                            {
                                1.0 / (selected_band.speaker_indices.len() as f32).sqrt()
                            } else {
                                0.0
                            };
                            band_slices
                                .as_ref()
                                .map(|slices| CartesianSpeakerHeatmapVolume {
                                    speaker_index: request.speaker_index,
                                    samples: build_constant_volume_samples(
                                        slices,
                                        fallback_value,
                                        max_samples,
                                    ),
                                })
                        };

                        if let Some(volume) = volume {
                            // Keep OSC/UDP packets comfortably below MTU once the JSON payload
                            // wraps the float array. Large chunks were getting fragmented and
                            // dropped, which left Studio waiting forever for the missing chunk.
                            const CHUNK_FLOATS: usize = 16 * 4;
                            let chunk_count = volume.samples.len().div_ceil(CHUNK_FLOATS).max(1);
                            if volume.samples.is_empty() {
                                let json =
                                    serde_json::to_string(&SpeakerHeatmapVolumeChunkPayload {
                                        request_id: request.request_id,
                                        speaker_index: request.speaker_index,
                                        band_index: request.band_index,
                                        chunk_index: 0,
                                        chunk_count: 1,
                                        samples: Vec::new(),
                                    })
                                    .unwrap_or_else(|_| "{}".to_string());
                                effects.broadcasts.push(BroadcastUpdate {
                                    addr: "/omniphony/state/debug/speaker_heatmap/volume_chunk"
                                        .to_string(),
                                    value: BroadcastValue::String(json),
                                });
                            } else {
                                for (chunk_index, chunk) in
                                    volume.samples.chunks(CHUNK_FLOATS).enumerate()
                                {
                                    let json =
                                        serde_json::to_string(&SpeakerHeatmapVolumeChunkPayload {
                                            request_id: request.request_id,
                                            speaker_index: request.speaker_index,
                                            band_index: request.band_index,
                                            chunk_index,
                                            chunk_count,
                                            samples: chunk.to_vec(),
                                        })
                                        .unwrap_or_else(|_| "{}".to_string());
                                    effects.broadcasts.push(BroadcastUpdate {
                                        addr: "/omniphony/state/debug/speaker_heatmap/volume_chunk"
                                            .to_string(),
                                        value: BroadcastValue::String(json),
                                    });
                                }
                            }
                        } else {
                            let json = serde_json::to_string(&SpeakerHeatmapUnavailablePayload {
                                request_id: request.request_id,
                                speaker_index: request.speaker_index,
                                band_index: request.band_index,
                                reason: "band_heatmap_unavailable",
                            })
                            .unwrap_or_else(|_| "{}".to_string());
                            effects.broadcasts.push(BroadcastUpdate {
                                addr: "/omniphony/state/debug/speaker_heatmap/unavailable"
                                    .to_string(),
                                value: BroadcastValue::String(json),
                            });
                        }
                    } else if let Some(slices) = band_slices {
                        for (addr_suffix, fixed_axis_value, axis_a, axis_b, values) in [
                            (
                                "slice_xy",
                                slices.speaker_position[2],
                                slices.x_positions.clone(),
                                slices.y_positions.clone(),
                                slices.xy_values,
                            ),
                            (
                                "slice_xz",
                                slices.speaker_position[1],
                                slices.x_positions.clone(),
                                slices.z_positions.clone(),
                                slices.xz_values,
                            ),
                            (
                                "slice_yz",
                                slices.speaker_position[0],
                                slices.y_positions.clone(),
                                slices.z_positions.clone(),
                                slices.yz_values,
                            ),
                        ] {
                            let json = serde_json::to_string(&SpeakerHeatmapSlicePayload {
                                request_id: request.request_id,
                                speaker_index: request.speaker_index,
                                band_index: request.band_index,
                                fixed_axis_value,
                                axis_a,
                                axis_b,
                                values,
                            })
                            .unwrap_or_else(|_| "{}".to_string());
                            effects.broadcasts.push(BroadcastUpdate {
                                addr: format!(
                                    "/omniphony/state/debug/speaker_heatmap/{addr_suffix}"
                                ),
                                value: BroadcastValue::String(json),
                            });
                        }
                    } else {
                        let json = serde_json::to_string(&SpeakerHeatmapUnavailablePayload {
                            request_id: request.request_id,
                            speaker_index: request.speaker_index,
                            band_index: request.band_index,
                            reason: "band_heatmap_unavailable",
                        })
                        .unwrap_or_else(|_| "{}".to_string());
                        effects.broadcasts.push(BroadcastUpdate {
                            addr: "/omniphony/state/debug/speaker_heatmap/unavailable".to_string(),
                            value: BroadcastValue::String(json),
                        });
                    }
                    effects.log_message = Some(format!(
                        "OSC: speaker heatmap requested -> speaker={} band={} mode={} request_id={}",
                        request.speaker_index, request.band_index, mode, request.request_id
                    ));
                }
            }
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/mode" {
        let requested = parse_string_arg(msg.args.first()).and_then(|value| {
            match value.to_ascii_lowercase().as_str() {
                "bridge" | "pipe_bridge" => Some(InputMode::Bridge),
                "live" | "pipewire" => Some(InputMode::Live),
                "pipewire_bridge" => Some(InputMode::PipewireBridge),
                _ => None,
            }
        });
        if let (Some(input), Some(requested)) = (ctx.input.as_ref(), requested) {
            input.set_requested_mode(requested);
            effects.mark_dirty = true;
            effects.log_message = Some(format!(
                "OSC: input mode staged → {}",
                match requested {
                    InputMode::Bridge => "pipe_bridge",
                    InputMode::Live => "pipewire",
                    InputMode::PipewireBridge => "pipewire_bridge",
                }
            ));
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/live/backend" {
        let requested = parse_string_arg(msg.args.first()).and_then(|value| {
            match value.to_ascii_lowercase().as_str() {
                "pipewire" => Some(InputBackend::Pipewire),
                "asio" => Some(InputBackend::Asio),
                _ => None,
            }
        });
        if let (Some(input), Some(requested)) = (ctx.input.as_ref(), requested) {
            input.set_requested_backend(Some(requested));
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/live/node" {
        let requested = parse_string_arg(msg.args.first());
        if let Some(input) = ctx.input.as_ref() {
            input.set_requested_node_name(requested.clone());
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/live/description" {
        let requested = parse_string_arg(msg.args.first());
        if let Some(input) = ctx.input.as_ref() {
            input.set_requested_node_description(requested.clone());
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/live/layout" {
        let requested = parse_string_arg(msg.args.first()).map(std::path::PathBuf::from);
        if let Some(input) = ctx.input.as_ref() {
            input.set_requested_layout_path(requested);
            input.set_requested_current_layout(None);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/live/layout_import" {
        let requested = parse_input_layout_arg(msg.args.first());
        if let Some(input) = ctx.input.as_ref() {
            input.set_requested_current_layout(requested);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/live/channels" {
        let requested = match msg.args.first() {
            Some(OscType::Int(i)) if *i > 0 => Some(*i as u16),
            Some(OscType::Float(f)) if *f > 0.0 => Some(*f as u16),
            _ => None,
        };
        if let (Some(input), Some(requested)) = (ctx.input.as_ref(), requested) {
            input.set_requested_channels(Some(requested));
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/live/sample_rate" {
        let requested = parse_positive_u32_arg(msg.args.first());
        if let (Some(input), Some(requested)) = (ctx.input.as_ref(), requested) {
            input.set_requested_sample_rate_hz(Some(requested));
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/live/format" {
        let requested = parse_string_arg(msg.args.first()).and_then(|value| {
            match value.to_ascii_lowercase().as_str() {
                "f32" => Some(InputSampleFormat::F32),
                "s16" => Some(InputSampleFormat::S16),
                _ => None,
            }
        });
        if let (Some(input), Some(requested)) = (ctx.input.as_ref(), requested) {
            input.set_requested_sample_format(Some(requested));
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/live/map" {
        let requested = parse_string_arg(msg.args.first()).and_then(|value| {
            match value.to_ascii_lowercase().as_str() {
                "7.1-fixed" => Some(InputMapMode::SevenOneFixed),
                _ => None,
            }
        });
        if let (Some(input), Some(requested)) = (ctx.input.as_ref(), requested) {
            input.set_requested_map_mode(requested);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/live/lfe_mode" {
        let requested = parse_string_arg(msg.args.first()).and_then(|value| {
            match value.to_ascii_lowercase().as_str() {
                "object" => Some(InputLfeMode::Object),
                "direct" => Some(InputLfeMode::Direct),
                "drop" => Some(InputLfeMode::Drop),
                _ => None,
            }
        });
        if let (Some(input), Some(requested)) = (ctx.input.as_ref(), requested) {
            input.set_requested_lfe_mode(requested);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/live/clock_mode" {
        let requested = parse_string_arg(msg.args.first()).and_then(|value| {
            match value.to_ascii_lowercase().as_str() {
                "dac" => Some(InputClockMode::Dac),
                "pipewire" => Some(InputClockMode::Pipewire),
                "upstream" => Some(InputClockMode::Upstream),
                _ => None,
            }
        });
        if let (Some(input), Some(requested)) = (ctx.input.as_ref(), requested) {
            input.set_requested_clock_mode(requested);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/input/apply" {
        if let Some(input) = ctx.input.as_ref() {
            input.request_apply();
            effects.mark_dirty = true;
            effects.log_message = Some("OSC: input apply requested".to_string());
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/ramp_mode" {
        let Some(mode) = msg.args.first().and_then(|arg| match arg {
            OscType::String(s) => renderer::live_params::RampMode::from_str(s),
            _ => None,
        }) else {
            return Some(effects);
        };
        ctx.renderer.set_requested_ramp_mode(mode);
        ctx.renderer.live.write().unwrap().ramp_mode = mode;
        effects.mark_dirty = true;
        effects.log_message = Some(format!("OSC: ramp_mode → {}", mode.as_str()));
        return Some(effects);
    }

    if addr == "/omniphony/control/audio/sample_rate" {
        let requested_hz = match msg.args.first() {
            Some(OscType::Int(i)) if *i > 0 => Some(*i as u32),
            Some(OscType::Float(f)) if *f > 0.0 => Some(*f as u32),
            Some(OscType::Int(_)) | Some(OscType::Float(_)) => None,
            _ => None,
        };
        if let Some(audio) = ctx.audio.as_ref() {
            audio.set_requested_output_sample_rate(requested_hz);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling" {
        let enabled = parse_bool_arg(msg.args.first());
        if let (Some(audio), Some(enabled)) = (ctx.audio.as_ref(), enabled) {
            audio.set_requested_adaptive_resampling(enabled);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/enable_far_mode" {
        let enabled = parse_bool_arg(msg.args.first());
        if let (Some(audio), Some(enabled)) = (ctx.audio.as_ref(), enabled) {
            audio.set_requested_adaptive_resampling_enable_far_mode(enabled);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/force_silence_in_far_mode" {
        let enabled = parse_bool_arg(msg.args.first());
        if let (Some(audio), Some(enabled)) = (ctx.audio.as_ref(), enabled) {
            audio.set_requested_adaptive_resampling_force_silence_in_far_mode(enabled);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/hard_recover_high_in_far_mode"
        || addr == "/omniphony/control/adaptive_resampling/hard_recover_in_far_mode"
    {
        let enabled = parse_bool_arg(msg.args.first());
        if let (Some(audio), Some(enabled)) = (ctx.audio.as_ref(), enabled) {
            audio.set_requested_adaptive_resampling_hard_recover_high_in_far_mode(enabled);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/hard_recover_low_in_far_mode" {
        let enabled = parse_bool_arg(msg.args.first());
        if let (Some(audio), Some(enabled)) = (ctx.audio.as_ref(), enabled) {
            audio.set_requested_adaptive_resampling_hard_recover_low_in_far_mode(enabled);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/far_mode_return_fade_in_ms" {
        let value = parse_nonnegative_u32_arg(msg.args.first());
        if let (Some(audio), Some(value)) = (ctx.audio.as_ref(), value) {
            audio.set_requested_adaptive_resampling_far_mode_return_fade_in_ms(value);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/kp_near" {
        let value = parse_positive_f32_arg(msg.args.first());
        if let (Some(audio), Some(value)) = (ctx.audio.as_ref(), value) {
            audio.set_requested_adaptive_resampling_kp_near(value);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/ki" {
        let value = parse_positive_f32_arg(msg.args.first());
        if let (Some(audio), Some(value)) = (ctx.audio.as_ref(), value) {
            audio.set_requested_adaptive_resampling_ki(value);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/integral_discharge_ratio" {
        let value = parse_nonnegative_f32_arg(msg.args.first()).map(|v| v.min(1.0));
        if let (Some(audio), Some(value)) = (ctx.audio.as_ref(), value) {
            audio.set_requested_adaptive_resampling_integral_discharge_ratio(value);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/max_adjust" {
        let value = parse_positive_f32_arg(msg.args.first());
        if let (Some(audio), Some(value)) = (ctx.audio.as_ref(), value) {
            audio.set_requested_adaptive_resampling_max_adjust(value);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/update_interval_callbacks" {
        let value = parse_positive_u32_arg(msg.args.first());
        if let (Some(audio), Some(value)) = (ctx.audio.as_ref(), value) {
            audio.set_requested_adaptive_resampling_update_interval_callbacks(value);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/near_far_threshold_ms" {
        let value = parse_positive_u32_arg(msg.args.first());
        if let (Some(audio), Some(value)) = (ctx.audio.as_ref(), value) {
            audio.set_requested_adaptive_resampling_near_far_threshold_ms(value);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/pause" {
        let paused = parse_bool_arg(msg.args.first());
        if let (Some(audio), Some(paused)) = (ctx.audio.as_ref(), paused) {
            audio.set_requested_adaptive_resampling_paused(paused);
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/adaptive_resampling/reset_ratio" {
        if let Some(audio) = ctx.audio.as_ref() {
            audio.request_ratio_reset();
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/latency_target" {
        let latency_ms = parse_positive_u32_arg(msg.args.first());
        if let (Some(audio), Some(latency_ms)) = (ctx.audio.as_ref(), latency_ms) {
            audio.set_requested_latency_target_ms(Some(latency_ms));
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/layout/radius_m" {
        if let Some(v) = parse_f32_arg(msg.args.first()).map(|f| f.max(0.01)) {
            ctx.renderer
                .with_editable_layout(|layout| layout.radius_m = v);
            effects.mark_dirty = true;
            effects.log_message = Some(format!("OSC: layout radius_m → {}", v));
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/gain" {
        if let Some(gain) = parse_f32_arg(msg.args.first()) {
            ctx.renderer.live.write().unwrap().master_gain = gain;
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    macro_rules! layout_float_with_recompute {
        ($path:literal, $field:ident, $state:literal) => {
            if addr == $path {
                if let Some(value) = parse_f32_arg(msg.args.first()) {
                    ctx.renderer.live.write().unwrap().$field = value;
                    effects.mark_dirty = true;
                    effects.trigger_layout_recompute = true;
                }
                return Some(effects);
            }
        };
    }

    layout_float_with_recompute!(
        "/omniphony/control/spread/min",
        spread_min,
        "/omniphony/state/spread/min"
    );
    layout_float_with_recompute!(
        "/omniphony/control/spread/max",
        spread_max,
        "/omniphony/state/spread/max"
    );
    layout_float_with_recompute!(
        "/omniphony/control/spread/distance_range",
        spread_distance_range,
        "/omniphony/state/spread/distance_range"
    );
    layout_float_with_recompute!(
        "/omniphony/control/spread/distance_curve",
        spread_distance_curve,
        "/omniphony/state/spread/distance_curve"
    );

    if addr == "/omniphony/control/spread/from_distance" {
        if let Some(v) = parse_bool_arg(msg.args.first()) {
            ctx.renderer.live.write().unwrap().spread_from_distance = v;
            effects.mark_dirty = true;
            effects.trigger_layout_recompute = true;
        }
        return Some(effects);
    }

    if let Some(rest) = addr.strip_prefix("/omniphony/control/render_evaluation/cartesian/") {
        let size = match msg.args.first() {
            Some(OscType::Int(i)) => Some((*i).max(1) as usize),
            Some(OscType::Float(f)) => Some((*f).round().max(1.0) as usize),
            _ => None,
        };
        if let Some(size) = size {
            let state_addr = match rest {
                "x_size" => {
                    ctx.renderer
                        .live
                        .write()
                        .unwrap()
                        .evaluation
                        .cartesian
                        .x_size = size;
                    Some("/omniphony/state/render_evaluation/cartesian/x_size")
                }
                "y_size" => {
                    ctx.renderer
                        .live
                        .write()
                        .unwrap()
                        .evaluation
                        .cartesian
                        .y_size = size;
                    Some("/omniphony/state/render_evaluation/cartesian/y_size")
                }
                "z_size" => {
                    ctx.renderer
                        .live
                        .write()
                        .unwrap()
                        .evaluation
                        .cartesian
                        .z_size = size;
                    Some("/omniphony/state/render_evaluation/cartesian/z_size")
                }
                "z_neg_size" => {
                    ctx.renderer
                        .live
                        .write()
                        .unwrap()
                        .evaluation
                        .cartesian
                        .z_neg_size = size;
                    Some("/omniphony/state/render_evaluation/cartesian/z_neg_size")
                }
                _ => None,
            };
            if let Some(state_addr) = state_addr {
                effects.mark_dirty = true;
                effects.trigger_layout_recompute = true;
                effects.broadcasts.push(BroadcastUpdate {
                    addr: state_addr.to_string(),
                    value: BroadcastValue::Int(size as i32),
                });
            }
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/render_evaluation/position_interpolation" {
        if let Some(enabled) = parse_bool_arg(msg.args.first()) {
            ctx.renderer
                .live
                .write()
                .unwrap()
                .evaluation
                .position_interpolation = enabled;
            effects.mark_dirty = true;
            effects.trigger_layout_recompute = true;
            effects.broadcasts.push(BroadcastUpdate {
                addr: "/omniphony/state/render_evaluation/position_interpolation".to_string(),
                value: BroadcastValue::Int(if enabled { 1 } else { 0 }),
            });
        }
        return Some(effects);
    }

    if let Some(rest) = addr.strip_prefix("/omniphony/control/render_evaluation/polar/") {
        match rest {
            "azimuth_resolution" | "elevation_resolution" => {
                let res = match msg.args.first() {
                    Some(OscType::Int(i)) => Some((*i).max(1)),
                    Some(OscType::Float(f)) => Some((*f as i32).max(1)),
                    _ => None,
                };
                if let Some(res) = res {
                    let state_addr = match rest {
                        "azimuth_resolution" => {
                            ctx.renderer
                                .live
                                .write()
                                .unwrap()
                                .evaluation
                                .polar
                                .azimuth_values = res;
                            Some("/omniphony/state/render_evaluation/polar/azimuth_resolution")
                        }
                        "elevation_resolution" => {
                            ctx.renderer
                                .live
                                .write()
                                .unwrap()
                                .evaluation
                                .polar
                                .elevation_values = res;
                            Some("/omniphony/state/render_evaluation/polar/elevation_resolution")
                        }
                        _ => None,
                    };
                    if let Some(state_addr) = state_addr {
                        effects.mark_dirty = true;
                        effects.trigger_layout_recompute = true;
                        effects.broadcasts.push(BroadcastUpdate {
                            addr: state_addr.to_string(),
                            value: BroadcastValue::Int(res),
                        });
                    }
                }
            }
            "distance_res" => {
                let res = match msg.args.first() {
                    Some(OscType::Int(i)) => Some((*i).max(1)),
                    Some(OscType::Float(f)) => Some((*f as i32).max(1)),
                    _ => None,
                };
                if let Some(res) = res {
                    ctx.renderer
                        .live
                        .write()
                        .unwrap()
                        .evaluation
                        .polar
                        .distance_res = res;
                    effects.mark_dirty = true;
                    effects.trigger_layout_recompute = true;
                    effects.broadcasts.push(BroadcastUpdate {
                        addr: "/omniphony/state/render_evaluation/polar/distance_res".to_string(),
                        value: BroadcastValue::Int(res),
                    });
                }
            }
            "distance_max" => {
                let max_v = match msg.args.first() {
                    Some(OscType::Int(i)) => Some((*i as f32).max(0.01)),
                    Some(OscType::Float(f)) => Some((*f).max(0.01)),
                    _ => None,
                };
                if let Some(max_v) = max_v {
                    ctx.renderer
                        .live
                        .write()
                        .unwrap()
                        .evaluation
                        .polar
                        .distance_max = max_v;
                    effects.mark_dirty = true;
                    effects.trigger_layout_recompute = true;
                    effects.broadcasts.push(BroadcastUpdate {
                        addr: "/omniphony/state/render_evaluation/polar/distance_max".to_string(),
                        value: BroadcastValue::Float(max_v),
                    });
                }
            }
            _ => {}
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/loudness" {
        if let Some(v) = parse_bool_arg(msg.args.first()) {
            ctx.renderer.live.write().unwrap().use_loudness = v;
            effects.mark_dirty = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/distance_model" {
        if let Some(OscType::String(model)) = msg.args.first() {
            if let Ok(model) = model.parse::<renderer::spatial_vbap::DistanceModel>() {
                ctx.renderer.live.write().unwrap().distance_model = model;
                effects.mark_dirty = true;
                effects.trigger_layout_recompute = true;
            }
        }
        return Some(effects);
    }

    if let Some(rest) = addr.strip_prefix("/omniphony/control/experimental_distance/") {
        let mut live = ctx.renderer.live.write().unwrap();
        let mut changed = false;
        match rest {
            "distance_floor" => {
                if let Some(v) = parse_f32_arg(msg.args.first()).map(|f| f.max(0.0)) {
                    live.experimental_distance.distance_floor = v;
                    changed = true;
                    effects.log_message =
                        Some(format!("OSC: experimental_distance/distance_floor -> {v}"));
                }
            }
            "min_active_speakers" => {
                if let Some(v) = parse_positive_u32_arg(msg.args.first()) {
                    live.experimental_distance.min_active_speakers = v as usize;
                    if live.experimental_distance.max_active_speakers
                        < live.experimental_distance.min_active_speakers
                    {
                        live.experimental_distance.max_active_speakers =
                            live.experimental_distance.min_active_speakers;
                    }
                    changed = true;
                    effects.log_message = Some(format!(
                        "OSC: experimental_distance/min_active_speakers -> {v}"
                    ));
                }
            }
            "max_active_speakers" => {
                if let Some(v) = parse_positive_u32_arg(msg.args.first()) {
                    live.experimental_distance.max_active_speakers =
                        (v as usize).max(live.experimental_distance.min_active_speakers);
                    changed = true;
                    effects.log_message = Some(format!(
                        "OSC: experimental_distance/max_active_speakers -> {}",
                        live.experimental_distance.max_active_speakers
                    ));
                }
            }
            "position_error_floor" => {
                if let Some(v) = parse_f32_arg(msg.args.first()).map(|f| f.max(0.0)) {
                    live.experimental_distance.position_error_floor = v;
                    changed = true;
                    effects.log_message = Some(format!(
                        "OSC: experimental_distance/position_error_floor -> {v}"
                    ));
                }
            }
            "position_error_nearest_scale" => {
                if let Some(v) = parse_f32_arg(msg.args.first()).map(|f| f.max(0.0)) {
                    live.experimental_distance.position_error_nearest_scale = v;
                    changed = true;
                    effects.log_message = Some(format!(
                        "OSC: experimental_distance/position_error_nearest_scale -> {v}"
                    ));
                }
            }
            "position_error_span_scale" => {
                if let Some(v) = parse_f32_arg(msg.args.first()).map(|f| f.max(0.0)) {
                    live.experimental_distance.position_error_span_scale = v;
                    changed = true;
                    effects.log_message = Some(format!(
                        "OSC: experimental_distance/position_error_span_scale -> {v}"
                    ));
                }
            }
            _ => {}
        }

        if changed {
            effects.mark_dirty = true;
            effects.trigger_layout_recompute = true;
        }
        return Some(effects);
    }

    if let Some(rest) = addr.strip_prefix("/omniphony/control/barycenter/") {
        let mut live = ctx.renderer.live.write().unwrap();
        let mut changed = false;
        match rest {
            "localize" => {
                if let Some(v) = parse_f32_arg(msg.args.first()).map(|f| f.max(0.0)) {
                    live.barycenter.localize = v;
                    changed = true;
                    effects.log_message = Some(format!("OSC: barycenter/localize -> {v}"));
                }
            }
            _ => {}
        }

        if changed {
            effects.mark_dirty = true;
            effects.trigger_layout_recompute = true;
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/room_ratio" {
        if msg.args.len() >= 3 {
            let w = parse_f32_arg(msg.args.first());
            let l = parse_f32_arg(msg.args.get(1));
            let h = parse_f32_arg(msg.args.get(2));
            if let (Some(w), Some(l), Some(h)) = (w, l, h) {
                ctx.renderer.live.write().unwrap().room_ratio = [w, l, h];
                effects.mark_dirty = true;
                effects.trigger_layout_recompute = true;
                effects.log_message = Some(format!("OSC: room_ratio → [{}, {}, {}]", w, l, h));
            }
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/room_ratio_rear" {
        if let Some(v) = parse_f32_arg(msg.args.first()).map(|f| f.max(0.01)) {
            ctx.renderer.live.write().unwrap().room_ratio_rear = v;
            effects.mark_dirty = true;
            effects.trigger_layout_recompute = true;
            effects.log_message = Some(format!("OSC: room_ratio_rear → {}", v));
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/room_ratio_lower" {
        if let Some(v) = parse_f32_arg(msg.args.first()).map(|f| f.max(0.01)) {
            ctx.renderer.live.write().unwrap().room_ratio_lower = v;
            effects.mark_dirty = true;
            effects.trigger_layout_recompute = true;
            effects.log_message = Some(format!("OSC: room_ratio_lower → {}", v));
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/room_ratio_center_blend" {
        if let Some(v) = parse_f32_arg(msg.args.first()).map(|f| f.clamp(0.0, 1.0)) {
            ctx.renderer.live.write().unwrap().room_ratio_center_blend = v;
            effects.mark_dirty = true;
            effects.trigger_layout_recompute = true;
            effects.log_message = Some(format!("OSC: room_ratio_center_blend → {}", v));
        }
        return Some(effects);
    }

    if let Some(rest) = addr.strip_prefix("/omniphony/control/distance_diffuse/") {
        match rest {
            "enabled" => {
                if let Some(v) = parse_bool_arg(msg.args.first()) {
                    ctx.renderer.live.write().unwrap().use_distance_diffuse = v;
                    effects.mark_dirty = true;
                    effects.trigger_layout_recompute = true;
                }
                return Some(effects);
            }
            "threshold" => {
                if let Some(v) = parse_f32_arg(msg.args.first()).map(|f| f.max(1e-6)) {
                    ctx.renderer
                        .live
                        .write()
                        .unwrap()
                        .distance_diffuse_threshold = v;
                    effects.mark_dirty = true;
                    effects.trigger_layout_recompute = true;
                }
                return Some(effects);
            }
            "curve" => {
                if let Some(v) = parse_f32_arg(msg.args.first()).map(|f| f.max(0.0)) {
                    ctx.renderer.live.write().unwrap().distance_diffuse_curve = v;
                    effects.mark_dirty = true;
                    effects.trigger_layout_recompute = true;
                }
                return Some(effects);
            }
            _ => {}
        }
    }

    if let Some(rest) = addr.strip_prefix("/omniphony/control/object/") {
        if let Some(idx_str) = rest.strip_suffix("/gain") {
            if let Ok(idx) = idx_str.parse::<usize>() {
                if let Some(gain) =
                    parse_f32_arg(msg.args.first()).map(|value| value.clamp(0.0, 2.0))
                {
                    ctx.renderer
                        .live
                        .write()
                        .unwrap()
                        .objects
                        .entry(idx)
                        .or_default()
                        .gain = gain;
                    ctx.renderer.mark_object_params_dirty();
                    effects.mark_dirty = true;
                    effects.broadcasts.push(BroadcastUpdate {
                        addr: format!("/omniphony/state/object/{}/gain", idx),
                        value: BroadcastValue::Float(gain),
                    });
                    effects.log_message = Some(format!("OSC: object[{}] gain → {}", idx, gain));
                }
            }
            return Some(effects);
        }
        if let Some(idx_str) = rest.strip_suffix("/mute") {
            if let Ok(idx) = idx_str.parse::<usize>() {
                if let Some(muted) = parse_bool_arg(msg.args.first()) {
                    ctx.renderer
                        .live
                        .write()
                        .unwrap()
                        .objects
                        .entry(idx)
                        .or_default()
                        .muted = muted;
                    ctx.renderer.mark_object_params_dirty();
                    effects.mark_dirty = true;
                    effects.broadcasts.push(BroadcastUpdate {
                        addr: format!("/omniphony/state/object/{}/mute", idx),
                        value: BroadcastValue::Int(if muted { 1 } else { 0 }),
                    });
                    effects.log_message = Some(format!("OSC: object[{}] mute → {}", idx, muted));
                }
            }
            return Some(effects);
        }
    }

    None
}

pub fn apply_speaker_osc_control(
    msg: &OscMessage,
    ctx: &RuntimeControlContext,
    pending_speakers: &mut HashMap<usize, SpeakerPatch>,
) -> Option<ControlEffects> {
    let addr = msg.addr.as_str();
    let mut effects = ControlEffects::default();

    if addr == "/omniphony/control/speakers/add" {
        pending_speakers.clear();
        let idx = ctx.renderer.editable_layout().speakers.len();
        let default_name = format!("spk-{}", idx);
        let name = match msg.args.first() {
            Some(OscType::String(s)) if !s.trim().is_empty() => s.trim().to_string(),
            _ => default_name,
        };
        let az = parse_f32_arg(msg.args.get(1)).unwrap_or(0.0);
        let el = parse_f32_arg(msg.args.get(2)).unwrap_or(0.0);
        let distance = parse_f32_arg(msg.args.get(3)).unwrap_or(1.0).max(0.01);
        let spatialize = parse_bool_arg(msg.args.get(4)).unwrap_or(true);
        let delay_ms = parse_f32_arg(msg.args.get(5)).unwrap_or(0.0).max(0.0);
        ctx.renderer.with_editable_layout(|layout| {
            layout
                .speakers
                .push(renderer::speaker_layout::Speaker::from_polar(
                    name,
                    az.clamp(-180.0, 180.0),
                    el.clamp(-90.0, 90.0),
                    distance,
                    spatialize,
                    delay_ms,
                ));
            layout.clone()
        });
        if delay_ms > 0.0 {
            ctx.renderer
                .live
                .write()
                .unwrap()
                .speakers
                .entry(idx)
                .or_default()
                .delay_ms = delay_ms;
            ctx.renderer.mark_speaker_params_dirty();
        }
        effects.mark_dirty = true;
        effects.trigger_layout_recompute = true;
        return Some(effects);
    }

    if addr == "/omniphony/control/speakers/remove" {
        pending_speakers.clear();
        let remove_idx = match msg.args.first() {
            Some(OscType::Int(v)) if *v >= 0 => *v as usize,
            Some(OscType::Float(v)) if *v >= 0.0 => *v as usize,
            _ => return Some(effects),
        };
        let Some(_) = ctx.renderer.with_editable_layout(|layout| {
            if remove_idx >= layout.speakers.len() {
                return None;
            }
            layout.speakers.remove(remove_idx);
            Some(layout.clone())
        }) else {
            return Some(effects);
        };
        {
            let mut live = ctx.renderer.live.write().unwrap();
            remap_live_speakers_remove(&mut live.speakers, remove_idx);
        }
        ctx.renderer.mark_speaker_params_dirty();
        effects.mark_dirty = true;
        effects.trigger_layout_recompute = true;
        return Some(effects);
    }

    if addr == "/omniphony/control/speakers/move" {
        pending_speakers.clear();
        let from_idx = match msg.args.first() {
            Some(OscType::Int(v)) if *v >= 0 => *v as usize,
            Some(OscType::Float(v)) if *v >= 0.0 => *v as usize,
            _ => return Some(effects),
        };
        let to_idx = match msg.args.get(1) {
            Some(OscType::Int(v)) if *v >= 0 => *v as usize,
            Some(OscType::Float(v)) if *v >= 0.0 => *v as usize,
            _ => return Some(effects),
        };
        let Some(_) = ctx.renderer.with_editable_layout(|layout| {
            let len = layout.speakers.len();
            if from_idx >= len || to_idx >= len || from_idx == to_idx {
                return None;
            }
            let speaker = layout.speakers.remove(from_idx);
            layout.speakers.insert(to_idx, speaker);
            Some(layout.clone())
        }) else {
            return Some(effects);
        };
        {
            let mut live = ctx.renderer.live.write().unwrap();
            remap_live_speakers_move(&mut live.speakers, from_idx, to_idx);
        }
        ctx.renderer.mark_speaker_params_dirty();
        effects.mark_dirty = true;
        effects.trigger_layout_recompute = true;
        return Some(effects);
    }

    if let Some(rest) = addr.strip_prefix("/omniphony/control/speaker/") {
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Some(effects);
        }
        let Ok(idx) = parts[0].parse::<usize>() else {
            return Some(effects);
        };
        let field = parts[1];
        if field == "mute" {
            if let Some(muted) = parse_bool_arg(msg.args.first()) {
                ctx.renderer
                    .live
                    .write()
                    .unwrap()
                    .speakers
                    .entry(idx)
                    .or_default()
                    .muted = muted;
                ctx.renderer.mark_speaker_params_dirty();
                effects.mark_dirty = true;
                effects.log_message = Some(format!("OSC: speaker[{}] mute → {}", idx, muted));
            }
            return Some(effects);
        }
        if field == "spatialize" {
            if let Some(spatialize) = parse_bool_arg(msg.args.first()) {
                let patch = pending_speakers.entry(idx).or_default();
                patch.spatialize = Some(spatialize);
            }
            return Some(effects);
        }
        if field == "name" {
            if let Some(OscType::String(name)) = msg.args.first() {
                let trimmed = name.trim();
                if !trimmed.is_empty() {
                    let patch = pending_speakers.entry(idx).or_default();
                    patch.name = Some(trimmed.to_string());
                }
            }
            return Some(effects);
        }
        if field == "freq_low" {
            let patch = pending_speakers.entry(idx).or_default();
            patch.freq_low = Some(parse_f32_arg(msg.args.first()).filter(|v| *v > 0.0));
            return Some(effects);
        }
        if field == "freq_high" {
            let patch = pending_speakers.entry(idx).or_default();
            patch.freq_high = Some(parse_f32_arg(msg.args.first()).filter(|v| *v > 0.0));
            return Some(effects);
        }
        if field == "coord_mode" {
            if let Some(OscType::String(mode)) = msg.args.first() {
                let normalized = if mode.eq_ignore_ascii_case("cartesian") {
                    "cartesian"
                } else {
                    "polar"
                };
                let patch = pending_speakers.entry(idx).or_default();
                patch.coord_mode = Some(normalized.to_string());
            }
            return Some(effects);
        }
        if let Some(f) = parse_f32_arg(msg.args.first()) {
            let patch = pending_speakers.entry(idx).or_default();
            match field {
                "az" => patch.az = Some(f),
                "el" => patch.el = Some(f),
                "distance" => patch.distance = Some(f),
                "x" => patch.x = Some(f.clamp(-1.0, 1.0)),
                "y" => patch.y = Some(f.clamp(-1.0, 1.0)),
                "z" => patch.z = Some(f.clamp(-1.0, 1.0)),
                "gain" => {
                    ctx.renderer
                        .live
                        .write()
                        .unwrap()
                        .speakers
                        .entry(idx)
                        .or_default()
                        .gain = f;
                    ctx.renderer.mark_speaker_params_dirty();
                    effects.mark_dirty = true;
                }
                "delay" => {
                    let delay_ms = f.max(0.0);
                    ctx.renderer
                        .live
                        .write()
                        .unwrap()
                        .speakers
                        .entry(idx)
                        .or_default()
                        .delay_ms = delay_ms;
                    ctx.renderer.mark_speaker_params_dirty();
                    ctx.renderer.with_editable_layout(|layout| {
                        if let Some(spk) = layout.speakers.get_mut(idx) {
                            spk.delay_ms = delay_ms;
                        }
                    });
                    effects.mark_dirty = true;
                    effects.log_message =
                        Some(format!("OSC: speaker[{}] delay → {:.2} ms", idx, delay_ms));
                }
                _ => {}
            }
        }
        return Some(effects);
    }

    if addr == "/omniphony/control/speakers/apply" {
        apply_pending_speakers(pending_speakers, ctx);
        effects.mark_dirty = true;
        effects.trigger_layout_recompute = true;
        return Some(effects);
    }

    if addr == "/omniphony/control/speakers/reset" {
        pending_speakers.clear();
        return Some(effects);
    }

    None
}

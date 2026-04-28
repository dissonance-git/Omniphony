use std::sync::Arc;

use audio_input::{
    InputBackend, InputClockMode, InputControl, InputLfeMode, InputMapMode, InputMode,
    InputSampleFormat,
};
use audio_output::AudioControl;
use renderer::live_params::{LiveParams, RenderTopology, RendererControl};
use rosc::{OscBundle, OscMessage, OscPacket, OscTime, OscType};
use serde::Serialize;
use serde_json::json;

fn input_mode_name(mode: InputMode) -> &'static str {
    match mode {
        InputMode::Bridge => "pipe_bridge",
        InputMode::Live => "pipewire",
        InputMode::PipewireBridge => "pipewire_bridge",
    }
}

fn input_backend_name(backend: InputBackend) -> &'static str {
    match backend {
        InputBackend::Pipewire => "pipewire",
        InputBackend::Asio => "asio",
    }
}

fn input_map_mode_name(mode: InputMapMode) -> &'static str {
    match mode {
        InputMapMode::SevenOneFixed => "7.1-fixed",
    }
}

fn input_lfe_mode_name(mode: InputLfeMode) -> &'static str {
    match mode {
        InputLfeMode::Object => "object",
        InputLfeMode::Direct => "direct",
        InputLfeMode::Drop => "drop",
    }
}

fn input_sample_format_name(format: InputSampleFormat) -> &'static str {
    match format {
        InputSampleFormat::F32 => "f32",
        InputSampleFormat::S16 => "s16",
    }
}

fn input_clock_mode_name(mode: InputClockMode) -> &'static str {
    match mode {
        InputClockMode::Dac => "dac",
        InputClockMode::Pipewire => "pipewire",
        InputClockMode::Upstream => "upstream",
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ExperimentalDistanceOptionsSnapshot {
    pub distance_floor: f32,
    pub min_active_speakers: usize,
    pub max_active_speakers: usize,
    pub position_error_floor: f32,
    pub position_error_nearest_scale: f32,
    pub position_error_span_scale: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct BarycenterOptionsSnapshot {
    pub localize: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct RenderBackendStateSnapshot {
    pub selection: String,
    pub effective: String,
    pub effective_label: String,
    pub capabilities: renderer::render_backend::BackendCapabilities,
    pub allowed_evaluation_modes: Vec<String>,
    pub frozen_room_ratio: bool,
    pub frozen_speakers: bool,
    pub restore_backend_available: bool,
    pub barycenter: BarycenterOptionsSnapshot,
    pub experimental_distance: ExperimentalDistanceOptionsSnapshot,
}

fn allowed_evaluation_modes(
    backend: &renderer::render_backend::PreparedRenderEngine,
    capabilities: renderer::render_backend::BackendCapabilities,
) -> Vec<String> {
    let _ = backend;
    let mut modes = vec!["auto".to_string()];
    if capabilities.supports_realtime {
        modes.push("realtime".to_string());
    }
    if capabilities.supports_precomputed_polar {
        modes.push("precomputed_polar".to_string());
    }
    if capabilities.supports_precomputed_cartesian {
        modes.push("precomputed_cartesian".to_string());
    }
    modes
}

pub fn build_render_backend_state_snapshot(
    live: &LiveParams,
    active_topology: &RenderTopology,
) -> RenderBackendStateSnapshot {
    let backend = &active_topology.backend;
    let capabilities = backend.capabilities();
    RenderBackendStateSnapshot {
        selection: live.backend_id().to_string(),
        effective: backend.backend_id().to_string(),
        effective_label: backend.backend_label().to_string(),
        capabilities,
        allowed_evaluation_modes: allowed_evaluation_modes(backend, capabilities),
        frozen_room_ratio: false,
        frozen_speakers: false,
        restore_backend_available: false,
        barycenter: BarycenterOptionsSnapshot {
            localize: live.barycenter.localize,
        },
        experimental_distance: ExperimentalDistanceOptionsSnapshot {
            distance_floor: live.experimental_distance.distance_floor,
            min_active_speakers: live.experimental_distance.min_active_speakers,
            max_active_speakers: live.experimental_distance.max_active_speakers,
            position_error_floor: live.experimental_distance.position_error_floor,
            position_error_nearest_scale: live.experimental_distance.position_error_nearest_scale,
            position_error_span_scale: live.experimental_distance.position_error_span_scale,
        },
    }
}

pub fn build_render_backend_state_json(
    live: &LiveParams,
    active_topology: &RenderTopology,
) -> String {
    serde_json::to_string(&build_render_backend_state_snapshot(live, active_topology))
        .unwrap_or_else(|_| "{}".to_string())
}

pub fn build_renderer_state_json(live: &LiveParams, active_topology: &RenderTopology) -> String {
    let effective_backend = active_topology.backend.kind().as_str();
    let effective_evaluation_mode = active_topology.backend.evaluation_mode().as_str();
    let render_backend_state_json = build_render_backend_state_json(live, active_topology);
    json!({
        "renderBackend": live.backend_id(),
        "renderBackendEffective": effective_backend,
        "renderEvaluationMode": live.requested_evaluation_mode().as_str(),
        "renderEvaluationModeEffective": effective_evaluation_mode,
        "masterGain": live.master_gain,
        "rampMode": live.ramp_mode.as_str(),
        "distanceModel": live.distance_model.to_string(),
        "roomRatio": {
            "width": live.room_ratio[0],
            "length": live.room_ratio[1],
            "height": live.room_ratio[2],
            "rear": live.room_ratio_rear,
            "lower": live.room_ratio_lower,
            "centerBlend": live.room_ratio_center_blend
        },
        "spread": {
            "min": live.spread_min,
            "max": live.spread_max,
            "fromDistance": live.spread_from_distance,
            "distanceRange": live.spread_distance_range,
            "distanceCurve": live.spread_distance_curve
        },
        "distanceDiffuse": {
            "enabled": live.use_distance_diffuse,
            "threshold": live.distance_diffuse_threshold,
            "curve": live.distance_diffuse_curve
        },
        "renderBackendState": serde_json::from_str::<serde_json::Value>(&render_backend_state_json)
            .unwrap_or_else(|_| json!({}))
    })
    .to_string()
}

fn build_renderer_capabilities_json() -> String {
    json!({
        "producer": "renderer",
        "domains": ["renderer", "audio", "layout", "speakers", "input", "loudness"],
        "realtime": ["master_gain", "speaker_gain", "object_gain"],
        "spatial": true,
        "metering": true,
        "controlConfig": ["audio", "input", "adaptive_resampling", "layout", "speakers"]
    })
    .to_string()
}

pub fn build_speakers_state_json(
    live: &LiveParams,
    layout: &renderer::speaker_layout::SpeakerLayout,
) -> String {
    let speakers = layout
        .speakers
        .iter()
        .enumerate()
        .map(|(idx, speaker)| {
            let live_state = live.speakers.get(&idx);
            json!({
                "id": idx,
                "gain": live_state.map(|state| state.gain).unwrap_or(1.0),
                "delayMs": live_state
                    .map(|state| state.delay_ms)
                    .unwrap_or(speaker.delay_ms)
                    .max(0.0),
                "muted": live_state.map(|state| state.muted).unwrap_or(false)
            })
        })
        .collect::<Vec<_>>();
    json!({ "speakers": speakers }).to_string()
}

pub fn build_live_state_bundle(
    control: &Arc<RendererControl>,
    audio_control: Option<&Arc<AudioControl>>,
    input_control: Option<&Arc<InputControl>>,
) -> Vec<u8> {
    let live = control.live.read().unwrap();
    let active_topology = control.active_topology();
    let editable_layout = control.editable_layout();
    let layout_json = serde_json::to_string(&editable_layout).unwrap_or_else(|_| "{}".to_string());
    let speakers_state_json = build_speakers_state_json(&live, &editable_layout);
    let loudness_gain: f32 = match (live.use_loudness, live.dialogue_level) {
        (true, Some(dl)) => 10.0_f32.powf((-31 - dl as i32) as f32 / 20.0),
        _ => 1.0,
    };
    let renderer_state_json = build_renderer_state_json(&live, &active_topology);

    let mut messages = vec![
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/capabilities".to_string(),
            args: vec![OscType::String(build_renderer_capabilities_json())],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/renderer".to_string(),
            args: vec![OscType::String(renderer_state_json)],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/layout".to_string(),
            args: vec![OscType::String(layout_json)],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/speakers".to_string(),
            args: vec![OscType::String(speakers_state_json)],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/loudness".to_string(),
            args: vec![OscType::String(
                json!({
                    "enabled": live.use_loudness,
                    "source": live.dialogue_level,
                    "gain": loudness_gain
                })
                .to_string(),
            )],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/render_evaluation/cartesian/x_size".to_string(),
            args: vec![OscType::Int(live.evaluation.cartesian.x_size as i32)],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/render_evaluation/cartesian/y_size".to_string(),
            args: vec![OscType::Int(live.evaluation.cartesian.y_size as i32)],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/render_evaluation/cartesian/z_size".to_string(),
            args: vec![OscType::Int(live.evaluation.cartesian.z_size as i32)],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/render_evaluation/cartesian/z_neg_size".to_string(),
            args: vec![OscType::Int(live.evaluation.cartesian.z_neg_size as i32)],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/render_evaluation/position_interpolation".to_string(),
            args: vec![OscType::Int(if live.evaluation.position_interpolation {
                1
            } else {
                0
            })],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/log_level".to_string(),
            args: vec![OscType::String(
                sys::live_log::current_runtime_level_name().to_string(),
            )],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/render_evaluation/polar/azimuth_resolution".to_string(),
            args: vec![OscType::Int(live.evaluation.polar.azimuth_values.max(1))],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/render_evaluation/polar/elevation_resolution".to_string(),
            args: vec![OscType::Int(live.evaluation.polar.elevation_values.max(1))],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/render_evaluation/polar/distance_res".to_string(),
            args: vec![OscType::Int(live.evaluation.polar.distance_res.max(1))],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/render_evaluation/polar/distance_max".to_string(),
            args: vec![OscType::Float(live.evaluation.polar.distance_max.max(0.01))],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/vbap/allow_negative_z".to_string(),
            args: vec![OscType::Int(
                if control
                    .backend_rebuild_params()
                    .map(|p| p.allow_negative_z)
                    .unwrap_or(true)
                {
                    1
                } else {
                    0
                },
            )],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/config/saved".to_string(),
            args: vec![OscType::Int(
                if control
                    .config_dirty
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    0
                } else {
                    1
                },
            )],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/input_pipe".to_string(),
            args: vec![OscType::String(control.input_path().unwrap_or_default())],
        }),
        OscPacket::Message(OscMessage {
            addr: "/omniphony/state/render/bridge_path".to_string(),
            args: vec![OscType::String(
                control
                    .bridge_path()
                    .map(|path| path.display().to_string())
                    .unwrap_or_default(),
            )],
        }),
    ];

    if let Some(audio_control) = audio_control {
        let requested = audio_control.requested_snapshot();
        let requested_output_device = requested.output_device.clone().unwrap_or_default();
        messages.push(OscPacket::Message(OscMessage {
            addr: "/omniphony/state/audio".to_string(),
            args: vec![OscType::String(
                json!({
                    "outputDevices": audio_control.available_output_devices(),
                    "outputDevice": requested.output_device.clone(),
                    "outputDeviceEffective": audio_control.effective_output_device(),
                    "sampleRate": requested.output_sample_rate_hz,
                    "sampleFormat": audio_control.audio_state().1,
                    "error": audio_control.audio_error(),
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
                .to_string(),
            )],
        }));
        let _ = requested_output_device;
    }

    if let Some(input_control) = input_control {
        let requested = input_control.requested_snapshot();
        let applied = input_control.applied_snapshot();
        messages.push(OscPacket::Message(OscMessage {
            addr: "/omniphony/state/input".to_string(),
            args: vec![OscType::String(
                json!({
                    "mode": input_mode_name(requested.mode),
                    "activeMode": input_mode_name(applied.active_mode),
                    "applyPending": input_control.is_apply_pending(),
                    "requested": {
                        "backend": requested.backend.map(input_backend_name),
                        "node": requested.node_name.clone(),
                        "description": requested.node_description.clone(),
                        "layout": requested.layout_path.as_ref().map(|path| path.display().to_string()),
                        "clockMode": input_clock_mode_name(requested.clock_mode),
                        "channels": requested.channels,
                        "sampleRate": requested.sample_rate_hz,
                        "format": requested.sample_format.map(input_sample_format_name),
                        "map": input_map_mode_name(requested.map_mode),
                        "lfeMode": input_lfe_mode_name(requested.lfe_mode)
                    },
                    "applied": {
                        "backend": applied.backend.map(input_backend_name),
                        "channels": applied.channels,
                        "sampleRate": applied.sample_rate_hz,
                        "node": applied.node_name.clone(),
                        "description": applied.node_description.clone(),
                        "streamFormat": applied.stream_format.clone(),
                        "error": applied.input_error.clone()
                    }
                })
                .to_string(),
            )],
        }));
        let _ = (requested, applied);
    }

    let mut all_messages = messages;

    for (&idx, obj) in &live.objects {
        if obj.gain != 1.0 {
            all_messages.push(OscPacket::Message(OscMessage {
                addr: format!("/omniphony/state/object/{}/gain", idx),
                args: vec![OscType::Float(obj.gain)],
            }));
        }
        if obj.muted {
            all_messages.push(OscPacket::Message(OscMessage {
                addr: format!("/omniphony/state/object/{}/mute", idx),
                args: vec![OscType::Int(1)],
            }));
        }
    }

    all_messages.push(OscPacket::Message(OscMessage {
        addr: "/omniphony/state/snapshot_complete".to_string(),
        args: vec![OscType::Int(1)],
    }));

    let bundle = OscPacket::Bundle(OscBundle {
        timetag: OscTime {
            seconds: 0,
            fractional: 1,
        },
        content: all_messages,
    });

    rosc::encoder::encode(&bundle).unwrap_or_default()
}

use anyhow::Result;
use rosc::{OscBundle, OscMessage, OscPacket, OscTime, OscType};
use serde_json::json;

use super::OscSender;
use super::export::build_live_state_bundle;
impl OscSender {
    pub fn send_live_state_bundle(&self) -> Result<()> {
        let control = match self.control {
            Some(ref c) => c,
            None => return Ok(()),
        };
        let bytes = build_live_state_bundle(
            control,
            self.audio_control.as_ref(),
            self.input_control.as_ref(),
        );
        self.send_to_all(&bytes);
        Ok(())
    }

    pub fn send_loudness_state(&self) {
        let control = match self.control {
            Some(ref c) => c,
            None => return,
        };
        let live = control.live.read().unwrap();
        let socket = &self.socket;
        let clients = &self.clients;

        let gain_linear: f32 = match (live.use_loudness, live.dialogue_level) {
            (true, Some(dl)) => 10.0_f32.powf((-31 - dl as i32) as f32 / 20.0),
            _ => 1.0,
        };
        let payload = json!({
            "enabled": live.use_loudness,
            "source": live.dialogue_level,
            "gain": gain_linear
        })
        .to_string();
        super::transport::broadcast_string(socket, clients, "/omniphony/state/loudness", &payload);
    }

    pub fn send_meter_bundle(
        &self,
        snapshot: &renderer::metering::MeterSnapshot,
        object_gains: &[(usize, renderer::spatial_vbap::Gains)],
        object_band_gains: &[(usize, Vec<renderer::spatial_vbap::Gains>)],
        decode_time_ms: Option<f32>,
        crossover_time_ms: Option<f32>,
        render_time_ms: Option<f32>,
        write_time_ms: Option<f32>,
        frame_duration_ms: Option<f32>,
        latency_instant_ms: Option<f32>,
        latency_control_ms: Option<f32>,
        latency_smoothed_ms: Option<f32>,
        latency_target_ms: Option<f32>,
        latency_downstream_ms: Option<f32>,
        latency_avail_input_ms: Option<f32>,
        latency_output_fifo_ms: Option<f32>,
        latency_resampler_pending_ms: Option<f32>,
        diag_schema_json: Option<String>,
        diag_values_json: Option<String>,
        resample_ratio: Option<f32>,
        adaptive_band: Option<&str>,
        adaptive_state: Option<&str>,
        drc_gain: Option<f32>,
    ) -> Result<()> {
        let max_gain_id = object_gains.iter().map(|(idx, _)| *idx).max().unwrap_or(0);
        let mut gains_by_id: Vec<Option<&renderer::spatial_vbap::Gains>> =
            vec![None; max_gain_id.saturating_add(1)];
        for (idx, g) in object_gains {
            if *idx < gains_by_id.len() {
                gains_by_id[*idx] = Some(g);
            }
        }

        let max_band_id = object_band_gains
            .iter()
            .map(|(idx, _)| *idx)
            .max()
            .unwrap_or(0);
        let mut band_gains_by_id: Vec<Option<&Vec<renderer::spatial_vbap::Gains>>> =
            vec![None; max_band_id.saturating_add(1)];
        for (idx, bg) in object_band_gains {
            if *idx < band_gains_by_id.len() {
                band_gains_by_id[*idx] = Some(bg);
            }
        }

        let mut messages = Vec::with_capacity(
            snapshot.object_levels.len() * 2 + snapshot.speaker_levels.len() + 1,
        );
        if let Some(ms) = latency_control_ms.or(latency_instant_ms).or(latency_target_ms) {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/latency".to_string(),
                args: vec![OscType::Float(ms)],
            }));
        }
        if let Some(ms) = decode_time_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/decode_time_ms".to_string(),
                args: vec![OscType::Float(ms.max(0.0))],
            }));
        }
        if let Some(ms) = render_time_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/render_time_ms".to_string(),
                args: vec![OscType::Float(ms.max(0.0))],
            }));
        }
        if let Some(ms) = crossover_time_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/crossover_time_ms".to_string(),
                args: vec![OscType::Float(ms.max(0.0))],
            }));
        }
        if let Some(ms) = write_time_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/write_time_ms".to_string(),
                args: vec![OscType::Float(ms.max(0.0))],
            }));
        }
        if let Some(ms) = frame_duration_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/frame_duration_ms".to_string(),
                args: vec![OscType::Float(ms.max(0.0))],
            }));
        }
        if let Some(ms) = latency_instant_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/latency_instant".to_string(),
                args: vec![OscType::Float(ms)],
            }));
        }
        if let Some(ms) = latency_control_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/latency_control".to_string(),
                args: vec![OscType::Float(ms)],
            }));
        }
        if let Some(ms) = latency_smoothed_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/latency_smoothed".to_string(),
                args: vec![OscType::Float(ms)],
            }));
        }
        if let Some(ms) = latency_target_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/latency_target".to_string(),
                args: vec![OscType::Float(ms)],
            }));
        }
        if let Some(ms) = latency_downstream_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/latency_downstream".to_string(),
                args: vec![OscType::Float(ms)],
            }));
        }
        if let Some(ms) = latency_avail_input_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/latency_avail_input".to_string(),
                args: vec![OscType::Float(ms)],
            }));
        }
        if let Some(ms) = latency_output_fifo_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/latency_output_fifo".to_string(),
                args: vec![OscType::Float(ms)],
            }));
        }
        if let Some(ms) = latency_resampler_pending_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/latency_resampler_pending".to_string(),
                args: vec![OscType::Float(ms)],
            }));
        }
        if let Some(json) = diag_schema_json {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/diag_schema".to_string(),
                args: vec![OscType::String(json)],
            }));
        }
        if let Some(json) = diag_values_json {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/diag_values".to_string(),
                args: vec![OscType::String(json)],
            }));
        }
        if let Some(ratio) = resample_ratio {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/resample_ratio".to_string(),
                args: vec![OscType::Float(ratio)],
            }));
        }
        if let Some(band) = adaptive_band {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/adaptive_resampling/band".to_string(),
                args: vec![OscType::String(band.to_string())],
            }));
        }
        if let Some(state) = adaptive_state {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/adaptive_resampling/state".to_string(),
                args: vec![OscType::String(state.to_string())],
            }));
        }
        if let Some(gain) = drc_gain {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/meter/drc_gain".to_string(),
                args: vec![OscType::Float(gain)],
            }));
        }

        for &(id, peak, rms) in &snapshot.object_levels {
            messages.push(OscPacket::Message(OscMessage {
                addr: format!("/omniphony/meter/object/{}", id),
                args: vec![OscType::Float(peak), OscType::Float(rms)],
            }));
            if let Some(gains) = gains_by_id.get(id as usize).and_then(|entry| *entry) {
                messages.push(OscPacket::Message(OscMessage {
                    addr: format!("/omniphony/meter/object/{}/gains", id),
                    args: gains.iter().map(|&g| OscType::Float(g)).collect(),
                }));
            }
            if let Some(bands) = band_gains_by_id.get(id as usize).and_then(|entry| *entry) {
                for (b, bg) in bands.iter().enumerate() {
                    messages.push(OscPacket::Message(OscMessage {
                        addr: format!("/omniphony/meter/object/{}/band/{}/gains", id, b),
                        args: bg.iter().map(|&g| OscType::Float(g)).collect(),
                    }));
                }
            }
        }
        for (idx, &(peak, rms)) in snapshot.speaker_levels.iter().enumerate() {
            messages.push(OscPacket::Message(OscMessage {
                addr: format!("/omniphony/meter/speaker/{}", idx),
                args: vec![OscType::Float(peak), OscType::Float(rms)],
            }));
        }

        let bundle = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: messages,
        });

        let bytes = rosc::encoder::encode(&bundle)?;
        self.send_to_metering_clients(&bytes);
        Ok(())
    }

    pub fn send_timing_update(
        &self,
        decode_time_ms: Option<f32>,
        render_time_ms: Option<f32>,
        write_time_ms: Option<f32>,
    ) -> Result<()> {
        let mut messages = Vec::new();
        if let Some(ms) = decode_time_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/decode_time_ms".to_string(),
                args: vec![OscType::Float(ms.max(0.0))],
            }));
        }
        if let Some(ms) = render_time_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/render_time_ms".to_string(),
                args: vec![OscType::Float(ms.max(0.0))],
            }));
        }
        if let Some(ms) = write_time_ms {
            messages.push(OscPacket::Message(OscMessage {
                addr: "/omniphony/state/write_time_ms".to_string(),
                args: vec![OscType::Float(ms.max(0.0))],
            }));
        }
        if messages.is_empty() {
            return Ok(());
        }
        let packet = OscPacket::Bundle(OscBundle {
            timetag: OscTime::from((0, 1)),
            content: messages,
        });
        let bytes = rosc::encoder::encode(&packet)?;
        self.send_to_metering_clients(&bytes);
        Ok(())
    }

    pub fn send_audio_state(&self, sample_rate_hz: u32, sample_format: &str) -> Result<()> {
        let output_device = self
            .audio_control
            .as_ref()
            .and_then(|control| control.requested_output_device());
        let output_device_effective = self
            .audio_control
            .as_ref()
            .and_then(|control| control.effective_output_device());
        let audio_error = self
            .audio_control
            .as_ref()
            .and_then(|control| control.audio_error());
        let output_devices = self
            .audio_control
            .as_ref()
            .map(|control| control.available_output_devices())
            .unwrap_or_default();
        let requested_latency_target_ms = self
            .audio_control
            .as_ref()
            .and_then(|control| control.requested_latency_target_ms());
        let announced_rate = self
            .audio_control
            .as_ref()
            .and_then(|control| control.requested_output_sample_rate())
            .unwrap_or(sample_rate_hz);
        let payload = json!({
            "outputDevices": output_devices,
            "outputDevice": output_device,
            "outputDeviceEffective": output_device_effective,
            "sampleRate": announced_rate,
            "sampleFormat": sample_format,
            "error": audio_error,
            "latencyTargetMs": requested_latency_target_ms
        })
        .to_string();
        let content = vec![OscPacket::Message(OscMessage {
            addr: "/omniphony/state/audio".to_string(),
            args: vec![OscType::String(payload)],
        })];
        let bundle = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content,
        });
        let bytes = rosc::encoder::encode(&bundle)?;
        self.send_to_all(&bytes);
        Ok(())
    }
}

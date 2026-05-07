use anyhow::{Result, anyhow};
use bridge_api::{FormatBridgeBox, RDecodedFrame, RInputTransport};
use spdif::SpdifParser;
use std::sync::mpsc;
use std::thread;
use std::time::Instant;

pub struct LiveBridgeIngestRuntime {
    raw_tx: mpsc::SyncSender<(u8, Vec<u8>)>,
    spdif_parser: SpdifParser,
}

impl LiveBridgeIngestRuntime {
    pub fn new(raw_tx: mpsc::SyncSender<(u8, Vec<u8>)>) -> Self {
        Self {
            raw_tx,
            spdif_parser: SpdifParser::new(),
        }
    }

    pub fn process_chunk(&mut self, chunk: &[u8]) -> (usize, usize) {
        let mut packet_count = 0usize;
        self.spdif_parser.push_bytes(chunk);
        while let Some(packet) = self.spdif_parser.get_next_packet() {
            packet_count += 1;
            let _ = self.raw_tx.try_send((packet.data_type, packet.payload));
        }
        if packet_count > 0 {
            log::info!(
                "LiveBridgeIngestRuntime: extracted {} SPDIF packet(s)",
                packet_count
            );
        }
        (packet_count, 0)
    }
}

pub fn spawn_bridge_decode_worker<OnFrame, OnFlush, OnFatal>(
    bridge: FormatBridgeBox,
    raw_rx: mpsc::Receiver<(u8, Vec<u8>)>,
    strict_mode: bool,
    mut on_frame: OnFrame,
    mut on_flush: OnFlush,
    mut on_fatal: OnFatal,
) -> Result<thread::JoinHandle<()>>
where
    OnFrame: FnMut(RDecodedFrame, f32) + Send + 'static,
    OnFlush: FnMut() + Send + 'static,
    OnFatal: FnMut(anyhow::Error) + Send + 'static,
{
    thread::Builder::new()
        .name("bridge-decode".to_string())
        .spawn(move || {
            let mut bridge = bridge;
            let mut first_frame_logs_remaining = 16usize;
            while let Ok((data_type, payload)) = raw_rx.recv() {
                let decode_started_at = Instant::now();
                let result = bridge.push_packet(
                    payload.as_slice().into(),
                    RInputTransport::Iec61937,
                    data_type,
                );
                let decode_time_ms = decode_started_at.elapsed().as_secs_f32() * 1000.0;
                if !result.error_message.is_empty() || result.did_reset {
                    log::warn!(
                        "PipeWire bridge packet: data_type=0x{:02X} payload_bytes={} frames={} reset={} error={}",
                        data_type,
                        payload.len(),
                        result.frames.len(),
                        result.did_reset,
                        result.error_message
                    );
                } else if result.frames.is_empty() {
                    log::info!(
                        "PipeWire bridge packet: data_type=0x{:02X} payload_bytes={} frames={} reset={} error={}",
                        data_type,
                        payload.len(),
                        result.frames.len(),
                        result.did_reset,
                        result.error_message
                    );
                }
                if result.did_reset {
                    if strict_mode && !result.error_message.is_empty() {
                        on_fatal(anyhow!("{}", result.error_message));
                        return;
                    }
                    if strict_mode {
                        on_flush();
                    }
                }
                let frame_count = result.frames.len().max(1) as f32;
                let per_frame_decode_time_ms = decode_time_ms / frame_count;
                if data_type == 0x15 {
                    let emitted_samples: u32 =
                        result.frames.iter().map(|frame| frame.sample_count).sum();
                    let emitted_ms: f64 = result
                        .frames
                        .iter()
                        .map(|frame| {
                            let rate = frame.sampling_frequency.max(1) as f64;
                            frame.sample_count as f64 / rate * 1000.0
                        })
                        .sum();
                    let sample_count_min =
                        result.frames.iter().map(|frame| frame.sample_count).min();
                    let sample_count_max =
                        result.frames.iter().map(|frame| frame.sample_count).max();
                    sys::live_log::emit_external_record(
                        log::Level::Info,
                        "audio_input::bridge",
                        &format!(
                            "E-AC3 PipeWire packet cadence: payload_bytes={} frames={} samples={} decoded_ms={:.3} decode_ms={:.3} sample_count_range={:?}..{:?} reset={} error={}",
                            payload.len(),
                            result.frames.len(),
                            emitted_samples,
                            emitted_ms,
                            decode_time_ms,
                            sample_count_min,
                            sample_count_max,
                            result.did_reset,
                            result.error_message
                        ),
                    );
                }
                for frame in result.frames {
                    if first_frame_logs_remaining > 0 {
                        first_frame_logs_remaining -= 1;
                        let frame_ms =
                            frame.sample_count as f64 / frame.sampling_frequency.max(1) as f64
                                * 1000.0;
                        log::debug!(
                            "PipeWire bridge decoded frame: sr={} sample_count={} ch={} frame_ms={:.3} data_type=0x{:02X} payload_bytes={}",
                            frame.sampling_frequency,
                            frame.sample_count,
                            frame.channel_count,
                            frame_ms,
                            data_type,
                            payload.len()
                        );
                    }
                    on_frame(frame, per_frame_decode_time_ms);
                }
            }
        })
        .map_err(|e| anyhow!("Failed to spawn bridge decode worker: {e}"))
}

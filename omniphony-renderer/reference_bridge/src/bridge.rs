//! `FormatBridge` implementation that turns a multichannel WAV/PCM file into a
//! channel bed for the renderer.
//!
//! The bridge buffers the raw bytes delivered through `push_packet`, parses the
//! RIFF/WAVE header once, then converts the accumulated PCM into
//! [`RDecodedFrame`]s. Each frame carries one [`RChannelLabel`] per channel and
//! **empty** metadata: that is exactly how the renderer recognises a plain
//! channel bed. Known legacy layouts remain accepted unchanged, while 16- and
//! 17-channel internal beds expose the complete 7.1.4.4 / 8.1.4.4 vocabulary.

use abi_stable::std_types::{RSlice, RStr, RString, RVec};
use bridge_api::{
    FormatBridge, RChannelLabel, RCoordinateFormat, RDecodedFrame, RInputTransport, RMetadataFrame,
    RPushResult, RVbapCartesianDefaults, RVbapTableMode,
};

use crate::logging::bridge_diag_log;
use crate::wav::{HeaderParse, WavFormat, parse_header};

const BLOCK_FRAMES: usize = 2048;

enum State {
    Header,
    Data { format: WavFormat, remaining: u64 },
}

pub(crate) struct WavBridge {
    buf: Vec<u8>,
    state: State,
    labels: Vec<RChannelLabel>,
    strict: bool,
    frames_emitted: u64,
}

impl WavBridge {
    pub(crate) fn new(strict: bool) -> Self {
        Self {
            buf: Vec::new(),
            state: State::Header,
            labels: Vec::new(),
            strict,
            frames_emitted: 0,
        }
    }

    fn reset_state(&mut self) {
        self.buf.clear();
        self.state = State::Header;
        self.labels.clear();
    }

    fn fail(&mut self, result: &mut RPushResult, message: &str) {
        bridge_diag_log(log::Level::Warn, message);
        self.reset_state();
        result.did_reset = true;
        if self.strict {
            result.error_message = RString::from(message);
        }
    }

    fn try_parse_header(&mut self, result: &mut RPushResult) -> bool {
        match parse_header(&self.buf) {
            HeaderParse::NeedMore => false,
            HeaderParse::Invalid(reason) => {
                self.fail(result, &format!("reference-bridge: invalid WAV: {reason}"));
                false
            }
            HeaderParse::Found {
                format,
                data_offset,
                data_len,
            } => {
                self.labels = channel_labels(format.channels);
                self.buf.drain(0..data_offset);
                bridge_diag_log(
                    log::Level::Info,
                    &format!(
                        "reference-bridge: WAV header parsed: {} ch, {} Hz, {:?}",
                        format.channels, format.sample_rate, format.sample_format
                    ),
                );
                self.state = State::Data {
                    format,
                    remaining: data_len,
                };
                true
            }
        }
    }

    fn drain_pcm(&mut self, result: &mut RPushResult) {
        let State::Data { format, remaining } = &mut self.state else {
            return;
        };
        let format = *format;
        let bytes_per_sample = format.sample_format.bytes_per_sample();
        let channels = format.channels as usize;
        let bytes_per_frame = format.bytes_per_frame();
        if bytes_per_frame == 0 {
            return;
        }

        let available_bytes = if *remaining == u64::MAX {
            self.buf.len()
        } else {
            self.buf.len().min(*remaining as usize)
        };
        let total_frames = available_bytes / bytes_per_frame;
        if total_frames == 0 {
            return;
        }

        let mut frame_start = 0usize;
        let mut frames_left = total_frames;
        while frames_left > 0 {
            let n = frames_left.min(BLOCK_FRAMES);
            let sample_total = n * channels;
            let mut pcm: RVec<i32> = RVec::with_capacity(sample_total);

            let mut byte_idx = frame_start;
            for _ in 0..sample_total {
                let s = format
                    .sample_format
                    .decode_sample(&self.buf[byte_idx..byte_idx + bytes_per_sample]);
                pcm.push(s);
                byte_idx += bytes_per_sample;
            }

            result.frames.push(RDecodedFrame {
                sampling_frequency: format.sample_rate,
                sample_count: n as u32,
                channel_count: format.channels as u32,
                pcm,
                channel_labels: RVec::from(self.labels.clone()),
                metadata: RVec::<RMetadataFrame>::new(),
                drc_gain: 1.0,
                drc_ramp_duration: 0,
                dialogue_level: abi_stable::std_types::ROption::RNone,
                is_new_segment: false,
            });

            frame_start += n * bytes_per_frame;
            frames_left -= n;
        }

        self.frames_emitted += total_frames as u64;
        let consumed = total_frames * bytes_per_frame;
        if let State::Data { remaining, .. } = &mut self.state {
            if *remaining != u64::MAX {
                *remaining -= consumed as u64;
            }
        }
        self.buf.drain(0..consumed);
    }
}

/// Map an unambiguous channel count to canonical labels.
///
/// Historical 1/2/5.1/7.1/7.1.4 mappings are preserved exactly. The two richer
/// internal orders are:
///
/// - 16ch 7.1.4.4: `L R C LFE Ls Rs Lb Rb Tfl Tfr Tbl Tbr Bfl Bfr Bbl Bbr`
/// - 17ch 8.1.4.4: `L R C LFE Ls Rs Lb Rb Cb Tfl Tfr Tbl Tbr Bfl Bfr Bbl Bbr`
///
/// A real external WAVE_FORMAT_EXTENSIBLE channel mask should be preferred when
/// available; these count-only orders are primarily the linked Current bridge
/// and deterministic compatibility fixtures.
fn channel_labels(channel_count: u16) -> Vec<RChannelLabel> {
    use RChannelLabel::*;
    let canonical: &[RChannelLabel] = match channel_count {
        1 => &[C],
        2 => &[L, R],
        6 => &[L, R, C, LFE, Ls, Rs],
        8 => &[L, R, C, LFE, Ls, Rs, Lb, Rb],
        12 => &[L, R, C, LFE, Ls, Rs, Lb, Rb, Tfl, Tfr, Tbl, Tbr],
        16 => &[
            L, R, C, LFE, Ls, Rs, Lb, Rb, Tfl, Tfr, Tbl, Tbr, Bfl, Bfr, Bbl, Bbr,
        ],
        17 => &[
            L, R, C, LFE, Ls, Rs, Lb, Rb, Cb, Tfl, Tfr, Tbl, Tbr, Bfl, Bfr, Bbl, Bbr,
        ],
        _ => &[],
    };
    if !canonical.is_empty() {
        return canonical.to_vec();
    }

    const BEST_EFFORT: &[RChannelLabel] = &[
        L, R, C, LFE, Ls, Rs, Lb, Rb, Cb, Tfl, Tfr, Tbl, Tbr, Bfl, Bfr, Bbl, Bbr,
    ];
    (0..channel_count as usize)
        .map(|i| BEST_EFFORT.get(i).copied().unwrap_or(RChannelLabel::Unknown))
        .collect()
}

impl FormatBridge for WavBridge {
    fn push_packet(
        &mut self,
        data: RSlice<'_, u8>,
        _transport: RInputTransport,
        _data_type: u8,
    ) -> RPushResult {
        let mut result = RPushResult {
            frames: RVec::new(),
            error_message: RString::new(),
            did_reset: false,
        };

        self.buf.extend_from_slice(data.as_slice());
        if matches!(self.state, State::Header) && !self.try_parse_header(&mut result) {
            return result;
        }
        self.drain_pcm(&mut result);
        result
    }

    fn reset(&mut self) {
        self.reset_state();
    }

    fn is_ready(&self) -> bool {
        self.frames_emitted > 0
    }

    fn has_objects(&self) -> bool {
        false
    }

    fn configure(&mut self, key: RStr<'_>, _value: RStr<'_>) -> bool {
        key.as_str() == "presentation"
    }

    fn coordinate_format(&self) -> RCoordinateFormat {
        RCoordinateFormat::Cartesian
    }

    fn vbap_cartesian_defaults(&self) -> RVbapCartesianDefaults {
        RVbapCartesianDefaults {
            x_size: 62,
            y_size: 62,
            z_size: 15,
            allow_negative_z: false,
        }
    }

    fn preferred_vbap_table_mode(&self) -> RVbapTableMode {
        RVbapTableMode::Cartesian
    }

    fn supported_drc_modes(&self) -> RVec<RString> {
        RVec::new()
    }

    fn set_drc_mode(&mut self, _mode: RStr<'_>) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_wav(channels: u16, sample_rate: u32, frames: &[Vec<i16>]) -> Vec<u8> {
        let mut data = Vec::new();
        for frame in frames {
            for &s in frame {
                data.extend_from_slice(&s.to_le_bytes());
            }
        }
        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&(36 + data.len() as u32).to_le_bytes());
        buf.extend_from_slice(b"WAVE");
        buf.extend_from_slice(b"fmt ");
        buf.extend_from_slice(&16u32.to_le_bytes());
        buf.extend_from_slice(&1u16.to_le_bytes());
        buf.extend_from_slice(&channels.to_le_bytes());
        buf.extend_from_slice(&sample_rate.to_le_bytes());
        buf.extend_from_slice(&(sample_rate * channels as u32 * 2).to_le_bytes());
        buf.extend_from_slice(&(channels * 2).to_le_bytes());
        buf.extend_from_slice(&16u16.to_le_bytes());
        buf.extend_from_slice(b"data");
        buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&data);
        buf
    }

    #[test]
    fn labels_for_supported_counts() {
        use RChannelLabel::*;
        assert_eq!(channel_labels(2), vec![L, R]);
        assert_eq!(channel_labels(6), vec![L, R, C, LFE, Ls, Rs]);
        assert_eq!(channel_labels(8), vec![L, R, C, LFE, Ls, Rs, Lb, Rb]);
        assert_eq!(
            channel_labels(12),
            vec![L, R, C, LFE, Ls, Rs, Lb, Rb, Tfl, Tfr, Tbl, Tbr]
        );
        assert_eq!(
            channel_labels(16),
            vec![L, R, C, LFE, Ls, Rs, Lb, Rb, Tfl, Tfr, Tbl, Tbr, Bfl, Bfr, Bbl, Bbr]
        );
        assert_eq!(
            channel_labels(17),
            vec![L, R, C, LFE, Ls, Rs, Lb, Rb, Cb, Tfl, Tfr, Tbl, Tbr, Bfl, Bfr, Bbl, Bbr]
        );

        let three = channel_labels(3);
        assert_eq!(three, vec![L, R, C]);
        let seven = channel_labels(7);
        assert_eq!(&seven[..6], &[L, R, C, LFE, Ls, Rs]);
    }

    #[test]
    fn decodes_full_file_in_one_push() {
        let frames = vec![vec![100i16, -100], vec![200, -200], vec![300, -300]];
        let wav = write_wav(2, 48_000, &frames);
        let mut bridge = WavBridge::new(false);
        let result = bridge.push_packet(RSlice::from_slice(&wav), RInputTransport::Raw, 0);
        assert!(result.error_message.is_empty());
        assert!(bridge.is_ready());
        assert!(!bridge.has_objects());
        let total: u32 = result.frames.iter().map(|f| f.sample_count).sum();
        assert_eq!(total, 3);
        let f = &result.frames[0];
        assert_eq!(f.channel_count, 2);
        assert_eq!(f.sampling_frequency, 48_000);
        assert!(f.metadata.is_empty(), "bed frames must carry no metadata");
        assert_eq!(f.pcm[0], 100 << 8);
        assert_eq!(f.pcm[1], -100 << 8);
    }

    #[test]
    fn decodes_across_byte_split_chunks() {
        let frames: Vec<Vec<i16>> = (0..50).map(|i| vec![i as i16, -(i as i16)]).collect();
        let wav = write_wav(2, 48_000, &frames);
        let mut bridge = WavBridge::new(false);
        let mut total = 0u32;
        for chunk in wav.chunks(7) {
            let r = bridge.push_packet(RSlice::from_slice(chunk), RInputTransport::Raw, 0);
            assert!(r.error_message.is_empty());
            total += r.frames.iter().map(|f| f.sample_count).sum::<u32>();
        }
        assert_eq!(total, 50);
    }

    #[test]
    fn honours_declared_data_size() {
        let frames = vec![vec![1i16, 2], vec![3, 4]];
        let mut wav = write_wav(2, 48_000, &frames);
        wav.extend_from_slice(b"LIST\x04\x00\x00\x00junk");
        let mut bridge = WavBridge::new(false);
        let r = bridge.push_packet(RSlice::from_slice(&wav), RInputTransport::Raw, 0);
        let total: u32 = r.frames.iter().map(|f| f.sample_count).sum();
        assert_eq!(total, 2, "trailing chunk must not be read as PCM");
    }
}

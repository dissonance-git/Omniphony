//! Portable PCM/time contract between Omniphony platform adapters and the
//! audio-free engine boundary.
//!
//! This crate deliberately contains **no** WASAPI, ASIO, CoreAudio, PipeWire,
//! AAudio, driver, device or service concepts. Platform code converts native
//! buffers into normalized interleaved `f32` PCM and publishes them on one
//! monotonically indexed sample-frame timeline.
//!
//! Rich authored object/scene inputs are a separate contract. Ordinary PCM must
//! not pretend to contain source metadata it does not actually carry.

use std::error::Error;
use std::fmt;

/// Channel meaning at the portable PCM boundary.
///
/// Stereo is the normal consumer-music case. `Discrete` exists for explicit
/// known-channel fixtures or future trusted multichannel inputs without baking
/// any one surround layout into the transport contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelLayout {
    Mono,
    Stereo,
    Discrete(u16),
}

impl ChannelLayout {
    pub const fn channels(self) -> u16 {
        match self {
            Self::Mono => 1,
            Self::Stereo => 2,
            Self::Discrete(channels) => channels,
        }
    }
}

/// Audio format after a platform adapter has converted native samples to the
/// core's normalized interleaved `f32` representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioStreamFormat {
    pub sample_rate_hz: u32,
    pub layout: ChannelLayout,
}

impl AudioStreamFormat {
    pub fn new(sample_rate_hz: u32, layout: ChannelLayout) -> Result<Self, AudioContractError> {
        if sample_rate_hz == 0 {
            return Err(AudioContractError::ZeroSampleRate);
        }
        if layout.channels() == 0 {
            return Err(AudioContractError::ZeroChannels);
        }
        Ok(Self {
            sample_rate_hz,
            layout,
        })
    }

    pub fn stereo(sample_rate_hz: u32) -> Result<Self, AudioContractError> {
        Self::new(sample_rate_hz, ChannelLayout::Stereo)
    }

    pub const fn channels(self) -> u16 {
        self.layout.channels()
    }
}

/// Why the next block should not be interpreted as sample-continuous with the
/// previous one.
///
/// The core owns how history/state reacts. The host owns detecting and declaring
/// the discontinuity instead of hiding it by resetting internal clocks silently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Discontinuity {
    SourceStarted,
    SourceReset,
    Gap,
    ClockResync,
    DeviceChanged,
}

/// A borrowed block of normalized interleaved PCM on one absolute sample-frame
/// timeline.
///
/// `start_frame` counts *frames*, not scalar interleaved samples. Changing the
/// host callback size therefore changes only how the same timeline is
/// partitioned; it must not alter the intended auditory/rendering trajectory.
#[derive(Debug, Clone, Copy)]
pub struct AudioInputBlock<'a> {
    pub format: AudioStreamFormat,
    pub start_frame: u64,
    pub samples: &'a [f32],
    pub discontinuity: Option<Discontinuity>,
}

impl<'a> AudioInputBlock<'a> {
    pub fn new(
        format: AudioStreamFormat,
        start_frame: u64,
        samples: &'a [f32],
        discontinuity: Option<Discontinuity>,
    ) -> Result<Self, AudioContractError> {
        let channels = usize::from(format.channels());
        if samples.len() % channels != 0 {
            return Err(AudioContractError::PartialFrame {
                sample_count: samples.len(),
                channels: format.channels(),
            });
        }
        Ok(Self {
            format,
            start_frame,
            samples,
            discontinuity,
        })
    }

    pub fn frame_count(self) -> usize {
        self.samples.len() / usize::from(self.format.channels())
    }

    pub fn end_frame(self) -> u64 {
        self.start_frame.saturating_add(self.frame_count() as u64)
    }

    pub fn duration_seconds(self) -> f64 {
        self.frame_count() as f64 / self.format.sample_rate_hz as f64
    }

    pub const fn is_continuous(self) -> bool {
        self.discontinuity.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioContractError {
    ZeroSampleRate,
    ZeroChannels,
    PartialFrame { sample_count: usize, channels: u16 },
}

impl fmt::Display for AudioContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSampleRate => formatter.write_str("sample rate must be greater than zero"),
            Self::ZeroChannels => formatter.write_str("channel count must be greater than zero"),
            Self::PartialFrame {
                sample_count,
                channels,
            } => write!(
                formatter,
                "{sample_count} interleaved samples do not form complete {channels}-channel frames"
            ),
        }
    }
}

impl Error for AudioContractError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_block_counts_frames_not_scalar_samples() {
        let format = AudioStreamFormat::stereo(48_000).unwrap();
        let samples = [0.0_f32; 80];
        let block = AudioInputBlock::new(format, 1_000, &samples, None).unwrap();

        assert_eq!(block.frame_count(), 40);
        assert_eq!(block.end_frame(), 1_040);
        assert!((block.duration_seconds() - 40.0 / 48_000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn callback_partition_does_not_change_timeline_extent() {
        let format = AudioStreamFormat::stereo(48_000).unwrap();
        let whole_samples = [0.0_f32; 1_920]; // 960 stereo frames
        let whole = AudioInputBlock::new(format, 5_000, &whole_samples, None).unwrap();

        let chunk_samples = [0.0_f32; 80]; // 40 stereo frames
        let mut cursor = 5_000_u64;
        for _ in 0..24 {
            let chunk = AudioInputBlock::new(format, cursor, &chunk_samples, None).unwrap();
            cursor = chunk.end_frame();
        }

        assert_eq!(whole.frame_count(), 960);
        assert_eq!(cursor, whole.end_frame());
    }

    #[test]
    fn discontinuity_is_explicit_transport_state() {
        let format = AudioStreamFormat::stereo(48_000).unwrap();
        let samples = [0.0_f32; 16];
        let continuous = AudioInputBlock::new(format, 0, &samples, None).unwrap();
        let reset = AudioInputBlock::new(
            format,
            continuous.end_frame(),
            &samples,
            Some(Discontinuity::ClockResync),
        )
        .unwrap();

        assert!(continuous.is_continuous());
        assert!(!reset.is_continuous());
        assert_eq!(reset.discontinuity, Some(Discontinuity::ClockResync));
    }

    #[test]
    fn malformed_formats_and_partial_frames_are_rejected() {
        assert_eq!(
            AudioStreamFormat::stereo(0),
            Err(AudioContractError::ZeroSampleRate)
        );
        assert_eq!(
            AudioStreamFormat::new(48_000, ChannelLayout::Discrete(0)),
            Err(AudioContractError::ZeroChannels)
        );

        let stereo = AudioStreamFormat::stereo(48_000).unwrap();
        assert_eq!(
            AudioInputBlock::new(stereo, 0, &[0.0, 0.0, 0.0], None).unwrap_err(),
            AudioContractError::PartialFrame {
                sample_count: 3,
                channels: 2,
            }
        );
    }
}

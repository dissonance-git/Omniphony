//! Pure routing helpers for the protected-master hybrid-height experiment.
//!
//! The Windows listening host may render the non-height evidence lanes through
//! Omniphony's cascaded virtual-speaker world while rendering TFL/TFR/TBL/TBR
//! through a second direct-binaural engine. This module owns only the algebraic
//! partition and stereo recombination so the safety law is portable and tested:
//!
//! `one evidence sample -> one spatial route`
//!
//! It does not own HRTFs, room simulation, host lifecycle or profile tuning.

use anyhow::{Result, bail};

use crate::music_field::MUSIC_FIELD_CHANNELS;

/// Canonical 7.1.4 height lanes: TFL, TFR, TBL, TBR.
pub const HEIGHT_CHANNEL_START: usize = 8;
pub const HEIGHT_CHANNEL_END: usize = 12;

/// Split interleaved 12-channel evidence into mutually-exclusive cascade and
/// direct-height buffers. Their sample-wise sum exactly reconstructs `input`.
pub fn split_height_routes(input: &[f32]) -> Result<(Vec<f32>, Vec<f32>)> {
    if input.len() % MUSIC_FIELD_CHANNELS != 0 {
        bail!(
            "hybrid height split requires {}-channel evidence frames; got {} samples",
            MUSIC_FIELD_CHANNELS,
            input.len()
        );
    }

    let mut cascade = vec![0.0; input.len()];
    let mut height = vec![0.0; input.len()];
    for (frame_index, frame) in input.chunks_exact(MUSIC_FIELD_CHANNELS).enumerate() {
        let base = frame_index * MUSIC_FIELD_CHANNELS;
        for (channel, &sample) in frame.iter().enumerate() {
            if (HEIGHT_CHANNEL_START..HEIGHT_CHANNEL_END).contains(&channel) {
                height[base + channel] = sample;
            } else {
                cascade[base + channel] = sample;
            }
        }
    }
    Ok((cascade, height))
}

/// Linear stereo recombination of independently rendered support branches.
/// Final output headroom remains the Windows host's responsibility.
pub fn sum_stereo_support(cascade: &[f32], height: &[f32]) -> Result<Vec<f32>> {
    if cascade.len() != height.len() {
        bail!(
            "hybrid support branches differ in length: cascade={} height={}",
            cascade.len(),
            height.len()
        );
    }
    if cascade.len() % 2 != 0 {
        bail!("hybrid support branches must be interleaved stereo");
    }

    Ok(cascade
        .iter()
        .zip(height.iter())
        .map(|(&a, &b)| {
            let a = if a.is_finite() { a } else { 0.0 };
            let b = if b.is_finite() { b } else { 0.0 };
            a + b
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_is_exclusive_and_lossless() {
        let input: Vec<f32> = (0..MUSIC_FIELD_CHANNELS * 5)
            .map(|i| i as f32 * 0.03125 - 0.8)
            .collect();
        let (cascade, height) = split_height_routes(&input).expect("valid 12ch evidence");

        for frame in 0..5 {
            let base = frame * MUSIC_FIELD_CHANNELS;
            for channel in 0..MUSIC_FIELD_CHANNELS {
                let c = cascade[base + channel];
                let h = height[base + channel];
                assert!(
                    c == 0.0 || h == 0.0,
                    "frame {frame} channel {channel} entered both routes"
                );
                assert!((c + h - input[base + channel]).abs() < 1.0e-7);
                if (HEIGHT_CHANNEL_START..HEIGHT_CHANNEL_END).contains(&channel) {
                    assert_eq!(c, 0.0);
                    assert_eq!(h, input[base + channel]);
                } else {
                    assert_eq!(c, input[base + channel]);
                    assert_eq!(h, 0.0);
                }
            }
        }
    }

    #[test]
    fn stereo_support_sum_is_linear() {
        let cascade = [0.5_f32, -0.25, 0.10, 0.20];
        let height = [0.05_f32, 0.15, -0.03, 0.07];
        let expected = [0.55_f32, -0.10, 0.07, 0.27];
        let out = sum_stereo_support(&cascade, &height).expect("matched stereo branches");
        for (actual, expected) in out.iter().zip(expected) {
            assert!(
                (*actual - expected).abs() < 1.0e-6,
                "linear support sum drifted: got {actual}, expected {expected}"
            );
        }
    }

    #[test]
    fn malformed_evidence_is_rejected() {
        assert!(split_height_routes(&[0.0; MUSIC_FIELD_CHANNELS - 1]).is_err());
    }
}

//! End-to-end spatial PCM contracts.
//!
//! The renderer's VBAP validation checks gain vectors directly. These fixtures
//! deliberately sit one layer farther out: a real object is rendered through
//! `SpatialRenderer`, including channel state, gain slew, speaker mixing and
//! per-speaker finalisation. That catches integration bugs where the panner is
//! correct but the sounding PCM still develops a hole.

#[cfg(test)]
mod tests {
    use renderer::live_params::{LiveEvaluationMode, PreferredEvaluationMode, RampMode};
    use renderer::spatial_renderer::{SpatialChannelEvent, SpatialRenderer};
    use renderer::spatial_vbap::{DistanceModel, VbapTableMode};
    use renderer::speaker_layout::SpeakerLayout;

    use crate::scene::{BLOCK_SAMPLES, SAMPLE_RATE};

    fn full_sphere_renderer() -> SpatialRenderer {
        SpatialRenderer::new(
            SpeakerLayout::preset("7.1.4").expect("known preset"),
            SAMPLE_RATE,
            5,
            5,
            0.0,
            2.0,
            VbapTableMode::Polar,
            true, // allow_negative_z: the nadir must reach the virtual-pole path.
            true,
            DistanceModel::Linear,
            false,
            1.0,
            1.0,
            0.0,
            1.0,
            false,
            [1.0, 2.0, 0.5],
            2.0,
            0.5,
            0.0,
            0.0,
            false,
            false,
            false,
            1.0,
            1.0,
            PreferredEvaluationMode::PrecomputedPolar,
            LiveEvaluationMode::PrecomputedPolar,
            31,
            31,
            15,
            15,
        )
        .expect("full-sphere renderer")
    }

    fn settled_block_energy(position: [f64; 3]) -> f64 {
        let mut renderer = full_sphere_renderer();
        {
            let control = renderer.renderer_control();
            control.set_requested_ramp_mode(RampMode::Frame);
            control.live.write().ramp_mode = RampMode::Frame;
        }

        let pcm = vec![0.125f32; BLOCK_SAMPLES];
        let event = vec![SpatialChannelEvent {
            channel_idx: 0,
            is_bed: false,
            gain_db: Some(0),
            ramp_length: Some(BLOCK_SAMPLES as u32),
            size: Some([0.0, 0.0, 0.0]),
            position: Some(position),
            sample_pos: Some(0),
        }];

        // Gain slew is 20 ms at 48 kHz, so 32 × 40-sample blocks comfortably
        // puts the measurement after every startup transient has settled.
        let mut scratch = Vec::new();
        for _ in 0..32 {
            let frame = renderer
                .render_frame(&pcm, 1, &event, scratch, false)
                .expect("spatial probe render");
            assert_eq!(frame.n_channels, 12, "7.1.4 speaker path width changed");
            scratch = frame.samples;
        }

        assert!(scratch.iter().all(|sample| sample.is_finite()));
        scratch
            .iter()
            .map(|sample| (*sample as f64) * (*sample as f64))
            .sum()
    }

    #[test]
    fn virtual_poles_preserve_energy_through_the_full_pcm_path() {
        let probes = [
            ("front", [0.0, 1.0, 0.0]),
            ("right", [1.0, 0.0, 0.0]),
            ("back", [0.0, -1.0, 0.0]),
            ("left", [-1.0, 0.0, 0.0]),
            ("zenith", [0.0, 0.0, 1.0]),
            ("nadir", [0.0, 0.0, -1.0]),
            ("below-diagonal", [0.55, 0.45, -0.70]),
        ];

        let reference = settled_block_energy(probes[0].1);
        assert!(reference > 1.0e-8, "front reference unexpectedly silent");

        for (name, position) in probes {
            let energy = settled_block_energy(position);
            assert!(energy > 1.0e-8, "{name} fell into a PCM energy hole");
            let delta_db = 10.0 * (energy / reference).log10();
            println!(
                "[measure] full-pipeline spatial energy {name:>14}: {delta_db:+.5} dB vs front"
            );
            assert!(
                delta_db.abs() <= 0.10,
                "{name} changed full-pipeline energy by {delta_db:+.4} dB; \
                 virtual-pole/out-of-hull routing must stay effectively constant-power"
            );
        }
    }
}

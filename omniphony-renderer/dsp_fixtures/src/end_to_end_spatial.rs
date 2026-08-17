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

    use crate::{
        orbit::{Orbit, OrbitAxis},
        scene::{BLOCK_SAMPLES, SAMPLE_RATE},
    };

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
        let horizontal = [
            ("front", [0.0, 1.0, 0.0]),
            ("right", [1.0, 0.0, 0.0]),
            ("back", [0.0, -1.0, 0.0]),
            ("left", [-1.0, 0.0, 0.0]),
        ];
        let out_of_hull = [
            ("zenith", [0.0, 0.0, 1.0]),
            ("nadir", [0.0, 0.0, -1.0]),
            ("below-diagonal", [0.55, 0.45, -0.70]),
        ];

        let horizontal_energy: Vec<_> = horizontal
            .iter()
            .map(|(name, position)| (*name, settled_block_energy(*position)))
            .collect();
        let horizontal_min = horizontal_energy
            .iter()
            .map(|(_, energy)| *energy)
            .fold(f64::INFINITY, f64::min);
        let horizontal_max = horizontal_energy
            .iter()
            .map(|(_, energy)| *energy)
            .fold(0.0f64, f64::max);
        assert!(
            horizontal_min > 1.0e-8,
            "horizontal reference contains a PCM energy hole"
        );

        for (name, energy) in &horizontal_energy {
            let delta_from_min_db = 10.0 * (*energy / horizontal_min).log10();
            println!(
                "[measure] horizontal spatial energy {name:>14}: {delta_from_min_db:+.5} dB vs horizontal minimum"
            );
        }

        // Full-pipeline energy is not required to be direction-flat: the
        // physical speaker geometry and downstream processing can legitimately
        // make an exact-speaker direction differ from a panned direction even
        // when the VBAP gain vector itself is constant-power. The useful
        // end-to-end contract is therefore that virtual-pole/out-of-hull
        // directions do not fall into a new energy hole or explode far beyond
        // the already healthy horizontal operating envelope.
        const EXTRA_ENVELOPE_DB: f64 = 12.0;
        for (name, position) in out_of_hull {
            let energy = settled_block_energy(position);
            assert!(energy > 1.0e-8, "{name} fell into a PCM energy hole");
            let below_floor_db = 10.0 * (energy / horizontal_min).log10();
            let above_ceiling_db = 10.0 * (energy / horizontal_max).log10();
            println!(
                "[measure] out-of-hull spatial energy {name:>14}: {below_floor_db:+.5} dB vs horizontal min, {above_ceiling_db:+.5} dB vs horizontal max"
            );
            assert!(
                below_floor_db >= -EXTRA_ENVELOPE_DB,
                "{name} created a new full-pipeline energy hole: {below_floor_db:+.4} dB below horizontal minimum"
            );
            assert!(
                above_ceiling_db <= EXTRA_ENVELOPE_DB,
                "{name} created a new full-pipeline energy spike: {above_ceiling_db:+.4} dB above horizontal maximum"
            );
        }
    }

    fn object_event(position: [f32; 3]) -> Vec<SpatialChannelEvent> {
        vec![SpatialChannelEvent {
            channel_idx: 0,
            is_bed: false,
            gain_db: Some(0),
            ramp_length: Some(BLOCK_SAMPLES as u32),
            size: Some([0.0, 0.0, 0.0]),
            position: Some(position.map(f64::from)),
            sample_pos: Some(0),
        }]
    }

    fn frame_norm(frame: &[f32]) -> f64 {
        frame
            .iter()
            .map(|sample| (*sample as f64) * (*sample as f64))
            .sum::<f64>()
            .sqrt()
    }

    fn normalized_frame_step(previous: &[f32], next: &[f32]) -> f64 {
        let delta = previous
            .iter()
            .zip(next)
            .map(|(a, b)| {
                let d = *b as f64 - *a as f64;
                d * d
            })
            .sum::<f64>()
            .sqrt();
        delta / frame_norm(previous).max(1.0e-12)
    }

    fn percentile(mut values: Vec<f64>, quantile: f64) -> f64 {
        values.sort_by(|a, b| a.total_cmp(b));
        let index = ((values.len() - 1) as f64 * quantile.clamp(0.0, 1.0)).round() as usize;
        values[index]
    }

    #[test]
    fn rendered_x_orbit_stays_audible_and_continuous_through_virtual_poles() {
        let mut renderer = full_sphere_renderer();
        {
            let control = renderer.renderer_control();
            control.set_requested_ramp_mode(RampMode::Interp);
            control.live.write().ramp_mode = RampMode::Interp;
        }

        let pcm = vec![0.125f32; BLOCK_SAMPLES];
        // X-axis motion traces the Y/Z meridian: front -> zenith -> back ->
        // nadir -> front. That makes one turn exercise both physical height
        // speakers and the virtual-pole closure below the speaker hull.
        let centre = [0.0f32; 3];
        let mut orbit = Orbit::new(OrbitAxis::X, 0.95, 1.0);
        let start = orbit.position(centre);
        let start_event = object_event(start);

        // Settle the 20 ms gain slew before measuring motion. The input is a
        // constant, so every later vector step comes from spatial movement,
        // not from the stimulus itself.
        let mut scratch = Vec::new();
        for _ in 0..32 {
            let frame = renderer
                .render_frame(&pcm, 1, &start_event, scratch, false)
                .expect("orbit warm-up render");
            scratch = frame.samples;
        }
        let width = 12usize;
        let mut previous_frame = scratch[scratch.len() - width..].to_vec();
        let mut seam_steps = Vec::new();
        let mut in_block_steps = Vec::new();
        let mut block_energies = Vec::new();

        let blocks_per_turn = (SAMPLE_RATE as usize / BLOCK_SAMPLES).max(1);
        for _ in 0..blocks_per_turn {
            let position = orbit.advance(centre, BLOCK_SAMPLES, SAMPLE_RATE);
            let event = object_event(position);
            let frame = renderer
                .render_frame(&pcm, 1, &event, scratch, false)
                .expect("orbit render");
            assert_eq!(frame.n_channels, width, "7.1.4 orbit width changed");
            assert_eq!(frame.samples.len(), BLOCK_SAMPLES * width);
            assert!(frame.samples.iter().all(|sample| sample.is_finite()));

            let first = &frame.samples[..width];
            seam_steps.push(normalized_frame_step(&previous_frame, first));
            for sample_idx in 1..BLOCK_SAMPLES {
                let a = &frame.samples[(sample_idx - 1) * width..sample_idx * width];
                let b = &frame.samples[sample_idx * width..(sample_idx + 1) * width];
                in_block_steps.push(normalized_frame_step(a, b));
            }

            let energy = frame
                .samples
                .iter()
                .map(|sample| (*sample as f64) * (*sample as f64))
                .sum::<f64>();
            assert!(
                energy > 1.0e-8,
                "moving source fell into a silent PCM block"
            );
            block_energies.push(energy);

            previous_frame.copy_from_slice(&frame.samples[frame.samples.len() - width..]);
            scratch = frame.samples;
        }

        let min_energy = block_energies.iter().copied().fold(f64::INFINITY, f64::min);
        let max_energy = block_energies.iter().copied().fold(0.0f64, f64::max);
        let energy_span_db = 10.0 * (max_energy / min_energy).log10();
        assert!(
            energy_span_db <= 24.0,
            "one continuous pole-crossing orbit developed a {energy_span_db:.3} dB block-energy span"
        );

        // A block boundary is not a special spatial event. Compare the worst
        // seam against ordinary per-sample motion inside blocks instead of an
        // arbitrary absolute PCM delta. This catches a callback-quantised pan
        // or pole handoff while remaining scale-independent.
        let ordinary_p999 = percentile(in_block_steps, 0.999);
        let worst_seam = seam_steps.iter().copied().fold(0.0f64, f64::max);
        println!(
            "[measure] rendered X-orbit: energy span={energy_span_db:.4} dB, in-block p99.9 step={ordinary_p999:.8}, worst seam={worst_seam:.8}"
        );
        assert!(
            worst_seam <= ordinary_p999 * 4.0 + 1.0e-4,
            "block boundary became a spatial discontinuity: seam={worst_seam:.8}, ordinary p99.9={ordinary_p999:.8}"
        );
    }
}

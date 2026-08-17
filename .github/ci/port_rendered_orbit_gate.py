from pathlib import Path

PATH = Path("omniphony-renderer/dsp_fixtures/src/end_to_end_spatial.rs")
text = PATH.read_text(encoding="utf-8")

old_import = "    use crate::scene::{BLOCK_SAMPLES, SAMPLE_RATE};\n"
new_import = "    use crate::{\n        orbit::{Orbit, OrbitAxis},\n        scene::{BLOCK_SAMPLES, SAMPLE_RATE},\n    };\n"
if old_import not in text:
    raise SystemExit("expected scene import not found")
text = text.replace(old_import, new_import, 1)

if "rendered_x_orbit_stays_audible_and_continuous_through_virtual_poles" in text:
    raise SystemExit("orbit PCM gate already present")

addition = r'''

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
            assert!(energy > 1.0e-8, "moving source fell into a silent PCM block");
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
'''

if not text.endswith("}\n"):
    raise SystemExit("unexpected module ending")
text = text[:-2] + addition + "}\n"
PATH.write_text(text, encoding="utf-8")
print("added rendered full-sphere orbit continuity gate")

from pathlib import Path

PATH = Path("omniphony-renderer/dsp_fixtures/src/end_to_end_spatial.rs")
text = PATH.read_text(encoding="utf-8")

if "current_shell_cascaded_binaural_orbit_stays_continuous" in text:
    raise SystemExit("cascaded headphone orbit gate already present")

old_import = "    use renderer::live_params::{LiveEvaluationMode, PreferredEvaluationMode, RampMode};\n"
new_import = "    use renderer::live_params::{\n        BinauralMode, LiveEvaluationMode, OutputMode, PreferredEvaluationMode, RampMode,\n    };\n"
if old_import not in text:
    raise SystemExit("live_params import anchor not found")
text = text.replace(old_import, new_import, 1)

helper_anchor = '''    fn full_sphere_renderer() -> SpatialRenderer {
        SpatialRenderer::new(
            SpeakerLayout::preset("7.1.4").expect("known preset"),
'''
if helper_anchor not in text:
    raise SystemExit("full_sphere_renderer anchor not found")

# Reuse exactly the same renderer construction, changing only the product layout.
current_helper = r'''
    fn current_shell_renderer() -> SpatialRenderer {
        const CURRENT_SHELL: &str = include_str!(
            "../../../layouts/system-h-derived-22.0-upper60-grid10.yaml"
        );
        SpatialRenderer::new(
            SpeakerLayout::from_yaml_str(CURRENT_SHELL).expect("embedded Current shell"),
            SAMPLE_RATE,
            5,
            5,
            0.0,
            2.0,
            VbapTableMode::Polar,
            true,
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
        .expect("Current-shell renderer")
    }

'''
# Put it after the full_sphere_renderer function, located by its stable tail.
tail = '''        .expect("full-sphere renderer")
    }

    fn settled_block_energy'''
if tail not in text:
    raise SystemExit("full_sphere_renderer tail not found")
text = text.replace(
    tail,
    '''        .expect("full-sphere renderer")
    }

''' + current_helper + '''    fn settled_block_energy''',
    1,
)

if not text.endswith("}\n"):
    raise SystemExit("unexpected module ending")

addition = r'''

    #[test]
    fn current_shell_cascaded_binaural_orbit_stays_continuous() {
        let mut renderer = current_shell_renderer();
        {
            let control = renderer.renderer_control();
            control.set_requested_ramp_mode(RampMode::Interp);
            let mut live = control.live.write();
            live.ramp_mode = RampMode::Interp;
            live.binaural.output_mode = OutputMode::Binaural;
            live.binaural.mode = BinauralMode::Cascaded;
            // The contract under test is the spatial hand-off itself. Keep the
            // default fixed HRIR set, but remove distance-dependent filtering so
            // block-to-block changes cannot be blamed on a second moving cue.
            live.binaural.air_absorption = false;
            live.binaural.reflections.enabled = false;
            live.binaural.reverb.enabled = false;
        }

        let pcm = vec![0.125f32; BLOCK_SAMPLES];
        let centre = [0.0f32; 3];
        // One complete Y/Z meridian in 250 ms is deliberately brisk: the gate
        // crosses both poles and every height region in only 300 render blocks,
        // keeping the dev fixture cheap while still exercising continuous ramps.
        let mut orbit = Orbit::new(OrbitAxis::X, 0.95, 0.25);
        let start = orbit.position(centre);
        let start_event = object_event(start);

        // Settle gain slew + the fixed virtual-speaker HRTFs before measuring.
        let mut scratch = Vec::new();
        for _ in 0..48 {
            let frame = renderer
                .render_frame(&pcm, 1, &start_event, scratch, false)
                .expect("cascaded orbit warm-up");
            assert_eq!(frame.n_channels, 2, "cascaded headphone output must be stereo");
            scratch = frame.samples;
        }

        let width = 2usize;
        let mut previous_frame = scratch[scratch.len() - width..].to_vec();
        let mut seam_steps = Vec::new();
        let mut in_block_steps = Vec::new();
        let mut block_energies = Vec::new();
        let blocks_per_turn = ((SAMPLE_RATE as f32 * 0.25) as usize / BLOCK_SAMPLES).max(1);

        for _ in 0..blocks_per_turn {
            let position = orbit.advance(centre, BLOCK_SAMPLES, SAMPLE_RATE);
            let event = object_event(position);
            let frame = renderer
                .render_frame(&pcm, 1, &event, scratch, false)
                .expect("cascaded headphone orbit render");
            assert_eq!(frame.n_channels, width, "cascaded headphone width changed");
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
            assert!(energy > 1.0e-10, "cascaded orbit fell into a silent stereo block");
            block_energies.push(energy);

            previous_frame.copy_from_slice(&frame.samples[frame.samples.len() - width..]);
            scratch = frame.samples;
        }

        let min_energy = block_energies.iter().copied().fold(f64::INFINITY, f64::min);
        let max_energy = block_energies.iter().copied().fold(0.0f64, f64::max);
        let energy_span_db = 10.0 * (max_energy / min_energy).log10();
        assert!(
            energy_span_db <= 36.0,
            "Current-shell cascaded orbit developed a {energy_span_db:.3} dB stereo energy span"
        );

        let ordinary_p999 = percentile(in_block_steps, 0.999);
        let worst_seam = seam_steps.iter().copied().fold(0.0f64, f64::max);
        println!(
            "[measure] Current-shell cascaded binaural X-orbit: energy span={energy_span_db:.4} dB, in-block p99.9 step={ordinary_p999:.8}, worst seam={worst_seam:.8}"
        );
        assert!(
            worst_seam <= ordinary_p999 * 4.0 + 1.0e-4,
            "cascaded headphone block boundary became a spatial discontinuity: seam={worst_seam:.8}, ordinary p99.9={ordinary_p999:.8}"
        );
    }
'''

text = text[:-2] + addition + "}\n"
PATH.write_text(text, encoding="utf-8")
print("added Current-shell cascaded binaural orbit gate")

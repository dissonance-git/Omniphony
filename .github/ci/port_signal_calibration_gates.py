from pathlib import Path

PATH = Path("omniphony-renderer/dsp_fixtures/src/diagnostic_signals.rs")
text = PATH.read_text(encoding="utf-8")

if "pink_generator_is_unit_rms_and_stable_over_time" in text:
    raise SystemExit("signal calibration gates already present")

anchor = '''    fn rms(samples: &[f32]) -> f32 {
        (samples
            .iter()
            .map(|sample| (*sample as f64) * (*sample as f64))
            .sum::<f64>()
            / samples.len() as f64)
            .sqrt() as f32
    }
'''
if anchor not in text:
    raise SystemExit("rms helper anchor not found")

addition = r'''

    fn fft_band_power_db(samples: &[f32], low_hz: f32, high_hz: f32) -> f64 {
        let response = crate::analysis::magnitude_response_db(samples, 48_000);
        let power = response
            .iter()
            .filter(|(freq, _)| *freq >= low_hz && *freq < high_hz)
            .map(|(_, db)| 10.0f64.powf(*db as f64 / 10.0))
            .sum::<f64>();
        assert!(power > 0.0 && power.is_finite());
        10.0 * power.log10()
    }

    #[test]
    fn pink_generator_is_unit_rms_and_stable_over_time() {
        let samples = run(DiagnosticSignal::PinkNoise, 480_000);
        let overall = rms(&samples);
        assert!(
            (overall - 1.0).abs() <= 0.05,
            "PINK_RAW_RMS calibration drifted: measured RMS={overall:.5}"
        );

        let chunk_rms: Vec<f32> = samples.chunks_exact(60_000).map(rms).collect();
        let lo = chunk_rms.iter().copied().fold(f32::INFINITY, f32::min);
        let hi = chunk_rms.iter().copied().fold(0.0f32, f32::max);
        let span_db = 20.0 * (hi / lo).log10();
        assert!(
            span_db <= 1.0,
            "pink level wanders over time: chunk RMS span={span_db:.3} dB, values={chunk_rms:?}"
        );
    }

    #[test]
    fn pink_generator_has_equal_power_per_octave_not_white_power() {
        // Warm the slowest pole, then analyse a power-of-two window so the FFT
        // measurement is fast and deterministic. Equal-power-per-octave pink
        // should keep octave sums level; white noise rises ~3 dB per octave.
        let mut pink = PinkNoise::new(0x85EB_CA6B);
        for _ in 0..48_000 {
            let _ = pink.next_sample();
        }
        let pink_samples: Vec<f32> = (0..65_536).map(|_| pink.next_sample()).collect();
        let pink_bands = [
            fft_band_power_db(&pink_samples, 500.0, 1_000.0),
            fft_band_power_db(&pink_samples, 1_000.0, 2_000.0),
            fft_band_power_db(&pink_samples, 2_000.0, 4_000.0),
            fft_band_power_db(&pink_samples, 4_000.0, 8_000.0),
        ];
        let pink_lo = pink_bands.iter().copied().fold(f64::INFINITY, f64::min);
        let pink_hi = pink_bands.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        assert!(
            pink_hi - pink_lo <= 2.0,
            "pink octave powers are not flat enough: {pink_bands:?} dB"
        );

        let mut white = PinkNoise::new(0x85EB_CA6B);
        let white_samples: Vec<f32> = (0..65_536).map(|_| white.white()).collect();
        let white_low = fft_band_power_db(&white_samples, 1_000.0, 2_000.0);
        let white_high = fft_band_power_db(&white_samples, 4_000.0, 8_000.0);
        let white_rise = white_high - white_low;
        assert!(
            white_rise >= 4.5,
            "white contrast no longer distinguishes the pinking filter: octave-width rise={white_rise:.3} dB"
        );
    }

    #[test]
    fn pink_peak_clamp_is_a_safety_net_not_the_normal_level_setter() {
        let raw = run(DiagnosticSignal::PinkNoise, 480_000);
        let would_clip = raw
            .iter()
            .filter(|sample| sample.abs() >= PinkNoise::CREST)
            .count();
        let fraction = would_clip as f64 / raw.len() as f64;
        assert!(
            fraction <= 0.0001,
            "crest calibration is clipping ordinary pink noise: {would_clip}/{} samples ({:.5}%)",
            raw.len(),
            fraction * 100.0
        );
    }
'''

text = text.replace(anchor, anchor + addition, 1)
PATH.write_text(text, encoding="utf-8")
print("added diagnostic signal calibration gates")

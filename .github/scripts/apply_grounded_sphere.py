from pathlib import Path


def replace(path: str, old: str, new: str) -> None:
    p = Path(path)
    s = p.read_text()
    if old not in s:
        raise SystemExit(f"anchor missing in {path}:\n{old}")
    p.write_text(s.replace(old, new, 1))


foundation = "omniphony-renderer/renderer/src/music_foundation.rs"
replace(
    foundation,
    "    pub low_shelf_db: f32,\n    /// Upper-bass / lower-mid body.\n",
    "    pub low_shelf_db: f32,\n    /// Coherent kick / upper-bass impact around 110 Hz.\n    pub punch_db: f32,\n    /// Upper-bass / lower-mid body.\n",
)
replace(
    foundation,
    "            low_shelf_db: 2.30,\n            body_db: 1.20,\n",
    "            low_shelf_db: 2.80,\n            punch_db: 1.00,\n            body_db: 1.20,\n",
)
replace(
    foundation,
    "struct ChannelFoundation {\n    pressure: Biquad,\n    body: Biquad,\n",
    "struct ChannelFoundation {\n    pressure: Biquad,\n    punch: Biquad,\n    body: Biquad,\n",
)
replace(
    foundation,
    "            pressure: Biquad::low_shelf(sample_rate_hz, 85.0, tuning.low_shelf_db),\n            body: Biquad::peaking(sample_rate_hz, 240.0, 0.80, tuning.body_db),\n",
    "            pressure: Biquad::low_shelf(sample_rate_hz, 85.0, tuning.low_shelf_db),\n            punch: Biquad::peaking(sample_rate_hz, 110.0, 0.80, tuning.punch_db),\n            body: Biquad::peaking(sample_rate_hz, 240.0, 0.80, tuning.body_db),\n",
)
replace(
    foundation,
    "        let x = self.pressure.process(sample);\n        let x = self.body.process(x);\n",
    "        let x = self.pressure.process(sample);\n        let x = self.punch.process(x);\n        let x = self.body.process(x);\n",
)
replace(
    foundation,
    "    #[test]\n    fn default_foundation_adds_body_at_240_hz() {\n",
    "    #[test]\n    fn default_foundation_adds_coherent_kick_punch_at_110_hz() {\n        let input = sine(110.0, 16_384);\n        let mut p = MusicFoundationProcessor::new(48_000);\n        let delta = p.process_interleaved_delta(&input);\n        let shaped: Vec<f32> = input.iter().zip(delta.iter()).map(|(a, b)| a + b).collect();\n        let start = 4_096 * 2;\n        assert!(rms(&shaped[start..]) > rms(&input[start..]) * 1.15);\n        for frame in delta[start..].chunks_exact(2) {\n            assert!((frame[0] - frame[1]).abs() < 1.0e-6);\n        }\n    }\n\n    #[test]\n    fn default_foundation_adds_body_at_240_hz() {\n",
)
replace(
    foundation,
    "            low_shelf_db: 0.0,\n            body_db: 0.0,\n",
    "            low_shelf_db: 0.0,\n            punch_db: 0.0,\n            body_db: 0.0,\n",
)

field = "omniphony-renderer/renderer/src/music_field.rs"
replace(field, "const HIGH_BAND_SUPPORT_SCALE: f32 = 0.52;", "const HIGH_BAND_SUPPORT_SCALE: f32 = 0.48;")
replace(field, "const PRESENCE_SUPPORT_SCALE: f32 = 0.86;", "const PRESENCE_SUPPORT_SCALE: f32 = 0.83;")

comp = "omniphony-renderer/renderer/src/binaural/diffuse_compensation.rs"
replace(
    comp,
    "            lower_pinna: Biquad::peaking(sample_rate_hz, 4_800.0, 0.65, -3.40),\n            upper_pinna: Biquad::peaking(sample_rate_hz, 10_000.0, 0.80, -3.00),\n            air_tail: Biquad::high_shelf(sample_rate_hz, 12_000.0, -1.20),\n",
    "            lower_pinna: Biquad::peaking(sample_rate_hz, 4_800.0, 0.65, -3.80),\n            upper_pinna: Biquad::peaking(sample_rate_hz, 10_000.0, 0.80, -3.30),\n            air_tail: Biquad::high_shelf(sample_rate_hz, 12_000.0, -1.35),\n",
)

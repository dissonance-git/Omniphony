from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    if old not in text:
        raise SystemExit(f"anchor missing in {path}:\n{old}")
    p.write_text(text.replace(old, new, 1))


def replace_count(path: str, old: str, new: str, count: int) -> None:
    p = Path(path)
    text = p.read_text()
    actual = text.count(old)
    if actual < count:
        raise SystemExit(f"expected at least {count} occurrences in {path}, found {actual}: {old}")
    p.write_text(text.replace(old, new, count))


config = "omniphony-renderer/assets/binaural-baselines/stereo-field-prototype.yaml"
replace_once(
    config,
    "      - name: \"TFL\"\n        coord_mode: \"cartesian\"\n        x: -0.96\n        y: 1.0\n        z: 1.65\n",
    "      - name: \"TFL\"\n        coord_mode: \"cartesian\"\n        x: -0.96\n        y: 1.0\n        z: 2.15\n",
)
replace_once(
    config,
    "      - name: \"TFR\"\n        coord_mode: \"cartesian\"\n        x: 0.96\n        y: 1.0\n        z: 1.65\n",
    "      - name: \"TFR\"\n        coord_mode: \"cartesian\"\n        x: 0.96\n        y: 1.0\n        z: 2.15\n",
)
replace_once(
    config,
    "  # These are the twelve *evidence-source* poses, not the output layout. The\n  # Windows host renders them through VBAP into the separate 22-direction shell.\n",
    "  # These are the twelve *evidence-source* poses, not the output layout. The\n  # Windows host renders them through VBAP into the separate 22-direction shell.\n  # The upper-front pair is deliberately steeper (~57 deg elevation) than the\n  # rear-height pair: physical listening wants the front face of the sphere to\n  # rise while preserving the successful below-ear / lower-hemisphere wrap.\n",
)

field = "omniphony-renderer/renderer/src/music_field.rs"
replace_once(
    field,
    "const PRESENCE_SUPPORT_SCALE: f32 = 0.83;\n",
    "const PRESENCE_SUPPORT_SCALE: f32 = 0.83;\n/// Raise the *front face* of the sphere without buying elevation with treble.\n/// Only body/presence support is rebalanced upward; >5 kHz is unchanged.\nconst FRONT_CANOPY_LOW_MID_GAIN: f32 = 1.24;\nconst FRONT_CANOPY_PRESENCE_GAIN: f32 = 1.18;\nconst FRONT_HORIZONTAL_LOW_MID_RETENTION: f32 = 0.94;\nconst FRONT_HORIZONTAL_PRESENCE_RETENTION: f32 = 0.96;\n",
)
replace_once(
    field,
    "                let mut band_top_rear_r =\n                    height * (0.06 * broad_r + 0.10 * lateral_r + 0.19 * diffuse_r);\n\n                // Keep the musical body region direct-dominant so kicks, toms,\n",
    "                let mut band_top_rear_r =\n                    height * (0.06 * broad_r + 0.10 * lateral_r + 0.19 * diffuse_r);\n\n                // Front-canopy recentering: move a little body/presence authority\n                // from the ear-level front toward the upper-front evidence pair.\n                // The top band is intentionally untouched so stronger elevation\n                // cannot reintroduce the cymbal/pinna glare fixed by prior builds.\n                let (front_retention, canopy_gain) = if band == 1 {\n                    (FRONT_HORIZONTAL_LOW_MID_RETENTION, FRONT_CANOPY_LOW_MID_GAIN)\n                } else if band == 2 {\n                    (FRONT_HORIZONTAL_PRESENCE_RETENTION, FRONT_CANOPY_PRESENCE_GAIN)\n                } else {\n                    (1.0, 1.0)\n                };\n                band_front_l *= front_retention;\n                band_front_r *= front_retention;\n                band_top_front_l *= canopy_gain;\n                band_top_front_r *= canopy_gain;\n\n                // Keep the musical body region direct-dominant so kicks, toms,\n",
)

worker = "omniphony-renderer/windows_host/src/music_worker_evidence.rs"
replace_once(
    worker,
    "const LINEAR_OUTPUT_GAIN: f32 = 0.90;\nconst METER_INTERVAL_SECS: u64 = 5;\n",
    "const LINEAR_OUTPUT_GAIN: f32 = 0.90;\n/// Requested listening-level reclaim relative to the current grounded build.\n/// This is deliberately downstream of every spatial mechanism.\nconst OUTPUT_MAKEUP_DB: f32 = 3.5;\nconst OUTPUT_MAKEUP_GAIN: f32 = 1.496_235_6;\n/// Conservative sample ceiling leaves margin for inter-sample reconstruction.\nconst OUTPUT_CEILING_DBFS: f32 = -1.0;\nconst OUTPUT_CEILING: f32 = 0.891_250_9;\nconst OUTPUT_LOOKAHEAD_FRAMES: usize = 240; // 5 ms at 48 kHz.\nconst OUTPUT_RELEASE_MS: f32 = 160.0;\nconst METER_INTERVAL_SECS: u64 = 5;\n",
)
replace_once(
    worker,
    "#[derive(Default)]\nstruct SignalMeter {\n",
    "/// Final-bus safety only. This is not a loudness leveller or spatial AGC.\n///\n/// The best spatial build already exists upstream of this point. The guard adds\n/// fixed makeup gain, delays both channels equally, and applies one stereo-linked\n/// attenuation envelope only when a future peak would cross the endpoint ceiling.\n/// Relative L/R amplitude and all upstream spatial relationships are preserved.\nstruct StereoLookaheadPeakGuard {\n    frames: VecDeque<[f32; 2]>,\n    gain: f32,\n    release_coeff: f32,\n    min_gain_since_report: f32,\n}\n\nimpl StereoLookaheadPeakGuard {\n    fn new(sample_rate_hz: u32) -> Self {\n        let release_seconds = OUTPUT_RELEASE_MS / 1000.0;\n        let release_coeff = (-1.0 / (release_seconds * sample_rate_hz.max(1) as f32)).exp();\n        Self {\n            frames: VecDeque::with_capacity(OUTPUT_LOOKAHEAD_FRAMES + 2),\n            gain: 1.0,\n            release_coeff,\n            min_gain_since_report: 1.0,\n        }\n    }\n\n    fn process_interleaved(&mut self, input: &[f32]) -> anyhow::Result<Vec<f32>> {\n        if input.len() % 2 != 0 {\n            bail!(\"output peak guard requires interleaved stereo samples\");\n        }\n        let mut out = Vec::with_capacity(input.len());\n        for frame in input.chunks_exact(2) {\n            let left = if frame[0].is_finite() { frame[0] } else { 0.0 };\n            let right = if frame[1].is_finite() { frame[1] } else { 0.0 };\n            self.frames\n                .push_back([left * OUTPUT_MAKEUP_GAIN, right * OUTPUT_MAKEUP_GAIN]);\n\n            if self.frames.len() <= OUTPUT_LOOKAHEAD_FRAMES {\n                continue;\n            }\n\n            let mut future_peak = 0.0_f32;\n            let mut peak_index = 0usize;\n            for (index, queued) in self.frames.iter().enumerate() {\n                let peak = queued[0].abs().max(queued[1].abs());\n                if peak > future_peak {\n                    future_peak = peak;\n                    peak_index = index;\n                }\n            }\n            let target_gain = if future_peak > OUTPUT_CEILING {\n                OUTPUT_CEILING / future_peak\n            } else {\n                1.0\n            };\n\n            if target_gain < self.gain {\n                // The peak is `peak_index` frames ahead of the sample leaving the\n                // delay line. Ramp only as fast as necessary to reach the target\n                // before that peak arrives.\n                if peak_index == 0 {\n                    self.gain = target_gain;\n                } else {\n                    self.gain += (target_gain - self.gain) / peak_index as f32;\n                }\n            } else {\n                // Release slowly toward the currently-safe target.\n                self.gain = target_gain\n                    - (target_gain - self.gain) * self.release_coeff;\n            }\n\n            let current = self.frames.pop_front().expect(\"lookahead queue is non-empty\");\n            let current_peak = current[0].abs().max(current[1].abs());\n            let immediate_safe_gain = if current_peak > OUTPUT_CEILING {\n                OUTPUT_CEILING / current_peak\n            } else {\n                1.0\n            };\n            let applied_gain = self.gain.min(immediate_safe_gain).clamp(0.0, 1.0);\n            self.gain = self.gain.min(applied_gain);\n            self.min_gain_since_report = self.min_gain_since_report.min(applied_gain);\n            out.push(current[0] * applied_gain);\n            out.push(current[1] * applied_gain);\n        }\n        Ok(out)\n    }\n\n    fn take_max_reduction_db(&mut self) -> f32 {\n        let reduction = if self.min_gain_since_report < 1.0 {\n            -20.0 * self.min_gain_since_report.max(1.0e-6).log10()\n        } else {\n            0.0\n        };\n        self.min_gain_since_report = 1.0;\n        reduction\n    }\n}\n\nfn report_output_peak_guard(guard: &mut StereoLookaheadPeakGuard) {\n    let reduction_db = guard.take_max_reduction_db();\n    println!(\n        \"  output: +{OUTPUT_MAKEUP_DB:.1} dB makeup, ceiling={OUTPUT_CEILING_DBFS:.1} dBFS, max peak reduction={reduction_db:.2} dB\"\n    );\n}\n\n#[derive(Default)]\nstruct SignalMeter {\n",
)
replace_once(
    worker,
    "    let mut foundation = MusicFoundationProcessor::new(SAMPLE_RATE_HZ);\n    let mut pcm_bytes = Vec::<u8>::new();\n",
    "    let mut foundation = MusicFoundationProcessor::new(SAMPLE_RATE_HZ);\n    let mut output_peak_guard = StereoLookaheadPeakGuard::new(SAMPLE_RATE_HZ);\n    let mut pcm_bytes = Vec::<u8>::new();\n",
)
replace_once(
    worker,
    "    println!(\n        \"  headroom: {:.1} dB fixed linear output gain, identical ON/OFF reference gain\",\n        20.0 * LINEAR_OUTPUT_GAIN.log10()\n    );\n",
    "    println!(\n        \"  output: {:.1} dB base trim + {OUTPUT_MAKEUP_DB:.1} dB makeup; {OUTPUT_CEILING_DBFS:.1} dBFS stereo-linked look-ahead safety ceiling\",\n        20.0 * LINEAR_OUTPUT_GAIN.log10()\n    );\n",
)
replace_once(
    worker,
    "        if args.start_off {\n            queue_block(&play_tx, output_reference)?;\n",
    "        if args.start_off {\n            let output_reference = output_peak_guard.process_interleaved(&output_reference)?;\n            queue_block(&play_tx, output_reference)?;\n",
)
replace_once(
    worker,
    "                report_playback_underruns(&playback_underrun_frames);\n                last_meter_report = Instant::now();\n",
    "                report_playback_underruns(&playback_underrun_frames);\n                report_output_peak_guard(&mut output_peak_guard);\n                last_meter_report = Instant::now();\n",
)
replace_once(
    worker,
    "            let dry_reference = apply_output_headroom(&dry);\n            added_meter.observe_delta(&mixed, &dry_reference);\n            queue_block(&play_tx, mixed)?;\n",
    "            let dry_reference = apply_output_headroom(&dry);\n            added_meter.observe_delta(&mixed, &dry_reference);\n            let mixed = output_peak_guard.process_interleaved(&mixed)?;\n            queue_block(&play_tx, mixed)?;\n",
)
# Replace the second meter-report occurrence (ON path) after the OFF-path one was already changed.
replace_once(
    worker,
    "            report_playback_underruns(&playback_underrun_frames);\n            last_meter_report = Instant::now();\n",
    "            report_playback_underruns(&playback_underrun_frames);\n            report_output_peak_guard(&mut output_peak_guard);\n            last_meter_report = Instant::now();\n",
)

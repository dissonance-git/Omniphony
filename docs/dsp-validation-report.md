# DSP Validation — Phase 1 Measurement Report

Measured on 2026-07-30, commit 8160dbb, x86_64 Linux, `cargo test -p renderer`.

Thresholds are theory-derived (see
`docs/superpowers/specs/2026-07-30-dsp-validation-harness-design.md`, D2). A
"misses" verdict is a finding about the engine, not a defect in the test.

| Metric | Theoretical target | Measured | Verdict |
| --- | --- | --- | --- |
| LR4 reconstruction flatness (4 bands, 48 kHz) | ±0.25 dB | +0.0018 dB at 320.8 Hz | meets |
| VBAP energy conservation (7.1.4, 512 dirs) | ±0.25 dB | −24.6592 dB at az=66.5° el=−86.4° | misses |
| VBAP seam continuity ratio (7.1.4, 512 dirs) | < 0.65 | 0.9991 at az=77.7° el=−22.6° | misses |
| ITD magnitude vs model (worst of 7 azimuths) | ±3 samples | +36.822 samples at az=+90° | misses |
| ITD antisymmetry (worst of ±30/60/90°) | \|sum\| ≤ 1 sample | −1.829 samples at ±60° | misses |
| ITD monotonicity (0/30/60/90°) | strictly increasing | [0.104, 13.713, 26.062, 5.343] — falls at 90° | misses |

## Raw output

```
[measure] lr4_flatness cutoffs=[80.0, 200.0, 500.0] fs=48000: worst deviation +0.0018 dB at 320.8 Hz (target ±0.25 dB)
[measure] vbap_energy 7.1.4 (11 spatialized speakers, 512 directions): worst -24.6592 dB at az=66.5 el=-86.4 (target ±0.25 dB)
[measure] vbap_seams 7.1.4 (512 directions): worst ‖Δ0.5°‖/‖Δ1°‖ ratio 0.9991 at az=77.7 el=-22.6 (continuous ⇒ ≈0.5, target <0.65)
[measure] itd az=  +0.0°: measured  +0.104 samples (   +2.2 µs), model  +0.000 samples, delta +0.104 samples (target ±3)
[measure] itd az= +30.0°: measured -13.713 samples ( -285.7 µs), model -12.534 samples, delta -1.180 samples (target ±3)
[measure] itd az= -30.0°: measured +12.212 samples ( +254.4 µs), model +12.534 samples, delta -0.322 samples (target ±3)
[measure] itd az= +60.0°: measured -26.062 samples ( -543.0 µs), model -23.427 samples, delta -2.635 samples (target ±3)
[measure] itd az= -60.0°: measured +24.234 samples ( +504.9 µs), model +23.427 samples, delta +0.806 samples (target ±3)
[measure] itd az= +90.0°: measured  +5.343 samples ( +111.3 µs), model -31.479 samples, delta +36.822 samples (target ±3)
[measure] itd az= -90.0°: measured  -3.903 samples (  -81.3 µs), model +31.479 samples, delta -35.383 samples (target ±3)
[measure] itd antisymmetry ±30°: -13.713 vs +12.212, sum -1.502 samples (target |sum| ≤ 1)
[measure] itd antisymmetry ±60°: -26.062 vs +24.234, sum -1.829 samples (target |sum| ≤ 1)
[measure] itd antisymmetry ±90°: +5.343 vs -3.903, sum +1.440 samples (target |sum| ≤ 1)
[measure] itd monotonicity |lag| at 0/30/60/90°: [0.103639506, 13.713405, 26.06226, 5.343322] (target strictly increasing)
```

## Observations

**LR4 flatness** is the one metric with margin to spare: +0.0018 dB is roughly
two orders of magnitude inside the ±0.25 dB tolerance, which is what the
allpass-cascade argument predicts once coefficient and float error are the only
contributors left.

**VBAP energy** is conserved almost everywhere but collapses toward the nadir.
The worst direction, el = −86.4°, is below every speaker in the 7.1.4 layout —
there is no triplet enclosing it, so the reported −24.66 dB is a measurement of
what the panner does outside the convex hull, not of the normalisation itself.
Whether that region should be gated at all, clamped, or excluded from the sweep
is a decision for phase 2 rather than something to ratchet the tolerance around.

**VBAP seams**: a ratio of 0.9991 is the signature of a jump — halving the
angular step did not halve the gain-vector difference at all. Energy stayed
conserved at that direction, which is exactly the failure mode the ratio metric
was added to catch.

**ITD** tracks the Woodworth model well at 0°, ±30° and ±60° (worst delta −2.635
samples, inside ±3). At ±90° the measurement breaks down entirely: the sign
flips and the magnitude collapses to ~5 and ~4 samples where the model predicts
∓31.5. That single pair of points is what drives all three ITD verdicts —
magnitude (+36.8 samples of error), monotonicity (|lag| falls from 26.1 at 60°
to 5.3 at 90°), and it is the ±90° row of the antisymmetry table. Antisymmetry
additionally misses at ±30° and ±60° by 0.5 and 0.8 samples beyond the
1-sample bound, so it is not solely a ±90° artefact.

The ±90° behaviour is consistent with either a genuine engine issue at full
interaural deflection or a limitation of the cross-correlation estimator when
the contralateral HRIR is heavily shadowed and spectrally dissimilar to the
ipsilateral one — the two signals are no longer near-copies of each other, which
is the assumption the lag estimate rests on. Phase 2 records the deferral; it
does not diagnose it.

## Gating decision

There is no issue tracker in this workflow, so each metric marked "misses" is
tracked in this report instead of by issue number. Task 11 lands each as a
tracked deferral (`#[ignore]` carrying the measured value) rather than a gate.

| Metric | Issue | Deferred value recorded in `#[ignore]` |
| --- | --- | --- |
| VBAP energy conservation (`vbap_conserves_energy_over_the_sphere`) | tracked in this report | measured −24.6592 dB at az=66.5° el=−86.4°, target ±0.25 dB |
| VBAP seam continuity (`vbap_gains_are_continuous_across_triplet_boundaries`) | tracked in this report | measured ratio 0.9991 at az=77.7° el=−22.6°, target < 0.65 |
| ITD magnitude vs model (`itd_magnitude_tracks_the_model`) | tracked in this report | measured delta +36.822 samples at az=+90°, target ±3 samples |
| ITD antisymmetry (`itd_is_antisymmetric_about_the_median_plane`) | tracked in this report | measured sum −1.829 samples at ±60°, target \|sum\| ≤ 1 sample |
| ITD monotonicity (`itd_magnitude_grows_toward_the_interaural_axis`) | tracked in this report | measured \|lag\| [0.104, 13.713, 26.062, 5.343] at 0/30/60/90°, target strictly increasing |

LR4 reconstruction flatness (`lr4_reconstruction_is_magnitude_flat`) meets its
target and lands as a live gate with no deferral.

### Wide matrix

Task 12 adds an opt-in wide matrix behind `--features renderer/wide-matrix`.
Its LR4 case is a live gate; the two cases that widen an already-deferred metric
inherit that deferral, so `cargo test --workspace --features renderer/wide-matrix`
stays green unless something *new* breaks.

| Wide case | Status | Deferred value recorded in `#[ignore]` |
| --- | --- | --- |
| `lr4_reconstruction_is_magnitude_flat_wide` (3 cutoff sets × 44.1/48/96 kHz) | gate | — |
| `vbap_conserves_energy_over_the_sphere_wide` (5.1, 7.1, 7.1.4, 9.1.6 × 4 spreads, 8192 dirs) | deferred | 5.1 spread=0 has a silent direction at az=−117.4° el=86.5° |
| `itd_magnitude_tracks_the_model_wide` (13 azimuths, 30° apart) | deferred | measured delta −39.954 samples at az=−120°, target ±3 samples |

The wide VBAP case fails earlier and harder than its narrow counterpart: on the
5.1 preset the spatialized speakers are coplanar, so directions near the zenith
fall outside the convex hull entirely and receive no energy at all rather than
merely mis-normalised energy. That is the same convex-hull question the
narrow-gate observation above raises, seen from a layout with no height layer.

The wide ITD case reaches azimuths the narrow gate never visits (±120° and
±150°) and breaks down there in the same way it does at ±90°: at az = −120° the
measured lag is −16.5 samples where the model predicts +23.4, a sign flip.

## Addendum — the ITD failure bracketed

The three ITD deferrals above all report their failure at the sampled azimuths
(±90°, and ±120° in the wide matrix), which makes the defect look like a
broad lateral/rear inaccuracy. A denser sweep run afterwards shows it is
something much sharper: a **discontinuity between 80° and 85°**.

| Azimuth | Measured lag (samples) | Model (samples) | Delta |
| ---: | ---: | ---: | ---: |
| +0° | +0.104 | +0.000 | +0.10 |
| +30° | −13.713 | −12.534 | −1.18 |
| +60° | −26.062 | −23.427 | −2.64 |
| +75° | −29.950 | −27.856 | −2.09 |
| **+80°** | **−31.387** | **−29.156** | **−2.23** |
| **+85°** | **+6.991** | **−30.364** | **+37.36** |
| +88° | +5.957 | −31.044 | +37.00 |
| +89° | +5.693 | −31.264 | +36.96 |
| +90° | +5.343 | −31.479 | +36.82 |
| +91° | +5.567 | −31.264 | +36.83 |
| +95° | +6.132 | −30.364 | +36.50 |
| +100° | +2.946 | −29.156 | +32.10 |
| +120° | +15.912 | −23.427 | +39.34 |

Up to +80° the rendered ITD tracks Woodworth closely and monotonically, running
2–3 samples long in exactly the way per-ear HRIR group delay predicts. Between
+80° and +85° it **inverts sign and collapses in magnitude**, and never
recovers: every azimuth beyond that point reports the contralateral ear as the
*early* one, which inverts the primary localisation cue for all lateral and rear
sources.

Two things follow from the bracketing:

- **The measurement is sound.** A harness that agrees with the model to within
  2–3 samples across 0–80°, then disagrees by 37, is not mis-measuring; it is
  reporting a real discontinuity in the rendered output.
- **The defect is narrow.** Whatever changes between 80° and 85° is a single
  branch or lookup boundary, not a diffuse accuracy problem. The three ITD
  deferrals are very likely one root cause, not three.

Worth noting for whoever picks this up: this region is where the source
approaches the interaural axis, and it is close to — but not exactly at — the
y = 0 plane where the front/rear hemisphere distinction flips (`cos 85° ≈ 0.09`).
Both the front/back folding in `binaural/itd.rs` and the HRIR direction lookup
in `binaural/measured.rs` are plausible places for a boundary at that angle.

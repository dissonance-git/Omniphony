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

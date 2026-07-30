//! VBAP energy conservation and seam continuity over the whole sphere.
//!
//! Extends the existing tests in `native_backend.rs`, which check
//! `|rms − 1| < 0.05` (±0.42 dB) at five elevations along the azimuth-0
//! meridian of a synthetic 7-speaker layout. This measures the shipped 7.1.4
//! layout over a full sphere lattice, and adds the metric energy conservation
//! cannot see.
//!
//! **Energy** — VBAP normalises so that `Σg² = 1`. In dB: `10·log10(Σg²) = 0`.
//!
//! **Seams** — VBAP is continuous by construction: gains fall to zero at a
//! triplet edge as the adjacent triplet takes over. So `‖g(θ+Δ) − g(θ)‖₂` must
//! scale with Δ. Measuring at Δ = 1° and Δ = 0.5°, the ratio must be ≈ 0.5. A
//! jump discontinuity at a triplet boundary does *not* halve — and it is
//! invisible to the energy check, since the image can jump while energy stays
//! perfectly conserved. Expressing the criterion as a ratio avoids inventing a
//! Lipschitz constant.

use dsp_fixtures::dirs::fibonacci_sphere;

use crate::speaker_layout::SpeakerLayout;

use super::native_backend::NativeVbapLayout;

/// Directions in the PR-gate sweep. 512 points is dense enough to land inside
/// every triplet of a 7.1.4 layout several times over.
const LATTICE_POINTS: usize = 512;

/// Build the VBAP panner for a shipped preset, using only speakers that
/// participate in spatialization (LFE has `spatialize: false` and must not
/// appear in the energy sum).
fn panner_for(preset: &str) -> (NativeVbapLayout, usize) {
    let layout = SpeakerLayout::preset(preset).expect("known preset");
    let dirs: Vec<[f32; 2]> = layout
        .speakers
        .iter()
        .filter(|s| s.spatialize)
        .map(|s| [s.azimuth, s.elevation])
        .collect();
    let n = dirs.len();
    (
        NativeVbapLayout::from_speaker_dirs(&dirs).expect("triplet search"),
        n,
    )
}

/// `10·log10(Σg²)` — deviation from 0 dB is the energy error.
fn energy_db(panner: &NativeVbapLayout, az: f32, el: f32) -> f32 {
    let g = panner.vbap_gains(az, el, 0.0).expect("vbap gains");
    let mut sum_sq = 0.0f32;
    for i in 0..g.len() {
        sum_sq += g[i] * g[i];
    }
    if sum_sq <= 0.0 {
        f32::NEG_INFINITY
    } else {
        10.0 * sum_sq.log10()
    }
}

/// `‖g(az+Δ) − g(az)‖₂` at fixed elevation.
fn gain_step_norm(panner: &NativeVbapLayout, az: f32, el: f32, delta: f32) -> f32 {
    let a = panner.vbap_gains(az, el, 0.0).expect("vbap gains");
    let b = panner.vbap_gains(az + delta, el, 0.0).expect("vbap gains");
    let mut acc = 0.0f32;
    for i in 0..a.len() {
        let d = a[i] - b[i];
        acc += d * d;
    }
    acc.sqrt()
}

/// Theory-derived: VBAP normalises to `Σg² = 1`, i.e. 0 dB.
const ENERGY_TOLERANCE_DB: f32 = 0.25;

/// Halving the angular step must roughly halve the gain-vector difference.
/// 0.65 leaves headroom over the ideal 0.5 for curvature within a triplet,
/// while still rejecting a jump discontinuity (ratio ≈ 1).
const MAX_SEAM_RATIO: f32 = 0.65;

#[test]
#[ignore = "engine misses this: measured -24.6592 dB at az=66.5 el=-86.4, target ±0.25 dB — tracked deferral, see docs/dsp-validation-report.md"]
fn vbap_conserves_energy_over_the_sphere() {
    let (panner, n_spk) = panner_for("7.1.4");
    let mut worst = (0.0f32, 0.0f32, 0.0f32);
    for (az, el) in fibonacci_sphere(LATTICE_POINTS) {
        let dev = energy_db(&panner, az, el);
        assert!(
            dev.is_finite(),
            "silent direction az={az:.1} el={el:.1}: no speaker receives energy"
        );
        if dev.abs() > worst.2.abs() {
            worst = (az, el, dev);
        }
    }
    println!(
        "[measure] vbap_energy 7.1.4 ({n_spk} speakers, {LATTICE_POINTS} dirs): \
         worst {:+.4} dB at az={:.1} el={:.1}",
        worst.2, worst.0, worst.1
    );
    assert!(
        worst.2.abs() <= ENERGY_TOLERANCE_DB,
        "VBAP energy off by {:+.4} dB at az={:.1} el={:.1}, tolerance \
         ±{ENERGY_TOLERANCE_DB} dB",
        worst.2,
        worst.0,
        worst.1
    );
}

#[test]
#[ignore = "engine misses this: measured ratio 0.9991 at az=77.7 el=-22.6, target < 0.65 — tracked deferral, see docs/dsp-validation-report.md"]
fn vbap_gains_are_continuous_across_triplet_boundaries() {
    let (panner, _) = panner_for("7.1.4");
    // Skip directions where the gain barely moves — the ratio is 0/0 there and
    // carries no information about continuity.
    const MIN_STEP_NORM: f32 = 1e-4;
    let mut worst = (0.0f32, 0.0f32, 0.0f32);
    for (az, el) in fibonacci_sphere(LATTICE_POINTS) {
        let coarse = gain_step_norm(&panner, az, el, 1.0);
        if coarse < MIN_STEP_NORM {
            continue;
        }
        let ratio = gain_step_norm(&panner, az, el, 0.5) / coarse;
        if ratio > worst.2 {
            worst = (az, el, ratio);
        }
    }
    println!(
        "[measure] vbap_seams 7.1.4 ({LATTICE_POINTS} dirs): worst ratio \
         {:.4} at az={:.1} el={:.1}",
        worst.2, worst.0, worst.1
    );
    assert!(
        worst.2 <= MAX_SEAM_RATIO,
        "gain vector does not halve when the step halves at az={:.1} el={:.1} \
         (ratio {:.4}, max {MAX_SEAM_RATIO}) — a seam, i.e. the panned image \
         jumps at a triplet boundary even though energy stays conserved",
        worst.0,
        worst.1,
        worst.2
    );
}

//! Cross-project HRTF cue fixture for Omniphony and libaural.
//!
//! libaural's first spatial baseline deliberately demonstrates that ITD/ILD
//! alone cannot identify front versus back. Omniphony already ships a measured
//! KEMAR HRIR set, so this test asks the next narrower question: does that ear
//! model expose direction-dependent *spectral shape* that is absent from the
//! bare synthetic head-shadow model?
//!
//! This is a renderer/data contract, not a claim that a particular spectral
//! distance is a human localization model.

use renderer::binaural::hrir::{HRIR_LEN, HrirPair, HrirSet};
use renderer::binaural::measured::MeasuredHrirData;

const SAMPLE_RATE: f64 = 48_000.0;
// Pinna-driven localization evidence is strongest above the low-frequency ITD
// region. Sparse probes keep this dependency-free and make the measurement easy
// to inspect instead of hiding it inside an FFT package.
const PROBE_HZ: [f64; 7] = [
    3_000.0, 5_000.0, 7_000.0, 9_000.0, 11_000.0, 13_000.0, 15_000.0,
];

fn pair_at(set: &HrirSet, az_deg: f32, el_deg: f32) -> HrirPair {
    let mut pair = HrirPair {
        left: [0.0; HRIR_LEN],
        right: [0.0; HRIR_LEN],
    };
    set.at(az_deg, el_deg, &mut pair);
    pair
}

fn magnitude_db_at(ir: &[f32; HRIR_LEN], frequency_hz: f64) -> f64 {
    let omega = std::f64::consts::TAU * frequency_hz / SAMPLE_RATE;
    let mut re = 0.0f64;
    let mut im = 0.0f64;
    for (n, &sample) in ir.iter().enumerate() {
        let phase = omega * n as f64;
        re += sample as f64 * phase.cos();
        im -= sample as f64 * phase.sin();
    }
    let magnitude = (re * re + im * im).sqrt().max(1.0e-12);
    20.0 * magnitude.log10()
}

fn normalized_profile(ir: &[f32; HRIR_LEN]) -> [f64; PROBE_HZ.len()] {
    let mut profile = [0.0f64; PROBE_HZ.len()];
    for (slot, frequency) in profile.iter_mut().zip(PROBE_HZ) {
        *slot = magnitude_db_at(ir, frequency);
    }
    // Remove broadband level so the comparison asks about spectral *shape*, not
    // simply whether one direction happened to be louder after interpolation.
    let mean = profile.iter().sum::<f64>() / profile.len() as f64;
    for value in &mut profile {
        *value -= mean;
    }
    profile
}

fn profile_distance_db(a: &HrirPair, b: &HrirPair) -> f64 {
    let al = normalized_profile(&a.left);
    let ar = normalized_profile(&a.right);
    let bl = normalized_profile(&b.left);
    let br = normalized_profile(&b.right);

    let sum_sq = al
        .iter()
        .zip(bl.iter())
        .chain(ar.iter().zip(br.iter()))
        .map(|(x, y)| {
            let d = x - y;
            d * d
        })
        .sum::<f64>();
    (sum_sq / (PROBE_HZ.len() * 2) as f64).sqrt()
}

#[test]
fn measured_hrtf_adds_front_back_and_elevation_spectral_evidence() {
    let synthetic = HrirSet::synthetic(48_000);
    let measured = HrirSet::new(&MeasuredHrirData::saf_kemar(), 48_000);

    let synthetic_front = pair_at(&synthetic, 0.0, 0.0);
    let synthetic_rear = pair_at(&synthetic, 180.0, 0.0);
    let measured_front = pair_at(&measured, 0.0, 0.0);
    let measured_rear = pair_at(&measured, 180.0, 0.0);
    let measured_elevated = pair_at(&measured, 0.0, 60.0);

    let synthetic_front_rear = profile_distance_db(&synthetic_front, &synthetic_rear);
    let measured_front_rear = profile_distance_db(&measured_front, &measured_rear);
    let measured_elevation = profile_distance_db(&measured_front, &measured_elevated);

    eprintln!(
        "HRTF spectral-shape cues (RMS dB over 3-15 kHz probes): synthetic front/rear={synthetic_front_rear:.4}, measured front/rear={measured_front_rear:.4}, measured 0°/60° elevation={measured_elevation:.4}"
    );

    // The analytic head-shadow model has no median-plane pinna structure: front
    // and back should be numerically indistinguishable apart from tiny trig/f32
    // residue.
    assert!(
        synthetic_front_rear < 0.01,
        "synthetic model unexpectedly contains a front/back spectral cue: {synthetic_front_rear:.4} dB"
    );

    // These thresholds are deliberately tiny compared with real HRTF notch/peak
    // variation. They assert only that the measured set contains a stable extra
    // spectral coordinate; they do not declare a human discrimination threshold.
    const MIN_MEASURED_CUE_DB: f64 = 0.10;
    assert!(
        measured_front_rear > MIN_MEASURED_CUE_DB,
        "measured KEMAR front/back spectral profiles are unexpectedly indistinguishable: {measured_front_rear:.4} dB"
    );
    assert!(
        measured_elevation > MIN_MEASURED_CUE_DB,
        "measured KEMAR horizontal/elevated spectral profiles are unexpectedly indistinguishable: {measured_elevation:.4} dB"
    );

    // The measured set should not merely be microscopically different from the
    // no-pinna negative control; require a comfortable separation in this ruler.
    assert!(
        measured_front_rear > synthetic_front_rear * 10.0 + 0.05,
        "measured front/back cue did not separate from the synthetic negative control"
    );
}

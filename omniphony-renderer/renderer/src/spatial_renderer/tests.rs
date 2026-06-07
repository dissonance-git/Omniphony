//! Unit tests for the spatial render path, split out of `mod.rs` to keep the
//! core renderer file focused. Child module of `spatial_renderer`, so `super`
//! resolves to the renderer module and its private items.

use super::*;
// Types the tests construct directly. Imported here (not relied upon via
// `super::*`) so the production `mod.rs` only imports what its own code uses.
use crate::live_params::{LiveEvaluationMode, PreferredEvaluationMode};
use crate::render_backend::EffectiveEvaluationMode;
use crate::spatial_vbap::VbapTableMode;
use crate::speaker_layout::SpeakerLayout;

/// The unified multi-band cartesian table must render bit-equivalently to the
/// per-band path it replaces. Build two identical crossover renderers, force
/// one onto the per-band path (`unified_table = None`), feed both the same
/// frame, and require matching output.
#[test]
fn unified_crossover_matches_per_band() {
    fn build() -> SpatialRenderer {
        let mut layout = SpeakerLayout::preset("7.1.4").unwrap();
        for (sp, cutoff) in layout.speakers.iter_mut().zip([80.0, 200.0, 500.0]) {
            sp.freq_low = Some(cutoff);
        }
        SpatialRenderer::new(
            layout,
            48_000,
            1,
            1,
            0.0,
            2.0,
            VbapTableMode::Cartesian {
                x_size: 21,
                y_size: 21,
                z_size: 9,
                z_neg_size: 9,
            },
            false,
            true, // position interpolation → trilinear lookup + per-sample motion
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
            PreferredEvaluationMode::PrecomputedCartesian,
            LiveEvaluationMode::PrecomputedCartesian,
            21,
            21,
            9,
            9,
        )
        .unwrap()
    }

    let mut unified = build();
    assert!(
        unified.unified_table.is_some(),
        "crossover layout should build a unified table"
    );
    let mut per_band = build();
    per_band.unified_table = None;

    let pcm: Vec<f32> = (0..40).map(|i| (i * 7 % 13) as f32 / 13.0 - 0.5).collect();
    let event = vec![SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(0),
        ramp_length: Some(40),
        size: Some([0.0, 0.0, 0.0]),
        position: Some([0.3, -0.2, 0.4]),
        sample_pos: Some(0),
    }];

    let a = unified
        .render_frame(&pcm, 1, &event, Vec::new(), false)
        .unwrap();
    let b = per_band
        .render_frame(&pcm, 1, &event, Vec::new(), false)
        .unwrap();
    assert_eq!(a.samples.len(), b.samples.len());
    let max_diff = a
        .samples
        .iter()
        .zip(&b.samples)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff < 1e-6,
        "unified vs per-band output mismatch: max diff {max_diff}"
    );
}

/// Polar counterpart of `unified_crossover_matches_per_band`: the unified
/// multi-band POLAR table must render bit-equivalently to the per-band polar
/// path. Same crossover layout, but a precomputed polar evaluator.
#[test]
fn unified_polar_matches_per_band() {
    fn build() -> SpatialRenderer {
        let mut layout = SpeakerLayout::preset("7.1.4").unwrap();
        for (sp, cutoff) in layout.speakers.iter_mut().zip([80.0, 200.0, 500.0]) {
            sp.freq_low = Some(cutoff);
        }
        SpatialRenderer::new(
            layout,
            48_000,
            1,
            1,
            0.0,
            2.0,
            VbapTableMode::Polar,
            false,
            true, // position interpolation → trilinear lookup + per-sample motion
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
        .unwrap()
    }

    let mut unified = build();
    assert!(
        unified.unified_table.is_some(),
        "polar crossover layout should build a unified table"
    );
    let mut per_band = build();
    per_band.unified_table = None;

    let pcm: Vec<f32> = (0..40).map(|i| (i * 7 % 13) as f32 / 13.0 - 0.5).collect();
    let event = vec![SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(0),
        ramp_length: Some(40),
        size: Some([0.0, 0.0, 0.0]),
        position: Some([0.3, -0.2, 0.4]),
        sample_pos: Some(0),
    }];

    let a = unified
        .render_frame(&pcm, 1, &event, Vec::new(), false)
        .unwrap();
    let b = per_band
        .render_frame(&pcm, 1, &event, Vec::new(), false)
        .unwrap();
    assert_eq!(a.samples.len(), b.samples.len());
    let max_diff = a
        .samples
        .iter()
        .zip(&b.samples)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff < 1e-6,
        "unified polar vs per-band output mismatch: max diff {max_diff}"
    );
}

/// A crossover band with only 1–2 speakers used to have no engine (hardcoded
/// equal-power), which disabled the unified table for the whole crossover.
/// Now such a band carries a `FewSpeakerBackend`, so the unified table builds
/// and must stay bit-equivalent to the per-band path. Here the top band keeps
/// exactly 2 spatializable speakers (pairwise-VBAP fallback).
#[test]
fn unified_table_with_two_speaker_fallback_band() {
    fn build() -> SpatialRenderer {
        let mut layout = SpeakerLayout::preset("7.1.4").unwrap();
        // Cut all spatializable speakers at 200 Hz except the first two, so the
        // [200, ∞) band has exactly 2 speakers (a fallback band) and the
        // [0, 200) band keeps the rest (a normal ≥3 VBAP band).
        let mut kept = 0;
        for sp in layout.speakers.iter_mut() {
            if !sp.spatialize {
                continue;
            }
            if kept < 2 {
                kept += 1;
                continue;
            }
            sp.freq_high = Some(200.0);
        }
        SpatialRenderer::new(
            layout,
            48_000,
            1,
            1,
            0.0,
            2.0,
            VbapTableMode::Cartesian {
                x_size: 21,
                y_size: 21,
                z_size: 9,
                z_neg_size: 9,
            },
            false,
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
            PreferredEvaluationMode::PrecomputedCartesian,
            LiveEvaluationMode::PrecomputedCartesian,
            21,
            21,
            9,
            9,
        )
        .unwrap()
    }

    let mut unified = build();
    assert!(
        unified.unified_table.is_some(),
        "a 2-speaker fallback band must not disable the unified table"
    );
    let mut per_band = build();
    per_band.unified_table = None;

    let pcm: Vec<f32> = (0..40).map(|i| (i * 7 % 13) as f32 / 13.0 - 0.5).collect();
    let event = vec![SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(0),
        ramp_length: Some(40),
        size: Some([0.0, 0.0, 0.0]),
        position: Some([0.3, -0.2, 0.4]),
        sample_pos: Some(0),
    }];

    let a = unified
        .render_frame(&pcm, 1, &event, Vec::new(), false)
        .unwrap();
    let b = per_band
        .render_frame(&pcm, 1, &event, Vec::new(), false)
        .unwrap();
    assert_eq!(a.samples.len(), b.samples.len());
    let max_diff = a
        .samples
        .iter()
        .zip(&b.samples)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max);
    assert!(
        max_diff < 1e-6,
        "unified vs per-band output mismatch (fallback band): max diff {max_diff}"
    );
}

/// An evaluation-mode change must reuse the triangulated gain model (the
/// geometry is mode-independent), rebuilding only the evaluation wrapper. A
/// geometry change (bumped generation) must rebuild the model. Verified via
/// `Arc::ptr_eq` on the decorated model.
#[test]
fn eval_mode_change_reuses_geometry() {
    let layout = SpeakerLayout::preset("7.1.4").unwrap();
    let r = SpatialRenderer::new(
        layout,
        48_000,
        1,
        1,
        0.0,
        2.0,
        VbapTableMode::Cartesian {
            x_size: 21,
            y_size: 21,
            z_size: 9,
            z_neg_size: 9,
        },
        false,
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
        PreferredEvaluationMode::PrecomputedCartesian,
        LiveEvaluationMode::PrecomputedCartesian,
        21,
        21,
        9,
        9,
    )
    .unwrap();
    let control = r.renderer_control();
    let topo0 = control.active_topology();
    let model0 = topo0
        .backend
        .decorated_model()
        .expect("vbap backend exposes a decorated model");

    // Evaluation-mode-only change: geometry generation unchanged → reuse model.
    control
        .live
        .write()
        .set_evaluation_mode(LiveEvaluationMode::Realtime);
    let plan = control.prepare_topology_rebuild().expect("rebuild plan");
    let reused = plan
        .build_topology_reusing(Some(&topo0))
        .expect("reuse build");
    assert_eq!(
        reused.backend.evaluation_mode(),
        EffectiveEvaluationMode::Realtime
    );
    assert!(
        Arc::ptr_eq(&model0, &reused.backend.decorated_model().unwrap()),
        "evaluation-mode change must reuse the triangulated gain model"
    );

    // Geometry change bumps the generation → full rebuild (different model).
    control.bump_geometry_generation();
    let plan2 = control.prepare_topology_rebuild().expect("rebuild plan 2");
    let rebuilt = plan2.build_topology_reusing(Some(&topo0)).expect("rebuild");
    assert!(
        !Arc::ptr_eq(&model0, &rebuilt.backend.decorated_model().unwrap()),
        "a geometry change must rebuild the gain model"
    );
}

#[test]
fn test_renderer_creation() {
    let layout = SpeakerLayout::preset("7.1.4").unwrap();
    let renderer = SpatialRenderer::new(
        layout,
        48000,
        1,
        1,
        0.0,
        2.0,
        VbapTableMode::Polar,
        false,
        false,
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
    );

    assert!(renderer.is_ok());

    let renderer = renderer.unwrap();
    assert_eq!(renderer.num_speakers(), 12);
}

/// Guard rail: the four ramp modes must stay wired and each keep its own
/// behaviour. `Off` snaps to the target, `Frame` holds the block-start
/// position, `Sample` interpolates the position per sample, and `Interp`
/// interpolates the gains per sample from the previous block's end. We render
/// TWO blocks per mode with a position change in between (the first block
/// seeds `Interp`'s start gains, so its ramp only shows on the second) and
/// compare the second block: every output must be finite, non-silent, and
/// the modes must not collapse onto one another for a moving object.
#[test]
fn all_four_ramp_modes_render_distinctly() {
    fn build() -> SpatialRenderer {
        let layout = SpeakerLayout::preset("7.1.4").unwrap();
        SpatialRenderer::new(
            layout,
            48_000,
            1,
            1,
            0.0,
            2.0,
            VbapTableMode::Cartesian {
                x_size: 21,
                y_size: 21,
                z_size: 9,
                z_neg_size: 9,
            },
            false,
            true, // position interpolation → trilinear lookup + per-sample motion
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
            PreferredEvaluationMode::PrecomputedCartesian,
            LiveEvaluationMode::PrecomputedCartesian,
            21,
            21,
            9,
            9,
        )
        .unwrap()
    }

    let pcm = vec![0.5f32; 40];
    let event_at = |position: [f64; 3]| {
        vec![SpatialChannelEvent {
            channel_idx: 0,
            is_bed: false,
            gain_db: Some(0),
            ramp_length: Some(40),
            size: Some([0.0, 0.0, 0.0]),
            position: Some(position),
            sample_pos: Some(0),
        }]
    };
    let block_a = event_at([-0.7, 0.5, 0.2]);
    let block_b = event_at([0.8, -0.6, 0.5]);

    let render = |mode: RampMode| -> Vec<f32> {
        let mut r = build();
        r.control.live.write().ramp_mode = mode;
        // First block establishes a position (and seeds Interp's start gains).
        r.render_frame(&pcm, 1, &block_a, Vec::new(), false)
            .unwrap();
        // Second block moves the object — this is what we compare.
        r.render_frame(&pcm, 1, &block_b, Vec::new(), false)
            .unwrap()
            .samples
    };

    let off = render(RampMode::Off);
    let frame = render(RampMode::Frame);
    let sample = render(RampMode::Sample);
    let interp = render(RampMode::Interp);

    let expected_len = 40 * 12;
    for (name, out) in [
        ("off", &off),
        ("frame", &frame),
        ("sample", &sample),
        ("interp", &interp),
    ] {
        assert_eq!(out.len(), expected_len, "{name}: wrong output length");
        assert!(
            out.iter().all(|x| x.is_finite()),
            "{name}: non-finite output"
        );
        let energy: f32 = out.iter().map(|x| x * x).sum();
        assert!(energy > 0.0, "{name}: produced silence");
    }

    let max_diff = |a: &[f32], b: &[f32]| {
        a.iter()
            .zip(b)
            .map(|(x, y)| (x - y).abs())
            .fold(0.0f32, f32::max)
    };

    assert!(max_diff(&off, &frame) > 1e-3, "Off vs Frame collapsed");
    assert!(max_diff(&off, &sample) > 1e-3, "Off vs Sample collapsed");
    assert!(max_diff(&off, &interp) > 1e-3, "Off vs Interp collapsed");
    assert!(
        max_diff(&frame, &sample) > 1e-3,
        "Frame vs Sample collapsed"
    );
    // Sample (position-space) and Interp (gain-space) interpolate the same
    // endpoints differently, so they diverge mid-block too.
    assert!(
        max_diff(&sample, &interp) > 1e-3,
        "Sample vs Interp collapsed"
    );
}

// TODO: Add integration test with real spatial metadata
// For now, testing is done via real spatial audio content decoding

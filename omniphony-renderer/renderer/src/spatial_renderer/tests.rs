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
/// Now such a band carries a `DegenerateVbapBackend`, so the unified table builds
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

/// The parametrable virtual bed mixes direct and virtualized channels in one
/// frame: `bed_indices` is full-length and carries `usize::MAX` for a channel
/// that must be VBAP-panned (object) even though its index is within
/// `num_beds`. This guards the render-loop generalisation from positional
/// (`idx < num_beds`) to sentinel-aware routing: channel 0 (sentinel) must
/// spread via VBAP while channel 1 (bed id 3 = LFE, a non-prefix bed) routes
/// one-hot to the LFE speaker.
#[test]
fn virtual_bed_mixes_direct_and_virtualized_channels() {
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

    // LFE is speaker index 3 in the 7.1.4 preset (spatialize:false).
    const LFE_SPK: usize = 3;
    let num_speakers = 12;
    let sample_length = 4;

    // Per-speaker summed |energy| across the block.
    let energy = |out: &[f32]| -> Vec<f32> {
        let mut e = vec![0.0f32; num_speakers];
        for s in 0..sample_length {
            for (spk, slot) in e.iter_mut().enumerate() {
                *slot += out[s * num_speakers + spk].abs();
            }
        }
        e
    };

    // bed_indices: channel 0 = sentinel (object), channel 1 = bed id 3 (LFE).
    let beds = [usize::MAX, 3usize];

    // Pass A: only the object channel (0) carries signal.
    let mut ra = build();
    ra.configure_beds(&beds);
    let pcm_a: Vec<f32> = (0..sample_length).flat_map(|_| [0.6f32, 0.0]).collect();
    let events_a = vec![
        SpatialChannelEvent {
            channel_idx: 0,
            is_bed: false,
            gain_db: Some(0),
            ramp_length: Some(0),
            size: Some([0.0, 0.0, 0.0]),
            position: Some([0.0, 1.0, 0.0]), // front-centre object
            sample_pos: Some(0),
        },
        SpatialChannelEvent {
            channel_idx: 1,
            is_bed: true,
            gain_db: Some(0),
            ramp_length: Some(0),
            size: None,
            position: None,
            sample_pos: Some(0),
        },
    ];
    let ea = energy(
        &ra.render_frame(&pcm_a, 2, &events_a, Vec::new(), false)
            .unwrap()
            .samples,
    );
    assert!(
        ea.iter().sum::<f32>() > 0.0,
        "object channel must produce output"
    );
    assert!(
        ea[LFE_SPK] < 1e-6,
        "front object must not leak into the non-spatialized LFE speaker (got {})",
        ea[LFE_SPK]
    );

    // Pass B: only the bed channel (1) carries signal → one-hot at the LFE.
    let mut rb = build();
    rb.configure_beds(&beds);
    let pcm_b: Vec<f32> = (0..sample_length).flat_map(|_| [0.0f32, 0.6]).collect();
    let eb = energy(
        &rb.render_frame(&pcm_b, 2, &events_a, Vec::new(), false)
            .unwrap()
            .samples,
    );
    assert!(eb[LFE_SPK] > 0.0, "bed channel must reach the LFE speaker");
    for (spk, e) in eb.iter().enumerate() {
        if spk != LFE_SPK {
            assert!(
                *e < 1e-6,
                "bed routing must be one-hot; speaker {spk} got {e}"
            );
        }
    }
}

/// Locks the documented subwoofer bass-management recipe: flip the LFE to
/// `spatialize: true` with `freq_high: 120` while every other spatialized
/// speaker carries `freq_low: 120`. The sub is then alone in the `[0, 120)`
/// band, so the single-speaker degenerate rule routes the low band of every
/// object to it; the bands above exclude it entirely; and the stream's own
/// LFE bed channel keeps its direct one-hot feed (bed routing is keyed on
/// the speaker name, not on `spatialize`).
#[test]
fn spatialized_lfe_alone_in_low_band_routes_object_bass() {
    const CUTOFF: f32 = 120.0;
    const LFE_SPK: usize = 3; // 7.1.4 preset order
    fn build() -> SpatialRenderer {
        let mut layout = SpeakerLayout::preset("7.1.4").unwrap();
        for (idx, sp) in layout.speakers.iter_mut().enumerate() {
            if idx == LFE_SPK {
                sp.spatialize = true;
                sp.freq_high = Some(CUTOFF);
            } else {
                sp.freq_low = Some(CUTOFF);
            }
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

    let num_speakers = 12;
    let sample_length = 9_600; // 200 ms — lets the 20 Hz tone settle
    let sample_rate = 48_000.0f32;

    // Per-speaker RMS over the second half of the block (filter steady state).
    let rms = |out: &[f32]| -> Vec<f32> {
        let half = sample_length / 2;
        let mut e = vec![0.0f32; num_speakers];
        for s in half..sample_length {
            for (spk, slot) in e.iter_mut().enumerate() {
                let v = out[s * num_speakers + spk];
                *slot += v * v;
            }
        }
        e.iter().map(|x| (x / half as f32).sqrt()).collect()
    };

    // A front-centre object playing a sine at `freq`.
    let render_tone = |freq: f32| -> Vec<f32> {
        let mut r = build();
        let pcm: Vec<f32> = (0..sample_length)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * freq * i as f32 / sample_rate).sin())
            .collect();
        let events = vec![SpatialChannelEvent {
            channel_idx: 0,
            is_bed: false,
            gain_db: Some(0),
            ramp_length: Some(0),
            size: Some([0.0, 0.0, 0.0]),
            position: Some([0.0, 1.0, 0.0]),
            sample_pos: Some(0),
        }];
        rms(&r
            .render_frame(&pcm, 1, &events, Vec::new(), false)
            .unwrap()
            .samples)
    };

    // Deep bass: the sub must dominate. (The other speakers still receive
    // some of it — the high band is the exact complement of the low-pass, so
    // its low-frequency rejection is gentle — but the sub carries the tone
    // at nearly full level while every main sits well below it.)
    // Absolute levels include the renderer's distance attenuation for this
    // object position (identical across both tones), so the thresholds below
    // are calibrated with ample margin rather than derived from the input.
    let low = render_tone(20.0);
    assert!(
        low[LFE_SPK] > 0.08,
        "sub must carry the 20 Hz object tone, got RMS {}",
        low[LFE_SPK]
    );
    let max_main = low
        .iter()
        .enumerate()
        .filter(|(spk, _)| *spk != LFE_SPK)
        .map(|(_, e)| *e)
        .fold(0.0f32, f32::max);
    assert!(
        low[LFE_SPK] > 1.5 * max_main,
        "sub must dominate at 20 Hz: sub {} vs loudest main {}",
        low[LFE_SPK],
        max_main
    );

    // Treble: the sub is out of the upper bands and the low-pass rejects
    // 4 kHz by >100 dB — it must stay silent.
    let high = render_tone(4_000.0);
    assert!(
        high[LFE_SPK] < 1e-4,
        "sub must not receive a 4 kHz object tone, got RMS {}",
        high[LFE_SPK]
    );
    assert!(
        high.iter().sum::<f32>() > 0.05,
        "the 4 kHz tone must reach the mains"
    );

    // The stream's own LFE bed channel still routes one-hot to the sub even
    // though the speaker is now spatialized.
    let mut r = build();
    let beds = [usize::MAX, 3usize];
    r.configure_beds(&beds);
    let bed_len = 8usize;
    let pcm: Vec<f32> = (0..bed_len).flat_map(|_| [0.0f32, 0.6]).collect();
    let events = vec![
        SpatialChannelEvent {
            channel_idx: 0,
            is_bed: false,
            gain_db: Some(0),
            ramp_length: Some(0),
            size: Some([0.0, 0.0, 0.0]),
            position: Some([0.0, 1.0, 0.0]),
            sample_pos: Some(0),
        },
        SpatialChannelEvent {
            channel_idx: 1,
            is_bed: true,
            gain_db: Some(0),
            ramp_length: Some(0),
            size: None,
            position: None,
            sample_pos: Some(0),
        },
    ];
    let out = r
        .render_frame(&pcm, 2, &events, Vec::new(), false)
        .unwrap()
        .samples;
    let mut e = vec![0.0f32; num_speakers];
    for s in 0..bed_len {
        for (spk, slot) in e.iter_mut().enumerate() {
            *slot += out[s * num_speakers + spk].abs();
        }
    }
    assert!(
        e[LFE_SPK] > 0.0,
        "the LFE bed channel must still reach the spatialized sub"
    );
    for (spk, v) in e.iter().enumerate() {
        if spk != LFE_SPK {
            assert!(
                *v < 1e-6,
                "LFE bed routing must stay one-hot with spatialize:true; speaker {spk} got {v}"
            );
        }
    }
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

/// Regression: in binaural mode the object position ramps MUST advance.
/// The VBAP mix loop that normally drives `advance_ramp` is bypassed, so the
/// binaural branch advances them itself; before that fix every object stayed
/// at the ramp default [0,0,0] — dead centre, and rotation-invariant (the
/// zero vector ignores the head pose) — which rendered as near-mono audio
/// that did not react to head tracking.
#[test]
fn binaural_object_ramp_advances_and_lateralizes() {
    let layout = SpeakerLayout::preset("7.1.4").unwrap();
    let mut r = SpatialRenderer::new(
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
    r.control.live.write().binaural.output_mode = crate::live_params::OutputMode::Binaural;

    // One object channel ramping from the default [0,0,0] to hard right.
    // Broadband pseudo-noise input: head-shadow ILD is a high-frequency
    // phenomenon, so a DC input would show almost no ear asymmetry.
    let mut lcg: u32 = 0x1234_5678;
    let mut noise_block = move || -> Vec<f32> {
        (0..40)
            .map(|_| {
                lcg = lcg.wrapping_mul(1664525).wrapping_add(1013904223);
                (lcg >> 8) as f32 / (1u32 << 24) as f32 - 0.5
            })
            .collect()
    };
    let pcm = noise_block();
    let event = vec![SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(0),
        ramp_length: Some(40),
        size: Some([0.0, 0.0, 0.0]),
        position: Some([1.0, 0.0, 0.0]),
        sample_pos: Some(0),
    }];

    let first = r.render_frame(&pcm, 1, &event, Vec::new(), false).unwrap();
    assert_eq!(
        first.samples.len(),
        40 * 2,
        "binaural output must be stereo"
    );

    // Let the ramp finish and the ITD delay lines / HRIR tails settle, then
    // measure ear energies over a few blocks.
    let (mut e_l, mut e_r) = (0.0f32, 0.0f32);
    for i in 0..8 {
        let pcm = noise_block();
        let out = r.render_frame(&pcm, 1, &[], Vec::new(), false).unwrap();
        if i >= 4 {
            for s in out.samples.chunks_exact(2) {
                e_l += s[0] * s[0];
                e_r += s[1] * s[1];
            }
        }
    }

    let pos = r
        .channel_states
        .lock()
        .get(&0)
        .expect("channel state")
        .ramp
        .current_position;
    assert!(
        pos[0] > 0.99,
        "object ramp did not advance in binaural mode: current_position = {pos:?}"
    );
    assert!(e_l + e_r > 0.0, "binaural output is silent");
    assert!(
        e_r > 1.5 * e_l,
        "hard-right object not lateralized: E_L={e_l} E_R={e_r}"
    );
}

/// Regression: the master gain must scale the binaural output exactly like
/// it scales the speaker path (it used to be applied only in the VBAP
/// branch, so the master control was inert on headphones).
#[test]
fn binaural_output_follows_master_gain() {
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

    let pcm: Vec<f32> = (0..40).map(|i| (i * 7 % 13) as f32 / 13.0 - 0.5).collect();
    let event = vec![SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(0),
        ramp_length: Some(40),
        size: Some([0.0, 0.0, 0.0]),
        position: Some([0.5, 1.0, 0.0]),
        sample_pos: Some(0),
    }];

    let render = |master: f32| -> Vec<f32> {
        let mut r = build();
        {
            let mut live = r.control.live.write();
            live.binaural.output_mode = crate::live_params::OutputMode::Binaural;
            live.master_gain = master;
        }
        let mut out = Vec::new();
        for i in 0..4 {
            let ev: &[SpatialChannelEvent] = if i == 0 { &event } else { &[] };
            out = r
                .render_frame(&pcm, 1, ev, Vec::new(), false)
                .unwrap()
                .samples;
        }
        out
    };

    let unity = render(1.0);
    let double = render(2.0);
    assert!(unity.iter().any(|x| x.abs() > 1e-6), "silent baseline");
    for (a, b) in unity.iter().zip(&double) {
        assert!(
            (b - a * 2.0).abs() <= a.abs() * 1e-4 + 1e-6,
            "master gain not applied: {a} vs {b}"
        );
    }
}

/// In binaural mode the first two speaker param slots act as the L/R ear
/// channels (Studio's headphone rows drive them): muting slot 0 must silence
/// the left ear and leave the right ear untouched.
#[test]
fn binaural_ear_mute_uses_first_speaker_slots() {
    let layout = SpeakerLayout::preset("7.1.4").unwrap();
    let mut r = SpatialRenderer::new(
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
    {
        let mut live = r.control.live.write();
        live.binaural.output_mode = crate::live_params::OutputMode::Binaural;
        live.speakers.insert(
            0,
            crate::live_params::SpeakerLiveParams {
                muted: true,
                ..Default::default()
            },
        );
    }
    r.control
        .speaker_params_generation
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let pcm: Vec<f32> = (0..40).map(|i| (i * 7 % 13) as f32 / 13.0 - 0.5).collect();
    let event = vec![SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(0),
        ramp_length: Some(40),
        size: Some([0.0, 0.0, 0.0]),
        position: Some([0.0, 1.0, 0.0]),
        sample_pos: Some(0),
    }];
    let mut out = Vec::new();
    for i in 0..4 {
        let ev: &[SpatialChannelEvent] = if i == 0 { &event } else { &[] };
        out = r
            .render_frame(&pcm, 1, ev, Vec::new(), false)
            .unwrap()
            .samples;
    }
    let e_l: f32 = out.iter().step_by(2).map(|x| x * x).sum();
    let e_r: f32 = out.iter().skip(1).step_by(2).map(|x| x * x).sum();
    assert!(e_l == 0.0, "left ear not silenced: {e_l}");
    assert!(e_r > 1e-6, "right ear should still play: {e_r}");
}

// TODO: Add integration test with real spatial metadata
// For now, testing is done via real spatial audio content decoding

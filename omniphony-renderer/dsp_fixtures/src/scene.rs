//! Deterministic scene generation, shared by the null tests, the criterion
//! benches, and the future worst-case-block-time gate.
//!
//! Everything here is a pure function of its arguments and a fixed seed: the
//! same call sequence produces byte-identical PCM and event streams on every
//! machine. That is what makes committed goldens meaningful.
//!
//! Moved here from `renderer/benches/render_frame.rs` so the benches and the
//! validation tests cannot drift apart.

use renderer::live_params::{LiveEvaluationMode, PreferredEvaluationMode, RampMode};
use renderer::spatial_renderer::{SpatialChannelEvent, SpatialRenderer};
use renderer::spatial_vbap::{DistanceModel, VbapTableMode};
use renderer::speaker_layout::SpeakerLayout;

/// Samples per access unit fed to `render_frame`. Measured from a real TrueHD
/// Atmos stream through the engine (`ORENDER_PERF_LOG`): the bridge emits a
/// constant 40-sample block at 48 kHz, so this matches the live per-call cost.
pub const BLOCK_SAMPLES: usize = 40;
pub const SAMPLE_RATE: u32 = 48_000;

/// Build a renderer with defaults matching the live decode path for `preset`.
/// `cartesian` selects the precomputed cartesian table/evaluator (vs polar).
pub fn make_renderer(
    preset: &str,
    position_interpolation: bool,
    cartesian: bool,
) -> SpatialRenderer {
    build_renderer(
        SpeakerLayout::preset(preset).expect("known preset"),
        position_interpolation,
        cartesian,
    )
}

/// A "mixed speaker sizes" layout: a few speakers are band-limited (finite
/// `freq_low`), which makes `compute_bands` split rendering into several
/// frequency bands. Every band shares the same VBAP grid, so the per-band table
/// lookups localise the same cell — the case the crossover concept targets.
pub fn crossover_layout() -> SpeakerLayout {
    let mut layout = SpeakerLayout::preset("7.1.4").expect("known preset");
    // Band-limit the first three speakers at distinct cutoffs → edges {80,200,500}
    // → 4 bands; the remaining full-range speakers populate every band.
    for (sp, cutoff) in layout.speakers.iter_mut().zip([80.0, 200.0, 500.0]) {
        sp.freq_low = Some(cutoff);
    }
    layout
}

pub fn build_renderer(
    layout: SpeakerLayout,
    position_interpolation: bool,
    cartesian: bool,
) -> SpatialRenderer {
    let (table_mode, preferred, initial) = if cartesian {
        (
            VbapTableMode::Cartesian {
                x_size: 31,
                y_size: 31,
                z_size: 15,
                z_neg_size: 15,
            },
            PreferredEvaluationMode::PrecomputedCartesian,
            LiveEvaluationMode::PrecomputedCartesian,
        )
    } else {
        (
            VbapTableMode::Polar,
            PreferredEvaluationMode::PrecomputedPolar,
            LiveEvaluationMode::PrecomputedPolar,
        )
    };
    SpatialRenderer::new(
        layout,
        SAMPLE_RATE,
        1, // az_res_deg
        1, // el_res_deg
        0.0,
        2.0,
        table_mode,
        false, // allow_negative_z
        position_interpolation,
        DistanceModel::Linear,
        false,
        1.0,
        1.0,
        0.0,
        1.0,
        false,           // log_object_positions
        [1.0, 2.0, 0.5], // room_ratio
        2.0,
        0.5,
        0.0,
        0.0,   // master_gain_db
        false, // auto_gain
        false, // use_loudness
        false, // distance_diffuse
        1.0,
        1.0,
        preferred,
        initial,
        31,
        31,
        15,
        15,
    )
    .expect("renderer build")
}

/// Deterministic pseudo-random in [-1, 1] from an integer seed (no rng dep).
pub fn pseudo(seed: u64) -> f32 {
    // splitmix64-ish, mapped to [-1, 1].
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    ((x >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
}

/// Interleaved white-ish noise for `n_objects` channels × `BLOCK_SAMPLES`.
pub fn make_pcm(n_objects: usize) -> Vec<f32> {
    let mut pcm = vec![0.0f32; BLOCK_SAMPLES * n_objects];
    for (i, s) in pcm.iter_mut().enumerate() {
        *s = pseudo(i as u64) * 0.25;
    }
    pcm
}

/// One movement event per object, positions spread deterministically over the
/// dome. `seed_round` rotates the positions so successive metadata frames
/// actually change the target (and thus start a ramp).
pub fn move_events(n_objects: usize, seed_round: u64) -> Vec<SpatialChannelEvent> {
    (0..n_objects)
        .map(|ch| {
            let p = ch as u64 + seed_round.wrapping_mul(2_654_435_761);
            SpatialChannelEvent {
                channel_idx: ch,
                is_bed: false,
                gain_db: Some(0),
                ramp_length: Some(BLOCK_SAMPLES as u32),
                size: Some([0.0, 0.0, 0.0]),
                position: Some([
                    pseudo(p) as f64,
                    pseudo(p ^ 0x1111) as f64,
                    (pseudo(p ^ 0x2222).abs()) as f64,
                ]),
                sample_pos: Some(0),
            }
        })
        .collect()
}

/// Build a renderer with `n_objects` already registered at initial positions,
/// returns it plus a reusable PCM buffer. The first `render_frame` consumes the
/// registration events so subsequent steady frames find populated channel state.
///
/// `ramp_mode` is forced explicitly (the constructor seeds `Sample`): the live
/// mpv default is now `Frame`, so the primary sweeps use `Frame` and a dedicated
/// group contrasts it against `Sample`.
pub fn prepared(
    preset: &str,
    n_objects: usize,
    ramp_mode: RampMode,
    position_interpolation: bool,
    cartesian: bool,
) -> (SpatialRenderer, Vec<f32>) {
    let mut r = make_renderer(preset, position_interpolation, cartesian);
    {
        let ctrl = r.renderer_control();
        ctrl.set_requested_ramp_mode(ramp_mode);
        ctrl.live.write().ramp_mode = ramp_mode;
    }
    let pcm = make_pcm(n_objects);
    let init = move_events(n_objects, 0);
    // Prime channel state + let the initial ramp settle so steady frames are
    // representative of the common case (objects mostly static between blocks).
    let mut buf = Vec::new();
    for round in 0..4 {
        let f = r
            .render_frame(&pcm, n_objects, &init, buf, false)
            .expect("prime render");
        buf = f.samples;
        let _ = round;
    }
    (r, pcm)
}

/// Render `blocks` consecutive blocks, concatenating the interleaved output.
///
/// `move_every` controls how often fresh movement events are injected: every
/// `move_every`-th block gets `move_events(n_objects, round)`, the others carry
/// no events. `move_every = 0` means "never move after priming". This is what
/// makes a golden exercise both the ramping and the steady path.
pub fn render_blocks(
    r: &mut SpatialRenderer,
    pcm: &[f32],
    n_objects: usize,
    blocks: usize,
    move_every: usize,
) -> Vec<f32> {
    let mut out = Vec::new();
    let mut buf = Vec::new();
    for round in 0..blocks {
        let events = if move_every > 0 && round % move_every == 0 {
            move_events(n_objects, round as u64 + 1)
        } else {
            Vec::new()
        };
        let frame = r
            .render_frame(pcm, n_objects, &events, buf, false)
            .expect("render_frame in fixture scene");
        out.extend_from_slice(&frame.samples);
        buf = frame.samples;
        buf.clear();
    }
    out
}

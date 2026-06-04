//! Reliable, offline baseline benchmarks for the spatial render hot path.
//!
//! These run entirely in-process against the public `renderer` API — no bridge,
//! no mpv, no PipeWire — so they are deterministic and reproducible across
//! machines and CI. They exist to *quantify* the render-time spikes observed in
//! the live meter (`render_time_ms` avg ≈ 0.15, max ≈ 0.25) by isolating the two
//! suspected amplitude drivers one factor at a time:
//!
//!   * `render_steady/<n_objects>` — cost of a frame carrying NO spatial
//!     metadata, swept over the number of simultaneously active object channels.
//!     Confirms hypothesis #1: steady render cost scales with active objects.
//!
//!   * `render_metadata_frame/<n_objects>` — same object count, but every object
//!     moves this frame (worst-case OAMD block: `update_metadata` + fresh ramps).
//!     Confirms hypothesis #2: metadata-bearing frames cost more than steady ones.
//!
//!   * `render_ramp_mode/<frame|sample>` — at a fixed object count, the cost of
//!     the ramp mode itself. `Frame` is the live mpv default after the engine
//!     parity fix; `Sample` is the old per-sample `compute_gains` behaviour.
//!
//! Run with:  cargo bench -p renderer
//! A single scenario:  cargo bench -p renderer -- render_steady/32

use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use renderer::live_params::{LiveEvaluationMode, PreferredEvaluationMode, RampMode};
use renderer::spatial_renderer::{SpatialChannelEvent, SpatialRenderer};
use renderer::spatial_vbap::{DistanceModel, VbapTableMode};
use renderer::speaker_layout::SpeakerLayout;

/// Samples per access unit fed to `render_frame`. Measured from a real TrueHD
/// Atmos stream through the engine (`ORENDER_PERF_LOG`): the bridge emits a
/// constant 40-sample block at 48 kHz, so this matches the live per-call cost.
const BLOCK_SAMPLES: usize = 40;
const SAMPLE_RATE: u32 = 48_000;

/// Build a renderer with defaults matching the live decode path for `preset`.
/// `cartesian` selects the precomputed cartesian table/evaluator (vs polar).
fn make_renderer(preset: &str, position_interpolation: bool, cartesian: bool) -> SpatialRenderer {
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
        SpeakerLayout::preset(preset).expect("known preset"),
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
fn pseudo(seed: u64) -> f32 {
    // splitmix64-ish, mapped to [-1, 1].
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    x ^= x >> 30;
    x = x.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    ((x >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
}

/// Interleaved white-ish noise for `n_objects` channels × `BLOCK_SAMPLES`.
fn make_pcm(n_objects: usize) -> Vec<f32> {
    let mut pcm = vec![0.0f32; BLOCK_SAMPLES * n_objects];
    for (i, s) in pcm.iter_mut().enumerate() {
        *s = pseudo(i as u64) * 0.25;
    }
    pcm
}

/// One movement event per object, positions spread deterministically over the
/// dome. `seed_round` rotates the positions so successive metadata frames
/// actually change the target (and thus start a ramp).
fn move_events(n_objects: usize, seed_round: u64) -> Vec<SpatialChannelEvent> {
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
fn prepared(
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
        ctrl.live.write().unwrap().ramp_mode = ramp_mode;
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

fn bench_steady(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_steady");
    for &n in &[1usize, 8, 16, 32, 64, 118] {
        let (mut r, pcm) = prepared("7.1.4", n, RampMode::Frame, false, false);
        let mut buf = Vec::new();
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                let f = r
                    .render_frame(
                        black_box(&pcm),
                        black_box(n),
                        &[],
                        std::mem::take(&mut buf),
                        false,
                    )
                    .expect("render");
                buf = f.samples;
                black_box(&buf);
            });
        });
    }
    group.finish();
}

fn bench_metadata_frame(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_metadata_frame");
    for &n in &[1usize, 8, 16, 32, 64, 118] {
        let (mut r, pcm) = prepared("7.1.4", n, RampMode::Frame, false, false);
        let mut buf = Vec::new();
        let mut round = 1u64;
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                // Every iteration moves all objects → exercises update_metadata +
                // fresh ramps, the worst-case OAMD block.
                let events = move_events(n, round);
                round = round.wrapping_add(1);
                let f = r
                    .render_frame(
                        black_box(&pcm),
                        black_box(n),
                        &events,
                        std::mem::take(&mut buf),
                        false,
                    )
                    .expect("render");
                buf = f.samples;
                black_box(&buf);
            });
        });
    }
    group.finish();
}

/// Quantify the cost of the ramp mode itself at a fixed object count. `Frame`
/// is the live mpv default after the engine parity fix; `Sample` is the old
/// (per-sample `compute_gains`) behaviour the embedded host used to run in.
fn bench_ramp_mode(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_ramp_mode");
    const N: usize = 32;
    for (label, mode) in [
        ("frame", RampMode::Frame),
        ("sample", RampMode::Sample),
        ("interp", RampMode::Interp),
    ] {
        let (mut r, pcm) = prepared("7.1.4", N, mode, false, false);
        let mut buf = Vec::new();
        let mut round = 1u64;
        group.bench_function(label, |b| {
            b.iter(|| {
                let events = move_events(N, round);
                round = round.wrapping_add(1);
                let f = r
                    .render_frame(
                        black_box(&pcm),
                        black_box(N),
                        &events,
                        std::mem::take(&mut buf),
                        false,
                    )
                    .expect("render");
                buf = f.samples;
                black_box(&buf);
            });
        });
    }
    group.finish();
}

/// The realistic Sample-mode common case: objects are NOT moving this block (no
/// metadata — ~97% of real frames). `frame` recomputes gains once; `sample`
/// recomputes per sample. Since the position is constant, the per-sample
/// `compute_gains` calls are redundant — this is what the static early-out
/// targets. Contrast with `render_ramp_mode` (objects move every frame).
fn bench_static(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_static");
    const N: usize = 32;
    for (label, mode) in [
        ("frame", RampMode::Frame),
        ("sample", RampMode::Sample),
        ("interp", RampMode::Interp),
    ] {
        let (mut r, pcm) = prepared("7.1.4", N, mode, false, false);
        let mut buf = Vec::new();
        group.bench_function(label, |b| {
            b.iter(|| {
                let f = r
                    .render_frame(
                        black_box(&pcm),
                        black_box(N),
                        &[],
                        std::mem::take(&mut buf),
                        false,
                    )
                    .expect("render");
                buf = f.samples;
                black_box(&buf);
            });
        });
    }
    group.finish();
}

/// The genuinely-moving case: position interpolation ON and a fresh ramp armed
/// every block, so the object's interpolated position changes every sample and
/// `Sample` must recompute the VBAP gains per sample. This is where the cost
/// distribution shows: `sample` pays N × `compute_gains`, `interp` pays one
/// `compute_gains` plus a per-sample gain lerp, `frame` pays one `compute_gains`
/// and no per-sample smoothing.
fn bench_moving(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_moving");
    const N: usize = 32;
    for (label, mode) in [
        ("frame", RampMode::Frame),
        ("sample", RampMode::Sample),
        ("interp", RampMode::Interp),
    ] {
        let (mut r, pcm) = prepared("7.1.4", N, mode, true, false);
        let mut buf = Vec::new();
        let mut round = 1u64;
        group.bench_function(label, |b| {
            b.iter(|| {
                let events = move_events(N, round);
                round = round.wrapping_add(1);
                let f = r
                    .render_frame(
                        black_box(&pcm),
                        black_box(N),
                        &events,
                        std::mem::take(&mut buf),
                        false,
                    )
                    .expect("render");
                buf = f.samples;
                black_box(&buf);
            });
        });
    }
    group.finish();
}

/// Same moving scenario as `render_moving` but with the precomputed CARTESIAN
/// table/evaluator (trilinear `sample_cartesian_table`) instead of polar, to
/// measure and optimise the cartesian `compute_gains` lookup specifically.
fn bench_cartesian(c: &mut Criterion) {
    let mut group = c.benchmark_group("render_cartesian");
    const N: usize = 32;
    for (label, mode) in [
        ("frame", RampMode::Frame),
        ("sample", RampMode::Sample),
        ("interp", RampMode::Interp),
    ] {
        let (mut r, pcm) = prepared("7.1.4", N, mode, true, true);
        let mut buf = Vec::new();
        let mut round = 1u64;
        group.bench_function(label, |b| {
            b.iter(|| {
                let events = move_events(N, round);
                round = round.wrapping_add(1);
                let f = r
                    .render_frame(
                        black_box(&pcm),
                        black_box(N),
                        &events,
                        std::mem::take(&mut buf),
                        false,
                    )
                    .expect("render");
                buf = f.samples;
                black_box(&buf);
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_steady,
    bench_metadata_frame,
    bench_ramp_mode,
    bench_static,
    bench_moving,
    bench_cartesian
);
criterion_main!(benches);

use renderer::live_params::RampMode;
use renderer::speaker_layout::SpeakerLayout;
use renderer::spatial_renderer::SpatialRenderer;
use renderer::spatial_vbap::{DistanceModel, VbapTableMode};
use renderer::SpatialChannelEvent;

pub const SAMPLE_RATE: u32 = 48_000;
pub const BLOCK_SAMPLES: usize = 40;

pub fn make_renderer(
    preset: &str,
    position_interpolation: bool,
    cartesian: bool,
) -> SpatialRenderer {
    let layout = SpeakerLayout::preset(preset).expect("fixture speaker preset");
    let (x_size, y_size, z_size, z_neg_size) = if cartesian {
        (21, 21, 11, 5)
    } else {
        (31, 31, 15, 15)
    };
    SpatialRenderer::new(
        layout,
        SAMPLE_RATE,
        1,
        1,
        0.0,
        2.0,
        if cartesian {
            VbapTableMode::Cartesian
        } else {
            VbapTableMode::Polar
        },
        false,
        position_interpolation,
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
        if cartesian {
            renderer::spatial_renderer::PreferredEvaluationMode::PrecomputedCartesian
        } else {
            renderer::spatial_renderer::PreferredEvaluationMode::PrecomputedPolar
        },
        if cartesian {
            renderer::spatial_renderer::LiveEvaluationMode::PrecomputedCartesian
        } else {
            renderer::spatial_renderer::LiveEvaluationMode::PrecomputedPolar
        },
        x_size,
        y_size,
        z_size,
        z_neg_size,
    )
    .expect("fixture renderer")
}

/// Build a renderer with a deterministic crossover layout. The exact cutoffs are
/// intentionally not psychoacoustic policy; they merely force the crossover and
/// per-band gain-table machinery to participate in tests and benchmarks.
pub fn crossover_layout() -> renderer::CrossoverLayout {
    renderer::CrossoverLayout::new(vec![
        renderer::CrossoverBand::new(0, 0.0, 250.0),
        renderer::CrossoverBand::new(1, 250.0, 2_500.0),
        renderer::CrossoverBand::new(2, 2_500.0, 24_000.0),
    ])
    .expect("fixture crossover layout")
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

/// Interleaved noise for block `block_index` of a *continuous* stream.
///
/// Unlike [`make_pcm`], successive blocks carry different samples, so a
/// multi-block capture is aperiodic. This matters for any measurement based on
/// cross-correlation: reusing one block as the excitation makes the signal
/// periodic at [`BLOCK_SAMPLES`], and correlation then resolves lag only modulo
/// 40 samples, which silently produces sign-flipped results.
pub fn make_pcm_block(n_objects: usize, block_index: usize) -> Vec<f32> {
    let base = (block_index * BLOCK_SAMPLES * n_objects) as u64;
    (0..BLOCK_SAMPLES * n_objects)
        .map(|i| pseudo(base + i as u64) * 0.25)
        .collect()
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

/// One movement event per object on a continuous trajectory.
///
/// This is the realistic counterpart to [`move_events`], which redraws every
/// position at random each round. Random redraw remains a useful pathological
/// upper bound, but real content moves coherently. Rates span 12°/s to 96°/s;
/// at 1200 forty-sample blocks per second that is 0.01° to 0.08° per block.
/// The fixture therefore exposes optimisations that preserve direction
/// coherence, such as the exact HRIR-direction cache, without changing the
/// renderer's geometry or inventing an audible quantisation policy.
pub fn drift_events(n_objects: usize, seed_round: u64) -> Vec<SpatialChannelEvent> {
    (0..n_objects)
        .map(|ch| {
            let deg_per_block = (12.0 + (ch % 8) as f64 * 12.0) / 1200.0;
            let az = (ch as f64 * 37.0 + seed_round as f64 * deg_per_block).to_radians();
            SpatialChannelEvent {
                channel_idx: ch,
                is_bed: false,
                gain_db: Some(0),
                ramp_length: Some(BLOCK_SAMPLES as u32),
                size: Some([0.0, 0.0, 0.0]),
                position: Some([az.sin(), az.cos(), (ch % 5) as f64 * 0.25]),
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

// Re-exported for the same reason as `RampMode` above: the dev-dependency
// cycle means `renderer`'s own test build is a distinct crate instance, so
// tests inside `renderer` must name this type through the fixture crate for
// the argument types to match.
pub use renderer::binaural::HrirSource;
use renderer::live_params::OutputMode;

/// A renderer switched to the independent binaural (headphone) path, with the
/// bundled SAF KEMAR set. Output is 2-channel regardless of the layout.
///
/// `HrirSource::SafKemar` is already the default, so it is not set explicitly —
/// the golden would silently change if that default moved, which is exactly the
/// kind of drift a null test should catch.
pub fn prepared_binaural(n_objects: usize, ramp_mode: RampMode) -> (SpatialRenderer, Vec<f32>) {
    let mut r = make_renderer("7.1.4", true, false);
    {
        let ctrl = r.renderer_control();
        ctrl.set_requested_ramp_mode(ramp_mode);
        let mut live = ctrl.live.write();
        live.ramp_mode = ramp_mode;
        live.binaural.output_mode = OutputMode::Binaural;
    }
    let pcm = make_pcm(n_objects);
    let init = move_events(n_objects, 0);
    let mut buf = Vec::new();
    for _ in 0..4 {
        let f = r
            .render_frame(&pcm, n_objects, &init, buf, false)
            .expect("binaural prime");
        buf = f.samples;
    }
    (r, pcm)
}

/// A renderer switched to the cascaded binaural path, where arbitrary object
/// count is first mixed to the fixed virtual 7.1.4 speaker bed and then that bed
/// is HRTF-rendered. This is the current scalable binaural architecture.
pub fn prepared_binaural_cascaded(
    n_objects: usize,
    ramp_mode: RampMode,
) -> (SpatialRenderer, Vec<f32>) {
    let mut r = make_renderer("7.1.4", true, false);
    {
        let ctrl = r.renderer_control();
        ctrl.set_requested_ramp_mode(ramp_mode);
        let mut live = ctrl.live.write();
        live.ramp_mode = ramp_mode;
        live.binaural.output_mode = OutputMode::Binaural;
        live.binaural.mode = renderer::live_params::BinauralMode::Cascaded;
    }
    let pcm = make_pcm(n_objects);
    let init = move_events(n_objects, 0);
    let mut buf = Vec::new();
    for _ in 0..4 {
        let f = r
            .render_frame(&pcm, n_objects, &init, buf, false)
            .expect("cascaded binaural prime");
        buf = f.samples;
    }
    (r, pcm)
}

/// Build a renderer configured with an explicit crossover layout.
pub fn prepared_crossover(
    preset: &str,
    n_objects: usize,
    ramp_mode: RampMode,
) -> (SpatialRenderer, Vec<f32>) {
    let mut r = make_renderer(preset, true, false);
    {
        let ctrl = r.renderer_control();
        ctrl.set_requested_ramp_mode(ramp_mode);
        ctrl.live.write().ramp_mode = ramp_mode;
    }
    r.configure_crossover(Some(crossover_layout()))
        .expect("fixture crossover configure");
    let pcm = make_pcm(n_objects);
    let init = move_events(n_objects, 0);
    let mut buf = Vec::new();
    for _ in 0..4 {
        let f = r
            .render_frame(&pcm, n_objects, &init, buf, false)
            .expect("crossover prime");
        buf = f.samples;
    }
    (r, pcm)
}

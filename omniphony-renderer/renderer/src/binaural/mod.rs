//! Independent binaural (headphone) output stage.
//!
//! This is a **parallel render path**, not a [`GainModel`] backend: a backend
//! only emits per-speaker gains and cannot carry the per-ear delay (ITD) or the
//! stateful HRTF convolution a binaural renderer needs. When
//! [`OutputMode::Binaural`] is selected, `SpatialRenderer::render_frame` skips
//! the whole VBAP / crossover / speaker chain and calls [`BinauralRenderer`]
//! instead, producing a 2-channel (L/R) interleaved frame.
//!
//! Pipeline per input channel, per frame:
//! `pos_adm → rotate(head_pose) → (az, el, dist)`
//!   → per-ear ITD delay → per-ear HRIR convolution
//!   → (+ shoebox early reflections, see [`reflections`])
//!   → sum into `[L, R]`.
//!
//! Space scaling is a single **isotropic** `unit_scale_m` (metres per ADM unit);
//! the anisotropic `room_ratio` is deliberately *not* reused here because it
//! would distort directions and corrupt HRTF localisation.
//!
//! [`GainModel`]: crate::render_backend::GainModel
//! [`OutputMode`]: crate::live_params::OutputMode

pub mod convolver;
pub mod diffuse_compensation;
pub mod head_pose;
pub mod hrir;
pub mod itd;
pub mod measured;
pub mod prtf;
pub mod reflections;
pub mod reverb;
pub mod tracking;

#[cfg(test)]
mod validation;

pub use head_pose::HeadPose;
pub use tracking::{HeadTracking, HeadTrackingFormat};

use crate::delay_line::DelayLine;
use crate::live_params::{BinauralReflections, BinauralReverb};
use convolver::EarConvolver;
use hrir::{HRIR_LEN, HrirPair, HrirSet, ParametricPinnaHrir};
use measured::MeasuredHrirData;
use prtf::SpagnolPrtfHrir;
use reflections::ReflectionBank;
use reverb::Fdn;

/// Per-listener `D_n` preset for the parametric pinna model (Brown & Duda 1998,
/// Table I). `D_n` is the only parameter the paper individualizes per subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PinnaPreset {
    /// Subjects PB & NH: D = [1, 0.5, 0.5, 0.5, 0.5].
    #[default]
    PbNh,
    /// Subject RD: D = [0.85, 0.35, 0.35, 0.35, 0.35].
    Rd,
}

impl PinnaPreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PbNh => "pbnh",
            Self::Rd => "rd",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "rd" => Self::Rd,
            _ => Self::PbNh,
        }
    }
    /// The published `D_n` column for this preset.
    fn d_base(&self) -> [f32; 5] {
        match self {
            Self::PbNh => ParametricPinnaHrir::D_PB_NH,
            Self::Rd => ParametricPinnaHrir::D_RD,
        }
    }
}

/// Which HRIR data set the binaural stage convolves with.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum HrirSource {
    /// Built-in analytic head-shadow model (no measured data, lightest).
    Synthetic,
    /// Embedded SAF KEMAR measured set (ISC). Default — real measured HRTF.
    #[default]
    SafKemar,
    /// A SOFA file loaded from disk (requires the `sofa` build feature).
    Sofa(String),
    /// Parametric structural model: analytic head shadow + the Brown-Duda pinna
    /// echo train (exact Table I coefficients). `preset` picks a published `D_n`
    /// column (the only per-listener parameter), `d_scale_pct` fine-tunes it,
    /// `depth_pct` is the echo strength (0 ≈ synthetic, 100 = full). No measured
    /// data — the "tune a few knobs" alternative to `saf`/`sofa`.
    Pinna {
        preset: PinnaPreset,
        d_scale_pct: u16,
        depth_pct: u16,
    },
    /// Structural PRTF model (Spagnol/Geronazzo/Avanzini): head shadow + two
    /// concha resonances + three elevation-dependent notches, population-average
    /// preset. `depth_pct` is the pinna-coloration amount (0 ≈ synthetic), and
    /// `freq_scale_pct` shifts all notch/resonance frequencies (individualization).
    Prtf { freq_scale_pct: u16, depth_pct: u16 },
}

impl HrirSource {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Synthetic => "synthetic",
            Self::SafKemar => "saf",
            Self::Sofa(_) => "sofa",
            Self::Pinna { .. } => "pinna",
            Self::Prtf { .. } => "prtf",
        }
    }

    /// Parse a source selector. `"sofa:<path>"` carries the file path; a bare
    /// `"sofa"` yields `Sofa("")` (path to be set separately).
    pub fn from_str(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(path) = s.strip_prefix("sofa:") {
            return Some(Self::Sofa(path.to_string()));
        }
        let lower = s.to_ascii_lowercase();
        // "pinna" | "pinna:<preset>:<dscale>:<depth>" (preset = pbnh|rd,
        // dscale/depth percent integers).
        if let Some(rest) = lower.strip_prefix("pinna:") {
            let mut it = rest.split(':');
            let preset = PinnaPreset::from_str(it.next().unwrap_or("pbnh"));
            let d_scale = it
                .next()
                .and_then(|v| v.trim().parse::<u16>().ok())
                .unwrap_or(100);
            let depth = it
                .next()
                .and_then(|v| v.trim().parse::<u16>().ok())
                .unwrap_or(100);
            return Some(Self::Pinna {
                preset,
                d_scale_pct: d_scale.clamp(50, 150),
                depth_pct: depth.clamp(0, 100),
            });
        }
        // "prtf" | "prtf:<freq_scale>:<depth>" (percent integers).
        if let Some(rest) = lower.strip_prefix("prtf:") {
            let mut it = rest.split(':');
            let freq = it
                .next()
                .and_then(|v| v.trim().parse::<u16>().ok())
                .unwrap_or(100);
            let depth = it
                .next()
                .and_then(|v| v.trim().parse::<u16>().ok())
                .unwrap_or(100);
            return Some(Self::Prtf {
                freq_scale_pct: freq.clamp(50, 150),
                depth_pct: depth.clamp(0, 100),
            });
        }
        match lower.as_str() {
            "synthetic" | "synth" => Some(Self::Synthetic),
            "saf" | "kemar" | "saf_kemar" => Some(Self::SafKemar),
            "sofa" => Some(Self::Sofa(String::new())),
            "pinna" | "parametric" => Some(Self::Pinna {
                preset: PinnaPreset::PbNh,
                d_scale_pct: 100,
                depth_pct: 100,
            }),
            "prtf" | "spagnol" => Some(Self::Prtf {
                freq_scale_pct: 100,
                depth_pct: 100,
            }),
            _ => None,
        }
    }
}

/// Reference distance (m) at which an early reflection's 1/d gain is unity.
const REF_DISTANCE_M: f32 = 1.0;
/// Closest image-source distance (m) for the reflection 1/d law, bounding the
/// near-source boost. (The direct path is deliberately not distance-attenuated.)
const MIN_DISTANCE_M: f32 = 0.25;
/// Maximum reflection distance gain, so an image at the origin can't blow up.
const MAX_DISTANCE_GAIN: f32 = 4.0;
/// Delay-line capacity for the ITD (s) — comfortably above the ~0.7 ms max.
const ITD_MAX_S: f32 = 0.003;

/// Per-input-channel binaural DSP state, lazily created on first use.
struct ChannelDsp {
    delay_l: DelayLine,
    delay_r: DelayLine,
    conv_l: EarConvolver,
    conv_r: EarConvolver,
    /// Early-reflection bank; allocated lazily when reflections are enabled
    /// and dropped when disabled (the ring is the big allocation here).
    refl: Option<ReflectionBank>,
    /// Air-absorption one-pole low-pass state (direct path).
    air_state: f32,
    /// Air-absorption smoothing coefficient for the current block
    /// (0 = bypass, →1 = heavy low-pass). Updated per block from distance.
    air_coeff: f32,
}

impl ChannelDsp {
    fn new(sample_rate: u32) -> Self {
        let max_delay = (ITD_MAX_S * sample_rate as f32).ceil() as usize;
        Self {
            delay_l: DelayLine::new(max_delay),
            delay_r: DelayLine::new(max_delay),
            conv_l: EarConvolver::new(),
            conv_r: EarConvolver::new(),
            refl: None,
            air_state: 0.0,
            air_coeff: 0.0,
        }
    }
}

impl ChannelDsp {
    fn reset_runtime_state(&mut self) {
        self.delay_l.reset_runtime_state();
        self.delay_r.reset_runtime_state();
        self.conv_l.reset_runtime_state();
        self.conv_r.reset_runtime_state();
        if let Some(bank) = self.refl.as_mut() {
            bank.reset_runtime_state();
        }
        self.air_state = 0.0;
        self.air_coeff = 0.0;
    }
}

/// Tagged result from the asynchronous HRIR worker.
///
/// A build can be expensive enough that a newer user request arrives while an
/// older grid is still being constructed. The audio thread must therefore know
/// which source produced a finished grid so it can reject stale completions
/// rather than briefly installing the wrong HRTF during rapid calibration/A-B
/// switching.
struct BuiltHrirSet {
    source: HrirSource,
    set: std::sync::Arc<HrirSet>,
}

/// Per-frame live parameters for [`BinauralRenderer::render_frame`], grouped
/// so the call site stays readable as the stage grows.
pub struct BinauralFrameParams {
    pub head_pose: HeadPose,
    pub unit_scale_m: f32,
    pub head_radius_m: f32,
    pub reflections: BinauralReflections,
    pub reverb: BinauralReverb,
    pub air_absorption: bool,
}

/// Owns the per-channel binaural DSP state and the HRIR set; renders all input
/// channels of a frame to interleaved stereo.
pub struct BinauralRenderer {
    sample_rate: u32,
    hrir: std::sync::Arc<HrirSet>,
    /// HRIR source last *requested* (the active grid may briefly lag it while
    /// the worker builds — see [`Self::ensure_source`]).
    source: HrirSource,
    /// Finished, source-tagged grids from the rebuild worker, awaiting the
    /// audio-thread swap. Stale results are discarded rather than installed.
    incoming: std::sync::Arc<arc_swap::ArcSwapOption<BuiltHrirSet>>,
    /// Requests to the long-lived rebuild worker. Dropping the renderer drops
    /// the sender, which terminates the worker.
    rebuild_tx: std::sync::mpsc::Sender<HrirSource>,
    /// Per-input-channel DSP state, indexed directly by channel (sparse tail is
    /// fine; reset when the channel count shrinks).
    channels: Vec<Option<ChannelDsp>>,
    /// Metadata/mute gain at the sample boundary immediately after the previous
    /// callback, per input channel. `SpatialRenderer::slew_gain` remains the one
    /// authority for the slew rate; this state merely reconstructs the linear
    /// segment between successive boundary values so callback size cannot turn
    /// the audible gain trajectory into a staircase.
    channel_gain_boundary: Vec<f32>,
    /// Reusable HRIR scratch so `at()` writes in place (no per-channel alloc).
    hrir_scratch: HrirPair,
    /// Shared late-reverb tail; allocated lazily while enabled.
    fdn: Option<Fdn>,
    /// Mono reverb send bus, one sample per frame (reused).
    reverb_bus: Vec<f32>,
}

impl BinauralRenderer {
    pub fn new(sample_rate: u32) -> Self {
        let source = HrirSource::default();
        let incoming: std::sync::Arc<arc_swap::ArcSwapOption<BuiltHrirSet>> =
            std::sync::Arc::new(arc_swap::ArcSwapOption::empty());
        // Long-lived rebuild worker: grid builds (allocations, provider
        // renders, SOFA file I/O) must never run on the audio thread. The
        // worker drains request bursts to the latest one, builds, and
        // publishes a source-tagged result into `incoming` for the audio thread
        // to accept only if that source is still current.
        let (rebuild_tx, rebuild_rx) = std::sync::mpsc::channel::<HrirSource>();
        {
            let slot = std::sync::Arc::clone(&incoming);
            std::thread::Builder::new()
                .name("binaural-hrir-rebuild".into())
                .spawn(move || {
                    while let Ok(mut req) = rebuild_rx.recv() {
                        while let Ok(newer) = rebuild_rx.try_recv() {
                            req = newer;
                        }
                        let set = std::sync::Arc::new(Self::build_hrir(&req, sample_rate));
                        slot.store(Some(std::sync::Arc::new(BuiltHrirSet { source: req, set })));
                    }
                })
                .expect("spawn binaural HRIR rebuild worker");
        }
        Self {
            sample_rate,
            // The initial (default) grid is built synchronously: `new` runs on
            // a control thread, and the renderer must be usable immediately.
            hrir: std::sync::Arc::new(Self::build_hrir(&source, sample_rate)),
            source,
            incoming,
            rebuild_tx,
            channels: Vec::new(),
            channel_gain_boundary: Vec::new(),
            hrir_scratch: HrirPair {
                left: [0.0; HRIR_LEN],
                right: [0.0; HRIR_LEN],
            },
            fdn: None,
            reverb_bus: Vec::new(),
        }
    }

    /// Reset one discontinuous audio stream while preserving expensive immutable
    /// configuration: the selected HRIR grid and rebuild worker remain alive,
    /// while every sample-history state is cleared in place.
    pub fn reset_runtime_state(&mut self) {
        for channel in &mut self.channels {
            if let Some(dsp) = channel.as_mut() {
                dsp.reset_runtime_state();
            }
        }
        self.channel_gain_boundary.fill(0.0);
        if let Some(fdn) = self.fdn.as_mut() {
            fdn.reset_runtime_state();
        }
        self.reverb_bus.fill(0.0);
    }

    /// Identity of the active HRIR grid (tests observe the async swap with it).
    #[cfg(test)]
    fn hrir_grid_id(&self) -> usize {
        std::sync::Arc::as_ptr(&self.hrir) as usize
    }

    fn build_hrir(source: &HrirSource, sample_rate: u32) -> HrirSet {
        match source {
            HrirSource::Synthetic => HrirSet::synthetic(sample_rate),
            HrirSource::Pinna {
                preset,
                d_scale_pct,
                depth_pct,
            } => {
                let scale = *d_scale_pct as f32 / 100.0;
                let mut d = preset.d_base();
                for x in &mut d {
                    *x *= scale;
                }
                HrirSet::new(
                    &ParametricPinnaHrir {
                        d,
                        depth: *depth_pct as f32 / 100.0,
                    },
                    sample_rate,
                )
            }
            HrirSource::Prtf {
                freq_scale_pct,
                depth_pct,
            } => HrirSet::new(
                &SpagnolPrtfHrir {
                    depth: *depth_pct as f32 / 100.0,
                    freq_scale: *freq_scale_pct as f32 / 100.0,
                },
                sample_rate,
            ),
            HrirSource::SafKemar => HrirSet::new(
                &MeasuredHrirData::saf_kemar().resampled_to(sample_rate),
                sample_rate,
            ),
            HrirSource::Sofa(path) => {
                #[cfg(feature = "sofa")]
                {
                    match MeasuredHrirData::from_sofa(std::path::Path::new(path)) {
                        Ok(data) => HrirSet::new(&data.resampled_to(sample_rate), sample_rate),
                        Err(e) => {
                            log::warn!("SOFA load failed ({e}); falling back to SAF KEMAR");
                            HrirSet::new(
                                &MeasuredHrirData::saf_kemar().resampled_to(sample_rate),
                                sample_rate,
                            )
                        }
                    }
                }
                #[cfg(not(feature = "sofa"))]
                {
                    log::warn!("SOFA support not compiled; falling back to SAF KEMAR");
                    let _ = path;
                    HrirSet::new(
                        &MeasuredHrirData::saf_kemar().resampled_to(sample_rate),
                        sample_rate,
                    )
                }
            }
        }
    }

    /// Lazily resize DSP channel state and the metadata-gain boundaries.
    fn ensure_channel_state(&mut self, count: usize) {
        if self.channels.len() < count {
            let sr = self.sample_rate;
            self.channels.resize_with(count, || Some(ChannelDsp::new(sr)));
        } else if self.channels.len() > count {
            self.channels.truncate(count);
        }
        self.channel_gain_boundary.resize(count, 0.0);
    }

    /// Apply an HRIR source change. Building a measured/parametric grid can
    /// allocate and run provider interpolation thousands of times, so it must
    /// never execute on the realtime audio thread. The long-lived worker owns
    /// that work and publishes a finished grid for a later block boundary.
    pub fn ensure_source(&mut self, source: &HrirSource) {
        // Install a completed grid only if it still matches the latest request.
        if let Some(tagged) = self.incoming.swap(None) {
            if tagged.source == self.source {
                self.hrir = std::sync::Arc::clone(&tagged.set);
            }
        }
        if &self.source == source {
            return;
        }
        self.source = source.clone();
        if self.rebuild_tx.send(source.clone()).is_err() {
            log::error!("binaural HRIR rebuild worker exited unexpectedly");
        }
    }

    /// Ensure optional reflection banks and reverb FDN match the live config.
    fn ensure_room_state(
        &mut self,
        n_ch: usize,
        reflections: &BinauralReflections,
        reverb: &BinauralReverb,
    ) {
        if reflections.enabled {
            for ch in 0..n_ch {
                let dsp = self.channels[ch].as_mut().unwrap();
                if dsp.refl.is_none() {
                    dsp.refl = Some(ReflectionBank::new(self.sample_rate));
                }
            }
        } else {
            for ch in 0..n_ch {
                self.channels[ch].as_mut().unwrap().refl = None;
            }
        }
        if reverb.enabled && self.fdn.is_none() {
            self.fdn = Some(Fdn::new(self.sample_rate));
        } else if !reverb.enabled {
            self.fdn = None;
        }
    }

    /// Render one block of channel-interleaved input samples to interleaved stereo.
    pub fn render_frame(
        &mut self,
        frame: &[f32],
        n_ch: usize,
        sample_count: usize,
        events: &[Option<bridge_api::SpatialChannelEvent>],
        gains: &[f32],
        params: &BinauralFrameParams,
    ) -> Vec<f32> {
        self.ensure_channel_state(n_ch);
        self.ensure_room_state(n_ch, &params.reflections, &params.reverb);

        let n_frames = sample_count / n_ch;
        let mut out = vec![0.0_f32; n_frames * 2];
        if self.reverb_bus.len() < n_frames {
            self.reverb_bus.resize(n_frames, 0.0);
        }
        self.reverb_bus[..n_frames].fill(0.0);

        for ch in 0..n_ch {
            let ev = events.get(ch).and_then(|e| e.as_ref());
            let Some(ev) = ev else { continue };

            let layout = ev.layout.as_ref();
            let pos = layout
                .and_then(|l| l.position.as_ref())
                .map(|p| [p.x as f32, p.y as f32, p.z as f32])
                .unwrap_or([0.0, 1.0, 0.0]);
            let p = params.head_pose.rotate_world_to_head(pos);
            let az = p[0].atan2(p[1]).to_degrees();
            let horizontal = (p[0] * p[0] + p[1] * p[1]).sqrt();
            let el = p[2].atan2(horizontal).to_degrees();
            let dist_units = (p[0] * p[0] + p[1] * p[1] + p[2] * p[2]).sqrt();
            let dist_m = dist_units * params.unit_scale_m.max(0.01);

            let mut gain_end = gains.get(ch).copied().unwrap_or(0.0).clamp(0.0, 1.0);
            if ev.mute.unwrap_or(false) {
                gain_end = 0.0;
            }
            if let Some(db) = ev.gain_db {
                gain_end *= 10.0_f32.powf(db as f32 / 20.0);
            }
            let gain_start = self.channel_gain_boundary.get(ch).copied().unwrap_or(0.0);
            if let Some(boundary) = self.channel_gain_boundary.get_mut(ch) {
                *boundary = gain_end;
            }

            let hrir = &self.hrir;
            hrir.at(az, el, &mut self.hrir_scratch);
            let pair = &self.hrir_scratch;

            let (itd_l, itd_r) = itd::woodworth_itd_seconds(az, el, params.head_radius_m);
            let delay_l = itd_l * self.sample_rate as f32;
            let delay_r = itd_r * self.sample_rate as f32;

            let dsp = self.channels[ch].as_mut().unwrap();
            dsp.air_coeff = if params.air_absorption {
                air_absorption_coeff(dist_m, self.sample_rate)
            } else {
                0.0
            };

            if let Some(bank) = dsp.refl.as_mut() {
                let room = [
                    params.reflections.room_width_m,
                    params.reflections.room_depth_m,
                    params.reflections.room_height_m,
                ];
                let src_m = [
                    p[0] * params.unit_scale_m,
                    p[1] * params.unit_scale_m,
                    p[2] * params.unit_scale_m,
                ];
                let images = reflections::first_order_images(src_m, room);
                for (i, img) in images.iter().enumerate() {
                    let d_img = (img[0] * img[0] + img[1] * img[1] + img[2] * img[2]).sqrt();
                    let rel_delay_s = ((d_img - dist_m).max(0.0)) / reflections::speed_of_sound();
                    let refl_az = img[0].atan2(img[1]).to_degrees();
                    let refl_el = img[2]
                        .atan2((img[0] * img[0] + img[1] * img[1]).sqrt())
                        .to_degrees();
                    let lateral = refl_az.to_radians().sin() * refl_el.to_radians().cos();
                    let (refl_itd_l, refl_itd_r) =
                        itd::woodworth_itd_seconds(refl_az, refl_el, params.head_radius_m);
                    let g_l = (1.0 - 0.35 * lateral).clamp(0.4, 1.6);
                    let g_r = (1.0 + 0.35 * lateral).clamp(0.4, 1.6);
                    let g_dist = (REF_DISTANCE_M / d_img.max(MIN_DISTANCE_M))
                        .min(MAX_DISTANCE_GAIN);
                    let g = params.reflections.level.clamp(0.0, 1.0) * g_dist;
                    bank.set_targets_binaural(
                        i,
                        rel_delay_s + refl_itd_l,
                        rel_delay_s + refl_itd_r,
                        g * g_l,
                        g * g_r,
                    );
                }
            }

            // Feed each input channel through its per-ear delays + convolvers and
            // optional early reflection bank. The channel's metadata/mute gain
            // is linearly interpolated across this callback from the previous
            // boundary to the new boundary, so arbitrary host block sizes follow
            // the same continuous gain trajectory rather than callback steps.
            for n in 0..n_frames {
                let src = frame[n * n_ch + ch];
                let x = if dsp.air_coeff > 0.0 {
                    dsp.air_state += dsp.air_coeff * (src - dsp.air_state);
                    dsp.air_state
                } else {
                    dsp.air_state = src;
                    src
                };
                let t = (n + 1) as f32 / n_frames.max(1) as f32;
                let g = gain_start + (gain_end - gain_start) * t;
                let x = x * g;
                let dl = dsp.delay_l.process(x, delay_l);
                let dr = dsp.delay_r.process(x, delay_r);
                let mut l = dsp.conv_l.process(dl, &pair.left);
                let mut r = dsp.conv_r.process(dr, &pair.right);

                if let Some(bank) = dsp.refl.as_mut() {
                    let (rl, rr) = bank.process(x);
                    l += rl;
                    r += rr;
                }

                out[n * 2] += l;
                out[n * 2 + 1] += r;
                self.reverb_bus[n] += 0.5 * (l + r);
            }
        }

        // Shared late field: mono send -> stereo FDN. Kept deliberately subtle;
        // the FDN owns its own level and predelay smoothing.
        if let Some(fdn) = self.fdn.as_mut() {
            fdn.set_params(&params.reverb);
            for n in 0..n_frames {
                let (l, r) = fdn.process(self.reverb_bus[n]);
                out[n * 2] += l;
                out[n * 2 + 1] += r;
            }
        }

        out
    }
}

/// One-pole air absorption for direct sound. Returns the smoothing coefficient
/// for `state += coeff * (x-state)`. At close range (<3 m) bypasses entirely;
/// beyond that the cutoff falls gradually from 20 kHz toward 5 kHz at 30 m.
fn air_absorption_coeff(distance_m: f32, sample_rate: u32) -> f32 {
    if distance_m <= 3.0 {
        return 0.0;
    }
    let t = ((distance_m - 3.0) / 27.0).clamp(0.0, 1.0);
    let cutoff = 20_000.0 + t * (5_000.0 - 20_000.0);
    let dt = 1.0 / sample_rate as f32;
    let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff);
    (dt / (rc + dt)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinna_preset_roundtrip() {
        assert_eq!(PinnaPreset::from_str("rd"), PinnaPreset::Rd);
        assert_eq!(PinnaPreset::from_str("pbnh"), PinnaPreset::PbNh);
        assert_eq!(PinnaPreset::Rd.as_str(), "rd");
    }

    #[test]
    fn hrir_source_parse() {
        assert_eq!(HrirSource::from_str("saf"), Some(HrirSource::SafKemar));
        assert_eq!(HrirSource::from_str("synthetic"), Some(HrirSource::Synthetic));
        assert_eq!(
            HrirSource::from_str("sofa:/tmp/foo.sofa"),
            Some(HrirSource::Sofa("/tmp/foo.sofa".into()))
        );
        assert_eq!(
            HrirSource::from_str("pinna:rd:90:70"),
            Some(HrirSource::Pinna {
                preset: PinnaPreset::Rd,
                d_scale_pct: 90,
                depth_pct: 70,
            })
        );
        assert_eq!(
            HrirSource::from_str("prtf:110:80"),
            Some(HrirSource::Prtf {
                freq_scale_pct: 110,
                depth_pct: 80,
            })
        );
    }

    #[test]
    fn air_absorption_bypasses_close_and_increases_with_distance() {
        assert_eq!(air_absorption_coeff(1.0, 48_000), 0.0);
        let a = air_absorption_coeff(10.0, 48_000);
        let b = air_absorption_coeff(25.0, 48_000);
        assert!(a > 0.0 && b > a);
    }
}

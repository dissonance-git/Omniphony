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
pub mod head_pose;
pub mod hrir;
pub mod itd;
pub mod measured;
pub mod prtf;
pub mod reflections;
pub mod reverb;
pub mod tracking;

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
    hrir: HrirSet,
    /// HRIR data set the current `hrir` grid was built from.
    source: HrirSource,
    /// Per-input-channel DSP state, indexed directly by channel (sparse tail is
    /// fine; reset when the channel count shrinks).
    channels: Vec<Option<ChannelDsp>>,
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
        Self {
            sample_rate,
            hrir: Self::build_hrir(&source, sample_rate),
            source,
            channels: Vec::new(),
            hrir_scratch: HrirPair {
                left: [0.0; HRIR_LEN],
                right: [0.0; HRIR_LEN],
            },
            fdn: None,
            reverb_bus: Vec::new(),
        }
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
                let d = preset.d_base().map(|x| x * scale);
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
            HrirSource::Sofa(path) => match Self::load_sofa(path, sample_rate) {
                Some(set) => set,
                None => {
                    log::warn!(
                        "binaural: SOFA source '{path}' unavailable; falling back to SAF KEMAR"
                    );
                    HrirSet::new(
                        &MeasuredHrirData::saf_kemar().resampled_to(sample_rate),
                        sample_rate,
                    )
                }
            },
        }
    }

    #[cfg(feature = "sofa")]
    fn load_sofa(path: &str, sample_rate: u32) -> Option<HrirSet> {
        match measured::hrir_set_from_sofa(path, sample_rate) {
            Ok(set) => Some(set),
            Err(e) => {
                log::warn!("binaural: failed to load SOFA '{path}': {e}");
                None
            }
        }
    }

    #[cfg(not(feature = "sofa"))]
    fn load_sofa(_path: &str, _sample_rate: u32) -> Option<HrirSet> {
        log::warn!("binaural: SOFA support not built (enable the 'sofa' feature)");
        None
    }

    /// Rebuild the HRIR grid if the requested source changed. Called once per
    /// frame; the (allocating) rebuild only runs on an actual change.
    pub fn ensure_source(&mut self, source: &HrirSource) {
        if &self.source != source {
            self.hrir = Self::build_hrir(source, self.sample_rate);
            self.source = source.clone();
        }
    }

    /// Render one frame to interleaved stereo.
    ///
    /// - `chan_pos[c]`: world (ADM) position of input channel `c`.
    /// - `chan_gain[c]`: linear gain for channel `c` (object mute/gain folded in).
    /// - `out`: must be `sample_length * 2`, pre-zeroed.
    #[allow(clippy::too_many_arguments)]
    pub fn render_frame(
        &mut self,
        input_pcm: &[f32],
        input_channel_count: usize,
        sample_length: usize,
        params: &BinauralFrameParams,
        chan_pos: &[[f64; 3]],
        chan_gain: &[f32],
        out: &mut [f32],
    ) {
        let BinauralFrameParams {
            head_pose,
            unit_scale_m,
            head_radius_m,
            ref reflections,
            ref reverb,
            air_absorption,
        } = *params;
        debug_assert_eq!(out.len(), sample_length * 2);
        if input_channel_count == 0 || sample_length == 0 {
            return;
        }
        if self.channels.len() < input_channel_count {
            self.channels.resize_with(input_channel_count, || None);
        }

        // Late-reverb bus: per-channel sends accumulate here; the shared FDN
        // turns the mono sum into a decorrelated stereo tail after the loop.
        let reverb_active = reverb.enabled;
        if reverb_active {
            let fdn = self.fdn.get_or_insert_with(|| Fdn::new(self.sample_rate));
            fdn.set_params(reverb.rt60_s, reverb.predelay_ms);
            self.reverb_bus.clear();
            self.reverb_bus.resize(sample_length, 0.0);
        } else if self.fdn.is_some() {
            self.fdn = None;
        }

        for c in 0..input_channel_count {
            let gain = chan_gain.get(c).copied().unwrap_or(0.0);
            if gain == 0.0 {
                // Keep an existing reflection bank fading out so a muted
                // channel does not freeze its taps at full gain.
                if let Some(Some(dsp)) = self.channels.get_mut(c) {
                    if let Some(bank) = dsp.refl.as_mut() {
                        bank.mute_targets();
                    }
                }
                continue;
            }
            let pos = chan_pos.get(c).copied().unwrap_or([0.0, 1.0, 0.0]);

            // World → head-relative direction, then spherical angles.
            let hp = head_pose.rotate(pos);
            let (hx, hy, hz) = (hp[0] as f32, hp[1] as f32, hp[2] as f32);
            let az_rad = hx.atan2(hy); // 0 = front, + = right
            let horiz = (hx * hx + hy * hy).sqrt();
            let el_rad = hz.atan2(horiz);

            // Isotropic distance scale → metric distance (m). Direction is
            // scale-invariant. Object/bed levels are authored upstream (Atmos
            // object gain), so the direct path applies NO inverse-distance gain;
            // dist_m only drives the distance *cues* (air absorption, reverb
            // send, early reflections), never the direct object level.
            let dist_norm = ((pos[0] * pos[0] + pos[1] * pos[1] + pos[2] * pos[2]).sqrt()) as f32;
            let dist_m = (dist_norm * unit_scale_m).max(0.0);

            // Per-frame HRIR + ITD update (continuous: delay/convolver state persists).
            self.hrir.at(
                az_rad.to_degrees(),
                el_rad.to_degrees(),
                &mut self.hrir_scratch,
            );
            let (itd_l, itd_r) = itd::ear_delays_seconds(az_rad, el_rad, head_radius_m);

            let dsp = self.channels[c].get_or_insert_with(|| ChannelDsp::new(self.sample_rate));
            dsp.conv_l.set_coeffs(&self.hrir_scratch.left);
            dsp.conv_r.set_coeffs(&self.hrir_scratch.right);
            dsp.delay_l.set_target_ms(itd_l * 1000.0, self.sample_rate);
            dsp.delay_r.set_target_ms(itd_r * 1000.0, self.sample_rate);

            // Air absorption: a one-pole low-pass whose cutoff falls with
            // distance (HF dies in air — true outdoors as much as indoors).
            // Bypass within 3 m; ~14 kHz at 10 m, ~5 kHz at 30 m, floor 2 kHz.
            dsp.air_coeff = if air_absorption && dist_m > 3.0 {
                let fc = (20_000.0 * (-0.05 * (dist_m - 3.0)).exp()).max(2_000.0);
                (-std::f32::consts::TAU * fc / self.sample_rate as f32).exp()
            } else {
                0.0
            };

            // Reverb send: distance-independent (a real room's reverberant
            // field barely depends on source distance — the 1/d on the direct
            // is what moves the direct/reverb ratio), with a near-field
            // roll-in so sources at the listener stay dry.
            let reverb_send = if reverb_active {
                // Multiplier on the (already gain-scaled) channel signal.
                dist_m / (dist_m + 0.5)
            } else {
                0.0
            };

            // ── Early reflections: per-block image-source update ─────────────
            if reflections.enabled {
                let bank = dsp
                    .refl
                    .get_or_insert_with(|| ReflectionBank::new(self.sample_rate));
                let phys = [
                    pos[0] as f32 * unit_scale_m,
                    pos[1] as f32 * unit_scale_m,
                    pos[2] as f32 * unit_scale_m,
                ];
                let images = reflections::first_order_images(phys, reflections.room_size_m);
                let c_sound = reflections::speed_of_sound();
                for (i, img) in images.iter().enumerate() {
                    let d_img = (img[0] * img[0] + img[1] * img[1] + img[2] * img[2])
                        .sqrt()
                        .max(MIN_DISTANCE_M);
                    // Relative to the direct path so the direct sound keeps
                    // zero added latency (A/V sync unchanged).
                    let rel_delay_s = (d_img - dist_m).max(0.0) / c_sound;
                    // Head-relative direction → broadband ILD pan (no HRIR
                    // conv per reflection: one tap + one multiply per ear).
                    let ih = head_pose.rotate([img[0] as f64, img[1] as f64, img[2] as f64]);
                    let inorm = ((ih[0] * ih[0] + ih[1] * ih[1] + ih[2] * ih[2]) as f32)
                        .sqrt()
                        .max(1e-6);
                    let lat = (ih[0] as f32 / inorm).clamp(-1.0, 1.0);
                    const SHADOW: f32 = 0.5;
                    let g_r = ((1.0 + SHADOW * lat) / (1.0 + SHADOW)).sqrt();
                    let g_l = ((1.0 - SHADOW * lat) / (1.0 + SHADOW)).sqrt();
                    let g_dist = (REF_DISTANCE_M / d_img).clamp(0.0, MAX_DISTANCE_GAIN);
                    let g = reflections.level.clamp(0.0, 1.0) * g_dist;
                    bank.set_targets(i, rel_delay_s, g * g_l, g * g_r);
                }
            } else if dsp.refl.is_some() {
                // Drop the bank when disabled — the ring is the big allocation.
                dsp.refl = None;
            }

            let air = dsp.air_coeff;
            for s in 0..sample_length {
                // `raw` carries the object/metadata gain only; the direct path
                // adds its distance gain, the reflection taps theirs. The air
                // low-pass applies to the propagated wave, so it feeds the
                // direct, the reflections and the reverb send alike.
                let mut raw = input_pcm[s * input_channel_count + c] * gain;
                if air > 0.0 {
                    dsp.air_state += (raw - dsp.air_state) * (1.0 - air);
                    raw = dsp.air_state;
                }
                // Authored object/bed level is respected: no 1/d attenuation.
                let x = raw;
                let mut yl = dsp.conv_l.process(dsp.delay_l.process(x));
                let mut yr = dsp.conv_r.process(dsp.delay_r.process(x));
                if let Some(bank) = dsp.refl.as_mut() {
                    let (rl, rr) = bank.process(raw);
                    yl += rl;
                    yr += rr;
                }
                if reverb_active {
                    self.reverb_bus[s] += raw * reverb_send;
                }
                let o = s * 2;
                out[o] += yl;
                out[o + 1] += yr;
            }
        }

        // Shared tail: one FDN pass over the summed sends, added to the mix.
        if reverb_active {
            if let Some(fdn) = self.fdn.as_mut() {
                fdn.process_block(&self.reverb_bus, reverb.level, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Anechoic frame params: identity pose, unit scale, default head radius,
    /// reflections off — the legacy expectations below are HRIR/ITD-only.
    fn dry_params() -> BinauralFrameParams {
        BinauralFrameParams {
            head_pose: HeadPose::identity(),
            unit_scale_m: 1.0,
            head_radius_m: itd::DEFAULT_HEAD_RADIUS_M,
            reflections: BinauralReflections {
                enabled: false,
                ..Default::default()
            },
            reverb: BinauralReverb {
                enabled: false,
                ..Default::default()
            },
            air_absorption: false,
        }
    }

    fn render_single(pos: [f64; 3]) -> (f32, f32) {
        let mut r = BinauralRenderer::new(48_000);
        let n = 512;
        // Single impulse: per-ear output energy then equals the (delay-preserved)
        // HRIR energy — a broadband probe that doesn't over-weight the Nyquist bin
        // the way an alternating ±1 input would on a measured HRIR.
        let mut input = vec![0.0f32; n];
        input[0] = 1.0;
        let mut out = vec![0.0f32; n * 2];
        r.render_frame(&input, 1, n, &dry_params(), &[pos], &[1.0], &mut out);
        let mut el = 0.0f32;
        let mut er = 0.0f32;
        for s in 0..n {
            el += out[s * 2] * out[s * 2];
            er += out[s * 2 + 1] * out[s * 2 + 1];
        }
        (el, er)
    }

    #[test]
    fn right_source_is_louder_in_right_channel() {
        let (el, er) = render_single([1.0, 0.0, 0.0]); // full right
        assert!(er > el, "L={el} R={er}");
    }

    #[test]
    fn left_source_is_louder_in_left_channel() {
        let (el, er) = render_single([-1.0, 0.0, 0.0]); // full left
        assert!(el > er, "L={el} R={er}");
    }

    #[test]
    fn front_source_is_balanced() {
        let (el, er) = render_single([0.0, 1.0, 0.0]); // front
        let ratio = el / er;
        assert!((0.5..2.0).contains(&ratio), "L={el} R={er}");
    }

    #[test]
    fn reflections_add_delayed_energy() {
        // Impulse at 2 m in a 4 m room: the side/ceiling images detour ~2.5 m
        // (~343 samples at 48 kHz) and the rear wall ~4 m (~553 samples), all
        // far past the 128-tap HRIR tail (~160 samples incl. ITD). With
        // reflections ON there must be energy out there; with them OFF, silence.
        let n = 4_096;
        let mut input = vec![0.0f32; n];
        input[0] = 1.0;
        let pos = [[0.0, 2.0, 0.0]];
        let tail = |out: &[f32]| -> f32 { out[250 * 2..].iter().map(|x| x * x).sum::<f32>() };

        let mut dry = vec![0.0f32; n * 2];
        let mut r = BinauralRenderer::new(48_000);
        r.render_frame(&input, 1, n, &dry_params(), &pos, &[1.0], &mut dry);

        let wet_params = BinauralFrameParams {
            reflections: BinauralReflections {
                enabled: true,
                room_size_m: [4.0, 4.0, 4.0],
                level: 0.5,
            },
            ..dry_params()
        };
        let mut wet = vec![0.0f32; n * 2];
        let mut r = BinauralRenderer::new(48_000);
        r.render_frame(&input, 1, n, &wet_params, &pos, &[1.0], &mut wet);

        assert!(tail(&dry) < 1e-9, "dry render must have no late energy");
        assert!(
            tail(&wet) > 1e-6,
            "reflections produced no late energy: {}",
            tail(&wet)
        );
    }

    #[test]
    fn reverb_adds_a_long_tail() {
        let n = 24_000; // 0.5 s
        let mut input = vec![0.0f32; n];
        input[0] = 1.0;
        let pos = [[0.0, 2.0, 0.0]];
        // Energy far past both the HRIR tail and the early-reflection window.
        let tail = |out: &[f32]| -> f32 { out[4_000 * 2..].iter().map(|x| x * x).sum() };

        let mut dry = vec![0.0f32; n * 2];
        let mut r = BinauralRenderer::new(48_000);
        r.render_frame(&input, 1, n, &dry_params(), &pos, &[1.0], &mut dry);
        assert!(tail(&dry) < 1e-9, "dry render must have no tail");

        let wet_params = BinauralFrameParams {
            reverb: BinauralReverb {
                enabled: true,
                level: 0.3,
                rt60_s: 0.4,
                predelay_ms: 20.0,
            },
            ..dry_params()
        };
        let mut wet = vec![0.0f32; n * 2];
        let mut r = BinauralRenderer::new(48_000);
        r.render_frame(&input, 1, n, &wet_params, &pos, &[1.0], &mut wet);
        assert!(tail(&wet) > 1e-7, "no reverb tail: {}", tail(&wet));
    }

    #[test]
    fn air_absorption_dulls_distant_sources() {
        // Nyquist-rate tone: a distance low-pass crushes it. Compare output
        // energy near vs far (the direct path has no 1/d gain to factor out).
        let n = 2_048;
        let input: Vec<f32> = (0..n)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let render = |dist: f64| -> f32 {
            let params = BinauralFrameParams {
                air_absorption: true,
                ..dry_params()
            };
            let mut out = vec![0.0f32; n * 2];
            let mut r = BinauralRenderer::new(48_000);
            r.render_frame(&input, 1, n, &params, &[[0.0, dist, 0.0]], &[1.0], &mut out);
            // Direct path no longer applies a 1/d gain, so the broadband level is
            // distance-independent; the only near/far difference is the
            // air-absorption HF roll-off under test.
            out[400 * 2..].iter().map(|x| x * x).sum()
        };
        let near = render(2.0); // within the 3 m bypass → full HF
        let far = render(30.0); // ~5 kHz cutoff → Nyquist crushed
        assert!(
            far < near * 0.2,
            "air absorption ineffective: near={near} far={far}"
        );
    }

    #[test]
    fn direct_level_is_distance_invariant() {
        // Object/bed level is authored (Atmos); the binaural direct path must
        // NOT re-attenuate by 1/d. A front source rendered near vs far — same
        // authored gain, air/reverb/reflections off — must yield identical
        // broadband energy. Before the fix this differed by the 1/d clamp
        // (here 2 m → ×0.5 vs 8 m → ×0.125, a 4× energy gap).
        let (nl, nr) = render_single([0.0, 2.0, 0.0]); // near
        let (fl, fr) = render_single([0.0, 8.0, 0.0]); // far
        let near = nl + nr;
        let far = fl + fr;
        assert!(near > 0.0, "near energy must be non-zero");
        assert!(
            (far - near).abs() <= near * 1e-6,
            "direct object level changed with distance: near={near} far={far}"
        );
    }

    #[test]
    fn muted_channel_is_silent() {
        let mut r = BinauralRenderer::new(48_000);
        let n = 64;
        let input = vec![1.0f32; n];
        let mut out = vec![0.0f32; n * 2];
        r.render_frame(
            &input,
            1,
            n,
            &dry_params(),
            &[[1.0, 0.0, 0.0]],
            &[0.0],
            &mut out,
        );
        assert!(out.iter().all(|&x| x == 0.0));
    }
}

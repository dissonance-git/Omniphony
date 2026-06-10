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
//!   → distance gain → per-ear ITD delay → per-ear HRIR convolution
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
pub mod reflections;
pub mod tracking;

pub use head_pose::HeadPose;
pub use tracking::{HeadTracking, HeadTrackingFormat};

use crate::delay_line::DelayLine;
use crate::live_params::BinauralReflections;
use convolver::EarConvolver;
use hrir::{HRIR_LEN, HrirPair, HrirSet};
use measured::MeasuredHrirData;
use reflections::ReflectionBank;

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
}

impl HrirSource {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Synthetic => "synthetic",
            Self::SafKemar => "saf",
            Self::Sofa(_) => "sofa",
        }
    }

    /// Parse a source selector. `"sofa:<path>"` carries the file path; a bare
    /// `"sofa"` yields `Sofa("")` (path to be set separately).
    pub fn from_str(s: &str) -> Option<Self> {
        let s = s.trim();
        if let Some(path) = s.strip_prefix("sofa:") {
            return Some(Self::Sofa(path.to_string()));
        }
        match s.to_ascii_lowercase().as_str() {
            "synthetic" | "synth" => Some(Self::Synthetic),
            "saf" | "kemar" | "saf_kemar" => Some(Self::SafKemar),
            "sofa" => Some(Self::Sofa(String::new())),
            _ => None,
        }
    }
}

/// Reference distance (m) at which the distance gain is unity.
const REF_DISTANCE_M: f32 = 1.0;
/// Closest distance (m) used for the 1/d law, bounding near-source boost.
const MIN_DISTANCE_M: f32 = 0.25;
/// Maximum distance gain, so a source at the origin can't blow up.
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
        }
    }

    fn build_hrir(source: &HrirSource, sample_rate: u32) -> HrirSet {
        match source {
            HrirSource::Synthetic => HrirSet::synthetic(sample_rate),
            HrirSource::SafKemar => HrirSet::new(&MeasuredHrirData::saf_kemar(), sample_rate),
            HrirSource::Sofa(path) => match Self::load_sofa(path, sample_rate) {
                Some(set) => set,
                None => {
                    log::warn!(
                        "binaural: SOFA source '{path}' unavailable; falling back to SAF KEMAR"
                    );
                    HrirSet::new(&MeasuredHrirData::saf_kemar(), sample_rate)
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
        } = *params;
        debug_assert_eq!(out.len(), sample_length * 2);
        if input_channel_count == 0 || sample_length == 0 {
            return;
        }
        if self.channels.len() < input_channel_count {
            self.channels.resize_with(input_channel_count, || None);
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

            // Isotropic distance scale → 1/d gain (direction is scale-invariant).
            let dist_norm = ((pos[0] * pos[0] + pos[1] * pos[1] + pos[2] * pos[2]).sqrt()) as f32;
            let dist_m = (dist_norm * unit_scale_m).max(0.0);
            let dist_gain =
                (REF_DISTANCE_M / dist_m.max(MIN_DISTANCE_M)).clamp(0.0, MAX_DISTANCE_GAIN);

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

            for s in 0..sample_length {
                // `raw` carries the object/metadata gain only; the direct path
                // adds its distance gain, the reflection taps theirs.
                let raw = input_pcm[s * input_channel_count + c] * gain;
                let x = raw * dist_gain;
                let mut yl = dsp.conv_l.process(dsp.delay_l.process(x));
                let mut yr = dsp.conv_r.process(dsp.delay_r.process(x));
                if let Some(bank) = dsp.refl.as_mut() {
                    let (rl, rr) = bank.process(raw);
                    yl += rl;
                    yr += rr;
                }
                let o = s * 2;
                out[o] += yl;
                out[o + 1] += yr;
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

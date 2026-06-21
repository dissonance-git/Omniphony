//! Phantom-source extraction pre-stage (runs *before* the bed→height lift).
//!
//! For each pair of bed channels — a ring of azimuth-adjacent speakers, optionally
//! widened to non-adjacent pairs — this estimates the **correlated ("primary")**
//! component shared by the two channels, re-emits it as a discrete object at the
//! **real panned position** (derived from the component's relative level in each
//! channel), and **subtracts** it from the two source channels. The bed handed to
//! the height stage therefore carries only the decorrelated residual. It is the
//! conceptual inverse of PAD (which keeps the correlated part grounded and lifts
//! the residual): here the correlated part becomes a localized object and the
//! residual stays in the bed (to be lifted by PAD/copy_up afterwards).
//!
//! Like the object generators it runs in the realtime audio thread: all setup is
//! in [`PhantomExtractStage::sync`]; the per-frame DSP in
//! [`PhantomExtractStage::process_and_extend`] does not allocate (steady state)
//! and never panics.

use bridge_api::RChannelLabel;

use crate::object_gen::{
    ObjectGenParamSpec, PrepareCtx, SynthObjectSpec, channel_top_position, input_has_back,
    one_pole_coeff,
};

/// Time constant (ms) of the one-pole smoothers for the inter-channel statistics.
/// A bit longer than PAD's, so the derived pan position moves smoothly (no jittery
/// object motion in the 3D view).
const PHANTOM_STAT_TC_MS: f32 = 80.0;
/// Relative regularization on the Wiener denominator (fraction of the pair's mean
/// power) plus a tiny absolute floor — keeps the weight finite without capping the
/// large ratios that strong pans legitimately produce.
const PHANTOM_REG: f32 = 1.0e-2;
const PHANTOM_FLOOR: f32 = 1.0e-9;
/// Weight clamp. Unlike PAD (which clamps near unity) the phantom weight *is* the
/// pan ratio, which is large for hard-panned content, so the clamp is generous.
const PHANTOM_W_CLAMP: f32 = 16.0;
const PHANTOM_DEFAULT_STRENGTH: f32 = 0.5;
const PHANTOM_MAX_PASSES: usize = 3;
const PHANTOM_GAIN_DB: i8 = 0;
const PHANTOM_SIZE: [f32; 3] = [0.3, 0.3, 0.3];

/// Live-tunable params this stage declares (the schema the UI builds sliders from).
pub const PHANTOM_PARAM_SPECS: [ObjectGenParamSpec; 3] = [
    ObjectGenParamSpec {
        key: "strength",
        label: "Extraction",
        i18n_key: "twoDSources.phantomStrength",
        min: 0.0,
        max: 1.0,
        step: 0.01,
        default: PHANTOM_DEFAULT_STRENGTH,
        unit: "",
    },
    ObjectGenParamSpec {
        key: "passes",
        label: "Passes",
        i18n_key: "twoDSources.phantomPasses",
        min: 1.0,
        max: 3.0,
        step: 1.0,
        default: 1.0,
        unit: "",
    },
    ObjectGenParamSpec {
        key: "lift",
        label: "Lift",
        i18n_key: "twoDSources.phantomLift",
        min: 0.0,
        max: 1.0,
        step: 0.01,
        default: 0.0,
        unit: "",
    },
];

/// JSON array of the declared params, published over OSC so Studio can build the
/// phantom-extraction sliders dynamically (same shape as the object-generator
/// `params` array).
pub fn phantom_schema_json() -> String {
    let arr: Vec<serde_json::Value> = PHANTOM_PARAM_SPECS
        .iter()
        .map(|p| {
            serde_json::json!({
                "key": p.key,
                "label": p.label,
                "i18nKey": p.i18n_key,
                "min": p.min,
                "max": p.max,
                "step": p.step,
                "default": p.default,
                "unit": p.unit,
            })
        })
        .collect();
    serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
}

/// One extracted phantom: the two source channels, the planar output slot, the two
/// source floor positions to interpolate between, and the persistent smoothed
/// statistics.
struct PhantomPair {
    a_ch: usize,
    b_ch: usize,
    out: usize,
    /// Source floor positions (z = 0); the phantom is interpolated between them.
    pos_a: [f64; 3],
    pos_b: [f64; 3],
    /// Smoothed auto/cross power for the Wiener weight.
    pow_a: f32,
    pow_b: f32,
    cross: f32,
    /// Smoothed energy of the correlated component in each channel → pan fraction.
    e_a: f32,
    e_b: f32,
}

/// Signature of the inputs the current plan was built for; a change (including a
/// `passes` change) triggers a rebuild. Compared without per-frame allocation.
#[derive(Default)]
struct PlanSig {
    enabled: bool,
    labels: Vec<RChannelLabel>,
    rate: u32,
    passes: usize,
}

/// Host-side state for the phantom-extraction stage.
pub struct PhantomExtractStage {
    pairs: Vec<PhantomPair>,
    specs: Vec<SynthObjectSpec>,
    /// Per-phantom planar audio scratch (persistent).
    planar: Vec<Vec<f32>>,
    /// Extended interleaved PCM (reduced bed + phantom object channels).
    pcm_ext: Vec<f32>,
    sig: PlanSig,
    strength: f32,
    passes: usize,
    lift: f32,
    alpha: f32,
}

impl PhantomExtractStage {
    pub fn new() -> Self {
        Self {
            pairs: Vec::new(),
            specs: Vec::new(),
            planar: Vec::new(),
            pcm_ext: Vec::new(),
            sig: PlanSig::default(),
            strength: PHANTOM_DEFAULT_STRENGTH,
            passes: 1,
            lift: 0.0,
            alpha: 0.0,
        }
    }

    pub fn specs(&self) -> &[SynthObjectSpec] {
        &self.specs
    }

    /// (Re)build the pair plan if the selection/environment/`passes` changed, then
    /// refresh the dynamic phantom positions from the smoothed statistics (cheap,
    /// every frame). Returns the phantom count (`0` = inactive / no-op).
    pub fn sync(&mut self, enabled: bool, ctx: &PrepareCtx) -> usize {
        let changed = self.sig.enabled != enabled
            || self.sig.rate != ctx.sample_rate
            || self.sig.passes != self.passes
            || self.sig.labels.as_slice() != ctx.input_labels;
        if changed {
            self.sig.enabled = enabled;
            self.sig.rate = ctx.sample_rate;
            self.sig.passes = self.passes;
            self.sig.labels.clear();
            self.sig.labels.extend_from_slice(ctx.input_labels);
            self.rebuild(enabled, ctx);
        }
        self.refresh_positions();
        self.specs.len()
    }

    /// Apply one declared parameter in place. A `passes` change re-plans on the next
    /// `sync`.
    pub fn set_param(&mut self, key: &str, value: f32, _sample_rate: u32) {
        match key {
            "strength" => self.strength = value.clamp(0.0, 1.0),
            "passes" => {
                self.passes = (value.round() as i64).clamp(1, PHANTOM_MAX_PASSES as i64) as usize
            }
            "lift" => self.lift = value.clamp(0.0, 1.0),
            _ => {}
        }
    }

    fn rebuild(&mut self, enabled: bool, ctx: &PrepareCtx) {
        self.pairs.clear();
        self.specs.clear();
        let fs = ctx.sample_rate.max(1) as f32;
        self.alpha = one_pole_coeff(PHANTOM_STAT_TC_MS, fs);
        if !enabled {
            self.planar.clear();
            return;
        }
        // Present, positionable bed channels with their floor position + azimuth.
        // Honour the Side/Back surround placement (matching the virtual bed), so a
        // 4.x/5.x surround phantom sits where the user put the surrounds.
        let use_7_1 = input_has_back(ctx.input_labels);
        let mut chans: Vec<(usize, RChannelLabel, [f64; 3], f64)> = Vec::new();
        for (idx, &label) in ctx.input_labels.iter().enumerate() {
            if let Some(top) = channel_top_position(label, use_7_1, ctx.surround_placement) {
                let floor = [top[0], top[1], 0.0];
                let az = floor[0].atan2(floor[1]); // atan2(x, y): 0 = front, +90° = right
                chans.push((idx, label, floor, az));
            }
        }
        if chans.len() < 2 {
            self.planar.clear();
            return;
        }
        chans.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));
        let n = chans.len();
        let passes = self.passes.clamp(1, PHANTOM_MAX_PASSES);
        // Ring distances 1..=passes around the azimuth circle, each unordered pair
        // once (distance d and n−d coincide for a cycle).
        let mut seen: Vec<(usize, usize)> = Vec::new();
        for d in 1..=passes {
            if d >= n {
                break;
            }
            for i in 0..n {
                let j = (i + d) % n;
                if i == j {
                    continue;
                }
                let key = if i < j { (i, j) } else { (j, i) };
                if seen.contains(&key) {
                    continue;
                }
                seen.push(key);
                let (a_ch, a_label, pos_a, _) = chans[key.0];
                let (b_ch, b_label, pos_b, _) = chans[key.1];
                let out = self.specs.len();
                self.pairs.push(PhantomPair {
                    a_ch,
                    b_ch,
                    out,
                    pos_a,
                    pos_b,
                    pow_a: 0.0,
                    pow_b: 0.0,
                    cross: 0.0,
                    e_a: 0.0,
                    e_b: 0.0,
                });
                self.specs.push(SynthObjectSpec {
                    name: format!("Phantom_{a_label:?}_{b_label:?}"),
                    position: [
                        0.5 * (pos_a[0] + pos_b[0]),
                        0.5 * (pos_a[1] + pos_b[1]),
                        self.lift as f64,
                    ],
                    gain_db: PHANTOM_GAIN_DB,
                    size: PHANTOM_SIZE,
                });
            }
        }
        self.planar.truncate(self.specs.len());
        self.planar.resize_with(self.specs.len(), Vec::new);
    }

    /// Interpolate each phantom's position from its smoothed per-side correlated
    /// energy (pan fraction `t = e_b / (e_a + e_b)`), with `z = lift`.
    fn refresh_positions(&mut self) {
        let lift = self.lift as f64;
        for (pair, spec) in self.pairs.iter().zip(self.specs.iter_mut()) {
            let t = (pair.e_b / (pair.e_a + pair.e_b + PHANTOM_FLOOR)) as f64;
            spec.position = [
                pair.pos_a[0] + t * (pair.pos_b[0] - pair.pos_a[0]),
                pair.pos_a[1] + t * (pair.pos_b[1] - pair.pos_a[1]),
                lift,
            ];
        }
    }

    /// Run the extraction (mutating `bed` in place — the correlated component is
    /// subtracted from each source channel) and return the bed extended with the
    /// phantom object channels, plus the new channel count. Call only when
    /// [`sync`](Self::sync) returned `> 0`.
    pub fn process_and_extend(
        &mut self,
        bed: &mut [f32],
        channel_count: usize,
        sample_count: usize,
        _sample_rate: u32,
    ) -> (&[f32], usize) {
        self.process(bed, channel_count, sample_count);
        let m = self.specs.len();
        let out_ch = channel_count + m;
        self.pcm_ext.clear();
        self.pcm_ext.resize(sample_count * out_ch, 0.0);
        for s in 0..sample_count {
            let src = &bed[s * channel_count..s * channel_count + channel_count];
            let dst = &mut self.pcm_ext[s * out_ch..s * out_ch + out_ch];
            dst[..channel_count].copy_from_slice(src);
            for (k, buf) in self.planar.iter().enumerate().take(m) {
                dst[channel_count + k] = buf[s];
            }
        }
        (&self.pcm_ext, out_ch)
    }

    fn process(&mut self, bed: &mut [f32], c: usize, n: usize) {
        let strength = self.strength;
        let alpha = self.alpha;
        let m = self.specs.len();
        // Borrow the planar scratch out of `self` so the pair loop can write it
        // while iterating `self.pairs` mutably (disjoint, but this keeps it simple).
        let mut planar = std::mem::take(&mut self.planar);
        for buf in planar.iter_mut() {
            buf.clear();
            buf.resize(n, 0.0);
        }
        for pair in self.pairs.iter_mut() {
            if pair.a_ch >= c || pair.b_ch >= c || pair.out >= m {
                continue;
            }
            let out = &mut planar[pair.out];
            for s in 0..n {
                let ia = s * c + pair.a_ch;
                let ib = s * c + pair.b_ch;
                let a = bed[ia];
                let b = bed[ib];

                pair.pow_a += alpha * (a * a - pair.pow_a);
                pair.pow_b += alpha * (b * b - pair.pow_b);
                pair.cross += alpha * (a * b - pair.cross);

                let reg = PHANTOM_REG * 0.5 * (pair.pow_a + pair.pow_b) + PHANTOM_FLOOR;
                let w_ab =
                    (pair.cross / (pair.pow_b + reg)).clamp(-PHANTOM_W_CLAMP, PHANTOM_W_CLAMP);
                let w_ba =
                    (pair.cross / (pair.pow_a + reg)).clamp(-PHANTOM_W_CLAMP, PHANTOM_W_CLAMP);

                // Coherence in [0, 1]: 0 leaves a decorrelated pair untouched.
                let denom = (pair.pow_a * pair.pow_b).sqrt() + PHANTOM_FLOOR;
                let coh = (pair.cross / denom).clamp(0.0, 1.0);
                let k = strength * coh;

                // Correlated component as it appears in each channel.
                let ca = w_ab * b;
                let cb = w_ba * a;

                // Remove it from the bed.
                bed[ia] = a - k * ca;
                bed[ib] = b - k * cb;

                // Object signal = the removed component projected onto the pan
                // direction (matched filter), so re-panning it ≈ what was removed.
                let ga = pair.pow_a.sqrt();
                let gb = pair.pow_b.sqrt();
                let gnorm = (pair.pow_a + pair.pow_b).sqrt() + PHANTOM_FLOOR;
                out[s] = k * (ca * ga + cb * gb) / gnorm;

                // Smoothed per-side correlated energy → pan position next frame.
                pair.e_a += alpha * (ca * ca - pair.e_a);
                pair.e_b += alpha * (cb * cb - pair.e_b);
            }
        }
        self.planar = planar;
    }
}

impl Default for PhantomExtractStage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderer::speaker_layout::{Speaker, SpeakerLayout};

    const LABELS_5_1: [RChannelLabel; 6] = [
        RChannelLabel::L,
        RChannelLabel::R,
        RChannelLabel::C,
        RChannelLabel::LFE,
        RChannelLabel::Ls,
        RChannelLabel::Rs,
    ];

    fn dummy_layout() -> SpeakerLayout {
        // The phantom stage ignores the output layout (planar objects render on
        // whatever speakers exist); any non-empty layout works.
        SpeakerLayout {
            radius_m: 1.0,
            speakers: vec![
                Speaker::from_cartesian("FL", -1.0, 1.0, 0.0, true, 0.0),
                Speaker::from_cartesian("FR", 1.0, 1.0, 0.0, true, 0.0),
            ],
        }
    }

    fn ctx<'a>(labels: &'a [RChannelLabel], layout: &'a SpeakerLayout) -> PrepareCtx<'a> {
        PrepareCtx {
            input_labels: labels,
            output_layout: layout,
            sample_rate: 48_000,
            surround_placement: renderer::live_params::SurroundPlacement::Side,
        }
    }

    fn sine(f: f32) -> impl Fn(usize) -> f32 {
        move |s: usize| (std::f32::consts::TAU * f * s as f32 / 48_000.0).sin()
    }

    #[test]
    fn disabled_is_noop() {
        let mut st = PhantomExtractStage::new();
        let layout = dummy_layout();
        assert_eq!(st.sync(false, &ctx(&LABELS_5_1, &layout)), 0);
    }

    #[test]
    fn plans_ring_pairs() {
        let mut st = PhantomExtractStage::new();
        let layout = dummy_layout();
        // 5 positionable channels (L,R,C,Ls,Rs; LFE excluded) → ring of 5 pairs.
        assert_eq!(st.sync(true, &ctx(&LABELS_5_1, &layout)), 5);
    }

    #[test]
    fn passes_widen_the_pair_set() {
        let mut st = PhantomExtractStage::new();
        let layout = dummy_layout();
        let ring = st.sync(true, &ctx(&LABELS_5_1, &layout));
        st.set_param("passes", 2.0, 48_000);
        let widened = st.sync(true, &ctx(&LABELS_5_1, &layout));
        assert!(
            widened > ring,
            "passes=2 ({widened}) should add pairs over ring ({ring})"
        );
    }

    #[test]
    fn extracts_panned_source_and_removes_it() {
        let mut st = PhantomExtractStage::new();
        let layout = dummy_layout();
        st.set_param("strength", 1.0, 48_000);
        st.sync(true, &ctx(&LABELS_5_1, &layout));
        // A correlated source panned between adjacent channels L and C (0.8 / 0.2).
        let c = 6usize;
        let n = 6000usize;
        let s = sine(700.0);
        let mut bed = vec![0.0f32; c * n];
        let mut in_l = 0.0f32;
        for i in 0..n {
            let v = s(i) * 0.5;
            bed[i * c] = 0.8 * v; // L (index 0)
            bed[i * c + 2] = 0.2 * v; // C (index 2)
            if i >= n / 2 {
                in_l += (0.8 * v).powi(2);
            }
        }
        let k = st
            .specs()
            .iter()
            .position(|sp| sp.name == "Phantom_L_C")
            .unwrap();
        let (pcm, out_ch) = st.process_and_extend(&mut bed, c, n, 48_000);
        let tail = n / 2..n;
        let ph: f32 = tail.clone().map(|i| pcm[i * out_ch + c + k].powi(2)).sum();
        let l_red: f32 = tail.map(|i| pcm[i * out_ch].powi(2)).sum();
        assert!(
            ph > 0.2 * in_l,
            "phantom should carry the panned energy ({ph} vs in_l {in_l})"
        );
        assert!(
            l_red < 0.1 * in_l,
            "L should be largely emptied ({l_red} vs {in_l})"
        );
        // Position: refresh from the converged stats and check it sits toward L.
        st.sync(true, &ctx(&LABELS_5_1, &layout));
        let pos = st.specs()[k].position;
        assert!(
            pos[0] < -0.5,
            "phantom should localize toward L, x = {}",
            pos[0]
        );
    }

    #[test]
    fn leaves_decorrelated_pair() {
        let mut st = PhantomExtractStage::new();
        let layout = dummy_layout();
        st.set_param("strength", 1.0, 48_000);
        st.sync(true, &ctx(&LABELS_5_1, &layout));
        let c = 6usize;
        let n = 6000usize;
        let (s1, s2) = (sine(700.0), sine(1130.0)); // independent tones
        let mut bed = vec![0.0f32; c * n];
        let mut in_l = 0.0f32;
        for i in 0..n {
            bed[i * c] = s1(i) * 0.5; // L
            bed[i * c + 2] = s2(i) * 0.5; // C
            if i >= n / 2 {
                in_l += (s1(i) * 0.5).powi(2);
            }
        }
        let k = st
            .specs()
            .iter()
            .position(|sp| sp.name == "Phantom_L_C")
            .unwrap();
        let (pcm, out_ch) = st.process_and_extend(&mut bed, c, n, 48_000);
        let tail = n / 2..n;
        let ph: f32 = tail.clone().map(|i| pcm[i * out_ch + c + k].powi(2)).sum();
        let l_red: f32 = tail.map(|i| pcm[i * out_ch].powi(2)).sum();
        assert!(
            ph < 0.05 * in_l,
            "decorrelated pair → near-silent phantom ({ph} vs {in_l})"
        );
        assert!(
            l_red > 0.7 * in_l,
            "decorrelated L should be mostly retained ({l_red} vs {in_l})"
        );
    }

    #[test]
    fn lift_raises_phantom_z() {
        let mut st = PhantomExtractStage::new();
        let layout = dummy_layout();
        st.set_param("lift", 0.8, 48_000);
        st.sync(true, &ctx(&LABELS_5_1, &layout));
        assert!(
            st.specs()
                .iter()
                .all(|sp| (sp.position[2] - 0.8).abs() < 1.0e-6)
        );
    }

    #[test]
    fn finite_output() {
        let mut st = PhantomExtractStage::new();
        let layout = dummy_layout();
        st.sync(true, &ctx(&LABELS_5_1, &layout));
        let c = 6usize;
        let n = 2000usize;
        let s = sine(500.0);
        let mut bed = vec![0.0f32; c * n];
        for i in 0..n {
            bed[i * c] = s(i) * 0.5;
            bed[i * c + 1] = s(i) * 0.3;
            bed[i * c + 2] = s(i) * 0.2;
        }
        let (pcm, _) = st.process_and_extend(&mut bed, c, n, 48_000);
        assert!(pcm.iter().all(|x| x.is_finite()));
    }
}

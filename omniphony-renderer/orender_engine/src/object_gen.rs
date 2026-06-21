//! Pluggable bed→height object generator stage.
//!
//! Turns channel-based ("2D") content that carries no height information into a
//! set of synthesized height *objects* placed above the listener, so an output
//! layout with top speakers (7.1.4, …) is exercised even when the source has
//! none (DTS core, E-AC3, plain 5.1/7.1). It mirrors the render-backend
//! extensibility: contributors add a compiled [`ObjectGeneratorFactory`] to the
//! [`ObjectGeneratorRegistry`]; the realtime DSP runs per frame in
//! [`ObjectGenerator::process`].
//!
//! The stage is OFF by default (live param `object_generator_id` empty/`none`).
//! It is a strict no-op — and costs nothing — when the output layout has no
//! height speaker, or when the input already carries height channels.
//!
//! Synthesized objects flow through the *existing* object path: their audio is
//! appended as extra object channels (so the VBAP mix spatializes them
//! unchanged), and their static descriptors are surfaced over OSC so they appear
//! in the Studio 3D view and object list like any ADM object.

use bridge_api::RChannelLabel;
use renderer::speaker_layout::SpeakerLayout;

/// What a generator needs from its environment; lets the host gate the UI.
#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectGenCapabilities {
    /// The generator only produces output on a height-capable output layout.
    pub requires_height_layer: bool,
}

/// One live-tunable parameter a generator declares, so the host can build a
/// control for it and the renderer can validate/apply it by key. Static per
/// generator type — the schema, not a value.
#[derive(Debug, Clone, Copy)]
pub struct ObjectGenParamSpec {
    /// Stable key used in the OSC param control and the value map.
    pub key: &'static str,
    /// English fallback label.
    pub label: &'static str,
    /// i18n key for a localized label in Studio (built-ins); empty for
    /// out-of-tree generators (the host then shows `label`).
    pub i18n_key: &'static str,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub default: f32,
    /// Display unit suffix (e.g. `"Hz"`, `"dB"`); empty for a bare number.
    pub unit: &'static str,
}

/// One synthesized object the generator emits. Position/name are static for a
/// given layout+input (planned in [`ObjectGenerator::prepare`]); only the audio
/// content varies per frame.
#[derive(Debug, Clone)]
pub struct SynthObjectSpec {
    /// Display name, surfaced over OSC to Studio (3D view + object list).
    pub name: String,
    /// Target position in ADM Cartesian coordinates `[x, y, z]`, `z > 0`
    /// (height): x ∈ [-1, 1] left/right · y ∈ [-1, 1] back/front · z floor/ceiling.
    pub position: [f64; 3],
    /// Object gain in dB (`-128` = muted).
    pub gain_db: i8,
    /// Object spatial extent `(w, d, h)` ∈ [0, 1]³.
    pub size: [f32; 3],
}

/// Read-only environment for [`ObjectGenerator::prepare`].
pub struct PrepareCtx<'a> {
    /// Input channel labels, in PCM channel order.
    pub input_labels: &'a [RChannelLabel],
    /// The active output speaker layout.
    pub output_layout: &'a SpeakerLayout,
    pub sample_rate: u32,
}

/// One block of channel-based bed PCM handed to [`ObjectGenerator::process`].
pub struct BedFrame<'a> {
    /// Interleaved input PCM, `channel_count` channels.
    pub pcm: &'a [f32],
    pub channel_count: usize,
    pub sample_count: usize,
    pub sample_rate: u32,
}

/// A compiled bed→height object generator. Stateful, runs in the realtime audio
/// thread. Like a render backend it MUST NOT panic or allocate in
/// [`process`](ObjectGenerator::process); do all setup in
/// [`prepare`](ObjectGenerator::prepare).
pub trait ObjectGenerator: Send {
    fn capabilities(&self) -> ObjectGenCapabilities;
    /// Plan the (static) objects this generator emits for the current
    /// layout + input and (re)allocate internal state. Returns an empty vector
    /// when the generator is a no-op (e.g. no height layer, or the input already
    /// has height). Called once per layout/format change, not per frame.
    fn prepare(&mut self, ctx: &PrepareCtx) -> Vec<SynthObjectSpec>;
    /// Fill `out[k]` (length `bed.sample_count`) with the audio for the k-th
    /// planned object. `out.len()` equals the number of specs returned by the
    /// last `prepare`. Must not allocate or panic.
    fn process(&mut self, bed: &BedFrame, out: &mut [Vec<f32>]);

    /// The live-tunable parameters this generator declares (default: none). The
    /// host builds a control per entry and validates incoming values against it.
    fn param_schema(&self) -> &'static [ObjectGenParamSpec] {
        &[]
    }

    /// Apply one parameter by `key`, in place (no DSP-state reset). Unknown keys
    /// are ignored. Default: no params.
    fn set_param(&mut self, _key: &str, _value: f32, _sample_rate: u32) {}
}

/// Factory for an [`ObjectGenerator`], keyed by a stable string id (mirrors the
/// render-backend `BackendFactory`).
pub trait ObjectGeneratorFactory: Send + Sync {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    fn requires_height_layer(&self) -> bool;
    fn build(&self) -> Box<dyn ObjectGenerator>;
    /// i18n key for a localized name in Studio (built-ins); empty otherwise.
    fn i18n_key(&self) -> &'static str {
        ""
    }
    /// The parameter schema this generator exposes (default: none). Available
    /// without building an instance, so the host can publish it to the UI.
    fn param_schema(&self) -> &'static [ObjectGenParamSpec] {
        &[]
    }
}

/// String-keyed registry of generator factories: the shipped built-ins plus any
/// contributor-registered factory.
pub struct ObjectGeneratorRegistry {
    factories: Vec<Box<dyn ObjectGeneratorFactory>>,
}

impl ObjectGeneratorRegistry {
    /// Registry with the shipped built-in generators.
    pub fn with_builtins() -> Self {
        Self {
            factories: vec![Box::new(CopyUpFactory), Box::new(PadFactory)],
        }
    }

    /// Register a contributor factory (out-of-tree generators).
    pub fn register(&mut self, factory: Box<dyn ObjectGeneratorFactory>) {
        self.factories.push(factory);
    }

    /// Build the generator for `id`, or `None` for the off sentinel
    /// (`""` / `"none"`) or an unknown id.
    pub fn build(&self, id: &str) -> Option<Box<dyn ObjectGenerator>> {
        let id = id.trim();
        if id.is_empty() || id.eq_ignore_ascii_case("none") {
            return None;
        }
        self.factories
            .iter()
            .find(|f| f.id().eq_ignore_ascii_case(id))
            .map(|f| f.build())
    }

    /// `(id, label, requires_height_layer)` for each registered generator, for
    /// state publication to the UI.
    pub fn list(&self) -> Vec<(&'static str, &'static str, bool)> {
        self.factories
            .iter()
            .map(|f| (f.id(), f.label(), f.requires_height_layer()))
            .collect()
    }

    /// JSON array of `{id,label,i18nKey,requiresHeightLayer,params:[…]}` for each
    /// registered generator, for the Studio UI to build the selector + the
    /// per-generator parameter sliders.
    pub fn listings_json(&self) -> String {
        let arr: Vec<serde_json::Value> = self
            .factories
            .iter()
            .map(|f| {
                let params: Vec<serde_json::Value> = f
                    .param_schema()
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
                serde_json::json!({
                    "id": f.id(),
                    "label": f.label(),
                    "i18nKey": f.i18n_key(),
                    "requiresHeightLayer": f.requires_height_layer(),
                    "params": params,
                })
            })
            .collect();
        serde_json::to_string(&arr).unwrap_or_else(|_| "[]".to_string())
    }
}

/// JSON listing of the built-in generators (id/label/schema), for the host to
/// publish to Studio over OSC without owning a registry instance.
pub fn builtin_listings_json() -> String {
    ObjectGeneratorRegistry::with_builtins().listings_json()
}

impl Default for ObjectGeneratorRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

// ─────────────────────────────── helpers ───────────────────────────────

const Z_EPS: f32 = 1e-3;

/// True when `layout` has at least one spatializable speaker above ear level.
pub fn layout_has_height(layout: &SpeakerLayout) -> bool {
    layout.speakers.iter().any(|s| s.spatialize && s.z > Z_EPS)
}

/// True when the input channel set already carries a height channel — then any
/// upmix is suppressed (the content is already 3D).
pub fn input_has_height(labels: &[RChannelLabel]) -> bool {
    labels.iter().any(|l| {
        matches!(
            l,
            RChannelLabel::Tfl
                | RChannelLabel::Tfr
                | RChannelLabel::Tsl
                | RChannelLabel::Tsr
                | RChannelLabel::Tbl
                | RChannelLabel::Tbr
                | RChannelLabel::Tc
                | RChannelLabel::Tfc
        )
    })
}

fn find_channel(labels: &[RChannelLabel], want: RChannelLabel) -> Option<usize> {
    labels.iter().position(|&l| l == want)
}

// ───────────────────────── built-in: copy_up ─────────────────────────

/// Simple reference generator: routes the floor front/surround pairs straight up
/// to the four top corners (no decorrelation). A basic, low-cost "lift" — also
/// useful as the minimal example for contributors. The PAD generator below adds
/// ambience extraction for a more natural result.
struct CopyUpFactory;

impl ObjectGeneratorFactory for CopyUpFactory {
    fn id(&self) -> &'static str {
        "copy_up"
    }
    fn label(&self) -> &'static str {
        "Direct copy to tops"
    }
    fn requires_height_layer(&self) -> bool {
        true
    }
    fn build(&self) -> Box<dyn ObjectGenerator> {
        Box::<CopyUpGenerator>::default()
    }
    fn i18n_key(&self) -> &'static str {
        "twoDSources.objectGenCopyUp"
    }
}

#[derive(Default)]
struct CopyUpGenerator {
    /// Source input channel index for each planned object (parallel to the specs
    /// returned by `prepare`).
    src_channels: Vec<usize>,
}

impl ObjectGenerator for CopyUpGenerator {
    fn capabilities(&self) -> ObjectGenCapabilities {
        ObjectGenCapabilities {
            requires_height_layer: true,
        }
    }

    fn prepare(&mut self, ctx: &PrepareCtx) -> Vec<SynthObjectSpec> {
        self.src_channels.clear();
        if !layout_has_height(ctx.output_layout) || input_has_height(ctx.input_labels) {
            return Vec::new();
        }
        let labels = ctx.input_labels;
        // Rear source prefers dedicated back channels (7.1), else the surrounds.
        let candidates: [(&str, [f64; 3], Option<usize>); 4] = [
            ("Height_FL_synth", [-1.0, 1.0, 1.0], find_channel(labels, RChannelLabel::L)),
            ("Height_FR_synth", [1.0, 1.0, 1.0], find_channel(labels, RChannelLabel::R)),
            (
                "Height_BL_synth",
                [-1.0, -1.0, 1.0],
                find_channel(labels, RChannelLabel::Lb).or_else(|| find_channel(labels, RChannelLabel::Ls)),
            ),
            (
                "Height_BR_synth",
                [1.0, -1.0, 1.0],
                find_channel(labels, RChannelLabel::Rb).or_else(|| find_channel(labels, RChannelLabel::Rs)),
            ),
        ];
        const GAIN_DB: i8 = -6;
        const SIZE: [f32; 3] = [0.3, 0.3, 0.3];
        let mut specs = Vec::new();
        for (name, position, src) in candidates {
            if let Some(ch) = src {
                self.src_channels.push(ch);
                specs.push(SynthObjectSpec {
                    name: name.to_string(),
                    position,
                    gain_db: GAIN_DB,
                    size: SIZE,
                });
            }
        }
        specs
    }

    fn process(&mut self, bed: &BedFrame, out: &mut [Vec<f32>]) {
        let c = bed.channel_count;
        for (k, &src) in self.src_channels.iter().enumerate() {
            if k >= out.len() || src >= c {
                continue;
            }
            let dst = &mut out[k];
            for s in 0..bed.sample_count {
                dst[s] = bed.pcm[s * c + src];
            }
        }
    }
}

// ─────────────────────────── built-in: pad ───────────────────────────

/// Minimal transposed-direct-form-II biquad, used by PAD to keep low frequencies
/// out of the height layer (a mild psychoacoustic "elevation" lean: bass stays
/// grounded, mids/highs rise).
#[derive(Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    /// RBJ high-pass at cutoff `fc` (Hz), quality `q`, sample rate `fs` (Hz).
    fn highpass(fs: f32, fc: f32, q: f32) -> Self {
        let mut b = Self {
            b0: 0.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        };
        b.set_highpass(fs, fc, q);
        b
    }

    /// Recompute the high-pass coefficients in place, preserving the filter state
    /// (`z1`/`z2`) so a live cutoff change does not click.
    fn set_highpass(&mut self, fs: f32, fc: f32, q: f32) {
        let w0 = std::f32::consts::TAU * (fc / fs).clamp(1.0e-4, 0.49);
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a0 = 1.0 + alpha;
        self.b0 = ((1.0 + cos) / 2.0) / a0;
        self.b1 = (-(1.0 + cos)) / a0;
        self.b2 = ((1.0 + cos) / 2.0) / a0;
        self.a1 = (-2.0 * cos) / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    #[inline]
    fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.z1;
        self.z1 = self.b1 * x - self.a1 * y + self.z2;
        self.z2 = self.b2 * x - self.a2 * y;
        y
    }
}

/// Primary-Ambient Decomposition generator: extracts the decorrelated *ambient*
/// component of each floor pair (front L/R, surround L/R) with a 1-tap adaptive
/// canceller (NLMS) and lifts it to the matching top corners. Correlated /
/// panned "primary" content (dialogue, hard-panned sources) is cancelled and
/// stays grounded; diffuse / reverberant content rises. Additive: the floor mix
/// is left untouched.
struct PadFactory;

impl ObjectGeneratorFactory for PadFactory {
    fn id(&self) -> &'static str {
        "pad"
    }
    fn label(&self) -> &'static str {
        "Ambience to height (PAD)"
    }
    fn requires_height_layer(&self) -> bool {
        true
    }
    fn build(&self) -> Box<dyn ObjectGenerator> {
        Box::<PadGenerator>::default()
    }
    fn i18n_key(&self) -> &'static str {
        "twoDSources.objectGenPad"
    }
    fn param_schema(&self) -> &'static [ObjectGenParamSpec] {
        &PAD_PARAM_SPECS
    }
}

/// High-pass cutoff for the height layer (Hz): keep bass grounded.
const PAD_HPF_HZ: f32 = 300.0;
/// High-pass quality (Butterworth).
const PAD_HPF_Q: f32 = std::f32::consts::FRAC_1_SQRT_2;
/// Default isolation depth (0..1): how much of the predicted primary is removed.
const PAD_DEFAULT_STRENGTH: f32 = 0.5;
/// Time constant (ms) of the one-pole smoothers for the inter-channel statistics
/// (auto/cross power). Long enough (tens of ms) that the derived Wiener weight is
/// quasi-stationary, so the residual gain does not jitter per sample — this is
/// what avoids the amplitude-modulation ("wind / mic saturation") artifact.
const PAD_STAT_TC_MS: f32 = 50.0;
/// Relative regularisation on the Wiener denominator (fraction of the pair's mean
/// power) so quiet passages and zero-crossings cannot blow the weight up.
const PAD_REG: f32 = 1.0e-2;
/// Absolute floor added to the denominator to keep it strictly positive in pure
/// silence (no NaN reaches the audio thread).
const PAD_FLOOR: f32 = 1.0e-9;
/// Clamp on the prediction weight: a correlated stereo weight is ≤ 1; the small
/// headroom prevents the residual from amplifying the partner channel.
const PAD_W_CLAMP: f32 = 1.2;
/// Object gain for the lifted ambience (dB). The ambient component is already
/// lower energy than the primary, so unity keeps it natural.
const PAD_GAIN_DB: i8 = 0;

/// Live-tunable parameters PAD declares (the schema the UI builds sliders from).
const PAD_PARAM_SPECS: [ObjectGenParamSpec; 3] = [
    ObjectGenParamSpec {
        key: "strength",
        label: "Ambience strength",
        i18n_key: "twoDSources.padStrength",
        min: 0.0,
        max: 1.0,
        step: 0.01,
        default: PAD_DEFAULT_STRENGTH,
        unit: "",
    },
    ObjectGenParamSpec {
        key: "hpf_hz",
        label: "Bass cutoff",
        i18n_key: "twoDSources.padHpf",
        min: 20.0,
        max: 2000.0,
        step: 10.0,
        default: PAD_HPF_HZ,
        unit: "Hz",
    },
    ObjectGenParamSpec {
        key: "gain_db",
        label: "Height level",
        i18n_key: "twoDSources.padGain",
        min: -24.0,
        max: 24.0,
        step: 0.5,
        default: 0.0,
        unit: "dB",
    },
];

/// One floor pair (L, R) → two height objects (ambient of L, ambient of R), with
/// the slowly-smoothed inter-channel statistics and high-pass state that must
/// persist across frames.
struct AmbientPair {
    l_ch: usize,
    r_ch: usize,
    out_l: usize,
    out_r: usize,
    /// One-pole smoothed auto-power of each channel (`E[L²]`, `E[R²]`).
    pow_l: f32,
    pow_r: f32,
    /// One-pole smoothed inter-channel cross-power (`E[L·R]`). Symmetric, so it
    /// feeds both prediction weights.
    cross: f32,
    hpf_l: Biquad,
    hpf_r: Biquad,
}

struct PadGenerator {
    pairs: Vec<AmbientPair>,
    /// Isolation depth (0..1): fraction of the predicted primary that is removed
    /// from each channel. `0` = clean copy of the floor up top, `1` = pure diffuse
    /// residual. Live-tunable via the `strength` param.
    strength: f32,
    /// Linear makeup gain applied to the lifted ambience.
    makeup: f32,
    /// One-pole smoothing coefficient for the statistics, derived from the sample
    /// rate in `prepare` (`α = 1 − exp(−1/(τ·fs))`).
    alpha: f32,
}

impl Default for PadGenerator {
    fn default() -> Self {
        Self {
            pairs: Vec::new(),
            strength: PAD_DEFAULT_STRENGTH,
            makeup: 1.0,
            // Replaced in `prepare` once the sample rate is known; a safe non-zero
            // default keeps the smoother stable if `process` ever runs first.
            alpha: 0.0,
        }
    }
}

impl ObjectGenerator for PadGenerator {
    fn capabilities(&self) -> ObjectGenCapabilities {
        ObjectGenCapabilities {
            requires_height_layer: true,
        }
    }

    fn prepare(&mut self, ctx: &PrepareCtx) -> Vec<SynthObjectSpec> {
        self.pairs.clear();
        if !layout_has_height(ctx.output_layout) || input_has_height(ctx.input_labels) {
            return Vec::new();
        }
        let fs = ctx.sample_rate.max(1) as f32;
        // One-pole smoothing coefficient for the statistics: α = 1 − exp(−1/(τ·fs)),
        // with τ = PAD_STAT_TC_MS. (Idiom shared with audio_output `step_one_pole`.)
        let tau_samples = (PAD_STAT_TC_MS * 1.0e-3 * fs).max(1.0);
        self.alpha = 1.0 - (-1.0 / tau_samples).exp();
        let labels = ctx.input_labels;
        let hpf = Biquad::highpass(fs, PAD_HPF_HZ, PAD_HPF_Q);
        const SIZE: [f32; 3] = [0.5, 0.5, 0.5];

        // Front pair = L/R; surround pair = (Ls|Lb)/(Rs|Rb).
        let front = (
            find_channel(labels, RChannelLabel::L),
            find_channel(labels, RChannelLabel::R),
        );
        let back = (
            find_channel(labels, RChannelLabel::Ls).or_else(|| find_channel(labels, RChannelLabel::Lb)),
            find_channel(labels, RChannelLabel::Rs).or_else(|| find_channel(labels, RChannelLabel::Rb)),
        );
        let defs: [(&str, &str, [f64; 3], [f64; 3], Option<usize>, Option<usize>); 2] = [
            ("Ambience_FL", "Ambience_FR", [-1.0, 1.0, 1.0], [1.0, 1.0, 1.0], front.0, front.1),
            ("Ambience_BL", "Ambience_BR", [-1.0, -1.0, 1.0], [1.0, -1.0, 1.0], back.0, back.1),
        ];

        let mut specs = Vec::new();
        for (name_l, name_r, pos_l, pos_r, l_ch, r_ch) in defs {
            let (Some(l_ch), Some(r_ch)) = (l_ch, r_ch) else {
                continue;
            };
            let out_l = specs.len();
            let out_r = out_l + 1;
            self.pairs.push(AmbientPair {
                l_ch,
                r_ch,
                out_l,
                out_r,
                pow_l: 0.0,
                pow_r: 0.0,
                cross: 0.0,
                hpf_l: hpf,
                hpf_r: hpf,
            });
            specs.push(SynthObjectSpec {
                name: name_l.to_string(),
                position: pos_l,
                gain_db: PAD_GAIN_DB,
                size: SIZE,
            });
            specs.push(SynthObjectSpec {
                name: name_r.to_string(),
                position: pos_r,
                gain_db: PAD_GAIN_DB,
                size: SIZE,
            });
        }
        specs
    }

    fn process(&mut self, bed: &BedFrame, out: &mut [Vec<f32>]) {
        let c = bed.channel_count;
        let strength = self.strength;
        let makeup = self.makeup;
        let alpha = self.alpha;
        for pair in self.pairs.iter_mut() {
            if pair.l_ch >= c || pair.r_ch >= c || pair.out_l >= out.len() || pair.out_r >= out.len()
            {
                continue;
            }
            for s in 0..bed.sample_count {
                let l = bed.pcm[s * c + pair.l_ch];
                let r = bed.pcm[s * c + pair.r_ch];

                // Slowly-smoothed inter-channel statistics (~PAD_STAT_TC_MS). Long
                // enough that the Wiener weight below is quasi-stationary, so the
                // residual gain does not jitter per sample (no AM / "wind").
                pair.pow_l += alpha * (l * l - pair.pow_l);
                pair.pow_r += alpha * (r * r - pair.pow_r);
                pair.cross += alpha * (l * r - pair.cross);

                // Relative regularisation: a fraction of the pair's mean power plus
                // a tiny absolute floor — quiet passages and zero-crossings can no
                // longer explode the weight.
                let reg = PAD_REG * 0.5 * (pair.pow_l + pair.pow_r) + PAD_FLOOR;
                let w_lr = (pair.cross / (pair.pow_r + reg)).clamp(-PAD_W_CLAMP, PAD_W_CLAMP);
                let w_rl = (pair.cross / (pair.pow_l + reg)).clamp(-PAD_W_CLAMP, PAD_W_CLAMP);

                // Ambient = each channel minus the `strength`-scaled part that is
                // predictable from its partner. strength=0 → clean copy up;
                // strength=1 → only the decorrelated (diffuse) residual rises.
                let amb_l = l - strength * w_lr * r;
                let amb_r = r - strength * w_rl * l;

                out[pair.out_l][s] = pair.hpf_l.process(amb_l) * makeup;
                out[pair.out_r][s] = pair.hpf_r.process(amb_r) * makeup;
            }
        }
    }

    fn param_schema(&self) -> &'static [ObjectGenParamSpec] {
        &PAD_PARAM_SPECS
    }

    fn set_param(&mut self, key: &str, value: f32, sample_rate: u32) {
        match key {
            "strength" => self.strength = value.clamp(0.0, 1.0),
            "gain_db" => self.makeup = 10.0_f32.powf(value.clamp(-24.0, 24.0) / 20.0),
            "hpf_hz" => {
                let fs = sample_rate.max(1) as f32;
                let fc = value.clamp(20.0, 2000.0);
                for pair in self.pairs.iter_mut() {
                    pair.hpf_l.set_highpass(fs, fc, PAD_HPF_Q);
                    pair.hpf_r.set_highpass(fs, fc, PAD_HPF_Q);
                }
            }
            _ => {}
        }
    }
}

// ─────────────────────────── host integration ───────────────────────────

/// Signature of the inputs the current plan was built for; a change triggers a
/// rebuild + re-plan. Compared field-by-field without per-frame allocation.
#[derive(Default)]
struct PlanSig {
    id: String,
    out_n: usize,
    out_height: bool,
    labels: Vec<RChannelLabel>,
    rate: u32,
}

/// Host-side state for the object-generator stage: the registry, the active
/// generator instance, its current plan, and reusable per-frame buffers.
pub struct ObjectGenStage {
    registry: ObjectGeneratorRegistry,
    generator: Option<Box<dyn ObjectGenerator>>,
    specs: Vec<SynthObjectSpec>,
    /// Per-object planar audio scratch (persistent; one Vec per planned object).
    planar: Vec<Vec<f32>>,
    /// Extended interleaved PCM (bed channels + synthesized object channels).
    pcm_ext: Vec<f32>,
    sig: PlanSig,
}

impl ObjectGenStage {
    pub fn new() -> Self {
        Self {
            registry: ObjectGeneratorRegistry::with_builtins(),
            generator: None,
            specs: Vec::new(),
            planar: Vec::new(),
            pcm_ext: Vec::new(),
            sig: PlanSig::default(),
        }
    }

    pub fn registry(&self) -> &ObjectGeneratorRegistry {
        &self.registry
    }

    /// Register a host-supplied (out-of-tree) generator factory.
    pub fn register(&mut self, factory: Box<dyn ObjectGeneratorFactory>) {
        self.registry.register(factory);
    }

    pub fn specs(&self) -> &[SynthObjectSpec] {
        &self.specs
    }

    /// (Re)build + plan if the selection or environment changed; returns the
    /// number of synthesized objects (`0` = inactive / no-op).
    pub fn sync(&mut self, desired_id: &str, ctx: &PrepareCtx) -> usize {
        let did = desired_id.trim();
        let out_n = ctx.output_layout.speakers.len();
        let out_height = layout_has_height(ctx.output_layout);
        let unchanged = self.sig.id == did
            && self.sig.out_n == out_n
            && self.sig.out_height == out_height
            && self.sig.rate == ctx.sample_rate
            && self.sig.labels.as_slice() == ctx.input_labels;
        if !unchanged {
            self.sig.id.clear();
            self.sig.id.push_str(did);
            self.sig.out_n = out_n;
            self.sig.out_height = out_height;
            self.sig.rate = ctx.sample_rate;
            self.sig.labels.clear();
            self.sig.labels.extend_from_slice(ctx.input_labels);

            self.generator = self.registry.build(did);
            self.specs = match self.generator.as_mut() {
                Some(g) => g.prepare(ctx),
                None => Vec::new(),
            };
            self.planar.truncate(self.specs.len());
            self.planar.resize_with(self.specs.len(), Vec::new);
        }
        self.specs.len()
    }

    /// Apply one live-tunable parameter to the active generator (in place, no DSP
    /// reset). Cheap and idempotent — the engine pushes the active generator's
    /// params each frame, so a fresh generator (after a rebuild) re-receives them.
    pub fn set_param(&mut self, key: &str, value: f32, sample_rate: u32) {
        if let Some(g) = self.generator.as_mut() {
            g.set_param(key, value, sample_rate);
        }
    }

    /// Run the per-frame DSP and return the bed PCM extended with the
    /// synthesized object channels, plus the new channel count. Call only when
    /// [`sync`](Self::sync) returned `> 0`.
    pub fn fill_and_extend(
        &mut self,
        bed_pcm: &[f32],
        channel_count: usize,
        sample_count: usize,
        sample_rate: u32,
    ) -> (&[f32], usize) {
        let m = self.specs.len();
        for buf in self.planar.iter_mut() {
            buf.clear();
            buf.resize(sample_count, 0.0);
        }
        if let Some(generator) = self.generator.as_mut() {
            let bed = BedFrame {
                pcm: bed_pcm,
                channel_count,
                sample_count,
                sample_rate,
            };
            generator.process(&bed, &mut self.planar);
        }
        let out_ch = channel_count + m;
        self.pcm_ext.clear();
        self.pcm_ext.resize(sample_count * out_ch, 0.0);
        for s in 0..sample_count {
            let src = &bed_pcm[s * channel_count..s * channel_count + channel_count];
            let dst = &mut self.pcm_ext[s * out_ch..s * out_ch + out_ch];
            dst[..channel_count].copy_from_slice(src);
            for (k, buf) in self.planar.iter().enumerate().take(m) {
                dst[channel_count + k] = buf[s];
            }
        }
        (&self.pcm_ext, out_ch)
    }
}

impl Default for ObjectGenStage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use renderer::speaker_layout::Speaker;

    fn layout(speakers: &[(&str, f32, f32, f32)]) -> SpeakerLayout {
        SpeakerLayout {
            radius_m: 1.0,
            speakers: speakers
                .iter()
                .map(|&(name, x, y, z)| Speaker::from_cartesian(name, x, y, z, true, 0.0))
                .collect(),
        }
    }

    fn flat_5_1() -> Vec<(&'static str, f32, f32, f32)> {
        vec![
            ("FL", -1.0, 1.0, 0.0),
            ("FR", 1.0, 1.0, 0.0),
            ("C", 0.0, 1.0, 0.0),
            ("LFE", 0.0, 1.0, 0.0),
            ("Ls", -1.0, -1.0, 0.0),
            ("Rs", 1.0, -1.0, 0.0),
        ]
    }

    fn layout_7_1_4() -> SpeakerLayout {
        let mut s = flat_5_1();
        s.extend_from_slice(&[
            ("TFL", -1.0, 1.0, 1.0),
            ("TFR", 1.0, 1.0, 1.0),
            ("TBL", -1.0, -1.0, 1.0),
            ("TBR", 1.0, -1.0, 1.0),
        ]);
        layout(&s)
    }

    const LABELS_5_1: [RChannelLabel; 6] = [
        RChannelLabel::L,
        RChannelLabel::R,
        RChannelLabel::C,
        RChannelLabel::LFE,
        RChannelLabel::Ls,
        RChannelLabel::Rs,
    ];

    #[test]
    fn no_op_without_height_layer() {
        let mut g = CopyUpGenerator::default();
        let out = layout(&flat_5_1());
        let specs = g.prepare(&PrepareCtx {
            input_labels: &LABELS_5_1,
            output_layout: &out,
            sample_rate: 48_000,
        });
        assert!(specs.is_empty());
    }

    #[test]
    fn no_op_when_input_already_has_height() {
        let mut g = CopyUpGenerator::default();
        let out = layout_7_1_4();
        let labels = [
            RChannelLabel::L,
            RChannelLabel::R,
            RChannelLabel::Tfl,
            RChannelLabel::Tfr,
        ];
        let specs = g.prepare(&PrepareCtx {
            input_labels: &labels,
            output_layout: &out,
            sample_rate: 48_000,
        });
        assert!(specs.is_empty());
    }

    #[test]
    fn plans_four_top_objects_for_5_1_on_7_1_4() {
        let mut g = CopyUpGenerator::default();
        let out = layout_7_1_4();
        let specs = g.prepare(&PrepareCtx {
            input_labels: &LABELS_5_1,
            output_layout: &out,
            sample_rate: 48_000,
        });
        assert_eq!(specs.len(), 4);
        assert!(specs.iter().all(|s| s.position[2] > 0.0), "all objects above ear level");
    }

    #[test]
    fn process_copies_source_channels_up() {
        let mut g = CopyUpGenerator::default();
        let out = layout_7_1_4();
        let _ = g.prepare(&PrepareCtx {
            input_labels: &LABELS_5_1,
            output_layout: &out,
            sample_rate: 48_000,
        });
        // 2 sample frames, 6 channels: channel value = channel index + sample*10.
        let c = 6usize;
        let n = 2usize;
        let mut pcm = vec![0.0f32; c * n];
        for s in 0..n {
            for ch in 0..c {
                pcm[s * c + ch] = ch as f32 + (s * 10) as f32;
            }
        }
        let mut outbuf: Vec<Vec<f32>> = vec![vec![0.0; n]; 4];
        g.process(
            &BedFrame {
                pcm: &pcm,
                channel_count: c,
                sample_count: n,
                sample_rate: 48_000,
            },
            &mut outbuf,
        );
        // Object 0 (Height_FL) ← channel L (index 0); object 2 (Height_BL) ← Ls (index 4).
        assert_eq!(outbuf[0], vec![0.0, 10.0]);
        assert_eq!(outbuf[2], vec![4.0, 14.0]);
    }

    // ── PAD ──

    /// Run PAD at full isolation depth over `n` samples with the given L/R source
    /// signals (5.1 → 7.1.4) and return the energy of the front-left ambience
    /// object over the converged tail.
    fn pad_front_ambient_energy(
        l: impl Fn(usize) -> f32,
        r: impl Fn(usize) -> f32,
        n: usize,
    ) -> f32 {
        let mut g = PadGenerator::default();
        let out_layout = layout_7_1_4();
        let specs = g.prepare(&PrepareCtx {
            input_labels: &LABELS_5_1,
            output_layout: &out_layout,
            sample_rate: 48_000,
        });
        assert_eq!(specs.len(), 4);
        // Full cancellation so the correlated/decorrelated contrast is maximal.
        g.set_param("strength", 1.0, 48_000);
        let c = 6usize;
        let mut pcm = vec![0.0f32; c * n];
        for s in 0..n {
            pcm[s * c] = l(s); // L (channel 0)
            pcm[s * c + 1] = r(s); // R (channel 1)
        }
        let mut outbuf: Vec<Vec<f32>> = vec![vec![0.0; n]; 4];
        g.process(
            &BedFrame {
                pcm: &pcm,
                channel_count: c,
                sample_count: n,
                sample_rate: 48_000,
            },
            &mut outbuf,
        );
        outbuf[0][n / 2..].iter().map(|&x| x * x).sum()
    }

    #[test]
    fn pad_no_op_without_height_layer() {
        let mut g = PadGenerator::default();
        let out = layout(&flat_5_1());
        assert!(
            g.prepare(&PrepareCtx {
                input_labels: &LABELS_5_1,
                output_layout: &out,
                sample_rate: 48_000,
            })
            .is_empty()
        );
    }

    #[test]
    fn pad_plans_four_objects_for_5_1_on_7_1_4() {
        let mut g = PadGenerator::default();
        let out = layout_7_1_4();
        let specs = g.prepare(&PrepareCtx {
            input_labels: &LABELS_5_1,
            output_layout: &out,
            sample_rate: 48_000,
        });
        assert_eq!(specs.len(), 4);
        assert!(specs.iter().all(|s| s.position[2] > 0.0));
    }

    #[test]
    fn pad_cancels_correlated_keeps_decorrelated() {
        let sine =
            |f: f32| move |s: usize| (std::f32::consts::TAU * f * s as f32 / 48_000.0).sin() * 0.5;
        let n = 4000;
        // Fully correlated (L == R): the primary is cancelled → little ambience.
        let correlated = pad_front_ambient_energy(sine(1000.0), sine(1000.0), n);
        // Decorrelated (different tones): nothing to cancel → ambience retained.
        let decorrelated = pad_front_ambient_energy(sine(1000.0), sine(1310.0), n);
        assert!(
            correlated.is_finite() && decorrelated.is_finite(),
            "outputs must be finite (no NaN reaches the audio thread)"
        );
        assert!(
            correlated < 0.25 * decorrelated,
            "correlated ambience {correlated} should be << decorrelated {decorrelated}"
        );
    }

    #[test]
    fn pad_output_is_stable_and_bounded() {
        // A steady, fully-correlated tone (L == R) at full strength. The old
        // per-sample NLMS produced gusty bursts near zero-crossings ("wind / mic
        // saturation"); the smoothed-Wiener residual must instead stay small,
        // bounded, and smooth.
        let amp = 0.5f32;
        let freq = 700.0f32;
        let n = 8000usize;
        let c = 6usize;
        let mut pcm = vec![0.0f32; c * n];
        for s in 0..n {
            let v = (std::f32::consts::TAU * freq * s as f32 / 48_000.0).sin() * amp;
            pcm[s * c] = v; // L
            pcm[s * c + 1] = v; // R
        }
        let mut g = PadGenerator::default();
        let out_layout = layout_7_1_4();
        let _ = g.prepare(&PrepareCtx {
            input_labels: &LABELS_5_1,
            output_layout: &out_layout,
            sample_rate: 48_000,
        });
        g.set_param("strength", 1.0, 48_000);
        let mut out: Vec<Vec<f32>> = vec![vec![0.0; n]; 4];
        g.process(
            &BedFrame {
                pcm: &pcm,
                channel_count: c,
                sample_count: n,
                sample_rate: 48_000,
            },
            &mut out,
        );
        let tail = &out[0][n / 2..];
        assert!(tail.iter().all(|x| x.is_finite()), "no NaN/Inf reaches output");
        let peak = tail.iter().fold(0.0f32, |m, &x| m.max(x.abs()));
        assert!(
            peak < 0.1 * amp,
            "correlated residual peak {peak} should be ≪ input {amp}"
        );
        let max_step = tail
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(
            max_step < 0.02,
            "output must be smooth (no per-sample gain jumps); max step was {max_step}"
        );
    }

    #[test]
    fn pad_finite_across_strength_sweep() {
        // Mixed L/R (a shared/correlated component plus independent tones) at every
        // strength: outputs must stay finite and bounded — no saturation at any
        // setting.
        let n = 6000usize;
        let c = 6usize;
        let mut pcm = vec![0.0f32; c * n];
        let mut peak_in = 0.0f32;
        for s in 0..n {
            let t = s as f32 / 48_000.0;
            let common = (std::f32::consts::TAU * 500.0 * t).sin() * 0.4;
            let l = common + (std::f32::consts::TAU * 900.0 * t).sin() * 0.2;
            let r = common + (std::f32::consts::TAU * 1100.0 * t).sin() * 0.2;
            pcm[s * c] = l;
            pcm[s * c + 1] = r;
            peak_in = peak_in.max(l.abs()).max(r.abs());
        }
        let out_layout = layout_7_1_4();
        for &strength in &[0.0f32, 0.25, 0.5, 0.75, 1.0] {
            let mut g = PadGenerator::default();
            let _ = g.prepare(&PrepareCtx {
                input_labels: &LABELS_5_1,
                output_layout: &out_layout,
                sample_rate: 48_000,
            });
            g.set_param("strength", strength, 48_000);
            let mut out: Vec<Vec<f32>> = vec![vec![0.0; n]; 4];
            g.process(
                &BedFrame {
                    pcm: &pcm,
                    channel_count: c,
                    sample_count: n,
                    sample_rate: 48_000,
                },
                &mut out,
            );
            for (ch, buf) in out.iter().enumerate() {
                assert!(
                    buf.iter().all(|x| x.is_finite()),
                    "finite output at strength {strength} (ch {ch})"
                );
                let peak = buf[n / 2..].iter().fold(0.0f32, |m, &x| m.max(x.abs()));
                assert!(
                    peak <= 2.5 * peak_in,
                    "strength {strength} ch {ch}: peak {peak} must stay bounded (≤ 2.5×{peak_in})"
                );
            }
        }
    }

    #[test]
    fn pad_declares_three_params() {
        let g = PadGenerator::default();
        let keys: Vec<&str> = g.param_schema().iter().map(|p| p.key).collect();
        assert_eq!(g.param_schema().len(), 3);
        assert!(
            keys.contains(&"strength") && keys.contains(&"hpf_hz") && keys.contains(&"gain_db")
        );
    }

    #[test]
    fn builtin_listings_json_includes_declared_schema() {
        let json = builtin_listings_json();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(v.is_array());
        assert!(json.contains("copy_up") && json.contains("\"pad\""));
        for key in ["strength", "hpf_hz", "gain_db"] {
            assert!(json.contains(key), "schema should declare {key}");
        }
    }

    #[test]
    fn set_param_gain_scales_height_output() {
        let out_layout = layout_7_1_4();
        let n = 1500usize;
        let c = 6usize;
        let mut pcm = vec![0.0f32; c * n];
        for s in 0..n {
            pcm[s * c] = (s as f32 * 0.11).sin() * 0.5;
            pcm[s * c + 1] = (s as f32 * 0.07).sin() * 0.5;
        }
        let run = |gain_db: Option<f32>| {
            let mut g = PadGenerator::default();
            let _ = g.prepare(&PrepareCtx {
                input_labels: &LABELS_5_1,
                output_layout: &out_layout,
                sample_rate: 48_000,
            });
            if let Some(db) = gain_db {
                g.set_param("gain_db", db, 48_000);
            }
            let mut out: Vec<Vec<f32>> = vec![vec![0.0; n]; 4];
            g.process(
                &BedFrame {
                    pcm: &pcm,
                    channel_count: c,
                    sample_count: n,
                    sample_rate: 48_000,
                },
                &mut out,
            );
            out[0][n / 2..].iter().map(|&x| x * x).sum::<f32>()
        };
        let unity = run(None);
        let attenuated = run(Some(-24.0));
        assert!(
            attenuated < 0.05 * unity,
            "gain_db=-24 should attenuate the height output ({attenuated} vs {unity})"
        );
    }

    // A minimal out-of-tree generator, to prove host registration surfaces it in
    // both the published schema and `build`.
    static DUMMY_PARAMS: [ObjectGenParamSpec; 1] = [ObjectGenParamSpec {
        key: "amount",
        label: "Amount",
        i18n_key: "",
        min: 0.0,
        max: 1.0,
        step: 0.1,
        default: 0.5,
        unit: "",
    }];

    struct DummyGenerator;
    impl ObjectGenerator for DummyGenerator {
        fn capabilities(&self) -> ObjectGenCapabilities {
            ObjectGenCapabilities::default()
        }
        fn prepare(&mut self, _ctx: &PrepareCtx) -> Vec<SynthObjectSpec> {
            Vec::new()
        }
        fn process(&mut self, _bed: &BedFrame, _out: &mut [Vec<f32>]) {}
        fn param_schema(&self) -> &'static [ObjectGenParamSpec] {
            &DUMMY_PARAMS
        }
    }

    struct DummyFactory;
    impl ObjectGeneratorFactory for DummyFactory {
        fn id(&self) -> &'static str {
            "dummy"
        }
        fn label(&self) -> &'static str {
            "Dummy"
        }
        fn requires_height_layer(&self) -> bool {
            false
        }
        fn build(&self) -> Box<dyn ObjectGenerator> {
            Box::new(DummyGenerator)
        }
        fn param_schema(&self) -> &'static [ObjectGenParamSpec] {
            &DUMMY_PARAMS
        }
    }

    #[test]
    fn registry_lists_and_builds_out_of_tree_generator() {
        let mut reg = ObjectGeneratorRegistry::with_builtins();
        reg.register(Box::new(DummyFactory));
        let json = reg.listings_json();
        assert!(json.contains("dummy") && json.contains("Amount"), "{json}");
        assert!(reg.build("dummy").is_some());
        // The built-ins are still listed alongside the registered generator.
        assert!(json.contains("copy_up") && json.contains("\"pad\""));
    }
}

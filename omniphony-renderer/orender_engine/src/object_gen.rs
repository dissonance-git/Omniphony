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

use std::sync::Arc;

use bridge_api::RChannelLabel;
use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use renderer::live_params::SurroundPlacement;
use renderer::speaker_layout::SpeakerLayout;

#[derive(Debug, Clone, Copy, Default)]
pub struct ObjectGenCapabilities {
    pub requires_height_layer: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ObjectGenParamSpec {
    pub key: &'static str,
    pub label: &'static str,
    pub i18n_key: &'static str,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub default: f32,
    pub unit: &'static str,
}

#[derive(Debug, Clone)]
pub struct SynthObjectSpec {
    pub name: String,
    pub position: [f64; 3],
    pub gain_db: i8,
    pub size: [f32; 3],
}

pub struct PrepareCtx<'a> {
    pub input_labels: &'a [RChannelLabel],
    pub output_layout: &'a SpeakerLayout,
    pub sample_rate: u32,
    pub surround_placement: SurroundPlacement,
}

pub struct BedFrame<'a> {
    pub pcm: &'a [f32],
    pub channel_count: usize,
    pub sample_count: usize,
    pub sample_rate: u32,
}

pub trait ObjectGenerator: Send {
    fn capabilities(&self) -> ObjectGenCapabilities;
    fn prepare(&mut self, ctx: &PrepareCtx) -> Vec<SynthObjectSpec>;
    fn process(&mut self, bed: &BedFrame, out: &mut [Vec<f32>]);
    fn param_schema(&self) -> &'static [ObjectGenParamSpec] {
        &[]
    }
    fn set_param(&mut self, _key: &str, _value: f32, _sample_rate: u32) {}
}

pub trait ObjectGeneratorFactory: Send + Sync {
    fn id(&self) -> &'static str;
    fn label(&self) -> &'static str;
    fn requires_height_layer(&self) -> bool;
    fn build(&self) -> Box<dyn ObjectGenerator>;
    fn i18n_key(&self) -> &'static str {
        ""
    }
    fn param_schema(&self) -> &'static [ObjectGenParamSpec] {
        &[]
    }
}

pub struct ObjectGeneratorRegistry {
    factories: Vec<Box<dyn ObjectGeneratorFactory>>,
}

impl ObjectGeneratorRegistry {
    pub fn with_builtins() -> Self {
        Self {
            factories: vec![
                Box::new(CopyUpFactory),
                Box::new(PadFactory),
                Box::new(DiracFactory),
            ],
        }
    }

    pub fn register(&mut self, factory: Box<dyn ObjectGeneratorFactory>) {
        self.factories.push(factory);
    }

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

    pub fn list(&self) -> Vec<(&'static str, &'static str, bool)> {
        self.factories
            .iter()
            .map(|f| (f.id(), f.label(), f.requires_height_layer()))
            .collect()
    }

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

pub fn builtin_listings_json() -> String {
    ObjectGeneratorRegistry::with_builtins().listings_json()
}

impl Default for ObjectGeneratorRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

const Z_EPS: f32 = 1e-3;

pub fn layout_has_height(layout: &SpeakerLayout) -> bool {
    layout.speakers.iter().any(|s| s.spatialize && s.z > Z_EPS)
}

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

pub(crate) fn find_channel(labels: &[RChannelLabel], want: RChannelLabel) -> Option<usize> {
    labels.iter().position(|&l| l == want)
}

pub(crate) fn one_pole_coeff(tc_ms: f32, fs: f32) -> f32 {
    let tau_samples = (tc_ms * 1.0e-3 * fs).max(1.0);
    1.0 - (-1.0 / tau_samples).exp()
}

pub(crate) fn top_position(label: RChannelLabel) -> Option<[f64; 3]> {
    use RChannelLabel::*;
    let pos = match label {
        L => [-1.0, 1.0, 1.0],
        R => [1.0, 1.0, 1.0],
        C => [0.0, 1.0, 1.0],
        Ls => [-1.0, 0.0, 1.0],
        Rs => [1.0, 0.0, 1.0],
        Lb => [-1.0, -1.0, 1.0],
        Rb => [1.0, -1.0, 1.0],
        Cb => [0.0, -1.0, 1.0],
        _ => return None,
    };
    Some(pos)
}

pub(crate) fn input_has_back(labels: &[RChannelLabel]) -> bool {
    labels
        .iter()
        .any(|l| matches!(l, RChannelLabel::Lb | RChannelLabel::Rb | RChannelLabel::Cb))
}

pub(crate) fn channel_top_position(
    label: RChannelLabel,
    use_7_1: bool,
    placement: SurroundPlacement,
) -> Option<[f64; 3]> {
    let mut pos = top_position(label)?;
    if let Some((x, y, _)) =
        crate::virtual_bed::surround_placement_override(label, use_7_1, placement)
    {
        pos[0] = x as f64;
        pos[1] = y as f64;
    }
    Some(pos)
}

pub(crate) fn channel_3d_position(
    label: RChannelLabel,
    use_7_1: bool,
    placement: SurroundPlacement,
) -> Option<[f64; 3]> {
    use RChannelLabel::*;
    let top = match label {
        Tfl => [-1.0, 1.0, 1.0],
        Tfr => [1.0, 1.0, 1.0],
        Tbl => [-1.0, -1.0, 1.0],
        Tbr => [1.0, -1.0, 1.0],
        Tsl => [-1.0, 0.0, 1.0],
        Tsr => [1.0, 0.0, 1.0],
        Tc => [0.0, 0.0, 1.0],
        Tfc => [0.0, 1.0, 1.0],
        _ => {
            let mut pos = channel_top_position(label, use_7_1, placement)?;
            pos[2] = 0.0;
            return Some(pos);
        }
    };
    Some(top)
}

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
        const GAIN_DB: i8 = -6;
        const SIZE: [f32; 3] = [0.3, 0.3, 0.3];
        let use_7_1 = input_has_back(ctx.input_labels);
        let mut specs = Vec::new();
        for (ch, &label) in ctx.input_labels.iter().enumerate() {
            let Some(position) = channel_top_position(label, use_7_1, ctx.surround_placement)
            else {
                continue;
            };
            self.src_channels.push(ch);
            specs.push(SynthObjectSpec {
                name: format!("Height_{label:?}_synth"),
                position,
                gain_db: GAIN_DB,
                size: SIZE,
            });
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

const PAD_HPF_HZ: f32 = 300.0;
const PAD_HPF_Q: f32 = std::f32::consts::FRAC_1_SQRT_2;
const PAD_DEFAULT_STRENGTH: f32 = 0.5;
const PAD_STAT_TC_MS: f32 = 50.0;
const PAD_REG: f32 = 1.0e-2;
const PAD_FLOOR: f32 = 1.0e-9;
const PAD_W_CLAMP: f32 = 1.2;
const PAD_GAIN_DB: i8 = 0;
const PAD_CENTER_DEFAULT_AMOUNT: f32 = 0.4;
const PAD_CENTER_DEFAULT_HPF_HZ: f32 = 3000.0;
const PAD_CENTER_HPF_MIN: f32 = 300.0;
const PAD_CENTER_HPF_MAX: f32 = 8000.0;

const PAD_PARAM_SPECS: [ObjectGenParamSpec; 5] = [
    ObjectGenParamSpec { key: "strength", label: "Ambience strength", i18n_key: "twoDSources.padStrength", min: 0.0, max: 1.0, step: 0.01, default: PAD_DEFAULT_STRENGTH, unit: "" },
    ObjectGenParamSpec { key: "hpf_hz", label: "Bass cutoff", i18n_key: "twoDSources.padHpf", min: 20.0, max: 2000.0, step: 10.0, default: PAD_HPF_HZ, unit: "Hz" },
    ObjectGenParamSpec { key: "gain_db", label: "Height level", i18n_key: "twoDSources.padGain", min: -24.0, max: 24.0, step: 0.5, default: 0.0, unit: "dB" },
    ObjectGenParamSpec { key: "center_amount", label: "Center to height", i18n_key: "twoDSources.padCenterAmount", min: 0.0, max: 1.0, step: 0.01, default: PAD_CENTER_DEFAULT_AMOUNT, unit: "" },
    ObjectGenParamSpec { key: "center_hpf_hz", label: "Center bass cutoff", i18n_key: "twoDSources.padCenterHpf", min: PAD_CENTER_HPF_MIN, max: PAD_CENTER_HPF_MAX, step: 50.0, default: PAD_CENTER_DEFAULT_HPF_HZ, unit: "Hz" },
];

struct AmbientPair {
    l_ch: usize,
    r_ch: usize,
    out_l: usize,
    out_r: usize,
    pow_l: f32,
    pow_r: f32,
    cross: f32,
    hpf_l: Biquad,
    hpf_r: Biquad,
}

struct CenterChannel {
    ch: usize,
    out: usize,
    hpf: Biquad,
}

struct PadGenerator {
    pairs: Vec<AmbientPair>,
    center: Option<CenterChannel>,
    strength: f32,
    makeup: f32,
    alpha: f32,
    center_amount: f32,
    center_hpf_hz: f32,
}

impl Default for PadGenerator {
    fn default() -> Self {
        Self { pairs: Vec::new(), center: None, strength: PAD_DEFAULT_STRENGTH, makeup: 1.0, alpha: 0.0, center_amount: PAD_CENTER_DEFAULT_AMOUNT, center_hpf_hz: PAD_CENTER_DEFAULT_HPF_HZ }
    }
}

impl ObjectGenerator for PadGenerator {
    fn capabilities(&self) -> ObjectGenCapabilities { ObjectGenCapabilities { requires_height_layer: true } }
    fn prepare(&mut self, ctx: &PrepareCtx) -> Vec<SynthObjectSpec> {
        self.pairs.clear(); self.center = None;
        if !layout_has_height(ctx.output_layout) || input_has_height(ctx.input_labels) { return Vec::new(); }
        let fs = ctx.sample_rate.max(1) as f32;
        self.alpha = one_pole_coeff(PAD_STAT_TC_MS, fs);
        let labels = ctx.input_labels;
        let use_7_1 = input_has_back(labels);
        let hpf = Biquad::highpass(fs, PAD_HPF_HZ, PAD_HPF_Q);
        const SIZE: [f32; 3] = [0.5, 0.5, 0.5];
        let pair_defs = [("Ambience_FL", "Ambience_FR", RChannelLabel::L, RChannelLabel::R), ("Ambience_SL", "Ambience_SR", RChannelLabel::Ls, RChannelLabel::Rs), ("Ambience_BL", "Ambience_BR", RChannelLabel::Lb, RChannelLabel::Rb)];
        let mut specs = Vec::new();
        for (name_l, name_r, label_l, label_r) in pair_defs {
            let (Some(l_ch), Some(r_ch)) = (find_channel(labels, label_l), find_channel(labels, label_r)) else { continue; };
            let (Some(pos_l), Some(pos_r)) = (channel_top_position(label_l, use_7_1, ctx.surround_placement), channel_top_position(label_r, use_7_1, ctx.surround_placement)) else { continue; };
            let out_l = specs.len(); let out_r = out_l + 1;
            self.pairs.push(AmbientPair { l_ch, r_ch, out_l, out_r, pow_l: 0.0, pow_r: 0.0, cross: 0.0, hpf_l: hpf, hpf_r: hpf });
            specs.push(SynthObjectSpec { name: name_l.to_string(), position: pos_l, gain_db: PAD_GAIN_DB, size: SIZE });
            specs.push(SynthObjectSpec { name: name_r.to_string(), position: pos_r, gain_db: PAD_GAIN_DB, size: SIZE });
        }
        if let (Some(ch), Some(position)) = (find_channel(labels, RChannelLabel::C), channel_top_position(RChannelLabel::C, use_7_1, ctx.surround_placement)) {
            let out = specs.len();
            self.center = Some(CenterChannel { ch, out, hpf: Biquad::highpass(fs, self.center_hpf_hz, PAD_HPF_Q) });
            specs.push(SynthObjectSpec { name: "Ambience_TC".to_string(), position, gain_db: PAD_GAIN_DB, size: SIZE });
        }
        specs
    }
    fn process(&mut self, bed: &BedFrame, out: &mut [Vec<f32>]) {
        let c = bed.channel_count; let strength = self.strength; let makeup = self.makeup; let alpha = self.alpha; let center_gain = self.center_amount * makeup;
        for pair in self.pairs.iter_mut() {
            if pair.l_ch >= c || pair.r_ch >= c || pair.out_l >= out.len() || pair.out_r >= out.len() { continue; }
            for s in 0..bed.sample_count {
                let l = bed.pcm[s*c + pair.l_ch]; let r = bed.pcm[s*c + pair.r_ch];
                pair.pow_l += alpha*(l*l-pair.pow_l); pair.pow_r += alpha*(r*r-pair.pow_r); pair.cross += alpha*(l*r-pair.cross);
                let reg = PAD_REG*0.5*(pair.pow_l+pair.pow_r)+PAD_FLOOR;
                let w_lr=(pair.cross/(pair.pow_r+reg)).clamp(-PAD_W_CLAMP,PAD_W_CLAMP); let w_rl=(pair.cross/(pair.pow_l+reg)).clamp(-PAD_W_CLAMP,PAD_W_CLAMP);
                let amb_l=l-strength*w_lr*r; let amb_r=r-strength*w_rl*l;
                out[pair.out_l][s]=pair.hpf_l.process(amb_l)*makeup; out[pair.out_r][s]=pair.hpf_r.process(amb_r)*makeup;
            }
        }
        if let Some(cc)=self.center.as_mut() { if cc.ch<c && cc.out<out.len() { for s in 0..bed.sample_count { let x=bed.pcm[s*c+cc.ch]; out[cc.out][s]=cc.hpf.process(x)*center_gain; } } }
    }
    fn param_schema(&self) -> &'static [ObjectGenParamSpec] { &PAD_PARAM_SPECS }
    fn set_param(&mut self,key:&str,value:f32,sample_rate:u32){ match key { "strength"=>self.strength=value.clamp(0.0,1.0), "gain_db"=>self.makeup=10.0_f32.powf(value.clamp(-24.0,24.0)/20.0), "hpf_hz"=>{let fs=sample_rate.max(1) as f32;let fc=value.clamp(20.0,2000.0);for pair in self.pairs.iter_mut(){pair.hpf_l.set_highpass(fs,fc,PAD_HPF_Q);pair.hpf_r.set_highpass(fs,fc,PAD_HPF_Q);}}, "center_amount"=>self.center_amount=value.clamp(0.0,1.0), "center_hpf_hz"=>{let fs=sample_rate.max(1) as f32;let fc=value.clamp(PAD_CENTER_HPF_MIN,PAD_CENTER_HPF_MAX);self.center_hpf_hz=fc;if let Some(cc)=self.center.as_mut(){cc.hpf.set_highpass(fs,fc,PAD_HPF_Q);}}, _=>{} } }
}

struct DiracFactory;
impl ObjectGeneratorFactory for DiracFactory { fn id(&self)->&'static str{"dirac"} fn label(&self)->&'static str{"Diffuse field to height (DirAC)"} fn requires_height_layer(&self)->bool{true} fn build(&self)->Box<dyn ObjectGenerator>{Box::<DiracGenerator>::default()} fn i18n_key(&self)->&'static str{"twoDSources.objectGenDirac"} fn param_schema(&self)->&'static [ObjectGenParamSpec]{&DIRAC_PARAM_SPECS} }
const DIRAC_FFT_SIZE:usize=1024; const DIRAC_HOP:usize=DIRAC_FFT_SIZE/2; const DIRAC_STAT_TC_MS:f32=80.0; const DIRAC_REG:f32=1.0e-2; const DIRAC_FLOOR:f32=1.0e-12; const DIRAC_GAIN_DB:i8=0; const DIRAC_DEFAULT_AMOUNT:f32=0.5; const DIRAC_DEFAULT_BIAS:f32=0.0; const DIRAC_DEFAULT_HPF_HZ:f32=500.0; const DIRAC_HPF_MIN:f32=300.0; const DIRAC_HPF_MAX:f32=8000.0; const DIRAC_MAX_BLOCK:usize=4096; const DIRAC_AP_DELAYS:[[usize;2];4]=[[37,211],[53,229],[67,251],[79,271]]; const DIRAC_AP_G:f32=0.6; const DIRAC_SIZE:[f32;3]=[0.6,0.6,0.6];
const DIRAC_PARAM_SPECS:[ObjectGenParamSpec;3]=[
ObjectGenParamSpec{key:"amount",label:"Diffuse level",i18n_key:"twoDSources.diracAmount",min:0.0,max:1.0,step:0.01,default:DIRAC_DEFAULT_AMOUNT,unit:""},
ObjectGenParamSpec{key:"diffuse_bias",label:"Diffuse bias",i18n_key:"twoDSources.diracBias",min:0.0,max:1.0,step:0.01,default:DIRAC_DEFAULT_BIAS,unit:""},
ObjectGenParamSpec{key:"hpf_hz",label:"Bass cutoff",i18n_key:"twoDSources.diracHpf",min:DIRAC_HPF_MIN,max:DIRAC_HPF_MAX,step:50.0,default:DIRAC_DEFAULT_HPF_HZ,unit:"Hz"}];
#[inline] pub(crate) fn flush_denorm(x:f32)->f32{if x.abs()<1.0e-20{0.0}else{x}}
fn build_hpf_mask(mask:&mut[f32],fs:f32,fc:f32){let bin_hz=fs/DIRAC_FFT_SIZE as f32;let f_lo=fc/2.0_f32.powf(1.0/3.0);let f_hi=fc.max(f_lo+bin_hz);for(b,m)in mask.iter_mut().enumerate(){let f=b as f32*bin_hz;*m=if f<=f_lo{0.0}else if f>=f_hi{1.0}else{let t=(f-f_lo)/(f_hi-f_lo);0.5-0.5*(std::f32::consts::PI*t).cos()};}}
#[derive(Clone)] struct AllPass{buf:Vec<f32>,idx:usize,g:f32} impl AllPass{fn new(delay:usize,g:f32)->Self{Self{buf:vec![0.0;delay.max(1)],idx:0,g}} #[inline] fn process(&mut self,x:f32)->f32{let buffered=self.buf[self.idx];let v=x+self.g*buffered;let y=buffered-self.g*v;self.buf[self.idx]=flush_denorm(v);self.idx+=1;if self.idx>=self.buf.len(){self.idx=0;}y}}
struct DiracGenerator{objects:usize,enc:Vec<(usize,f32,f32)>,fft_fwd:Arc<dyn RealToComplex<f32>>,fft_inv:Arc<dyn ComplexToReal<f32>>,win:Vec<f32>,frame_w:Vec<f32>,frame_x:Vec<f32>,frame_y:Vec<f32>,widx:usize,fill:usize,real_buf:Vec<f32>,spec_w:Vec<Complex<f32>>,spec_x:Vec<Complex<f32>>,spec_y:Vec<Complex<f32>>,fft_scratch:Vec<Complex<f32>>,ifft_scratch:Vec<Complex<f32>>,real_out:Vec<f32>,i_x:Vec<f32>,i_y:Vec<f32>,e:Vec<f32>,hpf_mask:Vec<f32>,ola:Vec<f32>,out_fifo:Vec<f32>,fifo_read:usize,fifo_write:usize,fifo_len:usize,allpass:Vec<[AllPass;2]>,alpha:f32,amount:f32,diffuse_bias:f32,hpf_hz:f32,sample_rate:u32,#[cfg(test)] force_diffuse:bool}
impl Default for DiracGenerator{fn default()->Self{let mut planner=RealFftPlanner::<f32>::new();let fft_fwd=planner.plan_fft_forward(DIRAC_FFT_SIZE);let fft_inv=planner.plan_fft_inverse(DIRAC_FFT_SIZE);Self{objects:0,enc:Vec::new(),fft_fwd,fft_inv,win:Vec::new(),frame_w:Vec::new(),frame_x:Vec::new(),frame_y:Vec::new(),widx:0,fill:0,real_buf:Vec::new(),spec_w:Vec::new(),spec_x:Vec::new(),spec_y:Vec::new(),fft_scratch:Vec::new(),ifft_scratch:Vec::new(),real_out:Vec::new(),i_x:Vec::new(),i_y:Vec::new(),e:Vec::new(),hpf_mask:Vec::new(),ola:Vec::new(),out_fifo:Vec::new(),fifo_read:0,fifo_write:0,fifo_len:0,allpass:Vec::new(),alpha:0.0,amount:DIRAC_DEFAULT_AMOUNT,diffuse_bias:DIRAC_DEFAULT_BIAS,hpf_hz:DIRAC_DEFAULT_HPF_HZ,sample_rate:48_000,#[cfg(test)] force_diffuse:false}}}
impl DiracGenerator{#[inline]fn push_fifo(&mut self,v:f32){if self.fifo_len<self.out_fifo.len(){self.out_fifo[self.fifo_write]=v;self.fifo_write=(self.fifo_write+1)%self.out_fifo.len();self.fifo_len+=1;}} #[inline]fn pop_fifo(&mut self)->f32{if self.fifo_len==0{return 0.0;}let v=self.out_fifo[self.fifo_read];self.fifo_read=(self.fifo_read+1)%self.out_fifo.len();self.fifo_len-=1;v} fn run_frame(&mut self){let n=DIRAC_FFT_SIZE;let nb=self.spec_w.len();for i in 0..n{self.real_buf[i]=self.win[i]*self.frame_w[(self.widx+i)%n];}let _=self.fft_fwd.process_with_scratch(&mut self.real_buf,&mut self.spec_w,&mut self.fft_scratch);for i in 0..n{self.real_buf[i]=self.win[i]*self.frame_x[(self.widx+i)%n];}let _=self.fft_fwd.process_with_scratch(&mut self.real_buf,&mut self.spec_x,&mut self.fft_scratch);for i in 0..n{self.real_buf[i]=self.win[i]*self.frame_y[(self.widx+i)%n];}let _=self.fft_fwd.process_with_scratch(&mut self.real_buf,&mut self.spec_y,&mut self.fft_scratch);let mut e_sum=0.0f32;for b in 0..nb{e_sum+=self.spec_w[b].norm_sqr();}let reg=DIRAC_REG*(e_sum/nb.max(1) as f32)+DIRAC_FLOOR;let alpha=self.alpha;let bias=self.diffuse_bias;let inv_1mb=1.0/(1.0-bias).max(1.0e-3);#[cfg(test)]let force=self.force_diffuse;#[cfg(not(test))]let force=false;for b in 0..nb{let w=self.spec_w[b];let x=self.spec_x[b];let y=self.spec_y[b];let ix=w.re*x.re+w.im*x.im;let iy=w.re*y.re+w.im*y.im;let ew=w.norm_sqr();self.i_x[b]=flush_denorm(self.i_x[b]+alpha*(ix-self.i_x[b]));self.i_y[b]=flush_denorm(self.i_y[b]+alpha*(iy-self.i_y[b]));self.e[b]=flush_denorm(self.e[b]+alpha*(ew-self.e[b]));let psi=if force{1.0}else{let imag=(self.i_x[b]*self.i_x[b]+self.i_y[b]*self.i_y[b]).sqrt();(1.0-imag/(self.e[b]+reg)).clamp(0.0,1.0)};let shaped=((psi-bias)*inv_1mb).clamp(0.0,1.0);let g=shaped.sqrt()*self.hpf_mask[b];self.spec_w[b]=w*g;}self.spec_w[0].im=0.0;if nb>1{self.spec_w[nb-1].im=0.0;}let _=self.fft_inv.process_with_scratch(&mut self.spec_w,&mut self.real_out,&mut self.ifft_scratch);let scale=(1.0/n as f32)*(4.0/3.0);for i in 0..n{self.ola[i]+=scale*self.win[i]*self.real_out[i];}for i in 0..DIRAC_HOP{self.push_fifo(self.ola[i]);}self.ola.copy_within(DIRAC_HOP..n,0);for v in self.ola[n-DIRAC_HOP..n].iter_mut(){*v=0.0;}}}
impl ObjectGenerator for DiracGenerator{fn capabilities(&self)->ObjectGenCapabilities{ObjectGenCapabilities{requires_height_layer:true}} fn prepare(&mut self,ctx:&PrepareCtx)->Vec<SynthObjectSpec>{self.objects=0;self.enc.clear();if !layout_has_height(ctx.output_layout)||input_has_height(ctx.input_labels){return Vec::new();}let fs=ctx.sample_rate.max(1) as f32;self.sample_rate=ctx.sample_rate.max(1);self.alpha=one_pole_coeff(DIRAC_STAT_TC_MS,fs/DIRAC_HOP as f32);let use_7_1=input_has_back(ctx.input_labels);for(idx,&label)in ctx.input_labels.iter().enumerate(){if let Some(pos)=channel_top_position(label,use_7_1,ctx.surround_placement){let(x,y)=(pos[0] as f32,pos[1] as f32);let r=(x*x+y*y).sqrt();let(ca,sa)=if r>1.0e-6{(y/r,x/r)}else{(0.0,0.0)};self.enc.push((idx,ca,sa));}}if self.enc.is_empty(){return Vec::new();}let n=DIRAC_FFT_SIZE;let nb=n/2+1;self.win=(0..n).map(|i|0.5-0.5*(std::f32::consts::TAU*i as f32/n as f32).cos()).collect();self.frame_w=vec![0.0;n];self.frame_x=vec![0.0;n];self.frame_y=vec![0.0;n];self.widx=0;self.fill=0;self.real_buf=self.fft_fwd.make_input_vec();self.spec_w=self.fft_fwd.make_output_vec();self.spec_x=self.fft_fwd.make_output_vec();self.spec_y=self.fft_fwd.make_output_vec();self.real_out=self.fft_inv.make_output_vec();self.fft_scratch=vec![Complex::new(0.0,0.0);self.fft_fwd.get_scratch_len()];self.ifft_scratch=vec![Complex::new(0.0,0.0);self.fft_inv.get_scratch_len()];self.i_x=vec![0.0;nb];self.i_y=vec![0.0;nb];self.e=vec![0.0;nb];self.hpf_mask=vec![0.0;nb];build_hpf_mask(&mut self.hpf_mask,fs,self.hpf_hz);self.ola=vec![0.0;n];self.out_fifo=vec![0.0;DIRAC_MAX_BLOCK+n];self.fifo_read=0;self.fifo_write=0;self.fifo_len=0;for _ in 0..n{self.push_fifo(0.0);}self.allpass=DIRAC_AP_DELAYS.iter().map(|d|[AllPass::new(d[0],DIRAC_AP_G),AllPass::new(d[1],DIRAC_AP_G)]).collect();let defs=[("Diffuse_TFL",[-1.0,1.0,1.0]),("Diffuse_TFR",[1.0,1.0,1.0]),("Diffuse_TBL",[-1.0,-1.0,1.0]),("Diffuse_TBR",[1.0,-1.0,1.0])];let specs:Vec<SynthObjectSpec>=defs.iter().map(|(name,pos)|SynthObjectSpec{name:name.to_string(),position:*pos,gain_db:DIRAC_GAIN_DB,size:DIRAC_SIZE}).collect();self.objects=specs.len();specs} fn process(&mut self,bed:&BedFrame,out:&mut[Vec<f32>]){if self.objects==0{return;}let c=bed.channel_count;let n=bed.sample_count;let amount=self.amount;let enc=std::mem::take(&mut self.enc);for s in 0..n{let base=s*c;let mut w=0.0f32;let mut x=0.0f32;let mut y=0.0f32;for &(ch,ca,sa) in enc.iter(){if ch<c{let v=bed.pcm[base+ch];w+=v;x+=ca*v;y+=sa*v;}}self.frame_w[self.widx]=w;self.frame_x[self.widx]=x;self.frame_y[self.widx]=y;self.widx=(self.widx+1)%DIRAC_FFT_SIZE;self.fill+=1;if self.fill>=DIRAC_HOP{self.fill=0;self.run_frame();}let d=self.pop_fifo();for(k,ap)in self.allpass.iter_mut().enumerate(){if k>=out.len(){break;}let[a0,a1]=ap;out[k][s]=a1.process(a0.process(d))*amount;}}self.enc=enc;} fn param_schema(&self)->&'static[ObjectGenParamSpec]{&DIRAC_PARAM_SPECS} fn set_param(&mut self,key:&str,value:f32,sample_rate:u32){match key{"amount"=>self.amount=value.clamp(0.0,1.0),"diffuse_bias"=>self.diffuse_bias=value.clamp(0.0,1.0),"hpf_hz"=>{self.hpf_hz=value.clamp(DIRAC_HPF_MIN,DIRAC_HPF_MAX);if !self.hpf_mask.is_empty(){let fs=sample_rate.max(1) as f32;build_hpf_mask(&mut self.hpf_mask,fs,self.hpf_hz);}},_=>{}}}}

#[derive(Default)] struct PlanSig{id:String,out_n:usize,out_height:bool,labels:Vec<RChannelLabel>,rate:u32,options_epoch:u64}
pub struct ObjectGenStage{registry:ObjectGeneratorRegistry,generator:Option<Box<dyn ObjectGenerator>>,specs:Vec<SynthObjectSpec>,planar:Vec<Vec<f32>>,pcm_ext:Vec<f32>,sig:PlanSig}
impl ObjectGenStage{pub fn new()->Self{Self{registry:ObjectGeneratorRegistry::with_builtins(),generator:None,specs:Vec::new(),planar:Vec::new(),pcm_ext:Vec::new(),sig:PlanSig::default()}} pub fn registry(&self)->&ObjectGeneratorRegistry{&self.registry} pub fn register(&mut self,factory:Box<dyn ObjectGeneratorFactory>){self.registry.register(factory);} pub fn specs(&self)->&[SynthObjectSpec]{&self.specs} pub fn sync(&mut self,desired_id:&str,ctx:&PrepareCtx,options_epoch:u64)->usize{let did=desired_id.trim();let out_n=ctx.output_layout.speakers.len();let out_height=layout_has_height(ctx.output_layout);let unchanged=self.sig.id==did&&self.sig.out_n==out_n&&self.sig.out_height==out_height&&self.sig.rate==ctx.sample_rate&&self.sig.options_epoch==options_epoch&&self.sig.labels.as_slice()==ctx.input_labels;if !unchanged{self.sig.id.clear();self.sig.id.push_str(did);self.sig.out_n=out_n;self.sig.out_height=out_height;self.sig.rate=ctx.sample_rate;self.sig.options_epoch=options_epoch;self.sig.labels.clear();self.sig.labels.extend_from_slice(ctx.input_labels);self.generator=self.registry.build(did);self.specs=match self.generator.as_mut(){Some(g)=>g.prepare(ctx),None=>Vec::new()};self.planar.truncate(self.specs.len());self.planar.resize_with(self.specs.len(),Vec::new);}self.specs.len()} pub fn set_param(&mut self,key:&str,value:f32,sample_rate:u32){if let Some(g)=self.generator.as_mut(){g.set_param(key,value,sample_rate);}} pub fn fill_and_extend(&mut self,bed_pcm:&[f32],channel_count:usize,sample_count:usize,sample_rate:u32)->(&[f32],usize){let m=self.specs.len();for buf in self.planar.iter_mut(){buf.clear();buf.resize(sample_count,0.0);}if let Some(generator)=self.generator.as_mut(){let bed=BedFrame{pcm:bed_pcm,channel_count,sample_count,sample_rate};generator.process(&bed,&mut self.planar);}let out_ch=channel_count+m;self.pcm_ext.clear();self.pcm_ext.resize(sample_count*out_ch,0.0);for s in 0..sample_count{let src=&bed_pcm[s*channel_count..s*channel_count+channel_count];let dst=&mut self.pcm_ext[s*out_ch..s*out_ch+out_ch];dst[..channel_count].copy_from_slice(src);for(k,buf)in self.planar.iter().enumerate().take(m){dst[channel_count+k]=buf[s];}}(&self.pcm_ext,out_ch)}} impl Default for ObjectGenStage{fn default()->Self{Self::new()}}

#[cfg(test)]
mod tests {
    use super::*;
    use renderer::speaker_layout::Speaker;
    fn layout(speakers:&[(&str,f32,f32,f32)])->SpeakerLayout{SpeakerLayout{radius_m:1.0,speakers:speakers.iter().map(|&(name,x,y,z)|Speaker::from_cartesian(name,x,y,z,true,0.0)).collect()}}
    fn flat_5_1()->Vec<(&'static str,f32,f32,f32)>{vec![("FL",-1.0,1.0,0.0),("FR",1.0,1.0,0.0),("C",0.0,1.0,0.0),("LFE",0.0,1.0,0.0),("Ls",-1.0,-1.0,0.0),("Rs",1.0,-1.0,0.0)]}
    fn layout_7_1_4()->SpeakerLayout{let mut s=flat_5_1();s.extend_from_slice(&[("TFL",-1.0,1.0,1.0),("TFR",1.0,1.0,1.0),("TBL",-1.0,-1.0,1.0),("TBR",1.0,-1.0,1.0)]);layout(&s)}
    const LABELS_5_1:[RChannelLabel;6]=[RChannelLabel::L,RChannelLabel::R,RChannelLabel::C,RChannelLabel::LFE,RChannelLabel::Ls,RChannelLabel::Rs];
    #[test] fn no_op_without_height_layer(){let mut g=CopyUpGenerator::default();let out=layout(&flat_5_1());let specs=g.prepare(&PrepareCtx{input_labels:&LABELS_5_1,output_layout:&out,sample_rate:48_000,surround_placement:SurroundPlacement::Side});assert!(specs.is_empty());}
    #[test] fn no_op_when_input_already_has_height(){let mut g=CopyUpGenerator::default();let out=layout_7_1_4();let labels=[RChannelLabel::L,RChannelLabel::R,RChannelLabel::Tfl,RChannelLabel::Tfr];let specs=g.prepare(&PrepareCtx{input_labels:&labels,output_layout:&out,sample_rate:48_000,surround_placement:SurroundPlacement::Side});assert!(specs.is_empty());}
    #[test] fn plans_five_top_objects_for_5_1_on_7_1_4(){let mut g=CopyUpGenerator::default();let out=layout_7_1_4();let specs=g.prepare(&PrepareCtx{input_labels:&LABELS_5_1,output_layout:&out,sample_rate:48_000,surround_placement:SurroundPlacement::Side});assert_eq!(specs.len(),5);assert!(specs.iter().all(|s|s.position[2]>0.0));}
    #[test] fn stage_replans_on_options_epoch_bump(){let mut stage=ObjectGenStage::new();let out=layout_7_1_4();let ctx_side=PrepareCtx{input_labels:&LABELS_5_1,output_layout:&out,sample_rate:48_000,surround_placement:SurroundPlacement::Side};assert_eq!(stage.sync("copy_up",&ctx_side,0),5);let ls_y=|stage:&ObjectGenStage|stage.specs().iter().find(|s|s.name.contains("Ls")).unwrap().position[1];let a=ls_y(&stage);let ctx_back=PrepareCtx{surround_placement:SurroundPlacement::Back,..ctx_side};assert_eq!(stage.sync("copy_up",&ctx_back,0),5);assert_eq!(ls_y(&stage),a);assert_eq!(stage.sync("copy_up",&ctx_back,1),5);assert!(a>ls_y(&stage)+0.5);}
    fn xorshift(seed:u32)->impl FnMut()->f32{let mut s=seed|1;move||{s^=s<<13;s^=s>>17;s^=s<<5;(s>>8)as f32/(1u32<<24)as f32*2.0-1.0}}
    fn dirac_on_7_1_4()->(DiracGenerator,Vec<SynthObjectSpec>){let mut g=DiracGenerator::default();let out=layout_7_1_4();let specs=g.prepare(&PrepareCtx{input_labels:&LABELS_5_1,output_layout:&out,sample_rate:48_000,surround_placement:SurroundPlacement::Side});(g,specs)}
    fn dirac_run(g:&mut DiracGenerator,pcm:&[f32],c:usize,n:usize)->Vec<Vec<f32>>{let mut out=vec![vec![0.0;n];4];g.process(&BedFrame{pcm,channel_count:c,sample_count:n,sample_rate:48_000},&mut out);out}
    fn ceiling_tail_energy(out:&[Vec<f32>],skip:usize)->f32{out.iter().map(|o|o[skip..].iter().map(|&x|x*x).sum::<f32>()).sum()}
    #[test] fn dirac_is_frequency_selective(){let c=6usize;let n=48_000usize;let skip=12_000usize;let fs=48_000.0f32;let lf=|s:usize|(std::f32::consts::TAU*150.0*s as f32/fs).sin()*0.4;let build=|with_lf:bool,with_hf:bool|->f32{let(mut g,_)=dirac_on_7_1_4();g.set_param("amount",1.0,48_000);let(mut na,mut nb)=(xorshift(11),xorshift(22));let mut hp_l=Biquad::highpass(fs,2000.0,PAD_HPF_Q);let mut hp_r=Biquad::highpass(fs,2000.0,PAD_HPF_Q);let mut pcm=vec![0.0;c*n];for s in 0..n{if with_lf{pcm[s*c]=lf(s);pcm[s*c+1]=lf(s);}if with_hf{pcm[s*c+4]=hp_l.process(na()*0.5);pcm[s*c+5]=hp_r.process(nb()*0.5);}}ceiling_tail_energy(&dirac_run(&mut g,&pcm,c,n),skip)};let mixed=build(true,true);let hf_only=build(false,true);let lf_only=build(true,false);assert!(mixed>4.0*lf_only);assert!((mixed-hf_only).abs()<0.6*hf_only);}
}

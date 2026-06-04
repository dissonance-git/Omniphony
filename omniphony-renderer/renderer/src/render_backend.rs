mod barycenter_backend;
mod distance_attenuation;
mod distance_diffuse;
mod evaluation_artifact;
mod experimental_distance_backend;
mod hybrid_backend;
mod room_transform;
pub mod size_to_spread;
mod vbap_backend;

use crate::spatial_vbap::{DistanceModel, Gains, adm_to_spherical, spherical_to_adm};
use crate::speaker_layout::SpeakerLayout;
use anyhow::Result;
use rayon::prelude::*;
use serde::Serialize;

pub use barycenter_backend::BarycenterBackend;
use distance_attenuation::DistanceAttenuatedModel;
use distance_diffuse::DistanceDiffuseModel;
pub use evaluation_artifact::{
    BackendRestoreSnapshot, SerializedEvaluationMode, build_backend_restore_snapshot,
};
pub use experimental_distance_backend::ExperimentalDistanceBackend;
pub use hybrid_backend::{BlendCurve, HybridBackend};
pub use size_to_spread::{SizeToSpreadMode, reduce_size_to_spread};
pub use vbap_backend::VbapBackend;

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct BackendCapabilities {
    pub supports_realtime: bool,
    pub supports_precomputed_polar: bool,
    pub supports_precomputed_cartesian: bool,
    pub supports_position_interpolation: bool,
    pub supports_distance_model: bool,
    pub supports_spread: bool,
    pub supports_spread_from_distance: bool,
    /// True when the backend consumes per-event object size (anisotropic
    /// w/d/h triplet) in addition to or instead of the global spread params.
    pub supports_event_size: bool,
    pub supports_distance_diffuse: bool,
    pub supports_table_export: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct BackendDescriptor {
    pub kind: RenderBackendKind,
    pub gain_model_kind: GainModelKind,
    pub id: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GainModelKind {
    Vbap,
    Barycenter,
    ExperimentalDistance,
    Hybrid,
}

impl GainModelKind {
    pub fn as_str(self) -> &'static str {
        backend_descriptor_by_gain_model_kind(self).id
    }

    pub fn from_str(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        if let Some(descriptor) = backend_descriptor_by_id(&normalized) {
            return Some(descriptor.gain_model_kind);
        }
        match normalized.as_str() {
            "barycentre" | "barycenter" => Some(Self::Barycenter),
            "distance" | "distance_based" => Some(Self::ExperimentalDistance),
            "hybrid" => Some(Self::Hybrid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderBackendKind {
    Vbap,
    Barycenter,
    ExperimentalDistance,
    Hybrid,
}

impl RenderBackendKind {
    pub fn as_str(self) -> &'static str {
        backend_descriptor(self).id
    }

    pub fn from_str(value: &str) -> Option<Self> {
        let normalized = value.trim().to_ascii_lowercase();
        if let Some(descriptor) = backend_descriptor_by_id(&normalized) {
            return Some(descriptor.kind);
        }
        match normalized.as_str() {
            "barycentre" | "barycenter" => Some(Self::Barycenter),
            "distance" | "distance_based" => Some(Self::ExperimentalDistance),
            "hybrid" => Some(Self::Hybrid),
            _ => None,
        }
    }

    pub fn as_gain_model_kind(self) -> GainModelKind {
        backend_descriptor(self).gain_model_kind
    }

    pub fn label(self) -> &'static str {
        backend_descriptor(self).label
    }
}

impl From<GainModelKind> for RenderBackendKind {
    fn from(value: GainModelKind) -> Self {
        match value {
            GainModelKind::Vbap => Self::Vbap,
            GainModelKind::Barycenter => Self::Barycenter,
            GainModelKind::ExperimentalDistance => Self::ExperimentalDistance,
            GainModelKind::Hybrid => Self::Hybrid,
        }
    }
}

impl From<RenderBackendKind> for GainModelKind {
    fn from(value: RenderBackendKind) -> Self {
        value.as_gain_model_kind()
    }
}

const BACKEND_DESCRIPTORS: [BackendDescriptor; 4] = [
    BackendDescriptor {
        kind: RenderBackendKind::Vbap,
        gain_model_kind: GainModelKind::Vbap,
        id: "vbap",
        label: "VBAP",
    },
    BackendDescriptor {
        kind: RenderBackendKind::Barycenter,
        gain_model_kind: GainModelKind::Barycenter,
        id: "barycenter",
        label: "Barycenter",
    },
    BackendDescriptor {
        kind: RenderBackendKind::ExperimentalDistance,
        gain_model_kind: GainModelKind::ExperimentalDistance,
        id: "experimental_distance",
        label: "Distance",
    },
    BackendDescriptor {
        kind: RenderBackendKind::Hybrid,
        gain_model_kind: GainModelKind::Hybrid,
        id: "hybrid",
        label: "Hybrid",
    },
];

pub fn backend_descriptors() -> &'static [BackendDescriptor] {
    &BACKEND_DESCRIPTORS
}

pub fn backend_descriptor(kind: RenderBackendKind) -> &'static BackendDescriptor {
    backend_descriptors()
        .iter()
        .find(|descriptor| descriptor.kind == kind)
        .expect("missing backend descriptor")
}

pub fn backend_descriptor_by_gain_model_kind(kind: GainModelKind) -> &'static BackendDescriptor {
    backend_descriptors()
        .iter()
        .find(|descriptor| descriptor.gain_model_kind == kind)
        .expect("missing backend descriptor")
}

pub fn backend_descriptor_by_id(id: &str) -> Option<&'static BackendDescriptor> {
    backend_descriptors()
        .iter()
        .find(|descriptor| descriptor.id == id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveEvaluationMode {
    Realtime,
    PrecomputedPolar,
    PrecomputedCartesian,
}

impl EffectiveEvaluationMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Realtime => "realtime",
            Self::PrecomputedPolar => "precomputed_polar",
            Self::PrecomputedCartesian => "precomputed_cartesian",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct RenderRequest {
    pub adm_position: [f64; 3],
    /// Per-event object size (w, d, h) ∈ [0, 1]³. `[0, 0, 0]` for a point
    /// source or when no size information is available.
    pub event_size: [f32; 3],
    /// Reduction policy applied to `event_size` to derive a scalar spread.
    /// Only consumed by backends whose `BackendCapabilities::supports_event_size`
    /// is `true` (currently VBAP).
    pub size_to_spread_mode: SizeToSpreadMode,
    pub spread_min: f32,
    pub spread_max: f32,
    pub spread_from_distance: bool,
    pub spread_distance_range: f32,
    pub spread_distance_curve: f32,
    pub room_ratio: [f32; 3],
    pub room_ratio_rear: f32,
    pub room_ratio_lower: f32,
    pub room_ratio_center_blend: f32,
    pub use_distance_diffuse: bool,
    pub distance_diffuse_threshold: f32,
    pub distance_diffuse_curve: f32,
    pub distance_model: DistanceModel,
    pub barycenter_localize: f32,
    pub experimental_distance_distance_floor: f32,
    pub experimental_distance_min_active_speakers: usize,
    pub experimental_distance_max_active_speakers: usize,
    pub experimental_distance_position_error_floor: f32,
    pub experimental_distance_position_error_nearest_scale: f32,
    pub experimental_distance_position_error_span_scale: f32,
}

pub struct RenderResponse {
    pub gains: Gains,
}

#[derive(Clone, Copy)]
pub struct CartesianEvaluationConfig {
    pub x_size: usize,
    pub y_size: usize,
    pub z_size: usize,
    pub z_neg_size: usize,
}

#[derive(Clone, Copy)]
pub struct PolarEvaluationConfig {
    pub azimuth_values: usize,
    pub elevation_values: usize,
    pub distance_values: usize,
    pub distance_max: f32,
    pub allow_negative_z: bool,
}

#[derive(Clone, Copy)]
pub struct EvaluationBuildConfig {
    pub request_template: RenderRequest,
    pub position_interpolation: bool,
    pub cartesian: CartesianEvaluationConfig,
    pub polar: PolarEvaluationConfig,
    /// Metric used to reduce a position to a scalar distance for the distance
    /// model and distance diffuse output stages (Spherical / Chebyshev).
    pub distance_model_metric: crate::spatial_vbap::DistanceMetric,
    pub distance_diffuse_metric: crate::spatial_vbap::DistanceMetric,
}

pub trait GainModel: Send + Sync + 'static {
    fn kind(&self) -> GainModelKind;
    fn backend_id(&self) -> &'static str;
    fn backend_label(&self) -> &'static str;
    fn capabilities(&self) -> BackendCapabilities;
    fn speaker_count(&self) -> usize;
    fn compute_gains(&self, req: &RenderRequest) -> RenderResponse;
    fn save_to_file(&self, path: &std::path::Path, speaker_layout: &SpeakerLayout) -> Result<()>;
}

pub trait EvaluationStrategy {
    fn effective_mode(&self) -> EffectiveEvaluationMode;
    fn prepare(
        self,
        model: Box<dyn GainModel>,
        config: &EvaluationBuildConfig,
    ) -> Result<Box<dyn PreparedEvaluator>>;
}

pub trait PreparedEvaluator: Send + Sync {
    fn speaker_count(&self) -> usize;
    fn compute_gains(&self, req: &RenderRequest) -> RenderResponse;
    fn save_to_file(&self, path: &std::path::Path, speaker_layout: &SpeakerLayout) -> Result<()>;
    /// Serialize the precomputed evaluation table (gains grid + metadata) to the
    /// portable artifact byte layout, so it can be shipped to clients (chunked) and
    /// rebuilt verbatim. Default: unsupported (realtime evaluators hold no table).
    fn artifact_bytes(&self, speaker_layout: &SpeakerLayout) -> Result<Vec<u8>> {
        let _ = speaker_layout;
        anyhow::bail!("evaluator has no precomputed table to serialize")
    }
}

pub struct RealtimeEvaluator {
    model: Box<dyn GainModel>,
}

impl RealtimeEvaluator {
    pub fn new(model: Box<dyn GainModel>) -> Self {
        Self { model }
    }
}

impl PreparedEvaluator for RealtimeEvaluator {
    fn speaker_count(&self) -> usize {
        self.model.speaker_count()
    }

    fn compute_gains(&self, req: &RenderRequest) -> RenderResponse {
        self.model.compute_gains(req)
    }

    fn save_to_file(&self, path: &std::path::Path, speaker_layout: &SpeakerLayout) -> Result<()> {
        let _ = (path, speaker_layout);
        anyhow::bail!("only precomputed evaluators can be exported to a from-file artifact")
    }
}

pub struct SampledCartesianEvaluator {
    model: Box<dyn GainModel>,
    x_positions: Vec<f32>,
    y_positions: Vec<f32>,
    z_positions: Vec<f32>,
    gains: Vec<f32>,
    speaker_count: usize,
    position_interpolation: bool,
    frozen_request: RenderRequest,
    backend_restore_snapshot: Option<BackendRestoreSnapshot>,
}

impl SampledCartesianEvaluator {
    pub fn new(model: Box<dyn GainModel>, config: &EvaluationBuildConfig) -> Self {
        // Intentionally sample and query the precomputed cartesian evaluator in native
        // ADM coordinates. The backend remains responsible for any room/depth transforms,
        // so the runtime can read gains directly from object positions without converting
        // into a backend-specific "effect space" first.
        let x_positions = evenly_spaced_axis(config.cartesian.x_size.max(2), -1.0, 1.0);
        let y_positions = evenly_spaced_axis(config.cartesian.y_size.max(2), -1.0, 1.0);
        let z_positions =
            cartesian_z_axis(config.cartesian.z_size.max(2), config.cartesian.z_neg_size);
        let speaker_count = model.speaker_count();
        let (nx, ny, nz) = (x_positions.len(), y_positions.len(), z_positions.len());
        let template = config.request_template;
        // Sampling the gain model over the full x×y×z volume dominates engine
        // startup, and it runs once per render backend. Each cell is independent
        // and GainModel is Sync, so evaluate them in parallel. The flat index
        // decodes to the SAME z→y→x order the sequential build produced, which
        // the runtime table lookup relies on.
        let per_cell: Vec<Gains> = (0..nx * ny * nz)
            .into_par_iter()
            .map(|idx| {
                let xi = idx % nx;
                let yi = (idx / nx) % ny;
                let zi = idx / (nx * ny);
                let mut request = template;
                request.adm_position = [
                    x_positions[xi] as f64,
                    y_positions[yi] as f64,
                    z_positions[zi] as f64,
                ];
                model.compute_gains(&request).gains
            })
            .collect();
        let mut gains = Vec::with_capacity(nx * ny * nz * speaker_count);
        for cell in &per_cell {
            gains.extend_from_slice(&cell[..]);
        }
        let backend_restore_snapshot = build_backend_restore_snapshot(
            model.backend_id(),
            model.backend_label(),
            SerializedEvaluationMode::PrecomputedCartesian,
            config,
        );
        Self {
            model,
            x_positions,
            y_positions,
            z_positions,
            gains,
            speaker_count,
            position_interpolation: config.position_interpolation,
            frozen_request: config.request_template,
            backend_restore_snapshot,
        }
    }
}

impl PreparedEvaluator for SampledCartesianEvaluator {
    fn speaker_count(&self) -> usize {
        self.speaker_count
    }

    fn compute_gains(&self, req: &RenderRequest) -> RenderResponse {
        // Read the table directly from native ADM coordinates. This avoids a render-time
        // round-trip through spherical/effect-space conversions for the cartesian path.
        let gains = sample_cartesian_table(
            &self.gains,
            self.speaker_count,
            &self.x_positions,
            &self.y_positions,
            &self.z_positions,
            req.adm_position.map(|value| value as f32),
            self.position_interpolation,
        );
        RenderResponse { gains }
    }

    fn save_to_file(&self, path: &std::path::Path, speaker_layout: &SpeakerLayout) -> Result<()> {
        std::fs::write(path, self.artifact_bytes(speaker_layout)?)?;
        Ok(())
    }

    fn artifact_bytes(&self, speaker_layout: &SpeakerLayout) -> Result<Vec<u8>> {
        evaluation_artifact::LoadedEvaluationArtifact::from_sampled_cartesian(
            self.model.backend_id(),
            self.model.backend_label(),
            speaker_layout,
            self.frozen_request,
            self.position_interpolation,
            self.backend_restore_snapshot.as_ref(),
            &self.x_positions,
            &self.y_positions,
            &self.z_positions,
            &self.gains,
            self.speaker_count,
        )?
        .to_serialized_bytes()
    }
}

pub struct SampledPolarEvaluator {
    model: Box<dyn GainModel>,
    azimuth_positions: Vec<f32>,
    elevation_positions: Vec<f32>,
    distance_positions: Vec<f32>,
    gains: Vec<f32>,
    speaker_count: usize,
    position_interpolation: bool,
    frozen_request: RenderRequest,
    backend_restore_snapshot: Option<BackendRestoreSnapshot>,
}

impl SampledPolarEvaluator {
    pub fn new(model: Box<dyn GainModel>, config: &EvaluationBuildConfig) -> Self {
        let azimuth_positions = polar_azimuth_axis(config.polar.azimuth_values.max(2));
        let elevation_positions = polar_elevation_axis(
            config.polar.elevation_values.max(2),
            config.polar.allow_negative_z,
        );
        let distance_positions = evenly_spaced_axis(
            config.polar.distance_values.max(2),
            0.0,
            config.polar.distance_max.max(0.01),
        );
        let speaker_count = model.speaker_count();
        let mut gains = Vec::with_capacity(
            azimuth_positions.len()
                * elevation_positions.len()
                * distance_positions.len()
                * speaker_count,
        );
        let mut request = config.request_template;
        for &distance in &distance_positions {
            for &elevation in &elevation_positions {
                for &azimuth in &azimuth_positions {
                    let (x, y, z) = spherical_to_adm(azimuth, elevation, distance);
                    request.adm_position = [x as f64, y as f64, z as f64];
                    gains.extend_from_slice(&model.compute_gains(&request).gains);
                }
            }
        }
        let backend_restore_snapshot = build_backend_restore_snapshot(
            model.backend_id(),
            model.backend_label(),
            SerializedEvaluationMode::PrecomputedPolar,
            config,
        );
        Self {
            model,
            azimuth_positions,
            elevation_positions,
            distance_positions,
            gains,
            speaker_count,
            position_interpolation: config.position_interpolation,
            frozen_request: config.request_template,
            backend_restore_snapshot,
        }
    }
}

impl PreparedEvaluator for SampledPolarEvaluator {
    fn speaker_count(&self) -> usize {
        self.speaker_count
    }

    fn compute_gains(&self, req: &RenderRequest) -> RenderResponse {
        let (azimuth, elevation, distance) = adm_to_spherical(
            req.adm_position[0] as f32,
            req.adm_position[1] as f32,
            req.adm_position[2] as f32,
        );
        let gains = sample_polar_table(
            &self.gains,
            self.speaker_count,
            &self.azimuth_positions,
            &self.elevation_positions,
            &self.distance_positions,
            [azimuth, elevation, distance],
            self.position_interpolation,
        );
        RenderResponse { gains }
    }

    fn save_to_file(&self, path: &std::path::Path, speaker_layout: &SpeakerLayout) -> Result<()> {
        std::fs::write(path, self.artifact_bytes(speaker_layout)?)?;
        Ok(())
    }

    fn artifact_bytes(&self, speaker_layout: &SpeakerLayout) -> Result<Vec<u8>> {
        evaluation_artifact::LoadedEvaluationArtifact::from_sampled_polar(
            self.model.backend_id(),
            self.model.backend_label(),
            speaker_layout,
            self.frozen_request,
            self.position_interpolation,
            self.backend_restore_snapshot.as_ref(),
            &self.azimuth_positions,
            &self.elevation_positions,
            &self.distance_positions,
            &self.gains,
            self.speaker_count,
        )?
        .to_serialized_bytes()
    }
}

pub struct RealtimeStrategy;

impl EvaluationStrategy for RealtimeStrategy {
    fn effective_mode(&self) -> EffectiveEvaluationMode {
        EffectiveEvaluationMode::Realtime
    }

    fn prepare(
        self,
        model: Box<dyn GainModel>,
        _config: &EvaluationBuildConfig,
    ) -> Result<Box<dyn PreparedEvaluator>> {
        Ok(Box::new(RealtimeEvaluator::new(model)))
    }
}

pub struct PrecomputedCartesianStrategy;

impl EvaluationStrategy for PrecomputedCartesianStrategy {
    fn effective_mode(&self) -> EffectiveEvaluationMode {
        EffectiveEvaluationMode::PrecomputedCartesian
    }

    fn prepare(
        self,
        model: Box<dyn GainModel>,
        config: &EvaluationBuildConfig,
    ) -> Result<Box<dyn PreparedEvaluator>> {
        Ok(Box::new(SampledCartesianEvaluator::new(model, config)))
    }
}

pub struct PrecomputedPolarStrategy;

impl EvaluationStrategy for PrecomputedPolarStrategy {
    fn effective_mode(&self) -> EffectiveEvaluationMode {
        EffectiveEvaluationMode::PrecomputedPolar
    }

    fn prepare(
        self,
        model: Box<dyn GainModel>,
        config: &EvaluationBuildConfig,
    ) -> Result<Box<dyn PreparedEvaluator>> {
        Ok(Box::new(SampledPolarEvaluator::new(model, config)))
    }
}

pub struct PreparedRenderEngine {
    gain_model_kind: GainModelKind,
    backend_id: &'static str,
    backend_label: &'static str,
    capabilities: BackendCapabilities,
    evaluation_mode: EffectiveEvaluationMode,
    backend_restore_snapshot: Option<BackendRestoreSnapshot>,
    evaluator: Box<dyn PreparedEvaluator>,
}

impl PreparedRenderEngine {
    pub fn new(
        gain_model_kind: GainModelKind,
        backend_id: &'static str,
        backend_label: &'static str,
        capabilities: BackendCapabilities,
        evaluation_mode: EffectiveEvaluationMode,
        backend_restore_snapshot: Option<BackendRestoreSnapshot>,
        evaluator: Box<dyn PreparedEvaluator>,
    ) -> Self {
        Self {
            gain_model_kind,
            backend_id,
            backend_label,
            capabilities,
            evaluation_mode,
            backend_restore_snapshot,
            evaluator,
        }
    }

    pub fn kind(&self) -> RenderBackendKind {
        self.gain_model_kind.into()
    }

    pub fn gain_model_kind(&self) -> GainModelKind {
        self.gain_model_kind
    }

    pub fn backend_id(&self) -> &'static str {
        self.backend_id
    }

    pub fn backend_label(&self) -> &'static str {
        self.backend_label
    }

    pub fn capabilities(&self) -> BackendCapabilities {
        self.capabilities
    }

    pub fn evaluation_mode(&self) -> EffectiveEvaluationMode {
        self.evaluation_mode
    }

    pub fn has_backend_restore_snapshot(&self) -> bool {
        self.backend_restore_snapshot.is_some()
    }

    pub fn backend_restore_snapshot(&self) -> Option<&BackendRestoreSnapshot> {
        self.backend_restore_snapshot.as_ref()
    }

    pub fn speaker_count(&self) -> usize {
        self.evaluator.speaker_count()
    }

    pub fn compute_gains(&self, req: &RenderRequest) -> RenderResponse {
        self.evaluator.compute_gains(req)
    }

    pub fn save_to_file(
        &self,
        path: &std::path::Path,
        speaker_layout: &SpeakerLayout,
    ) -> Result<()> {
        self.evaluator.save_to_file(path, speaker_layout)
    }

    /// Serialize the precomputed evaluation table (gains grid) to portable artifact
    /// bytes for shipping to clients. Errors on non-precomputed (realtime) backends.
    pub fn artifact_bytes(&self, speaker_layout: &SpeakerLayout) -> Result<Vec<u8>> {
        self.evaluator.artifact_bytes(speaker_layout)
    }
}

pub fn build_prepared_render_engine(
    model: Box<dyn GainModel>,
    evaluation_mode: EffectiveEvaluationMode,
    config: &EvaluationBuildConfig,
) -> Result<PreparedRenderEngine> {
    // Wrap the backend with the shared output stages, applied uniformly for
    // every backend. Order matters: distance diffuse blends + renormalizes, so
    // distance attenuation must wrap it (be applied last) or the renorm would
    // cancel the attenuation. Identity/metadata still delegate to the inner
    // backend; capabilities gain `supports_distance_diffuse` / `_model`.
    let model: Box<dyn GainModel> = Box::new(DistanceDiffuseModel::new(
        model,
        config.distance_diffuse_metric,
    ));
    let model: Box<dyn GainModel> = Box::new(DistanceAttenuatedModel::new(
        model,
        config.distance_model_metric,
    ));
    let gain_model_kind = model.kind();
    let backend_id = model.backend_id();
    let backend_label = model.backend_label();
    let capabilities = model.capabilities();
    let evaluator = match evaluation_mode {
        EffectiveEvaluationMode::Realtime => RealtimeStrategy.prepare(model, config)?,
        EffectiveEvaluationMode::PrecomputedCartesian => {
            PrecomputedCartesianStrategy.prepare(model, config)?
        }
        EffectiveEvaluationMode::PrecomputedPolar => {
            PrecomputedPolarStrategy.prepare(model, config)?
        }
    };
    Ok(PreparedRenderEngine::new(
        gain_model_kind,
        backend_id,
        backend_label,
        capabilities,
        evaluation_mode,
        None,
        evaluator,
    ))
}

#[derive(Clone, Copy)]
struct AxisSample {
    lower: usize,
    upper: usize,
    fraction: f32,
}

pub(crate) fn evenly_spaced_axis(count: usize, min: f32, max: f32) -> Vec<f32> {
    if count <= 1 {
        return vec![min];
    }
    let step = (max - min) / (count.saturating_sub(1) as f32);
    (0..count).map(|index| min + step * index as f32).collect()
}

pub(crate) fn cartesian_z_axis(z_size: usize, z_neg_size: usize) -> Vec<f32> {
    let mut values = Vec::with_capacity(z_neg_size + z_size);
    if z_neg_size > 0 {
        for index in 0..z_neg_size {
            let t = (index + 1) as f32 / z_neg_size as f32;
            values.push(-1.0 + (t - 1.0 / z_neg_size as f32));
        }
    }
    values.extend(evenly_spaced_axis(z_size.max(2), 0.0, 1.0));
    values
}

fn polar_azimuth_axis(count: usize) -> Vec<f32> {
    let count = count.max(2);
    let step = 360.0 / count as f32;
    (0..count)
        .map(|index| -180.0 + step * index as f32)
        .collect()
}

fn polar_elevation_axis(count: usize, allow_negative_z: bool) -> Vec<f32> {
    if allow_negative_z {
        evenly_spaced_axis(count.max(2), -90.0, 90.0)
    } else {
        evenly_spaced_axis(count.max(2), 0.0, 90.0)
    }
}

pub(crate) fn sample_cartesian_table(
    table: &[f32],
    speaker_count: usize,
    x_positions: &[f32],
    y_positions: &[f32],
    z_positions: &[f32],
    position: [f32; 3],
    interpolate: bool,
) -> Gains {
    let x = sample_axis(x_positions, position[0].clamp(-1.0, 1.0), interpolate);
    let y = sample_axis(y_positions, position[1].clamp(-1.0, 1.0), interpolate);
    let z = sample_axis(z_positions, position[2].clamp(-1.0, 1.0), interpolate);
    let mut gains = Gains::zeroed(speaker_count);
    if !interpolate {
        write_flat_sample(
            table,
            speaker_count,
            x_positions.len(),
            y_positions.len(),
            x.lower,
            y.lower,
            z.lower,
            &mut gains,
        );
        return gains;
    }

    for (iz, wz) in [(z.lower, 1.0 - z.fraction), (z.upper, z.fraction)] {
        for (iy, wy) in [(y.lower, 1.0 - y.fraction), (y.upper, y.fraction)] {
            for (ix, wx) in [(x.lower, 1.0 - x.fraction), (x.upper, x.fraction)] {
                let weight = wx * wy * wz;
                if weight <= 0.0 {
                    continue;
                }
                accumulate_flat_sample(
                    table,
                    speaker_count,
                    x_positions.len(),
                    y_positions.len(),
                    ix,
                    iy,
                    iz,
                    weight,
                    &mut gains,
                );
            }
        }
    }
    gains
}

pub(crate) fn sample_polar_table(
    table: &[f32],
    speaker_count: usize,
    azimuth_positions: &[f32],
    elevation_positions: &[f32],
    distance_positions: &[f32],
    position: [f32; 3],
    interpolate: bool,
) -> Gains {
    let azimuth = sample_wrapped_axis(azimuth_positions, wrap_degrees(position[0]), interpolate);
    let elevation = sample_axis(
        elevation_positions,
        position[1].clamp(
            *elevation_positions.first().unwrap_or(&-90.0),
            *elevation_positions.last().unwrap_or(&90.0),
        ),
        interpolate,
    );
    let distance = sample_axis(
        distance_positions,
        position[2].clamp(0.0, *distance_positions.last().unwrap_or(&0.0)),
        interpolate,
    );
    let mut gains = Gains::zeroed(speaker_count);
    if !interpolate {
        write_flat_sample(
            table,
            speaker_count,
            azimuth_positions.len(),
            elevation_positions.len(),
            azimuth.lower,
            elevation.lower,
            distance.lower,
            &mut gains,
        );
        return gains;
    }

    for (id, wd) in [
        (distance.lower, 1.0 - distance.fraction),
        (distance.upper, distance.fraction),
    ] {
        for (ie, we) in [
            (elevation.lower, 1.0 - elevation.fraction),
            (elevation.upper, elevation.fraction),
        ] {
            for (ia, wa) in [
                (azimuth.lower, 1.0 - azimuth.fraction),
                (azimuth.upper, azimuth.fraction),
            ] {
                let weight = wa * we * wd;
                if weight <= 0.0 {
                    continue;
                }
                accumulate_flat_sample(
                    table,
                    speaker_count,
                    azimuth_positions.len(),
                    elevation_positions.len(),
                    ia,
                    ie,
                    id,
                    weight,
                    &mut gains,
                );
            }
        }
    }
    gains
}

fn write_flat_sample(
    table: &[f32],
    speaker_count: usize,
    x_len: usize,
    y_len: usize,
    x_index: usize,
    y_index: usize,
    z_index: usize,
    gains: &mut Gains,
) {
    let offset = flat_sample_offset(speaker_count, x_len, y_len, x_index, y_index, z_index);
    // Slice both sides up front so the copy is bounds-check-free and vectorizable.
    gains[..speaker_count].copy_from_slice(&table[offset..offset + speaker_count]);
}

fn accumulate_flat_sample(
    table: &[f32],
    speaker_count: usize,
    x_len: usize,
    y_len: usize,
    x_index: usize,
    y_index: usize,
    z_index: usize,
    weight: f32,
    gains: &mut Gains,
) {
    let offset = flat_sample_offset(speaker_count, x_len, y_len, x_index, y_index, z_index);
    // Slice both sides so the weighted accumulation is bounds-check-free and the
    // compiler can vectorize the multiply-add over speakers.
    let row = &table[offset..offset + speaker_count];
    for (g, &t) in gains[..speaker_count].iter_mut().zip(row) {
        *g += t * weight;
    }
}

fn flat_sample_offset(
    speaker_count: usize,
    x_len: usize,
    y_len: usize,
    x_index: usize,
    y_index: usize,
    z_index: usize,
) -> usize {
    (((z_index * y_len) + y_index) * x_len + x_index) * speaker_count
}

fn sample_axis(values: &[f32], position: f32, interpolate: bool) -> AxisSample {
    if values.len() <= 1 {
        return AxisSample {
            lower: 0,
            upper: 0,
            fraction: 0.0,
        };
    }
    if !interpolate {
        let nearest = values
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| ((*a - position).abs()).total_cmp(&((*b - position).abs())))
            .map(|(index, _)| index)
            .unwrap_or(0);
        return AxisSample {
            lower: nearest,
            upper: nearest,
            fraction: 0.0,
        };
    }
    if position <= values[0] {
        return AxisSample {
            lower: 0,
            upper: 0,
            fraction: 0.0,
        };
    }
    // Fast path: assume an evenly-spaced axis (true for the cartesian x/y axes
    // and the polar elevation/distance axes) and jump straight to the bracket in
    // O(1). Verify the guess against its neighbours so a non-uniform axis (e.g.
    // the two-region cartesian z) correctly falls through to the binary search.
    // The fraction is computed from the stored grid values either way, so the
    // result is bit-identical to the search path.
    let last = values.len() - 1;
    let step = (values[last] - values[0]) / last as f32;
    if step > 0.0 {
        let guess = (((position - values[0]) / step) as usize).min(last - 1);
        if position >= values[guess] && position <= values[guess + 1] {
            let span = (values[guess + 1] - values[guess]).max(1e-6);
            return AxisSample {
                lower: guess,
                upper: guess + 1,
                fraction: ((position - values[guess]) / span).clamp(0.0, 1.0),
            };
        }
    }
    let upper = values.partition_point(|value| *value < position);
    if upper >= values.len() {
        let last = values.len() - 1;
        return AxisSample {
            lower: last,
            upper: last,
            fraction: 0.0,
        };
    }
    let lower = upper.saturating_sub(1);
    let span = (values[upper] - values[lower]).max(1e-6);
    AxisSample {
        lower,
        upper,
        fraction: ((position - values[lower]) / span).clamp(0.0, 1.0),
    }
}

fn sample_wrapped_axis(values: &[f32], position: f32, interpolate: bool) -> AxisSample {
    if values.len() <= 1 {
        return AxisSample {
            lower: 0,
            upper: 0,
            fraction: 0.0,
        };
    }
    if !interpolate {
        let nearest = values
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                wrapped_angle_distance(**a, position)
                    .total_cmp(&wrapped_angle_distance(**b, position))
            })
            .map(|(index, _)| index)
            .unwrap_or(0);
        return AxisSample {
            lower: nearest,
            upper: nearest,
            fraction: 0.0,
        };
    }
    let mut best = AxisSample {
        lower: 0,
        upper: 0,
        fraction: 0.0,
    };
    let mut best_distance = f32::MAX;
    for index in 0..values.len() {
        let next = (index + 1) % values.len();
        let start = values[index];
        let end = if next == 0 {
            values[0] + 360.0
        } else {
            values[next]
        };
        let value = if position < start {
            position + 360.0
        } else {
            position
        };
        if value < start || value > end {
            continue;
        }
        let span = (end - start).max(1e-6);
        return AxisSample {
            lower: index,
            upper: next,
            fraction: ((value - start) / span).clamp(0.0, 1.0),
        };
    }
    for (index, axis) in values.iter().enumerate() {
        let distance = wrapped_angle_distance(*axis, position);
        if distance < best_distance {
            best_distance = distance;
            best = AxisSample {
                lower: index,
                upper: index,
                fraction: 0.0,
            };
        }
    }
    best
}

#[inline]
fn wrap_degrees(value: f32) -> f32 {
    let wrapped = (value + 180.0).rem_euclid(360.0) - 180.0;
    if wrapped == -180.0 { 180.0 } else { wrapped }
}

#[inline]
fn wrapped_angle_distance(a: f32, b: f32) -> f32 {
    let delta = (a - b).abs().rem_euclid(360.0);
    delta.min(360.0 - delta)
}

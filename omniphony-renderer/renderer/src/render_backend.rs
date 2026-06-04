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

/// Borrowed view of a sampled cartesian evaluator's table + axes, used to merge
/// several per-band tables into one [`MultiBandCartesianTable`].
#[derive(Clone, Copy)]
pub(crate) struct CartesianParts<'a> {
    /// Flat gains grid, layout `[cell][speaker]` (band-local speakers).
    pub gains: &'a [f32],
    pub speaker_count: usize,
    pub x: &'a AxisLut,
    pub y: &'a AxisLut,
    pub z: &'a AxisLut,
    pub position_interpolation: bool,
}

pub trait PreparedEvaluator: Send + Sync {
    fn speaker_count(&self) -> usize;
    fn compute_gains(&self, req: &RenderRequest) -> RenderResponse;
    fn save_to_file(&self, path: &std::path::Path, speaker_layout: &SpeakerLayout) -> Result<()>;
    /// Borrow the sampled cartesian table + axes, when this evaluator is a
    /// precomputed cartesian one. Default `None` (realtime/polar). Crate-internal
    /// view type, used only to merge bands into a `MultiBandCartesianTable`.
    #[allow(private_interfaces)]
    fn cartesian_parts(&self) -> Option<CartesianParts<'_>> {
        None
    }
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
    // Precomputed division-free lookups for the runtime table read. Kept in sync
    // with the *_positions arrays above (the source of truth for serialization).
    x_lut: AxisLut,
    y_lut: AxisLut,
    z_lut: AxisLut,
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
        let x_lut = AxisLut::from_values(&x_positions);
        let y_lut = AxisLut::from_values(&y_positions);
        let z_lut = AxisLut::from_values(&z_positions);
        Self {
            model,
            x_positions,
            y_positions,
            z_positions,
            x_lut,
            y_lut,
            z_lut,
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

    fn cartesian_parts(&self) -> Option<CartesianParts<'_>> {
        Some(CartesianParts {
            gains: &self.gains,
            speaker_count: self.speaker_count,
            x: &self.x_lut,
            y: &self.y_lut,
            z: &self.z_lut,
            position_interpolation: self.position_interpolation,
        })
    }

    fn compute_gains(&self, req: &RenderRequest) -> RenderResponse {
        // Read the table directly from native ADM coordinates. This avoids a render-time
        // round-trip through spherical/effect-space conversions for the cartesian path.
        let gains = sample_cartesian_table(
            &self.gains,
            self.speaker_count,
            &self.x_lut,
            &self.y_lut,
            &self.z_lut,
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

    pub(crate) fn cartesian_parts(&self) -> Option<CartesianParts<'_>> {
        self.evaluator.cartesian_parts()
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

/// Precomputed per-axis lookup: turns a position into the `(lower, upper,
/// fraction)` bracket without a per-call division or binary search. Built once
/// when the table is created (`inv_step` = `1.0 / grid_step`), so the runtime
/// lookup is a multiply instead of the `partition_point` search + step/fraction
/// divisions that dominated the cartesian `compute_gains` cost.
#[derive(Clone)]
pub(crate) enum AxisLut {
    /// Evenly spaced grid: `values[k] == min + k / inv_step`.
    Uniform { min: f32, inv_step: f32, len: usize },
    /// Two evenly spaced regions joined at the value `0.0` (the cartesian z
    /// axis): indices `0..=split` span `[-1, 0]`, `split..len` span `[0, 1]`.
    SplitZero {
        split: usize,
        neg_inv_step: f32,
        pos_inv_step: f32,
        len: usize,
    },
    /// Arbitrary ascending values — falls back to the binary-search path.
    Irregular(Vec<f32>),
}

impl AxisLut {
    /// Classify an axis grid. Detects an evenly-spaced axis (x/y) or the two
    /// uniform-region cartesian z axis; anything else keeps the search path, so
    /// the result is always correct regardless of grid shape.
    pub(crate) fn from_values(values: &[f32]) -> Self {
        let n = values.len();
        if n < 2 {
            return Self::Irregular(values.to_vec());
        }
        // Returns inv_step iff values[lo..=hi] are evenly spaced.
        let uniform_inv_step = |lo: usize, hi: usize| -> Option<f32> {
            let step = (values[hi] - values[lo]) / (hi - lo) as f32;
            if step <= 0.0 {
                return None;
            }
            let tol = 1e-5 * step.max(1.0);
            for k in lo..=hi {
                let expected = values[lo] + (k - lo) as f32 * step;
                if (values[k] - expected).abs() > tol {
                    return None;
                }
            }
            Some(1.0 / step)
        };
        if let Some(inv_step) = uniform_inv_step(0, n - 1) {
            return Self::Uniform {
                min: values[0],
                inv_step,
                len: n,
            };
        }
        if let Some(split) = values.iter().position(|&v| v == 0.0) {
            if split > 0 && split < n - 1 {
                if let (Some(neg), Some(pos)) =
                    (uniform_inv_step(0, split), uniform_inv_step(split, n - 1))
                {
                    return Self::SplitZero {
                        split,
                        neg_inv_step: neg,
                        pos_inv_step: pos,
                        len: n,
                    };
                }
            }
        }
        Self::Irregular(values.to_vec())
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Uniform { len, .. } | Self::SplitZero { len, .. } => *len,
            Self::Irregular(values) => values.len(),
        }
    }

    /// Bracket within an evenly-spaced region of `len` points, given the
    /// position already expressed in cell units (`f = (pos - min) * inv_step`).
    fn bracket_uniform(f: f32, len: usize, interpolate: bool) -> AxisSample {
        let f = f.clamp(0.0, (len - 1) as f32);
        if !interpolate {
            let nearest = ((f + 0.5) as usize).min(len - 1);
            return AxisSample {
                lower: nearest,
                upper: nearest,
                fraction: 0.0,
            };
        }
        let lower = (f as usize).min(len - 2);
        AxisSample {
            lower,
            upper: lower + 1,
            fraction: f - lower as f32,
        }
    }

    fn sample(&self, position: f32, interpolate: bool) -> AxisSample {
        match self {
            Self::Uniform { min, inv_step, len } => {
                Self::bracket_uniform((position - min) * inv_step, *len, interpolate)
            }
            Self::SplitZero {
                split,
                neg_inv_step,
                pos_inv_step,
                len,
            } => {
                if position < 0.0 {
                    // Region [-1, 0] occupies indices 0..=split (split+1 points).
                    Self::bracket_uniform((position + 1.0) * neg_inv_step, split + 1, interpolate)
                } else {
                    // Region [0, 1] occupies indices split..len; offset back.
                    let mut s =
                        Self::bracket_uniform(position * pos_inv_step, len - split, interpolate);
                    s.lower += split;
                    s.upper += split;
                    s
                }
            }
            Self::Irregular(values) => sample_axis(values, position, interpolate),
        }
    }
}

pub(crate) fn sample_cartesian_table(
    table: &[f32],
    speaker_count: usize,
    x_axis: &AxisLut,
    y_axis: &AxisLut,
    z_axis: &AxisLut,
    position: [f32; 3],
    interpolate: bool,
) -> Gains {
    let x = x_axis.sample(position[0].clamp(-1.0, 1.0), interpolate);
    let y = y_axis.sample(position[1].clamp(-1.0, 1.0), interpolate);
    let z = z_axis.sample(position[2].clamp(-1.0, 1.0), interpolate);
    let x_len = x_axis.len();
    let y_len = y_axis.len();
    let mut gains = Gains::zeroed(speaker_count);
    if !interpolate {
        write_flat_sample(
            table,
            speaker_count,
            x_len,
            y_len,
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
                    table, speaker_count, x_len, y_len, ix, iy, iz, weight, &mut gains,
                );
            }
        }
    }
    gains
}

/// One cartesian table covering several crossover bands at once. The per-band
/// gains for each grid cell are stored contiguously (`[cell][band][speaker]`,
/// each band full-size with the speaker scatter baked in), so a lookup localises
/// the cell ONCE and accumulates every band's gains in a single pass — instead
/// of one full lookup (localise + accumulate + scatter) per band. The cost no
/// longer scales with the band count beyond the accumulation itself.
pub(crate) struct MultiBandCartesianTable {
    x: AxisLut,
    y: AxisLut,
    z: AxisLut,
    /// `[cell][band][num_speakers]`, row-major.
    gains: Vec<f32>,
    n_bands: usize,
    num_speakers: usize,
    position_interpolation: bool,
}

impl MultiBandCartesianTable {
    /// Merge per-band cartesian tables (band-local speakers) into the unified
    /// `[cell][band][num_speakers]` layout, baking each band's speaker scatter.
    /// All bands must share the same grid; returns `None` otherwise.
    pub(crate) fn build(bands: &[(CartesianParts<'_>, &[usize])], num_speakers: usize) -> Option<Self> {
        let (first, _) = bands.first()?;
        let x = first.x.clone();
        let y = first.y.clone();
        let z = first.z.clone();
        let position_interpolation = first.position_interpolation;
        let n_cells = x.len() * y.len() * z.len();
        let n_bands = bands.len();
        let mut gains = vec![0.0f32; n_cells * n_bands * num_speakers];
        for (b, (parts, indices)) in bands.iter().enumerate() {
            let sc = parts.speaker_count;
            if parts.gains.len() != n_cells * sc || indices.len() != sc {
                return None; // grid mismatch — fall back to the per-band path
            }
            for cell in 0..n_cells {
                let src = &parts.gains[cell * sc..cell * sc + sc];
                let dst_base = (cell * n_bands + b) * num_speakers;
                for (i, &g) in src.iter().enumerate() {
                    gains[dst_base + indices[i]] = g;
                }
            }
        }
        Some(Self {
            x,
            y,
            z,
            gains,
            n_bands,
            num_speakers,
            position_interpolation,
        })
    }

    /// Trilinear lookup for all bands at `position`. Fills `out` with `n_bands`
    /// full-size `Gains` (one localisation, contiguous per-cell accumulation).
    pub(crate) fn sample_into(&self, position: [f32; 3], out: &mut Vec<Gains>) {
        let interp = self.position_interpolation;
        let x = self.x.sample(position[0].clamp(-1.0, 1.0), interp);
        let y = self.y.sample(position[1].clamp(-1.0, 1.0), interp);
        let z = self.z.sample(position[2].clamp(-1.0, 1.0), interp);
        let x_len = self.x.len();
        let y_len = self.y.len();
        let band_stride = self.num_speakers;
        let cell_stride = self.n_bands * self.num_speakers;

        out.clear();
        out.resize(self.n_bands, Gains::zeroed(self.num_speakers));

        let table = &self.gains;
        let mut accumulate = |ix: usize, iy: usize, iz: usize, weight: f32| {
            let cell_base = (((iz * y_len) + iy) * x_len + ix) * cell_stride;
            for (b, g) in out.iter_mut().enumerate() {
                let base = cell_base + b * band_stride;
                let src = &table[base..base + band_stride];
                for (d, &s) in g[..band_stride].iter_mut().zip(src) {
                    *d += s * weight;
                }
            }
        };

        if !interp {
            accumulate(x.lower, y.lower, z.lower, 1.0);
            return;
        }
        for (iz, wz) in [(z.lower, 1.0 - z.fraction), (z.upper, z.fraction)] {
            for (iy, wy) in [(y.lower, 1.0 - y.fraction), (y.upper, y.fraction)] {
                for (ix, wx) in [(x.lower, 1.0 - x.fraction), (x.upper, x.fraction)] {
                    let weight = wx * wy * wz;
                    if weight <= 0.0 {
                        continue;
                    }
                    accumulate(ix, iy, iz, weight);
                }
            }
        }
    }
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

#[cfg(test)]
mod cartesian_lookup_bench {
    //! Microbench of the cartesian table lookup, contrasting the production
    //! `AxisLut` (division-free `inv_step` index for x/y + split z) against the
    //! generic binary-search path (`AxisLut::Irregular`, the pre-optimisation
    //! behaviour). Both go through `sample_cartesian_table`, so this isolates the
    //! lookup-localisation cost the `inv_step` precompute removed.
    //!
    //! Run: cargo test -p renderer --release cartesian_lookup_bench -- --ignored --nocapture
    use super::*;
    use std::hint::black_box;
    use std::time::Instant;

    #[test]
    #[ignore = "microbenchmark, run explicitly: cargo test -p renderer --release \
                cartesian_lookup_bench -- --ignored --nocapture"]
    fn inv_step_vs_search() {
        let sc = 12usize;
        let xs = evenly_spaced_axis(31, -1.0, 1.0);
        let ys = evenly_spaced_axis(31, -1.0, 1.0);
        let zs = cartesian_z_axis(15, 15);
        let (nx, ny, nz) = (xs.len(), ys.len(), zs.len());
        let mut table = vec![0.0f32; nx * ny * nz * sc];
        for (i, v) in table.iter_mut().enumerate() {
            *v = ((i * 2654435761) % 1000) as f32 / 1000.0;
        }

        // Production luts (uniform x/y, split z) vs forced binary-search luts.
        let (fx, fy, fz) = (
            AxisLut::from_values(&xs),
            AxisLut::from_values(&ys),
            AxisLut::from_values(&zs),
        );
        let (sx, sy, sz) = (
            AxisLut::Irregular(xs.clone()),
            AxisLut::Irregular(ys.clone()),
            AxisLut::Irregular(zs.clone()),
        );

        let n = 40;
        let positions: Vec<[f32; 3]> = (0..n)
            .map(|s| {
                let t = s as f32 / (n - 1) as f32;
                [-0.3 + 0.6 * t, -0.2 + 0.4 * t, -0.1 + 0.3 * t]
            })
            .collect();

        let reps = 300_000;
        let run = |x: &AxisLut, y: &AxisLut, z: &AxisLut| {
            for p in &positions {
                black_box(sample_cartesian_table(&table, sc, x, y, z, *p, true));
            }
        };
        run(&fx, &fy, &fz); // warm up

        let t0 = Instant::now();
        for _ in 0..reps {
            run(&fx, &fy, &fz);
        }
        let inv = t0.elapsed();

        let t1 = Instant::now();
        for _ in 0..reps {
            run(&sx, &sy, &sz);
        }
        let search = t1.elapsed();

        let calls = (reps * n) as f64;
        let inv_ns = inv.as_secs_f64() * 1e9 / calls;
        let search_ns = search.as_secs_f64() * 1e9 / calls;
        eprintln!(
            "cartesian lookup: inv_step {inv_ns:.1} ns/call | binary-search {search_ns:.1} ns/call \
             | inv_step is {:.0}% faster",
            (search_ns - inv_ns) / search_ns * 100.0,
        );
    }
}

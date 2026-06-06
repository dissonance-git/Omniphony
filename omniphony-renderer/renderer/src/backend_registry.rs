use std::sync::Arc;

use anyhow::Result;

use crate::live_params::{
    BackendRebuildParams, LiveEvaluationMode, LiveParams, PreferredEvaluationMode, RenderTopology,
};
use crate::render_backend::{
    BlendCurve, EffectiveEvaluationMode, EvaluationBuildConfig, FewSpeakerBackend, GainModel,
    GainModelKind, HybridBackend, PreparedRenderEngine, RenderBackendKind,
    backend_descriptor_by_id, build_prepared_render_engine, wrap_prepared_engine,
};
use crate::speaker_layout::SpeakerLayout;

/// Reference positions probed by [`smoke_test_engine`]: scene centre, the eight
/// cube corners, and a few off-axis points. They are intentionally cheap and
/// fixed — the goal is to exercise a freshly built backend once, on the build
/// thread, not to characterise it.
const SMOKE_TEST_POSITIONS: [[f64; 3]; 11] = [
    [0.0, 0.0, 0.0],
    [1.0, 1.0, 1.0],
    [-1.0, -1.0, -1.0],
    [1.0, -1.0, 1.0],
    [-1.0, 1.0, -1.0],
    [1.0, 1.0, -1.0],
    [-1.0, -1.0, 1.0],
    [1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.5, -0.5, 0.25],
];

/// Extract a human-readable message from a `catch_unwind` panic payload.
fn panic_detail(payload: &(dyn std::any::Any + Send)) -> String {
    if let Some(msg) = payload.downcast_ref::<&'static str>() {
        (*msg).to_string()
    } else if let Some(msg) = payload.downcast_ref::<String>() {
        msg.clone()
    } else {
        "panic with non-string payload".to_string()
    }
}

/// Exercise a freshly built engine on a handful of reference positions, on the
/// build thread, so a misbehaving backend is rejected here instead of taking
/// down the realtime audio thread on its first frame.
///
/// This is the build-time guard behind the [`GainModel`] hot-path contract: a
/// contributor backend that panics, returns the wrong number of gains, or emits
/// a non-finite gain turns into a plain `Err` from topology construction (which
/// the OSC recompute path already surfaces to Studio), never an uncaught panic
/// in `SpatialRenderer::render_frame`.
fn smoke_test_engine(
    engine: &PreparedRenderEngine,
    config: &EvaluationBuildConfig,
    backend_id: &str,
) -> Result<()> {
    let expected = engine.speaker_count();
    for position in SMOKE_TEST_POSITIONS {
        let mut request = config.request_template;
        request.adm_position = position;

        let response = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            engine.compute_gains(&request)
        }))
        .map_err(|payload| {
            anyhow::anyhow!(
                "backend '{backend_id}' panicked in compute_gains at position {position:?}: {}",
                panic_detail(payload.as_ref())
            )
        })?;

        if response.gains.len() != expected {
            return Err(anyhow::anyhow!(
                "backend '{backend_id}' returned {} gains at position {position:?}, expected {expected}",
                response.gains.len()
            ));
        }
        if let Some((speaker, gain)) = response
            .gains
            .iter()
            .enumerate()
            .find(|(_, gain)| !gain.is_finite())
        {
            return Err(anyhow::anyhow!(
                "backend '{backend_id}' returned non-finite gain {gain} for speaker {speaker} at position {position:?}"
            ));
        }
    }
    Ok(())
}

#[derive(Clone)]
pub enum BackendBuildPlan {
    Vbap(VbapTopologyBuildPlan),
    /// VBAP for degenerate (1–2 speaker) geometry, where the panner cannot
    /// triangulate. Substituted for `Vbap` by `build_vbap_build_plan` when the
    /// resolved layout has fewer than 3 spatializable speakers.
    FewSpeaker(FewSpeakerBuildPlan),
    Barycenter(BarycenterBuildPlan),
    ExperimentalDistance(ExperimentalDistanceBuildPlan),
    Hybrid(HybridBuildPlan),
}

impl BackendBuildPlan {
    /// Build the gain model for this plan as a realtime model. Used both when a
    /// backend is the top-level model and when it is an inner model of the
    /// hybrid backend (which queries `compute_gains` directly).
    pub fn build_gain_model(&self) -> Result<Box<dyn GainModel>> {
        match self {
            BackendBuildPlan::Vbap(plan) => plan.build_gain_model(LiveEvaluationMode::Realtime),
            BackendBuildPlan::FewSpeaker(plan) => plan.build_gain_model(),
            BackendBuildPlan::Barycenter(plan) => plan.build_gain_model(),
            BackendBuildPlan::ExperimentalDistance(plan) => plan.build_gain_model(),
            BackendBuildPlan::Hybrid(plan) => plan.build_gain_model(),
        }
    }
}

#[derive(Clone)]
pub struct FewSpeakerBuildPlan {
    /// Speaker `[azimuth, elevation]` in degrees (room-adjusted), 1 or 2 entries.
    pub positions: Vec<[f32; 2]>,
}

impl FewSpeakerBuildPlan {
    pub fn build_gain_model(&self) -> Result<Box<dyn GainModel>> {
        Ok(Box::new(FewSpeakerBackend::new(self.positions.clone())))
    }
}

#[derive(Clone)]
pub struct HybridBuildPlan {
    pub external: Box<BackendBuildPlan>,
    pub internal: Box<BackendBuildPlan>,
    pub curve: Vec<[f32; 2]>,
    pub curve_smoothing: f32,
    pub metric: crate::spatial_vbap::DistanceMetric,
}

impl HybridBuildPlan {
    pub fn build_gain_model(&self) -> Result<Box<dyn GainModel>> {
        let external = self.external.build_gain_model()?;
        let internal = self.internal.build_gain_model()?;
        Ok(Box::new(HybridBackend::new(
            external,
            internal,
            BlendCurve::new(self.curve.clone(), self.curve_smoothing),
            self.metric,
        )))
    }
}

#[derive(Clone)]
pub struct ExperimentalDistanceBuildPlan {
    pub speaker_positions: Vec<[f32; 3]>,
}

#[derive(Clone)]
pub struct BarycenterBuildPlan {
    pub speaker_positions: Vec<[f32; 3]>,
}

#[derive(Clone)]
pub struct VbapTopologyBuildPlan {
    pub layout: SpeakerLayout,
    pub positions: Vec<[f32; 2]>,
    pub azimuth_resolution: i32,
    pub elevation_resolution: i32,
    pub distance_res: f32,
    pub distance_max: f32,
    pub allow_negative_z: bool,
    pub distance_model: crate::spatial_vbap::DistanceModel,
    pub spread_min: f32,
    pub spread_max: f32,
    pub spread_from_distance: bool,
    pub spread_distance_range: f32,
    pub spread_distance_curve: f32,
    pub room_ratio: [f32; 3],
    pub room_ratio_rear: f32,
    pub room_ratio_lower: f32,
    pub room_ratio_center_blend: f32,
    pub diffuse: bool,
    pub diffuse_thr: f32,
    pub diffuse_curve: f32,
}

impl VbapTopologyBuildPlan {
    pub fn build_gain_model(
        &self,
        _evaluation_mode: LiveEvaluationMode,
    ) -> Result<Box<dyn GainModel>> {
        // The panner is geometry-only: it computes gains directly per position and
        // owns no table, so the evaluation mode does not affect how it is built.
        // Any precomputation happens in the evaluation layer that samples it.
        let vbap = crate::spatial_vbap::VbapPanner::new(
            &self.positions,
            self.azimuth_resolution,
            self.elevation_resolution,
            0.0,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create VBAP panner: {}", e))?
        .with_negative_z(self.allow_negative_z);

        Ok(Box::new(crate::render_backend::VbapBackend::new(vbap)))
    }
}

impl ExperimentalDistanceBuildPlan {
    pub fn build_gain_model(&self) -> Result<Box<dyn GainModel>> {
        Ok(Box::new(
            crate::render_backend::ExperimentalDistanceBackend::new(self.speaker_positions.clone()),
        ))
    }
}

impl BarycenterBuildPlan {
    pub fn build_gain_model(&self) -> Result<Box<dyn GainModel>> {
        Ok(Box::new(crate::render_backend::BarycenterBackend::new(
            self.speaker_positions.clone(),
        )))
    }
}

#[derive(Clone)]
pub struct TopologyBuildPlan {
    pub layout: SpeakerLayout,
    pub backend_id: String,
    pub backend_build: BackendBuildPlan,
    pub evaluation_mode: LiveEvaluationMode,
    pub evaluation_build_config: crate::render_backend::EvaluationBuildConfig,
    /// The geometry generation captured when this plan was prepared. The built
    /// topology records it; a later recompute compares to decide whether the gain
    /// models can be reused (see `build_topology_reusing`). Set by
    /// `RendererControl::prepare_topology_rebuild_for_layout`.
    pub geometry_generation: u64,
}

impl TopologyBuildPlan {
    pub fn build_topology(&self) -> Result<RenderTopology> {
        self.build_topology_reusing(None)
    }

    /// Build the topology, reusing `current`'s decorated gain model when the
    /// geometry generation is unchanged (only the evaluation mode / grid changed).
    /// Reuse skips re-triangulation: realtime just re-wraps the model, precomputed
    /// re-samples it. A geometry change (different generation, or no current model)
    /// falls back to a full rebuild.
    pub fn build_topology_reusing(
        &self,
        current: Option<&RenderTopology>,
    ) -> Result<RenderTopology> {
        let effective_mode = match self.evaluation_mode {
            LiveEvaluationMode::Realtime => EffectiveEvaluationMode::Realtime,
            LiveEvaluationMode::PrecomputedPolar => EffectiveEvaluationMode::PrecomputedPolar,
            LiveEvaluationMode::PrecomputedCartesian => {
                EffectiveEvaluationMode::PrecomputedCartesian
            }
            LiveEvaluationMode::Auto => unreachable!("topology build plan must resolve auto mode"),
        };

        if let Some(model) = current.and_then(|cur| {
            (cur.geometry_generation == self.geometry_generation)
                .then(|| cur.backend.decorated_model())
                .flatten()
        }) {
            let engine =
                wrap_prepared_engine(model, effective_mode, &self.evaluation_build_config)?;
            let topology = RenderTopology::new(Arc::new(engine), self.layout.clone())?
                .with_geometry_generation(self.geometry_generation);
            smoke_test_engine(
                &topology.backend,
                &self.evaluation_build_config,
                self.backend_id(),
            )?;
            return Ok(topology);
        }

        // The panner is geometry-only and ignores the evaluation mode, so the
        // shared realtime builder applies to every backend (the mode is resolved
        // later by `build_prepared_render_engine`'s evaluation wrapper).
        let model = self.backend_build.build_gain_model()?;
        let topology = RenderTopology::new(
            Arc::new(build_prepared_render_engine(
                model,
                effective_mode,
                &self.evaluation_build_config,
            )?),
            self.layout.clone(),
        )?
        .with_geometry_generation(self.geometry_generation);
        smoke_test_engine(
            &topology.backend,
            &self.evaluation_build_config,
            self.backend_id(),
        )?;
        Ok(topology)
    }

    pub fn backend_id(&self) -> &str {
        self.backend_id.as_str()
    }

    pub fn backend_kind(&self) -> Option<RenderBackendKind> {
        RenderBackendKind::from_str(self.backend_id())
    }

    pub fn gain_model_kind(&self) -> GainModelKind {
        backend_descriptor_by_id(self.backend_id())
            .map(|descriptor| descriptor.gain_model_kind)
            .unwrap_or(GainModelKind::Vbap)
    }

    pub fn evaluation_mode(&self) -> LiveEvaluationMode {
        self.evaluation_mode
    }

    pub fn layout(&self) -> &SpeakerLayout {
        &self.layout
    }

    pub fn log_summary(&self) -> String {
        match &self.backend_build {
            BackendBuildPlan::Vbap(plan) => format!(
                "gain_model=vbap evaluation_mode={} azimuth_resolution={} elevation_resolution={} distance_res={} distance_max={}",
                self.evaluation_mode().as_str(),
                plan.azimuth_resolution,
                plan.elevation_resolution,
                plan.distance_res,
                plan.distance_max,
            ),
            BackendBuildPlan::FewSpeaker(plan) => format!(
                "gain_model=vbap(few-speaker) evaluation_mode={} speakers={}",
                self.evaluation_mode().as_str(),
                plan.positions.len()
            ),
            BackendBuildPlan::ExperimentalDistance(plan) => format!(
                "gain_model=experimental_distance evaluation_mode={} speakers={}",
                self.evaluation_mode().as_str(),
                plan.speaker_positions.len()
            ),
            BackendBuildPlan::Barycenter(plan) => format!(
                "gain_model=barycenter evaluation_mode={} speakers={}",
                self.evaluation_mode().as_str(),
                plan.speaker_positions.len()
            ),
            BackendBuildPlan::Hybrid(plan) => format!(
                "gain_model=hybrid evaluation_mode={} external={} internal={} curve_points={}",
                self.evaluation_mode().as_str(),
                inner_backend_summary(&plan.external),
                inner_backend_summary(&plan.internal),
                plan.curve.len()
            ),
        }
    }
}

fn inner_backend_summary(plan: &BackendBuildPlan) -> &'static str {
    match plan {
        BackendBuildPlan::Vbap(_) => "vbap",
        BackendBuildPlan::FewSpeaker(_) => "vbap",
        BackendBuildPlan::Barycenter(_) => "barycenter",
        BackendBuildPlan::ExperimentalDistance(_) => "experimental_distance",
        BackendBuildPlan::Hybrid(_) => "hybrid",
    }
}

fn effective_live_evaluation_mode(
    requested: LiveEvaluationMode,
    preferred: PreferredEvaluationMode,
) -> LiveEvaluationMode {
    match requested {
        LiveEvaluationMode::Auto => match preferred {
            PreferredEvaluationMode::PrecomputedPolar => LiveEvaluationMode::PrecomputedPolar,
            PreferredEvaluationMode::PrecomputedCartesian => {
                LiveEvaluationMode::PrecomputedCartesian
            }
        },
        mode => mode,
    }
}

fn collect_spatializable_positions(layout: &SpeakerLayout) -> Vec<[f32; 3]> {
    layout
        .speakers
        .iter()
        .filter(|speaker| speaker.spatialize)
        .map(|speaker| [speaker.x, speaker.y, speaker.z])
        .collect()
}

/// Build the VBAP build plan for the given (already resolved) evaluation mode.
/// Shared by the top-level VBAP backend and by hybrid inner models (which pass
/// `Realtime`, since the hybrid backend queries `compute_gains` directly).
fn build_vbap_build_plan(
    layout: &SpeakerLayout,
    live: &LiveParams,
    rebuild_params: BackendRebuildParams,
) -> Option<BackendBuildPlan> {
    let rebuild = rebuild_params.vbap?;
    let positions = layout
        .spatializable_positions_for_room(
            live.room_ratio,
            live.room_ratio_rear,
            live.room_ratio_lower,
            live.room_ratio_center_blend,
        )
        .0;
    let azimuth_resolution = if live.evaluation.polar.azimuth_values > 0 {
        ((360.0f32 / (live.evaluation.polar.azimuth_values as f32)).round() as i32).clamp(1, 360)
    } else {
        rebuild.az_res_deg.clamp(1, 360)
    };
    let elevation_resolution = if live.evaluation.polar.elevation_values > 0 {
        (((if rebuild.allow_negative_z {
            180.0
        } else {
            90.0
        }) / (live.evaluation.polar.elevation_values as f32))
            .round() as i32)
            .clamp(1, if rebuild.allow_negative_z { 180 } else { 90 })
    } else {
        rebuild
            .el_res_deg
            .clamp(1, if rebuild.allow_negative_z { 180 } else { 90 })
    };
    let distance_max = if live.evaluation.polar.distance_max > 0.0 {
        live.evaluation.polar.distance_max
    } else {
        rebuild.distance_max.max(0.01)
    };
    let distance_res = if live.evaluation.polar.distance_res > 0 {
        distance_max / (live.evaluation.polar.distance_res as f32)
    } else if rebuild.spread_resolution > 0.0 {
        rebuild.spread_resolution
    } else {
        0.25
    };

    // Fewer than 3 spatializable speakers can't be triangulated: pan them with
    // the degenerate-VBAP backend (same direction-only model) instead.
    if positions.len() < 3 {
        return Some(BackendBuildPlan::FewSpeaker(FewSpeakerBuildPlan {
            positions,
        }));
    }

    Some(BackendBuildPlan::Vbap(VbapTopologyBuildPlan {
        layout: layout.clone(),
        positions,
        azimuth_resolution,
        elevation_resolution,
        distance_res,
        distance_max,
        allow_negative_z: rebuild.allow_negative_z,
        distance_model: live.distance_model,
        spread_min: live.spread_min,
        spread_max: live.spread_max,
        spread_from_distance: live.spread_from_distance,
        spread_distance_range: live.spread_distance_range,
        spread_distance_curve: live.spread_distance_curve,
        room_ratio: live.room_ratio,
        room_ratio_rear: live.room_ratio_rear,
        room_ratio_lower: live.room_ratio_lower,
        room_ratio_center_blend: live.room_ratio_center_blend,
        diffuse: live.use_distance_diffuse,
        diffuse_thr: live.distance_diffuse_threshold,
        diffuse_curve: live.distance_diffuse_curve,
    }))
}

/// Build a `BackendBuildPlan` for one of the concrete (non-hybrid) backends.
/// Used directly by the top-level barycenter/experimental_distance branches and
/// by the hybrid backend for each of its inner models. Returns `None` for an
/// unknown id or `"hybrid"` (no nested hybrids).
fn build_inner_backend_plan(
    backend_id: &str,
    layout: &SpeakerLayout,
    live: &LiveParams,
    backend_rebuild_params: Option<BackendRebuildParams>,
) -> Option<BackendBuildPlan> {
    match backend_id {
        "barycenter" => Some(BackendBuildPlan::Barycenter(BarycenterBuildPlan {
            speaker_positions: collect_spatializable_positions(layout),
        })),
        "experimental_distance" => Some(BackendBuildPlan::ExperimentalDistance(
            ExperimentalDistanceBuildPlan {
                speaker_positions: collect_spatializable_positions(layout),
            },
        )),
        "vbap" => {
            let rebuild_params = backend_rebuild_params?;
            build_vbap_build_plan(layout, live, rebuild_params)
        }
        _ => None,
    }
}

fn preferred_evaluation_mode(
    backend_rebuild_params: Option<BackendRebuildParams>,
) -> PreferredEvaluationMode {
    backend_rebuild_params
        .map(|params| params.preferred_evaluation_mode())
        .unwrap_or(PreferredEvaluationMode::PrecomputedCartesian)
}

/// Inputs available to a [`BackendFactory`] when it builds its plan: the resolved
/// speaker layout, the live parameters, and the optional geometry rebuild params
/// (present for backends that need triangulation, e.g. VBAP).
pub struct BackendBuildCtx<'a> {
    pub layout: &'a SpeakerLayout,
    pub live: &'a LiveParams,
    pub backend_rebuild_params: Option<BackendRebuildParams>,
}

/// A render backend's registration entry: a stable id plus how to build its gain
/// model plan for a given context.
///
/// Implement this and `register` it into a [`BackendRegistry`] to add a backend
/// without editing the central dispatch. Identity is data (a string id), not an
/// enum variant, so a backend can live in its own crate.
pub trait BackendFactory: Send + Sync {
    /// Stable identifier matched against `LiveParams::backend_id()` (e.g. `"vbap"`).
    fn id(&self) -> &'static str;
    /// Build this backend's plan, or `None` if it cannot be prepared for the
    /// given context (e.g. VBAP without geometry rebuild params).
    fn build_plan(&self, ctx: &BackendBuildCtx<'_>) -> Option<BackendBuildPlan>;
}

/// Ordered set of backend factories keyed by id. Use [`BackendRegistry::builtin`]
/// for the shipped backends; a host can `register` additional ones at startup.
pub struct BackendRegistry {
    factories: Vec<Box<dyn BackendFactory>>,
}

impl BackendRegistry {
    pub fn new() -> Self {
        Self {
            factories: Vec::new(),
        }
    }

    /// Registry preloaded with the built-in backends.
    pub fn builtin() -> Self {
        let mut registry = Self::new();
        registry.register(Box::new(VbapFactory));
        registry.register(Box::new(BarycenterFactory));
        registry.register(Box::new(ExperimentalDistanceFactory));
        registry.register(Box::new(HybridFactory));
        registry
    }

    /// Register a backend. A later registration with the same id replaces the
    /// earlier one, so a host can override a built-in.
    pub fn register(&mut self, factory: Box<dyn BackendFactory>) {
        let id = factory.id();
        match self.factories.iter_mut().find(|f| f.id() == id) {
            Some(slot) => *slot = factory,
            None => self.factories.push(factory),
        }
    }

    /// Look up a factory by id.
    pub fn get(&self, id: &str) -> Option<&dyn BackendFactory> {
        self.factories
            .iter()
            .find(|f| f.id() == id)
            .map(|f| f.as_ref())
    }

    /// Ids of all registered backends, in registration order.
    pub fn ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.factories.iter().map(|f| f.id())
    }
}

impl Default for BackendRegistry {
    fn default() -> Self {
        Self::builtin()
    }
}

struct VbapFactory;
impl BackendFactory for VbapFactory {
    fn id(&self) -> &'static str {
        "vbap"
    }
    fn build_plan(&self, ctx: &BackendBuildCtx<'_>) -> Option<BackendBuildPlan> {
        build_vbap_build_plan(ctx.layout, ctx.live, ctx.backend_rebuild_params?)
    }
}

struct BarycenterFactory;
impl BackendFactory for BarycenterFactory {
    fn id(&self) -> &'static str {
        "barycenter"
    }
    fn build_plan(&self, ctx: &BackendBuildCtx<'_>) -> Option<BackendBuildPlan> {
        Some(BackendBuildPlan::Barycenter(BarycenterBuildPlan {
            speaker_positions: collect_spatializable_positions(ctx.layout),
        }))
    }
}

struct ExperimentalDistanceFactory;
impl BackendFactory for ExperimentalDistanceFactory {
    fn id(&self) -> &'static str {
        "experimental_distance"
    }
    fn build_plan(&self, ctx: &BackendBuildCtx<'_>) -> Option<BackendBuildPlan> {
        Some(BackendBuildPlan::ExperimentalDistance(
            ExperimentalDistanceBuildPlan {
                speaker_positions: collect_spatializable_positions(ctx.layout),
            },
        ))
    }
}

struct HybridFactory;
impl BackendFactory for HybridFactory {
    fn id(&self) -> &'static str {
        "hybrid"
    }
    fn build_plan(&self, ctx: &BackendBuildCtx<'_>) -> Option<BackendBuildPlan> {
        // The hybrid backend composes two inner backends. Inner composition still
        // goes through `build_inner_backend_plan` (vbap / barycenter /
        // experimental_distance), so its supported inner set is unchanged.
        let external = build_inner_backend_plan(
            &ctx.live.hybrid.external_backend_id,
            ctx.layout,
            ctx.live,
            ctx.backend_rebuild_params,
        )?;
        let internal = build_inner_backend_plan(
            &ctx.live.hybrid.internal_backend_id,
            ctx.layout,
            ctx.live,
            ctx.backend_rebuild_params,
        )?;
        Some(BackendBuildPlan::Hybrid(HybridBuildPlan {
            external: Box::new(external),
            internal: Box::new(internal),
            curve: ctx.live.hybrid.curve.clone(),
            curve_smoothing: ctx.live.hybrid.curve_smoothing,
            metric: ctx.live.hybrid.metric,
        }))
    }
}

pub fn prepare_topology_build_plan(
    layout: SpeakerLayout,
    live: &LiveParams,
    backend_rebuild_params: Option<BackendRebuildParams>,
    evaluation_build_config: crate::render_backend::EvaluationBuildConfig,
) -> Option<TopologyBuildPlan> {
    // Dispatch through the registry instead of a hard-coded `match` on the id, so
    // a backend's construction lives with the backend rather than here.
    let registry = BackendRegistry::builtin();
    let factory = registry.get(live.backend_id())?;
    let ctx = BackendBuildCtx {
        layout: &layout,
        live,
        backend_rebuild_params,
    };
    let backend_build = factory.build_plan(&ctx)?;
    let preferred = preferred_evaluation_mode(backend_rebuild_params);
    let evaluation_mode = effective_live_evaluation_mode(live.evaluation.mode, preferred);
    Some(TopologyBuildPlan {
        layout,
        backend_id: live.backend_id().to_string(),
        backend_build,
        evaluation_mode,
        evaluation_build_config,
        geometry_generation: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render_backend::{
        BackendCapabilities, CartesianEvaluationConfig, PolarEvaluationConfig, RenderRequest,
        RenderResponse,
    };
    use crate::spatial_vbap::{DistanceMetric, DistanceModel, Gains};

    const TEST_SPEAKERS: usize = 4;

    fn realtime_caps() -> BackendCapabilities {
        BackendCapabilities {
            supports_realtime: true,
            supports_precomputed_polar: false,
            supports_precomputed_cartesian: false,
            supports_position_interpolation: false,
            supports_distance_model: false,
            supports_spread: false,
            supports_spread_from_distance: false,
            supports_event_size: false,
            supports_distance_diffuse: false,
            supports_table_export: false,
        }
    }

    fn build_config() -> EvaluationBuildConfig {
        EvaluationBuildConfig {
            request_template: RenderRequest {
                adm_position: [0.0, 0.0, 0.0],
                event_size: [0.0; 3],
                size_to_spread_mode: Default::default(),
                spread_min: 0.0,
                spread_max: 0.0,
                spread_from_distance: false,
                spread_distance_range: 1.0,
                spread_distance_curve: 1.0,
                room_ratio: [1.0, 1.0, 1.0],
                room_ratio_rear: 1.0,
                room_ratio_lower: 1.0,
                room_ratio_center_blend: 0.5,
                use_distance_diffuse: false,
                distance_diffuse_threshold: 1.0,
                distance_diffuse_curve: 1.0,
                distance_model: DistanceModel::default(),
                barycenter_localize: 0.0,
                experimental_distance_distance_floor: 0.0,
                experimental_distance_min_active_speakers: 1,
                experimental_distance_max_active_speakers: 1,
                experimental_distance_position_error_floor: 0.0,
                experimental_distance_position_error_nearest_scale: 0.0,
                experimental_distance_position_error_span_scale: 0.0,
            },
            position_interpolation: false,
            cartesian: CartesianEvaluationConfig {
                x_size: 5,
                y_size: 5,
                z_size: 3,
                z_neg_size: 0,
            },
            polar: PolarEvaluationConfig {
                azimuth_values: 8,
                elevation_values: 5,
                distance_values: 4,
                distance_max: 1.0,
                allow_negative_z: false,
            },
            distance_model_metric: DistanceMetric::default(),
            distance_diffuse_metric: DistanceMetric::default(),
        }
    }

    /// Build a realtime engine for `model`. Realtime wrapping does not sample the
    /// model, so a backend that misbehaves in `compute_gains` builds fine here and
    /// is only caught by the smoke test — exactly the case we want to cover.
    fn realtime_engine(model: Box<dyn GainModel>) -> PreparedRenderEngine {
        build_prepared_render_engine(model, EffectiveEvaluationMode::Realtime, &build_config())
            .expect("realtime engine builds")
    }

    macro_rules! fake_backend {
        ($name:ident, $id:literal, $compute:expr) => {
            struct $name;
            impl GainModel for $name {
                fn kind(&self) -> GainModelKind {
                    GainModelKind::Vbap
                }
                fn backend_id(&self) -> &'static str {
                    $id
                }
                fn backend_label(&self) -> &'static str {
                    $id
                }
                fn capabilities(&self) -> BackendCapabilities {
                    realtime_caps()
                }
                fn speaker_count(&self) -> usize {
                    TEST_SPEAKERS
                }
                fn compute_gains(&self, _req: &RenderRequest) -> RenderResponse {
                    $compute
                }
                fn save_to_file(
                    &self,
                    _path: &std::path::Path,
                    _layout: &SpeakerLayout,
                ) -> Result<()> {
                    Ok(())
                }
            }
        };
    }

    fake_backend!(
        PanicBackend,
        "panic_backend",
        panic!("boom from contributor backend")
    );
    fake_backend!(NonFiniteBackend, "nan_backend", {
        let mut gains = Gains::zeroed(TEST_SPEAKERS);
        gains.set(0, f32::NAN);
        RenderResponse { gains }
    });
    fake_backend!(
        WrongCountBackend,
        "wrong_count_backend",
        RenderResponse {
            gains: Gains::zeroed(TEST_SPEAKERS + 1),
        }
    );
    fake_backend!(
        GoodBackend,
        "good_backend",
        RenderResponse {
            gains: Gains::zeroed(TEST_SPEAKERS),
        }
    );

    #[test]
    fn smoke_test_rejects_panicking_backend() {
        let engine = realtime_engine(Box::new(PanicBackend));
        let err = smoke_test_engine(&engine, &build_config(), "panic_backend").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("panicked in compute_gains"), "got: {msg}");
        assert!(msg.contains("panic_backend"), "got: {msg}");
    }

    #[test]
    fn smoke_test_rejects_non_finite_gains() {
        let engine = realtime_engine(Box::new(NonFiniteBackend));
        let err = smoke_test_engine(&engine, &build_config(), "nan_backend").unwrap_err();
        assert!(err.to_string().contains("non-finite"), "got: {err}");
    }

    #[test]
    fn smoke_test_rejects_wrong_gain_count() {
        let engine = realtime_engine(Box::new(WrongCountBackend));
        let err = smoke_test_engine(&engine, &build_config(), "wrong_count_backend").unwrap_err();
        assert!(err.to_string().contains("expected"), "got: {err}");
    }

    #[test]
    fn smoke_test_accepts_well_behaved_backend() {
        let engine = realtime_engine(Box::new(GoodBackend));
        smoke_test_engine(&engine, &build_config(), "good_backend")
            .expect("well-behaved backend passes the smoke test");
    }

    struct DummyFactory(&'static str);
    impl BackendFactory for DummyFactory {
        fn id(&self) -> &'static str {
            self.0
        }
        fn build_plan(&self, _ctx: &BackendBuildCtx<'_>) -> Option<BackendBuildPlan> {
            None
        }
    }

    #[test]
    fn builtin_registry_exposes_known_backends() {
        let registry = BackendRegistry::builtin();
        let ids: Vec<_> = registry.ids().collect();
        for expected in ["vbap", "barycenter", "experimental_distance", "hybrid"] {
            assert!(
                ids.contains(&expected),
                "missing builtin backend {expected}"
            );
        }
        assert!(registry.get("vbap").is_some());
        assert!(registry.get("does_not_exist").is_none());
    }

    #[test]
    fn register_adds_then_overrides_by_id() {
        let mut registry = BackendRegistry::new();
        registry.register(Box::new(DummyFactory("custom")));
        assert!(registry.get("custom").is_some());

        let count = registry.ids().count();
        // Re-registering the same id replaces the entry instead of appending.
        registry.register(Box::new(DummyFactory("custom")));
        assert_eq!(registry.ids().count(), count);
    }
}

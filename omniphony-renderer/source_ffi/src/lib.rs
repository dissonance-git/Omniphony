//! C ABI for already-separated causal source rendering.
//!
//! This is the native boundary intended for game-music integrations. The host
//! supplies interleaved source lanes plus source evidence; Omniphony returns
//! binaural stereo. Ordinary stereo inference is not involved.

use orender_engine::{
    SourceRendererOptions, SourceSpatialMode, build_source_frame_renderer,
    source_presentation_policy,
};
use renderer::binaural::HrirSource;
use renderer::source_frame::SourceFrameRenderer;
use renderer::source_scene::{
    NativeStereoRoute, SourceLaneKind, SourcePresentationPolicy, SourceSceneEvidence,
};
use std::ptr;

const ABI_MAJOR: u32 = 0;
const ABI_MINOR: u32 = 4;

pub const SOURCE_FLAG_PERSISTENT_PART: u32 = 1 << 0;
pub const SOURCE_FLAG_NATIVE_STEREO_ROUTE: u32 = 1 << 1;
pub const SOURCE_FLAG_AUTHORED_POSITION: u32 = 1 << 2;
pub const SOURCE_FLAG_ROUTE_GAIN_PREAPPLIED: u32 = 1 << 3;

pub const SOURCE_LANE_DRY: u32 = 0;
pub const SOURCE_LANE_SHARED_WET: u32 = 1;
pub const SOURCE_LANE_REFERENCE_MIX: u32 = 2;

pub const SOURCE_SPATIAL_NATIVE_ROUTING: u32 = 0;
pub const SOURCE_SPATIAL_FULL_SPHERE: u32 = 1;

pub const SOURCE_HRIR_SAF_KEMAR: u32 = 0;
pub const SOURCE_HRIR_SYNTHETIC: u32 = 1;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct OmniphonySourceConfig {
    pub sample_rate_hz: u32,
    pub spatial_mode: u32,
    pub externalization: u32,
    pub hrir_source: u32,
    pub unit_scale_m: f32,
    pub reflection_level: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OmniphonySourceMixBudgetV1 {
    pub depth_scale: f32,
    pub height_scale: f32,
    pub shared_wet_strength_scale: f32,
    pub shared_wet_extent_scale: f32,
    pub externalization_scale: f32,
}

impl Default for OmniphonySourceMixBudgetV1 {
    fn default() -> Self {
        Self {
            depth_scale: 1.0,
            height_scale: 1.0,
            shared_wet_strength_scale: 1.0,
            shared_wet_extent_scale: 1.0,
            externalization_scale: 1.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct OmniphonySourceEvidenceV1 {
    pub lane_kind: u32,
    pub flags: u32,
    pub source_id: u64,
    pub persistent_part_id: u64,
    pub left_gain: f32,
    pub right_gain: f32,
    pub authored_x: f32,
    pub authored_y: f32,
    pub authored_z: f32,
    pub foundation: f32,
    pub foreground: f32,
    pub diffuse: f32,
    pub width: f32,
    pub vertical_affinity: f32,
    pub confidence: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct OmniphonySourceEvidenceEventV1 {
    pub frame_offset: u32,
    pub lane_index: u32,
    pub evidence: OmniphonySourceEvidenceV1,
}

#[derive(Clone, Copy)]
struct ConvertedSourceEvent {
    frame_offset: usize,
    lane_index: usize,
    evidence: SourceSceneEvidence,
    gain_preapplied: bool,
}

pub struct OmniphonySourceProcessor {
    renderer: SourceFrameRenderer,
    spatial_mode: SourceSpatialMode,
    base_reflection_level: f32,
    mix_budget: OmniphonySourceMixBudgetV1,
    source_buf: Vec<SourceSceneEvidence>,
    gain_preapplied_buf: Vec<bool>,
    event_buf: Vec<ConvertedSourceEvent>,
    samples_buf: Vec<f32>,
}

fn spatial_mode(value: u32) -> Option<SourceSpatialMode> {
    match value {
        SOURCE_SPATIAL_NATIVE_ROUTING => Some(SourceSpatialMode::NativeRouting),
        SOURCE_SPATIAL_FULL_SPHERE => Some(SourceSpatialMode::FullSphere),
        _ => None,
    }
}

fn hrir_source(value: u32) -> Option<HrirSource> {
    match value {
        SOURCE_HRIR_SAF_KEMAR => Some(HrirSource::SafKemar),
        SOURCE_HRIR_SYNTHETIC => Some(HrirSource::Synthetic),
        _ => None,
    }
}

fn mix_budget_valid(budget: OmniphonySourceMixBudgetV1) -> bool {
    let bounded = |value: f32, max: f32| value.is_finite() && (0.0..=max).contains(&value);
    bounded(budget.depth_scale, 1.5)
        && bounded(budget.height_scale, 1.5)
        && bounded(budget.shared_wet_strength_scale, 1.5)
        && bounded(budget.shared_wet_extent_scale, 1.5)
        && bounded(budget.externalization_scale, 1.0)
}

fn budgeted_policy(
    mode: SourceSpatialMode,
    budget: OmniphonySourceMixBudgetV1,
) -> SourcePresentationPolicy {
    let mut policy = source_presentation_policy(mode);
    policy.max_distance = 1.0 + (policy.max_distance - 1.0) * budget.depth_scale;
    policy.max_elevation_deg = (policy.max_elevation_deg * budget.height_scale).clamp(0.0, 80.0);
    policy.shared_wet.distance =
        1.0 + (policy.shared_wet.distance - 1.0) * budget.depth_scale;
    policy.shared_wet.elevation_deg =
        (policy.shared_wet.elevation_deg * budget.height_scale).clamp(-80.0, 80.0);
    policy.shared_wet.strength =
        (policy.shared_wet.strength * budget.shared_wet_strength_scale).clamp(0.0, 1.0);
    policy.shared_wet.extent = policy
        .shared_wet
        .extent
        .map(|value| (value * budget.shared_wet_extent_scale).clamp(0.0, 1.0));
    policy
}

fn apply_mix_budget(processor: &mut OmniphonySourceProcessor) {
    processor.renderer.set_policy(budgeted_policy(
        processor.spatial_mode,
        processor.mix_budget,
    ));
    let control = processor.renderer.renderer_mut().renderer_control();
    control.live.write().binaural.reflections.level =
        (processor.base_reflection_level * processor.mix_budget.externalization_scale)
            .clamp(0.0, 1.0);
}

fn convert_source(raw: OmniphonySourceEvidenceV1) -> Option<SourceSceneEvidence> {
    let lane_kind = match raw.lane_kind {
        SOURCE_LANE_DRY => SourceLaneKind::DrySource,
        SOURCE_LANE_SHARED_WET => SourceLaneKind::SharedWetReturn,
        SOURCE_LANE_REFERENCE_MIX => SourceLaneKind::ReferenceMix,
        _ => return None,
    };
    let persistent_part_id = (raw.flags & SOURCE_FLAG_PERSISTENT_PART != 0)
        .then_some(raw.persistent_part_id);
    let native_stereo_route = (raw.flags & SOURCE_FLAG_NATIVE_STEREO_ROUTE != 0).then_some(
        NativeStereoRoute {
            left_gain: raw.left_gain,
            right_gain: raw.right_gain,
        },
    );
    let authored_position = (raw.flags & SOURCE_FLAG_AUTHORED_POSITION != 0).then_some([
        raw.authored_x as f64,
        raw.authored_y as f64,
        raw.authored_z as f64,
    ]);

    Some(SourceSceneEvidence {
        lane_kind,
        source_id: raw.source_id,
        persistent_part_id,
        native_stereo_route,
        authored_position,
        foundation: raw.foundation,
        foreground: raw.foreground,
        diffuse: raw.diffuse,
        width: raw.width,
        vertical_affinity: raw.vertical_affinity,
        confidence: raw.confidence,
    })
}

fn validate_event_headers(
    events: &[OmniphonySourceEvidenceEventV1],
    source_count: usize,
    frames: usize,
) -> bool {
    let mut previous_offset = 0usize;
    let mut have_previous = false;
    for event in events {
        let offset = event.frame_offset as usize;
        let lane_index = event.lane_index as usize;
        if offset > frames || lane_index >= source_count {
            return false;
        }
        if have_previous && offset < previous_offset {
            return false;
        }
        previous_offset = offset;
        have_previous = true;
    }
    true
}

fn render_segment(
    processor: &mut OmniphonySourceProcessor,
    input: &[f32],
    output: &mut [f32],
    source_count: usize,
    start_frame: usize,
    end_frame: usize,
    sample_pos: u64,
    ramp_frames: u32,
) -> Result<(), i32> {
    if start_frame == end_frame {
        return Ok(());
    }
    let input_start = start_frame.checked_mul(source_count).ok_or(-3)?;
    let input_end = end_frame.checked_mul(source_count).ok_or(-3)?;
    let output_start = start_frame.checked_mul(2).ok_or(-3)?;
    let output_end = end_frame.checked_mul(2).ok_or(-3)?;
    let absolute_sample = sample_pos.checked_add(start_frame as u64).ok_or(-3)?;

    let samples_buf = std::mem::take(&mut processor.samples_buf);
    let rendered = match processor.renderer.render_source_frame_with_gain_policy(
        &input[input_start..input_end],
        &processor.source_buf,
        Some(&processor.gain_preapplied_buf),
        absolute_sample,
        ramp_frames,
        samples_buf,
        false,
    ) {
        Ok(rendered) => rendered,
        Err(_) => return Err(-6),
    };
    if rendered.samples.len() != output_end - output_start {
        processor.samples_buf = rendered.samples;
        return Err(-7);
    }
    output[output_start..output_end].copy_from_slice(&rendered.samples);
    processor.samples_buf = rendered.samples;
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn omniphony_source_abi_major() -> u32 {
    ABI_MAJOR
}

#[unsafe(no_mangle)]
pub extern "C" fn omniphony_source_abi_minor() -> u32 {
    ABI_MINOR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_source_create(
    config: *const OmniphonySourceConfig,
) -> *mut OmniphonySourceProcessor {
    if config.is_null() {
        return ptr::null_mut();
    }
    // SAFETY: null was rejected above; caller owns the config for this call.
    let config = unsafe { *config };
    if config.sample_rate_hz == 0 {
        return ptr::null_mut();
    }
    let Some(mode) = spatial_mode(config.spatial_mode) else {
        return ptr::null_mut();
    };
    let Some(hrir_source) = hrir_source(config.hrir_source) else {
        return ptr::null_mut();
    };

    let options = SourceRendererOptions {
        mode,
        externalization: config.externalization != 0,
        hrir_source,
        unit_scale_m: config.unit_scale_m,
        reflection_level: config.reflection_level,
        ..SourceRendererOptions::default()
    };
    let Ok(renderer) = build_source_frame_renderer(config.sample_rate_hz, None, options) else {
        return ptr::null_mut();
    };

    Box::into_raw(Box::new(OmniphonySourceProcessor {
        renderer,
        spatial_mode: mode,
        base_reflection_level: config.reflection_level.clamp(0.0, 1.0),
        mix_budget: OmniphonySourceMixBudgetV1::default(),
        source_buf: Vec::new(),
        gain_preapplied_buf: Vec::new(),
        event_buf: Vec::new(),
        samples_buf: Vec::new(),
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_source_destroy(
    processor: *mut OmniphonySourceProcessor,
) {
    if !processor.is_null() {
        // SAFETY: ABI requires a pointer returned by create, exactly once.
        unsafe { drop(Box::from_raw(processor)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_source_reset(
    processor: *mut OmniphonySourceProcessor,
) -> i32 {
    if processor.is_null() {
        return -1;
    }
    // SAFETY: null was rejected above.
    let processor = unsafe { &mut *processor };
    processor.renderer.reset_runtime_state();
    processor.mix_budget = OmniphonySourceMixBudgetV1::default();
    apply_mix_budget(processor);
    processor.source_buf.clear();
    processor.gain_preapplied_buf.clear();
    processor.event_buf.clear();
    processor.samples_buf.clear();
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_source_set_spatial_mode(
    processor: *mut OmniphonySourceProcessor,
    mode: u32,
) -> i32 {
    if processor.is_null() {
        return -1;
    }
    let Some(mode) = spatial_mode(mode) else {
        return -2;
    };
    // SAFETY: null was rejected above.
    let processor = unsafe { &mut *processor };
    processor.spatial_mode = mode;
    apply_mix_budget(processor);
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_source_set_externalization(
    processor: *mut OmniphonySourceProcessor,
    enabled: u32,
) -> i32 {
    if processor.is_null() {
        return -1;
    }
    // SAFETY: null was rejected above.
    let processor = unsafe { &mut *processor };
    let control = processor.renderer.renderer_mut().renderer_control();
    control.live.write().binaural.reflections.enabled = enabled != 0;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_source_set_mix_budget(
    processor: *mut OmniphonySourceProcessor,
    budget: *const OmniphonySourceMixBudgetV1,
) -> i32 {
    if processor.is_null() || budget.is_null() {
        return -1;
    }
    // SAFETY: null was rejected above; caller owns the value for this call.
    let budget = unsafe { *budget };
    if !mix_budget_valid(budget) {
        return -2;
    }
    // SAFETY: null was rejected above.
    let processor = unsafe { &mut *processor };
    processor.mix_budget = budget;
    apply_mix_budget(processor);
    0
}

/// Legacy whole-block entry point. Equivalent to the timed event path with an
/// empty event list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_source_process_f32(
    processor: *mut OmniphonySourceProcessor,
    input: *const f32,
    sources: *const OmniphonySourceEvidenceV1,
    source_count: usize,
    frames: usize,
    sample_pos: u64,
    ramp_frames: u32,
    output: *mut f32,
) -> i32 {
    // SAFETY: this function forwards the exact caller-owned pointers and sizes
    // to the stricter timed entry point with no event slice.
    unsafe {
        omniphony_source_process_events_f32(
            processor,
            input,
            sources,
            source_count,
            ptr::null(),
            0,
            frames,
            sample_pos,
            ramp_frames,
            output,
        )
    }
}

/// Render interleaved causal source lanes while applying ordered evidence
/// transitions at exact frame offsets inside the current block.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_source_process_events_f32(
    processor: *mut OmniphonySourceProcessor,
    input: *const f32,
    sources: *const OmniphonySourceEvidenceV1,
    source_count: usize,
    events: *const OmniphonySourceEvidenceEventV1,
    event_count: usize,
    frames: usize,
    sample_pos: u64,
    ramp_frames: u32,
    output: *mut f32,
) -> i32 {
    if processor.is_null() {
        return -1;
    }
    if frames == 0 {
        return 0;
    }
    if source_count == 0 || input.is_null() || sources.is_null() || output.is_null()
        || (event_count != 0 && events.is_null())
    {
        return -2;
    }
    let Some(input_samples) = frames.checked_mul(source_count) else {
        return -3;
    };
    let Some(output_samples) = frames.checked_mul(2) else {
        return -3;
    };

    // SAFETY: pointers were checked for null; caller promises the documented
    // slice lengths for the duration of this call.
    let input = unsafe { std::slice::from_raw_parts(input, input_samples) };
    let raw_sources = unsafe { std::slice::from_raw_parts(sources, source_count) };
    let raw_events: &[OmniphonySourceEvidenceEventV1] = if event_count == 0 {
        &[]
    } else {
        // SAFETY: non-zero event_count requires a non-null event pointer above.
        unsafe { std::slice::from_raw_parts(events, event_count) }
    };
    let output = unsafe { std::slice::from_raw_parts_mut(output, output_samples) };

    if !validate_event_headers(raw_events, source_count, frames) {
        return -4;
    }

    // SAFETY: processor pointer contract is the same as create/destroy.
    let processor = unsafe { &mut *processor };

    // Convert the complete initial state and complete event timeline before the
    // first sample is rendered. A malformed lane/event therefore cannot leave
    // this call half-rendered merely because the bad metadata appeared late.
    processor.source_buf.clear();
    processor.source_buf.reserve(source_count);
    processor.gain_preapplied_buf.clear();
    processor.gain_preapplied_buf.reserve(source_count);
    for raw in raw_sources.iter().copied() {
        let gain_preapplied = raw.flags & SOURCE_FLAG_ROUTE_GAIN_PREAPPLIED != 0;
        let Some(source) = convert_source(raw) else {
            return -4;
        };
        if source.lane_kind == SourceLaneKind::ReferenceMix {
            return -5;
        }
        processor.source_buf.push(source);
        processor.gain_preapplied_buf.push(gain_preapplied);
    }

    processor.event_buf.clear();
    processor.event_buf.reserve(event_count);
    for raw_event in raw_events.iter().copied() {
        let Some(evidence) = convert_source(raw_event.evidence) else {
            return -4;
        };
        if evidence.lane_kind == SourceLaneKind::ReferenceMix {
            return -5;
        }
        processor.event_buf.push(ConvertedSourceEvent {
            frame_offset: raw_event.frame_offset as usize,
            lane_index: raw_event.lane_index as usize,
            evidence,
            gain_preapplied: raw_event.evidence.flags & SOURCE_FLAG_ROUTE_GAIN_PREAPPLIED != 0,
        });
    }

    let mut start_frame = 0usize;
    let mut event_index = 0usize;
    while event_index < processor.event_buf.len() {
        let boundary = processor.event_buf[event_index].frame_offset;
        if let Err(code) = render_segment(
            processor,
            input,
            output,
            source_count,
            start_frame,
            boundary,
            sample_pos,
            ramp_frames,
        ) {
            return code;
        }

        // Apply every state change at this exact frame before rendering the next
        // sample. This mirrors standard sample-offset event processing used by
        // realtime audio plugin APIs rather than inventing a precomputed song
        // automation timeline.
        while event_index < processor.event_buf.len()
            && processor.event_buf[event_index].frame_offset == boundary
        {
            let event = processor.event_buf[event_index];
            processor.source_buf[event.lane_index] = event.evidence;
            processor.gain_preapplied_buf[event.lane_index] = event.gain_preapplied;
            event_index += 1;
        }
        start_frame = boundary;
    }

    if let Err(code) = render_segment(
        processor,
        input,
        output,
        source_count,
        start_frame,
        frames,
        sample_pos,
        ramp_frames,
    ) {
        return code;
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_flags_preserve_signed_route_and_persistent_identity() {
        let converted = convert_source(OmniphonySourceEvidenceV1 {
            lane_kind: SOURCE_LANE_DRY,
            flags: SOURCE_FLAG_PERSISTENT_PART | SOURCE_FLAG_NATIVE_STEREO_ROUTE,
            source_id: 7,
            persistent_part_id: 99,
            left_gain: -1.0,
            right_gain: 0.5,
            confidence: 1.0,
            ..OmniphonySourceEvidenceV1::default()
        })
        .expect("valid source");
        assert_eq!(converted.source_id, 7);
        assert_eq!(converted.persistent_part_id, Some(99));
        let route = converted.native_stereo_route.expect("route");
        assert_eq!(route.left_gain, -1.0);
        assert_eq!(route.right_gain, 0.5);
    }

    #[test]
    fn preapplied_gain_flag_does_not_remove_route_pose_evidence() {
        let raw = OmniphonySourceEvidenceV1 {
            lane_kind: SOURCE_LANE_DRY,
            flags: SOURCE_FLAG_NATIVE_STEREO_ROUTE | SOURCE_FLAG_ROUTE_GAIN_PREAPPLIED,
            left_gain: 1.0,
            right_gain: 0.0,
            ..OmniphonySourceEvidenceV1::default()
        };
        assert_ne!(raw.flags & SOURCE_FLAG_ROUTE_GAIN_PREAPPLIED, 0);
        let converted = convert_source(raw).expect("valid source");
        let route = converted.native_stereo_route.expect("route remains pose evidence");
        assert_eq!(route.left_gain, 1.0);
        assert_eq!(route.right_gain, 0.0);
    }

    #[test]
    fn authored_position_is_not_inferred_or_relabelled() {
        let converted = convert_source(OmniphonySourceEvidenceV1 {
            lane_kind: SOURCE_LANE_DRY,
            flags: SOURCE_FLAG_AUTHORED_POSITION,
            authored_x: 0.25,
            authored_y: -0.75,
            authored_z: 0.5,
            ..OmniphonySourceEvidenceV1::default()
        })
        .expect("valid source");
        assert_eq!(converted.authored_position, Some([0.25, -0.75, 0.5]));
    }

    #[test]
    fn unknown_lane_and_modes_are_rejected() {
        let unknown = OmniphonySourceEvidenceV1 {
            lane_kind: 99,
            ..OmniphonySourceEvidenceV1::default()
        };
        assert!(convert_source(unknown).is_none());
        assert!(spatial_mode(99).is_none());
        assert!(hrir_source(99).is_none());
    }

    #[test]
    fn timed_event_headers_are_sample_ordered_and_lane_bounded() {
        let evidence = OmniphonySourceEvidenceV1 {
            lane_kind: SOURCE_LANE_DRY,
            source_id: 1,
            ..OmniphonySourceEvidenceV1::default()
        };
        let valid = [
            OmniphonySourceEvidenceEventV1 { frame_offset: 12, lane_index: 0, evidence },
            OmniphonySourceEvidenceEventV1 { frame_offset: 12, lane_index: 1, evidence },
            OmniphonySourceEvidenceEventV1 { frame_offset: 64, lane_index: 0, evidence },
        ];
        assert!(validate_event_headers(&valid, 2, 64));

        let backwards = [valid[2], valid[0]];
        assert!(!validate_event_headers(&backwards, 2, 64));

        let bad_lane = [OmniphonySourceEvidenceEventV1 {
            frame_offset: 1,
            lane_index: 2,
            evidence,
        }];
        assert!(!validate_event_headers(&bad_lane, 2, 64));

        let past_end = [OmniphonySourceEvidenceEventV1 {
            frame_offset: 65,
            lane_index: 0,
            evidence,
        }];
        assert!(!validate_event_headers(&past_end, 2, 64));
    }

    #[test]
    fn mix_budget_scales_only_renderer_capacity() {
        let budget = OmniphonySourceMixBudgetV1 {
            depth_scale: 0.7,
            height_scale: 0.6,
            shared_wet_strength_scale: 0.8,
            shared_wet_extent_scale: 0.75,
            externalization_scale: 0.4,
        };
        assert!(mix_budget_valid(budget));
        let base = source_presentation_policy(SourceSpatialMode::FullSphere);
        let adapted = budgeted_policy(SourceSpatialMode::FullSphere, budget);
        assert!(adapted.max_distance < base.max_distance);
        assert!(adapted.max_elevation_deg < base.max_elevation_deg);
        assert!(adapted.shared_wet.distance < base.shared_wet.distance);
        assert!(adapted.shared_wet.elevation_deg < base.shared_wet.elevation_deg);
        assert!(adapted.shared_wet.strength < base.shared_wet.strength);
        assert!(adapted.shared_wet.extent[0] < base.shared_wet.extent[0]);
    }

    #[test]
    fn mix_budget_rejects_non_finite_and_unbounded_controls() {
        assert!(mix_budget_valid(OmniphonySourceMixBudgetV1::default()));
        assert!(!mix_budget_valid(OmniphonySourceMixBudgetV1 {
            depth_scale: f32::NAN,
            ..OmniphonySourceMixBudgetV1::default()
        }));
        assert!(!mix_budget_valid(OmniphonySourceMixBudgetV1 {
            externalization_scale: 1.1,
            ..OmniphonySourceMixBudgetV1::default()
        }));
    }

    #[test]
    fn source_abi_minor_advertises_scene_mix_budget_support() {
        assert_eq!(omniphony_source_abi_major(), 0);
        assert!(omniphony_source_abi_minor() >= 4);
    }
}

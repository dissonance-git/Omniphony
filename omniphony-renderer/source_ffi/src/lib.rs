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
    NativeStereoRoute, SourceLaneKind, SourceSceneEvidence,
};
use std::ptr;

const ABI_MAJOR: u32 = 0;
const ABI_MINOR: u32 = 2;

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

pub struct OmniphonySourceProcessor {
    renderer: SourceFrameRenderer,
    source_buf: Vec<SourceSceneEvidence>,
    gain_preapplied_buf: Vec<bool>,
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
        source_buf: Vec::new(),
        gain_preapplied_buf: Vec::new(),
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
    processor.source_buf.clear();
    processor.gain_preapplied_buf.clear();
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
    processor.renderer.set_policy(source_presentation_policy(mode));
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

/// Render interleaved causal source lanes to interleaved stereo f32.
///
/// `input` contains `frames * source_count` samples. `sources` contains one
/// evidence record per source channel in the same order. `output` must have
/// space for `frames * 2` samples. The protected ReferenceMix is deliberately
/// not accepted as an object lane; keep it beside this call for A/B/reference
/// validation.
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
    if processor.is_null() {
        return -1;
    }
    if frames == 0 {
        return 0;
    }
    if source_count == 0 || input.is_null() || sources.is_null() || output.is_null() {
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
    // SAFETY: processor pointer contract is the same as create/destroy.
    let processor = unsafe { &mut *processor };

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

    let samples_buf = std::mem::take(&mut processor.samples_buf);
    let rendered = match processor.renderer.render_source_frame_with_gain_policy(
        input,
        &processor.source_buf,
        Some(&processor.gain_preapplied_buf),
        sample_pos,
        ramp_frames,
        samples_buf,
        false,
    ) {
        Ok(rendered) => rendered,
        Err(_) => return -6,
    };
    if rendered.samples.len() != output_samples {
        processor.samples_buf = rendered.samples;
        return -7;
    }

    // SAFETY: output has documented capacity frames*2; source Vec remains alive
    // for the duration of the copy and cannot overlap host output memory.
    unsafe {
        ptr::copy_nonoverlapping(rendered.samples.as_ptr(), output, output_samples);
    }
    processor.samples_buf = rendered.samples;
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
}

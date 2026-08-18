//! Adapter from source-aware game-music presentation decisions into the
//! renderer's existing format-agnostic object-event path.
//!
//! The source interpreter owns audio-lane construction and causal truth. This
//! adapter owns only the presentation metadata for a lane that already exists.

use crate::source_scene::{
    SourcePresentation, SourcePresentationPolicy, SourceSceneEvidence, present_source,
};
use crate::spatial_renderer::SpatialChannelEvent;

#[derive(Debug, Clone)]
pub struct SourceChannelPresentation {
    pub presentation: SourcePresentation,
    /// None means the lane is a protected reference/control and must not be
    /// rendered as another object on top of the causal source scene.
    pub event: Option<SpatialChannelEvent>,
}

pub fn present_source_channel(
    channel_idx: usize,
    source: SourceSceneEvidence,
    policy: SourcePresentationPolicy,
    sample_pos: Option<u64>,
    ramp_length: Option<u32>,
) -> SourceChannelPresentation {
    let mut presentation = present_source(source, policy);

    // The source ABI currently has authored position authority, but no authored
    // source-extent authority. `width` and `diffuse` are presentation evidence.
    // Therefore a closed source sphere must close added object extent too,
    // regardless of whether the object's centre itself was authored. This keeps
    // NativeRouting a real geometry control while still using the same physical
    // 22-direction/cascaded renderer topology as FullSphere.
    if !policy.sphere_strength.is_finite() || policy.sphere_strength <= 0.0 {
        presentation.size = [0.0; 3];
    }

    let event = presentation.render_as_object.then(|| SpatialChannelEvent {
        channel_idx,
        is_bed: false,
        // ChannelState defaults to -128 dB. A source-aware lane is a newly
        // admitted object, so initialize it explicitly at unity rather than
        // depending on cached/default state. Subsequent gain automation can
        // still arrive as ordinary SpatialChannelEvents.
        gain_db: Some(0),
        ramp_length,
        size: Some(presentation.size),
        position: Some(presentation.position),
        sample_pos,
    });

    SourceChannelPresentation {
        presentation,
        event,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source_scene::{NativeStereoRoute, SourceLaneKind, SourcePositionAuthority};

    fn source(id: u64) -> SourceSceneEvidence {
        SourceSceneEvidence {
            source_id: id,
            confidence: 1.0,
            ..SourceSceneEvidence::default()
        }
    }

    #[test]
    fn protected_reference_produces_no_object_event() {
        let result = present_source_channel(
            0,
            SourceSceneEvidence {
                lane_kind: SourceLaneKind::ReferenceMix,
                ..source(1)
            },
            SourcePresentationPolicy::default(),
            Some(100),
            Some(64),
        );
        assert!(!result.presentation.render_as_object);
        assert!(result.event.is_none());
    }

    #[test]
    fn dry_source_maps_to_existing_virtual_object_contract() {
        let result = present_source_channel(
            5,
            SourceSceneEvidence {
                native_stereo_route: Some(NativeStereoRoute {
                    left_gain: 0.0,
                    right_gain: 1.0,
                }),
                foreground: 1.0,
                ..source(2)
            },
            SourcePresentationPolicy::default(),
            Some(480),
            Some(96),
        );
        let event = result.event.expect("dry source should become an object");
        assert_eq!(event.channel_idx, 5);
        assert!(!event.is_bed);
        assert_eq!(event.gain_db, Some(0));
        assert_eq!(event.sample_pos, Some(480));
        assert_eq!(event.ramp_length, Some(96));
        assert!(event.position.expect("position")[0] > 0.0);
    }

    #[test]
    fn authored_position_survives_adapter_exactly() {
        let position = [0.2, -0.8, 0.4];
        let result = present_source_channel(
            7,
            SourceSceneEvidence {
                authored_position: Some(position),
                ..source(3)
            },
            SourcePresentationPolicy::default(),
            None,
            None,
        );
        assert_eq!(
            result.presentation.authority,
            SourcePositionAuthority::Authored
        );
        assert_eq!(result.event.expect("object event").position, Some(position));
    }

    #[test]
    fn shared_wet_return_reuses_object_path_as_broad_environment() {
        let policy = SourcePresentationPolicy::default();
        let result = present_source_channel(
            8,
            SourceSceneEvidence {
                lane_kind: SourceLaneKind::SharedWetReturn,
                diffuse: 1.0,
                width: 1.0,
                ..source(4)
            },
            policy,
            Some(0),
            Some(256),
        );
        let event = result
            .event
            .expect("shared wet return should be renderable");
        assert_eq!(event.size, Some(policy.shared_wet.extent));
        assert_eq!(result.presentation.size, policy.shared_wet.extent);
        assert!(result.presentation.azimuth_deg.abs() > 90.0);
    }

    #[test]
    fn closed_sphere_zeroes_added_extent_without_changing_source_center_authority() {
        let authored_position = [0.25, 0.90, 0.15];
        let closed = SourcePresentationPolicy {
            sphere_strength: 0.0,
            max_elevation_deg: 0.0,
            max_distance: 1.0,
            shared_wet: crate::source_scene::SharedWetPresentationPolicy {
                strength: 0.0,
                extent: [0.0; 3],
                ..crate::source_scene::SharedWetPresentationPolicy::default()
            },
            ..SourcePresentationPolicy::default()
        };
        let result = present_source_channel(
            9,
            SourceSceneEvidence {
                authored_position: Some(authored_position),
                width: 1.0,
                diffuse: 1.0,
                ..source(5)
            },
            closed,
            Some(0),
            Some(0),
        );
        assert_eq!(result.presentation.authority, SourcePositionAuthority::Authored);
        assert_eq!(result.presentation.position, authored_position);
        assert_eq!(result.presentation.size, [0.0; 3]);
        assert_eq!(result.event.expect("source remains renderable").size, Some([0.0; 3]));
    }

    #[test]
    fn full_sphere_keeps_source_extent_available() {
        let result = present_source_channel(
            10,
            SourceSceneEvidence {
                width: 1.0,
                diffuse: 1.0,
                ..source(6)
            },
            SourcePresentationPolicy::default(),
            Some(0),
            Some(0),
        );
        let size = result.presentation.size;
        assert!(size[0] > 0.5);
        assert!(size[1] > 0.3);
        assert!(size[2] > 0.2);
        assert_eq!(result.event.expect("source object").size, Some(size));
    }
}

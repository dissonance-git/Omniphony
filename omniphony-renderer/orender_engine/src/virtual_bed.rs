//! Virtual-bed rendering for bed-only / pre-metadata frames.
//!
//! When a stream carries no spatial-object metadata (a plain multichannel bed,
//! or the frames before the first major-sync metadata payload), each input
//! channel is
//! turned into a fixed-position "virtual object" placed at its speaker pose, so
//! the bed still renders through VBAP instead of being dropped. Shared by the
//! `orender` CLI and the embedded engine for identical behaviour.

use crate::osc::ObjectMeta;
use bridge_api::RChannelLabel;
use renderer::speaker_layout::SpeakerLayout;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[inline]
fn map_depth_with_room_ratios(
    depth: f32,
    front_ratio: f32,
    rear_ratio: f32,
    center_blend: f32,
) -> f32 {
    let d = depth.clamp(-1.0, 1.0);
    let blend = center_blend.clamp(0.0, 1.0);
    let center_ratio = rear_ratio + (front_ratio - rear_ratio) * blend;
    if d >= 0.0 {
        let t = d;
        let a = center_ratio - front_ratio;
        let b = 2.0 * (front_ratio - center_ratio);
        a * t * t * t + b * t * t + center_ratio * t
    } else {
        let t = -d;
        let a = center_ratio - rear_ratio;
        let b = 2.0 * (rear_ratio - center_ratio);
        -(a * t * t * t + b * t * t + center_ratio * t)
    }
}

fn inverse_map_depth_with_room_ratios(
    mapped_depth: f32,
    front_ratio: f32,
    rear_ratio: f32,
    center_blend: f32,
) -> f32 {
    let y = mapped_depth;
    if y >= 0.0 {
        let target = y.clamp(0.0, front_ratio.max(0.0));
        let mut lo = 0.0f32;
        let mut hi = 1.0f32;
        for _ in 0..28 {
            let mid = (lo + hi) * 0.5;
            let val = map_depth_with_room_ratios(mid, front_ratio, rear_ratio, center_blend);
            if val < target {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (lo + hi) * 0.5
    } else {
        let target = y.clamp(-rear_ratio.max(0.0), 0.0);
        let mut lo = -1.0f32;
        let mut hi = 0.0f32;
        for _ in 0..28 {
            let mid = (lo + hi) * 0.5;
            let val = map_depth_with_room_ratios(mid, front_ratio, rear_ratio, center_blend);
            if val < target {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        (lo + hi) * 0.5
    }
}

fn inverse_room_ratio_map_for_virtual_object(
    target_x: f32,
    target_y: f32,
    target_z: f32,
    room_ratio: [f32; 3],
    room_ratio_rear: f32,
    room_ratio_lower: f32,
    room_ratio_center_blend: f32,
) -> (f32, f32, f32) {
    let width = room_ratio[0].max(0.01);
    let front = room_ratio[1].max(0.01);
    let height = room_ratio[2].max(0.01);
    let rear = room_ratio_rear.max(0.01);
    let lower = room_ratio_lower.max(0.01);

    let x = (target_x / width).clamp(-1.0, 1.0);
    let y = inverse_map_depth_with_room_ratios(target_y, front, rear, room_ratio_center_blend)
        .clamp(-1.0, 1.0);
    let z = if target_z >= 0.0 {
        (target_z / height).clamp(-1.0, 1.0)
    } else {
        (target_z / lower).clamp(-1.0, 1.0)
    };
    (x, y, z)
}

#[derive(Clone)]
struct VirtualBedLayouts {
    layout_5_1: Option<SpeakerLayout>,
    layout_7_1: Option<SpeakerLayout>,
}

static VIRTUAL_BED_LAYOUTS: OnceLock<VirtualBedLayouts> = OnceLock::new();

fn virtual_bed_layouts() -> &'static VirtualBedLayouts {
    VIRTUAL_BED_LAYOUTS.get_or_init(|| VirtualBedLayouts {
        layout_5_1: load_virtual_bed_layout("5.1.yaml"),
        layout_7_1: load_virtual_bed_layout("7.1.yaml"),
    })
}

fn load_virtual_bed_layout(file_name: &str) -> Option<SpeakerLayout> {
    // The 5.1 / 7.1 virtual-bed layouts are height-less, so they now live in
    // the layouts/legacy/ subfolder. Try that first, then the historical
    // top-level path (older installs / packaging that still ships them flat).
    // `mut` is used only on Windows (the %ProgramData% push below).
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut bases: Vec<PathBuf> = vec![
        // cwd-relative first — matches the CLI run from the workspace root.
        PathBuf::from("layouts"),
        PathBuf::from("omniphony").join("layouts"),
        Path::new(env!("CARGO_MANIFEST_DIR")).join("layouts"),
        // Fixed install dirs for the embedded host (mpv has no workspace cwd);
        // reached only when the cwd-relative lookups miss, so CLI parity holds.
        PathBuf::from("/usr/lib/orender/layouts"),
        PathBuf::from("/usr/share/orender/layouts"),
    ];
    // Windows: the embedded host (mpv) has no workspace cwd, and the shared
    // install lives under %ProgramData%\omniphony (machine-wide, same as the
    // config + service). Search its layouts dir so layouts ship/resolve there.
    #[cfg(windows)]
    if let Ok(program_data) = std::env::var("ProgramData") {
        let mut p = PathBuf::from(program_data);
        p.push("omniphony");
        p.push("layouts");
        bases.push(p);
    }
    let mut candidates: Vec<PathBuf> = Vec::with_capacity(bases.len() * 2);
    for base in &bases {
        candidates.push(base.join("legacy").join(file_name));
        candidates.push(base.join(file_name));
    }
    candidates.dedup();

    for path in candidates {
        if !path.exists() {
            continue;
        }
        match SpeakerLayout::from_file(&path) {
            Ok(layout) => {
                log::info!("Loaded virtual bed layout from {}", path.display());
                return Some(layout);
            }
            Err(e) => {
                log::warn!(
                    "Failed to load virtual bed layout '{}' ({}): {}",
                    file_name,
                    path.display(),
                    e
                );
            }
        }
    }

    log::warn!(
        "Virtual bed layout '{}' not found on disk, using built-in fallback positions",
        file_name
    );
    None
}

fn find_speaker_in_layout(
    layout: &SpeakerLayout,
    aliases: &[&str],
) -> Option<(String, f32, f32, f32)> {
    for speaker in &layout.speakers {
        if aliases
            .iter()
            .any(|alias| speaker.name.eq_ignore_ascii_case(alias))
        {
            return Some((
                speaker.name.clone(),
                speaker.azimuth,
                speaker.elevation,
                speaker.distance,
            ));
        }
    }
    None
}

fn label_aliases(label: RChannelLabel, use_7_1: bool) -> Option<&'static [&'static str]> {
    match label {
        RChannelLabel::L => Some(&["FL", "L", "FrontLeft", "LeftFront"]),
        RChannelLabel::R => Some(&["FR", "R", "FrontRight", "RightFront"]),
        RChannelLabel::C => Some(&["C", "FC", "Center", "Centre"]),
        RChannelLabel::LFE | RChannelLabel::LFE2 => {
            Some(&["LFE", "LFE1", "Sub", "Subwoofer", "SW"])
        }
        RChannelLabel::Ls => {
            if use_7_1 {
                Some(&["SL", "Ls", "LeftSurround", "SurroundLeft"])
            } else {
                Some(&[
                    "SL",
                    "Ls",
                    "BL",
                    "Lb",
                    "LeftSurround",
                    "SurroundLeft",
                    "BackLeft",
                    "LeftBack",
                ])
            }
        }
        RChannelLabel::Rs => {
            if use_7_1 {
                Some(&["SR", "Rs", "RightSurround", "SurroundRight"])
            } else {
                Some(&[
                    "SR",
                    "Rs",
                    "BR",
                    "Rb",
                    "RightSurround",
                    "SurroundRight",
                    "BackRight",
                    "RightBack",
                ])
            }
        }
        RChannelLabel::Lb => Some(&[
            "BL", "Lb", "Lrs", "BackLeft", "LeftBack", "RearLeft", "LeftRear",
        ]),
        RChannelLabel::Rb => Some(&[
            "BR",
            "Rb",
            "Rrs",
            "BackRight",
            "RightBack",
            "RearRight",
            "RightRear",
        ]),
        RChannelLabel::Cb => Some(&["BC", "Cb", "BackCenter", "RearCenter"]),
        _ => None,
    }
}

fn fallback_virtual_bed_pose(
    label: RChannelLabel,
    use_7_1: bool,
) -> Option<(String, f32, f32, f32)> {
    let (name, az, el, dist) = match label {
        RChannelLabel::L => ("FL", if use_7_1 { -26.0 } else { -30.0 }, 0.0, 2.0),
        RChannelLabel::R => ("FR", if use_7_1 { 26.0 } else { 30.0 }, 0.0, 2.0),
        RChannelLabel::C => ("C", 0.0, 0.0, 2.0),
        RChannelLabel::LFE | RChannelLabel::LFE2 => ("LFE", 0.0, 0.0, 1.0),
        RChannelLabel::Ls => ("SL", if use_7_1 { -100.0 } else { -110.0 }, 0.0, 1.0),
        RChannelLabel::Rs => ("SR", if use_7_1 { 100.0 } else { 110.0 }, 0.0, 1.0),
        RChannelLabel::Lb => ("BL", -142.5, 0.0, 1.0),
        RChannelLabel::Rb => ("BR", 142.5, 0.0, 1.0),
        RChannelLabel::Cb => ("BC", 180.0, 0.0, 1.0),
        _ => return None,
    };
    Some((name.to_string(), az, el, dist))
}

fn resolve_virtual_bed_pose(
    label: RChannelLabel,
    use_7_1: bool,
    input_layout: Option<&SpeakerLayout>,
) -> Option<(String, f32, f32, f32)> {
    if let (Some(layout), Some(aliases)) = (input_layout, label_aliases(label, use_7_1)) {
        if let Some(found) = find_speaker_in_layout(layout, aliases) {
            return Some(found);
        }
    }

    let layouts = virtual_bed_layouts();
    let layout_opt = if use_7_1 {
        layouts.layout_7_1.as_ref()
    } else {
        layouts.layout_5_1.as_ref()
    };

    if let (Some(layout), Some(aliases)) = (layout_opt, label_aliases(label, use_7_1)) {
        if let Some(found) = find_speaker_in_layout(layout, aliases) {
            return Some(found);
        }
    }

    fallback_virtual_bed_pose(label, use_7_1)
}

pub fn build_virtual_bed_events(
    channel_labels: &[RChannelLabel],
    input_layout: Option<&SpeakerLayout>,
    room_ratio: [f32; 3],
    room_ratio_rear: f32,
    room_ratio_lower: f32,
    room_ratio_center_blend: f32,
) -> Option<Vec<renderer::spatial_renderer::SpatialChannelEvent>> {
    let has_back = channel_labels
        .iter()
        .any(|l| matches!(l, RChannelLabel::Lb | RChannelLabel::Rb | RChannelLabel::Cb));
    let use_7_1 = has_back;

    let mut events: Vec<renderer::spatial_renderer::SpatialChannelEvent> =
        Vec::with_capacity(channel_labels.len());

    for (channel_idx, label) in channel_labels.iter().enumerate() {
        let (_name, az_deg, el_deg, dist_m) =
            match resolve_virtual_bed_pose(*label, use_7_1, input_layout) {
                Some(v) => v,
                None => continue,
            };

        let (sx, sy, sz) = renderer::spatial_vbap::spherical_to_adm(az_deg, el_deg, dist_m);
        let (x, y, z) = inverse_room_ratio_map_for_virtual_object(
            sx,
            sy,
            sz,
            room_ratio,
            room_ratio_rear,
            room_ratio_lower,
            room_ratio_center_blend,
        );
        events.push(renderer::spatial_renderer::SpatialChannelEvent {
            channel_idx,
            is_bed: false,
            gain_db: Some(0),
            ramp_length: Some(0),
            size: None,
            position: Some([x as f64, y as f64, z as f64]),
            sample_pos: Some(0),
        });
    }

    if events.is_empty() {
        None
    } else {
        Some(events)
    }
}

pub fn build_virtual_bed_objects(
    channel_labels: &[RChannelLabel],
    input_layout: Option<&SpeakerLayout>,
    room_ratio: [f32; 3],
    room_ratio_rear: f32,
    room_ratio_lower: f32,
    room_ratio_center_blend: f32,
) -> Option<Vec<ObjectMeta>> {
    let has_back = channel_labels
        .iter()
        .any(|l| matches!(l, RChannelLabel::Lb | RChannelLabel::Rb | RChannelLabel::Cb));
    let use_7_1 = has_back;

    let mut objects: Vec<ObjectMeta> = Vec::with_capacity(channel_labels.len());
    for label in channel_labels {
        let (name, az_deg, el_deg, dist_m) =
            match resolve_virtual_bed_pose(*label, use_7_1, input_layout) {
                Some(v) => v,
                None => continue,
            };
        let (sx, sy, sz) = renderer::spatial_vbap::spherical_to_adm(az_deg, el_deg, dist_m);
        let (x, y, z) = inverse_room_ratio_map_for_virtual_object(
            sx,
            sy,
            sz,
            room_ratio,
            room_ratio_rear,
            room_ratio_lower,
            room_ratio_center_blend,
        );
        objects.push(ObjectMeta {
            name,
            x,
            y,
            z,
            coord_mode: "cartesian".to_string(),
            direct_speaker_index: None,
            gain: 0,
            priority: 0.0,
            size: [0.0, 0.0, 0.0],
        });
    }
    if objects.is_empty() {
        None
    } else {
        Some(objects)
    }
}

/// Bed id (0–9, the scheme used by [`SpeakerLayout::bed_to_speaker_mapping`])
/// for a channel label, or `None` when the label has no direct-speaker slot in
/// that scheme (e.g. back-centre / top-side channels). Used by `Direct` mode.
fn bed_id_for_label(label: RChannelLabel) -> Option<usize> {
    match label {
        RChannelLabel::L => Some(0),
        RChannelLabel::R => Some(1),
        RChannelLabel::C => Some(2),
        RChannelLabel::LFE | RChannelLabel::LFE2 => Some(3),
        RChannelLabel::Ls => Some(4),
        RChannelLabel::Rs => Some(5),
        RChannelLabel::Lb => Some(6),
        RChannelLabel::Rb => Some(7),
        RChannelLabel::Tfl => Some(8),
        RChannelLabel::Tfr => Some(9),
        _ => None,
    }
}

/// What the renderer should do with a channel-based (non-object) frame, decided
/// once and applied identically by the CLI/spdif decode path and the embedded
/// mpv host. See [`ChannelRenderMode`].
pub enum ChannelRenderPlan {
    /// Let the host / sink handle the channels (no spatialization). The CLI
    /// writes the decoded channels straight out; the embedded mpv decoder
    /// declines so mpv falls back to its native decoder.
    HostPassthrough,
    /// Render the events. `bed_indices` is `Some(..)` for direct speaker routing
    /// (the renderer must `configure_beds` to it so every channel routes as a
    /// bed) and `None` for the virtual-object path (beds must be empty so every
    /// channel goes through VBAP).
    Events {
        events: Vec<renderer::spatial_renderer::SpatialChannelEvent>,
        bed_indices: Option<Vec<usize>>,
    },
    /// No renderable mapping for these labels → emit silence (advance the host
    /// by the frame's sample count without producing sound).
    Silence,
}

/// Decide how to render a channel-based frame for the given `mode`. Pure: no
/// renderer interaction, so both decode paths can call it and apply the result
/// the same way.
#[allow(clippy::too_many_arguments)]
pub fn plan_channel_render(
    mode: renderer::live_params::ChannelRenderMode,
    channel_labels: &[RChannelLabel],
    input_layout: Option<&SpeakerLayout>,
    room_ratio: [f32; 3],
    room_ratio_rear: f32,
    room_ratio_lower: f32,
    room_ratio_center_blend: f32,
) -> ChannelRenderPlan {
    use renderer::live_params::ChannelRenderMode;
    match mode {
        ChannelRenderMode::Host => ChannelRenderPlan::HostPassthrough,
        ChannelRenderMode::Direct => build_direct_bed_plan(channel_labels),
        ChannelRenderMode::Virtual => match build_virtual_bed_events(
            channel_labels,
            input_layout,
            room_ratio,
            room_ratio_rear,
            room_ratio_lower,
            room_ratio_center_blend,
        ) {
            Some(events) => ChannelRenderPlan::Events {
                events,
                bed_indices: None,
            },
            None => ChannelRenderPlan::Silence,
        },
    }
}

/// Direct mode: route each channel straight to its layout speaker. Builds the
/// per-channel `bed_indices` (one entry per channel; `usize::MAX` for channels
/// with no direct slot, which the renderer skips) plus a bed event per routed
/// channel so its gain is unity instead of the silent default.
fn build_direct_bed_plan(channel_labels: &[RChannelLabel]) -> ChannelRenderPlan {
    let mut bed_indices: Vec<usize> = Vec::with_capacity(channel_labels.len());
    let mut events: Vec<renderer::spatial_renderer::SpatialChannelEvent> =
        Vec::with_capacity(channel_labels.len());
    for (channel_idx, label) in channel_labels.iter().enumerate() {
        match bed_id_for_label(*label) {
            Some(bed_id) => {
                bed_indices.push(bed_id);
                events.push(renderer::spatial_renderer::SpatialChannelEvent {
                    channel_idx,
                    is_bed: true,
                    gain_db: Some(0),
                    ramp_length: Some(0),
                    size: None,
                    position: None,
                    sample_pos: Some(0),
                });
            }
            // No direct slot: keep index alignment but route nowhere (silent).
            None => bed_indices.push(usize::MAX),
        }
    }
    if events.is_empty() {
        ChannelRenderPlan::Silence
    } else {
        ChannelRenderPlan::Events {
            events,
            bed_indices: Some(bed_indices),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIT_ROOM: [f32; 3] = [1.0, 1.0, 1.0];

    #[test]
    fn maps_a_5_1_bed_with_fallback_poses() {
        // No input layout → resolves via bundled layouts or built-in fallbacks.
        let labels = [
            RChannelLabel::L,
            RChannelLabel::R,
            RChannelLabel::C,
            RChannelLabel::LFE,
            RChannelLabel::Ls,
            RChannelLabel::Rs,
        ];
        let events = build_virtual_bed_events(&labels, None, UNIT_ROOM, 1.0, 1.0, 0.0)
            .expect("5.1 bed must map to virtual events");
        assert_eq!(events.len(), labels.len());
        for (i, ev) in events.iter().enumerate() {
            assert_eq!(ev.channel_idx, i);
            assert!(!ev.is_bed);
            let pos = ev.position.expect("virtual event carries a position");
            assert!(
                pos.iter()
                    .all(|c| c.is_finite() && (-1.0..=1.0).contains(c)),
                "position {pos:?} must be finite and within the unit room"
            );
        }
    }

    #[test]
    fn left_and_right_beds_are_mirrored() {
        let labels = [RChannelLabel::L, RChannelLabel::R];
        let events = build_virtual_bed_events(&labels, None, UNIT_ROOM, 1.0, 1.0, 0.0).unwrap();
        let l = events[0].position.unwrap();
        let r = events[1].position.unwrap();
        // L sits on the negative-x side, R on the positive-x side.
        assert!(l[0] < 0.0, "L x={} should be negative", l[0]);
        assert!(r[0] > 0.0, "R x={} should be positive", r[0]);
    }

    #[test]
    fn objects_match_events_for_the_same_bed() {
        let labels = [RChannelLabel::L, RChannelLabel::R, RChannelLabel::C];
        let events = build_virtual_bed_events(&labels, None, UNIT_ROOM, 1.0, 1.0, 0.0).unwrap();
        let objects = build_virtual_bed_objects(&labels, None, UNIT_ROOM, 1.0, 1.0, 0.0).unwrap();
        assert_eq!(events.len(), objects.len());
        for (ev, obj) in events.iter().zip(objects.iter()) {
            let pos = ev.position.unwrap();
            assert!((pos[0] - obj.x as f64).abs() < 1e-6);
            assert!((pos[1] - obj.y as f64).abs() < 1e-6);
            assert!((pos[2] - obj.z as f64).abs() < 1e-6);
        }
    }

    const BED_5_1: [RChannelLabel; 6] = [
        RChannelLabel::L,
        RChannelLabel::R,
        RChannelLabel::C,
        RChannelLabel::LFE,
        RChannelLabel::Ls,
        RChannelLabel::Rs,
    ];

    #[test]
    fn plan_host_is_passthrough() {
        let plan = plan_channel_render(
            renderer::live_params::ChannelRenderMode::Host,
            &BED_5_1,
            None,
            UNIT_ROOM,
            1.0,
            1.0,
            0.0,
        );
        assert!(matches!(plan, ChannelRenderPlan::HostPassthrough));
    }

    #[test]
    fn plan_virtual_renders_objects_without_bed_routing() {
        let plan = plan_channel_render(
            renderer::live_params::ChannelRenderMode::Virtual,
            &BED_5_1,
            None,
            UNIT_ROOM,
            1.0,
            1.0,
            0.0,
        );
        match plan {
            ChannelRenderPlan::Events {
                events,
                bed_indices,
            } => {
                assert_eq!(events.len(), BED_5_1.len());
                // Virtual: every channel is a VBAP object, no bed routing.
                assert!(bed_indices.is_none());
                assert!(events.iter().all(|e| !e.is_bed));
            }
            other => panic!("expected Events, got {:?}", PlanKind::from(&other)),
        }
    }

    #[test]
    fn plan_direct_routes_each_channel_to_its_speaker() {
        let plan = plan_channel_render(
            renderer::live_params::ChannelRenderMode::Direct,
            &BED_5_1,
            None,
            UNIT_ROOM,
            1.0,
            1.0,
            0.0,
        );
        match plan {
            ChannelRenderPlan::Events {
                events,
                bed_indices,
            } => {
                // Direct: bed routing with one bed id per channel (full length).
                let bi = bed_indices.expect("direct mode configures bed_indices");
                assert_eq!(bi.len(), BED_5_1.len());
                // 5.1 maps to bed ids 0..=5 in order (L,R,C,LFE,Ls,Rs).
                assert_eq!(bi, vec![0, 1, 2, 3, 4, 5]);
                assert!(events.iter().all(|e| e.is_bed && e.gain_db == Some(0)));
            }
            other => panic!("expected Events, got {:?}", PlanKind::from(&other)),
        }
    }

    #[test]
    fn plan_direct_keeps_alignment_for_unroutable_channels() {
        // Back-centre (Cb) has no bed slot: it must keep index alignment with a
        // sentinel and emit no event (silent), not shift the other channels.
        let labels = [RChannelLabel::L, RChannelLabel::Cb, RChannelLabel::R];
        let plan = plan_channel_render(
            renderer::live_params::ChannelRenderMode::Direct,
            &labels,
            None,
            UNIT_ROOM,
            1.0,
            1.0,
            0.0,
        );
        match plan {
            ChannelRenderPlan::Events {
                events,
                bed_indices,
            } => {
                let bi = bed_indices.unwrap();
                assert_eq!(bi.len(), 3);
                assert_eq!(bi[0], 0);
                assert_eq!(bi[1], usize::MAX, "Cb has no slot → sentinel");
                assert_eq!(bi[2], 1);
                // Only L and R produce events.
                assert_eq!(events.len(), 2);
                assert_eq!(events[0].channel_idx, 0);
                assert_eq!(events[1].channel_idx, 2);
            }
            other => panic!("expected Events, got {:?}", PlanKind::from(&other)),
        }
    }

    // Small helper so panics in the matches above print something readable.
    #[derive(Debug)]
    enum PlanKind {
        Host,
        Events,
        Silence,
    }
    impl From<&ChannelRenderPlan> for PlanKind {
        fn from(p: &ChannelRenderPlan) -> Self {
            match p {
                ChannelRenderPlan::HostPassthrough => PlanKind::Host,
                ChannelRenderPlan::Events { .. } => PlanKind::Events,
                ChannelRenderPlan::Silence => PlanKind::Silence,
            }
        }
    }
}

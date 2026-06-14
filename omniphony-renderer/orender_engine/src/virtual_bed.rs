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
    virtual_bed: Option<&SpeakerLayout>,
    output_layout: Option<&SpeakerLayout>,
    room_ratio: [f32; 3],
    room_ratio_rear: f32,
    room_ratio_lower: f32,
    room_ratio_center_blend: f32,
) -> Option<Vec<ObjectMeta>> {
    let has_back = channel_labels
        .iter()
        .any(|l| matches!(l, RChannelLabel::Lb | RChannelLabel::Rb | RChannelLabel::Cb));
    let use_7_1 = has_back;

    // Used to anchor a direct channel onto its output speaker so Studio shows it
    // snapped to that speaker (its `directSpeakerIndex` decoration).
    let bed_to_speaker = output_layout.map(|layout| layout.bed_to_speaker_mapping());

    let mut objects: Vec<ObjectMeta> = Vec::with_capacity(channel_labels.len());
    for label in channel_labels {
        // Emit every channel so the editor/overlay can show them all: virtualized
        // channels carry a free position, direct channels (e.g. LFE) carry a
        // `direct_speaker_index` so Studio anchors them onto their speaker.
        let spatialize = channel_is_spatialized(virtual_bed, *label, use_7_1);
        let (name, az_deg, el_deg, dist_m) =
            match resolve_virtual_bed_pose(*label, use_7_1, virtual_bed) {
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
        let direct_speaker_index = if spatialize {
            None
        } else {
            bed_id_for_label(*label).and_then(|bed_id| {
                bed_to_speaker
                    .as_ref()
                    .and_then(|m| m.get(&bed_id).map(|&spk| spk as u32))
            })
        };
        // Per-channel gain from the virtual bed (dB); 0 = unity when unset.
        let gain = find_virtual_bed_entry(virtual_bed, *label, use_7_1)
            .map(|entry| entry.gain_db)
            .unwrap_or(0);
        objects.push(ObjectMeta {
            name,
            x,
            y,
            z,
            coord_mode: "cartesian".to_string(),
            direct_speaker_index,
            gain,
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
/// that scheme (e.g. back-centre / top-side channels). Used to route a channel
/// the virtual bed marks `spatialize:false` direct to its output speaker.
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

/// Default placement for a channel label when the virtual bed has no entry for
/// it (or no virtual bed is configured): every channel is virtualized except the
/// LFE, which cannot be VBAP-panned and routes direct to the sub.
fn default_channel_spatialize(label: RChannelLabel) -> bool {
    !matches!(label, RChannelLabel::LFE | RChannelLabel::LFE2)
}

/// Find the virtual-bed entry (a [`renderer::speaker_layout::Speaker`]) for a
/// channel label, matching by the same name aliases used to resolve poses.
fn find_virtual_bed_entry(
    layout: Option<&SpeakerLayout>,
    label: RChannelLabel,
    use_7_1: bool,
) -> Option<&renderer::speaker_layout::Speaker> {
    let layout = layout?;
    let aliases = label_aliases(label, use_7_1)?;
    layout.speakers.iter().find(|speaker| {
        aliases
            .iter()
            .any(|alias| speaker.name.eq_ignore_ascii_case(alias))
    })
}

/// Whether a channel should be virtualized (`true`) or routed direct to its
/// speaker (`false`): the virtual bed's per-entry `spatialize` flag, falling
/// back to [`default_channel_spatialize`] when the bed has no entry for it.
fn channel_is_spatialized(
    layout: Option<&SpeakerLayout>,
    label: RChannelLabel,
    use_7_1: bool,
) -> bool {
    find_virtual_bed_entry(layout, label, use_7_1)
        .map(|entry| entry.spatialize)
        .unwrap_or_else(|| default_channel_spatialize(label))
}

/// What the renderer should do with a channel-based (non-object) frame, decided
/// once and applied identically by the CLI/spdif decode path and the embedded
/// mpv host. See [`renderer::live_params::ChannelRenderMode`].
pub enum ChannelRenderPlan {
    /// Let the host / sink handle the channels (no spatialization). The CLI
    /// writes the decoded channels straight out; the embedded mpv decoder
    /// declines so mpv falls back to its native decoder.
    HostPassthrough,
    /// Render the events through the virtual bed. `bed_indices` is always
    /// `Some(..)` with one entry per input channel: a bed id (0–9) for a channel
    /// routed direct to its speaker, or `usize::MAX` for a channel virtualized as
    /// a VBAP object (its position is carried by the matching event). The
    /// renderer must `configure_beds` to it.
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
/// the same way. In `Spatial` mode the placement of each channel — direct to a
/// speaker or virtualized at a position — is decided per channel by the
/// `virtual_bed` layout (falling back to canonical poses).
#[allow(clippy::too_many_arguments)]
pub fn plan_channel_render(
    mode: renderer::live_params::ChannelRenderMode,
    channel_labels: &[RChannelLabel],
    virtual_bed: Option<&SpeakerLayout>,
    room_ratio: [f32; 3],
    room_ratio_rear: f32,
    room_ratio_lower: f32,
    room_ratio_center_blend: f32,
) -> ChannelRenderPlan {
    use renderer::live_params::ChannelRenderMode;
    match mode {
        ChannelRenderMode::Host => ChannelRenderPlan::HostPassthrough,
        ChannelRenderMode::Spatial => build_virtual_bed_plan(
            channel_labels,
            virtual_bed,
            room_ratio,
            room_ratio_rear,
            room_ratio_lower,
            room_ratio_center_blend,
        ),
    }
}

/// Spatial mode: decide each channel's placement against the virtual bed. A
/// channel marked `spatialize:false` (e.g. LFE) routes direct to its speaker
/// (a bed id in `bed_indices` + a bed event); a `spatialize:true` channel is
/// virtualized at the bed's position (the `usize::MAX` sentinel in `bed_indices`
/// + an object event carrying the position). A frame may freely mix the two.
fn build_virtual_bed_plan(
    channel_labels: &[RChannelLabel],
    virtual_bed: Option<&SpeakerLayout>,
    room_ratio: [f32; 3],
    room_ratio_rear: f32,
    room_ratio_lower: f32,
    room_ratio_center_blend: f32,
) -> ChannelRenderPlan {
    let has_back = channel_labels
        .iter()
        .any(|l| matches!(l, RChannelLabel::Lb | RChannelLabel::Rb | RChannelLabel::Cb));
    let use_7_1 = has_back;

    let mut bed_indices: Vec<usize> = Vec::with_capacity(channel_labels.len());
    let mut events: Vec<renderer::spatial_renderer::SpatialChannelEvent> =
        Vec::with_capacity(channel_labels.len());

    for (channel_idx, label) in channel_labels.iter().enumerate() {
        let spatialize = channel_is_spatialized(virtual_bed, *label, use_7_1);
        if spatialize {
            // Virtualize: place an object at the bed's (or fallback) pose.
            match resolve_virtual_bed_pose(*label, use_7_1, virtual_bed) {
                Some((_name, az_deg, el_deg, dist_m)) => {
                    let (sx, sy, sz) =
                        renderer::spatial_vbap::spherical_to_adm(az_deg, el_deg, dist_m);
                    let (x, y, z) = inverse_room_ratio_map_for_virtual_object(
                        sx,
                        sy,
                        sz,
                        room_ratio,
                        room_ratio_rear,
                        room_ratio_lower,
                        room_ratio_center_blend,
                    );
                    bed_indices.push(usize::MAX);
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
                // No resolvable pose: keep index alignment, route nowhere.
                None => bed_indices.push(usize::MAX),
            }
        } else {
            // Direct: route to the matching output speaker (by bed id / name).
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
                // No direct slot for a channel asked to route direct: silent.
                None => bed_indices.push(usize::MAX),
            }
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
        let objects =
            build_virtual_bed_objects(&labels, None, None, UNIT_ROOM, 1.0, 1.0, 0.0).unwrap();
        assert_eq!(events.len(), objects.len());
        for (ev, obj) in events.iter().zip(objects.iter()) {
            let pos = ev.position.unwrap();
            assert!((pos[0] - obj.x as f64).abs() < 1e-6);
            assert!((pos[1] - obj.y as f64).abs() < 1e-6);
            assert!((pos[2] - obj.z as f64).abs() < 1e-6);
        }
    }

    #[test]
    fn objects_anchor_direct_channels_to_their_speaker() {
        // Default 5.1: LFE is direct, the rest virtualized. With an output
        // layout, the LFE object must carry its speaker's index so Studio shows
        // it snapped there; the virtualized channels carry no direct index.
        let labels = [
            RChannelLabel::L,
            RChannelLabel::R,
            RChannelLabel::C,
            RChannelLabel::LFE,
            RChannelLabel::Ls,
            RChannelLabel::Rs,
        ];
        let output = SpeakerLayout::preset("5.1").expect("5.1 preset");
        // LFE is speaker index 3 in the 5.1 preset (FL,FR,C,LFE,BL,BR).
        let objects =
            build_virtual_bed_objects(&labels, None, Some(&output), UNIT_ROOM, 1.0, 1.0, 0.0)
                .expect("all channels emitted");
        assert_eq!(objects.len(), labels.len(), "every channel is shown");
        let lfe = objects
            .iter()
            .find(|o| o.name.eq_ignore_ascii_case("LFE"))
            .expect("LFE object present");
        assert_eq!(lfe.direct_speaker_index, Some(3), "LFE anchored to its sub");
        for obj in objects
            .iter()
            .filter(|o| !o.name.eq_ignore_ascii_case("LFE"))
        {
            assert!(
                obj.direct_speaker_index.is_none(),
                "{} is virtualized, no direct anchor",
                obj.name
            );
        }
    }

    #[test]
    fn objects_carry_per_channel_gain_from_virtual_bed() {
        use renderer::speaker_layout::Speaker;
        // A virtual bed sets C to -6 dB; channels with no explicit gain stay at 0.
        let mut center = Speaker::new("C", 0.0, 0.0);
        center.gain_db = -6;
        let bed = vbed(vec![
            Speaker::new("L", -30.0, 0.0),
            center,
            Speaker::new("R", 30.0, 0.0),
        ]);
        let labels = [RChannelLabel::L, RChannelLabel::C, RChannelLabel::LFE];
        let objects =
            build_virtual_bed_objects(&labels, Some(&bed), None, UNIT_ROOM, 1.0, 1.0, 0.0)
                .expect("all channels emitted");
        let c = objects
            .iter()
            .find(|o| o.name.eq_ignore_ascii_case("C"))
            .expect("C object present");
        assert_eq!(c.gain, -6, "C gain comes from the virtual bed");
        for obj in objects.iter().filter(|o| !o.name.eq_ignore_ascii_case("C")) {
            assert_eq!(obj.gain, 0, "{} has no configured gain", obj.name);
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

    fn vbed(speakers: Vec<renderer::speaker_layout::Speaker>) -> SpeakerLayout {
        SpeakerLayout::from_speakers(speakers).expect("valid virtual bed")
    }

    #[test]
    fn plan_spatial_default_virtualizes_all_but_lfe() {
        // No virtual bed configured → built-in defaults: every channel is a VBAP
        // object except LFE, which routes direct to its sub (bed id 3).
        let plan = plan_channel_render(
            renderer::live_params::ChannelRenderMode::Spatial,
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
                let bi = bed_indices.expect("spatial mode configures bed_indices");
                // One entry per channel: LFE (idx 3) is direct, the rest virtual.
                assert_eq!(
                    bi,
                    vec![
                        usize::MAX,
                        usize::MAX,
                        usize::MAX,
                        3,
                        usize::MAX,
                        usize::MAX
                    ]
                );
                // Six events: five virtual objects + the LFE bed.
                assert_eq!(events.len(), BED_5_1.len());
                let lfe = events.iter().find(|e| e.channel_idx == 3).unwrap();
                assert!(lfe.is_bed, "LFE must be a bed event");
                assert!(lfe.position.is_none());
                for ev in events.iter().filter(|e| e.channel_idx != 3) {
                    assert!(!ev.is_bed, "non-LFE channels are virtual objects");
                    assert!(ev.position.is_some());
                }
            }
            other => panic!("expected Events, got {:?}", PlanKind::from(&other)),
        }
    }

    #[test]
    fn plan_spatial_respects_explicit_per_channel_spatialize() {
        use renderer::speaker_layout::Speaker;
        // Flip the defaults: C routed direct, LFE virtualized. Other channels
        // keep their defaults (virtual).
        let bed = vbed(vec![
            Speaker::new_with_spatialize("C", 0.0, 0.0, false),
            Speaker::new_with_spatialize("LFE", 0.0, 0.0, true),
            Speaker::new_with_spatialize("FL", -30.0, 0.0, true),
        ]);
        let plan = plan_channel_render(
            renderer::live_params::ChannelRenderMode::Spatial,
            &BED_5_1,
            Some(&bed),
            UNIT_ROOM,
            1.0,
            1.0,
            0.0,
        );
        match plan {
            ChannelRenderPlan::Events { bed_indices, .. } => {
                let bi = bed_indices.unwrap();
                // C (idx 2) is now direct → bed id 2; LFE (idx 3) is virtual → MAX.
                assert_eq!(bi[2], 2, "C explicitly direct");
                assert_eq!(bi[3], usize::MAX, "LFE explicitly virtualized");
                assert_eq!(bi[0], usize::MAX, "L keeps the virtual default");
            }
            other => panic!("expected Events, got {:?}", PlanKind::from(&other)),
        }
    }

    #[test]
    fn plan_spatial_keeps_alignment_for_unroutable_direct_channel() {
        use renderer::speaker_layout::Speaker;
        // Back-centre (Cb) has no direct-speaker slot. Even if asked to route
        // direct, it must keep index alignment with a sentinel and emit no event
        // (silent), not shift the other channels.
        let bed = vbed(vec![
            Speaker::new_with_spatialize("BC", 180.0, 0.0, false),
            Speaker::new_with_spatialize("FL", -30.0, 0.0, true),
            Speaker::new_with_spatialize("FR", 30.0, 0.0, true),
        ]);
        let labels = [RChannelLabel::L, RChannelLabel::Cb, RChannelLabel::R];
        let plan = plan_channel_render(
            renderer::live_params::ChannelRenderMode::Spatial,
            &labels,
            Some(&bed),
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
                assert_eq!(bi[1], usize::MAX, "Cb direct but no slot → sentinel");
                // L and R virtualize (objects); Cb produces no event.
                assert!(events.iter().all(|e| e.channel_idx != 1));
                assert_eq!(events.len(), 2);
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

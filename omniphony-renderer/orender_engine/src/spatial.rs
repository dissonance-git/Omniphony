//! Format-agnostic mapping from decoded spatial metadata to the renderer's
//! per-channel events. Shared by the engine and the CLI host.

use crate::events::{Configuration, Event};
use crate::osc::ObjectMeta;
use bridge_api::RCoordinateFormat;
use renderer::spatial_renderer::SpatialChannelEvent;
use renderer::speaker_layout::SpeakerLayout;
use std::collections::HashMap;

/// Canonical display name for the bed-id scheme shared by bridge metadata and
/// [`SpeakerLayout::bed_to_speaker_mapping`]. Dynamic objects start at id 10.
pub fn canonical_bed_name(id: u32) -> Option<&'static str> {
    match id {
        0 => Some("L"),
        1 => Some("R"),
        2 => Some("C"),
        3 => Some("LFE"),
        4 => Some("Ls"),
        5 => Some("Rs"),
        6 => Some("Lb"),
        7 => Some("Rb"),
        8 => Some("TFL"),
        9 => Some("TFR"),
        _ => None,
    }
}

/// Wrap an azimuth in degrees into `[-180, 180]`.
pub fn normalize_azimuth_deg(mut azimuth_deg: f32) -> f32 {
    while azimuth_deg < -180.0 {
        azimuth_deg += 360.0;
    }
    while azimuth_deg > 180.0 {
        azimuth_deg -= 360.0;
    }
    azimuth_deg
}

/// Raw, unconverted position `[x, y, z]` exactly as the event carries it (no
/// coordinate-format conversion). Used for OSC object broadcast, where the
/// coordinate format is sent alongside the values.
pub fn event_pos_raw(event: &Event) -> Option<[f64; 3]> {
    let p = event.pos()?;
    if p.len() < 3 {
        return None;
    }
    Some([p[0], p[1], p[2]])
}

/// Build the OSC broadcast objects for one metadata payload.
///
/// Bed objects (id `< 10`) are reported at their layout speaker position with
/// `direct_speaker_index` set; dynamic objects report their raw event position
/// in the bridge's coordinate format. Names come from the accumulated
/// `object_names` map. Unnamed beds use their canonical speaker name; only
/// unnamed dynamic objects fall back to `Obj_<id>`.
pub fn build_object_metas(
    conf: &Configuration,
    coordinate_format: RCoordinateFormat,
    layout: Option<&SpeakerLayout>,
    object_names: &HashMap<u32, String>,
) -> Vec<ObjectMeta> {
    let bed_to_speaker = layout
        .map(|l| l.bed_to_speaker_mapping())
        .unwrap_or_default();
    conf.events
        .iter()
        .enumerate()
        .map(|(idx, event)| {
            let logical_id = event.id().unwrap_or(idx as u32);
            let direct_speaker_index = if logical_id < 10 {
                bed_to_speaker
                    .get(&(logical_id as usize))
                    .copied()
                    .map(|i| i as u32)
            } else {
                None
            };
            let (ox, oy, oz, coord_mode) = direct_speaker_index
                .and_then(|spk| {
                    layout.and_then(|l| {
                        l.speakers.get(spk as usize).map(|speaker| {
                            if speaker.coord_mode.eq_ignore_ascii_case("cartesian") {
                                (
                                    speaker.x as f64,
                                    speaker.y as f64,
                                    speaker.z as f64,
                                    "cartesian".to_string(),
                                )
                            } else {
                                (
                                    speaker.azimuth as f64,
                                    speaker.elevation as f64,
                                    speaker.distance as f64,
                                    "polar".to_string(),
                                )
                            }
                        })
                    })
                })
                .unwrap_or_else(|| {
                    let [x, y, z] = event_pos_raw(event).unwrap_or([0.0; 3]);
                    (
                        x,
                        y,
                        z,
                        match coordinate_format {
                            RCoordinateFormat::Cartesian => "cartesian".to_string(),
                            RCoordinateFormat::Polar => "polar".to_string(),
                        },
                    )
                });
            ObjectMeta {
                name: object_names.get(&logical_id).cloned().unwrap_or_else(|| {
                    canonical_bed_name(logical_id)
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("Obj_{logical_id}"))
                }),
                x: ox as f32,
                y: oy as f32,
                z: oz as f32,
                coord_mode,
                direct_speaker_index,
                gain: event.gain_db().map_or(-128, |g| g as i32),
                priority: 0.0,
                size: event
                    .size()
                    .map(|s| [s[0] as f32, s[1] as f32, s[2] as f32])
                    .unwrap_or([0.0, 0.0, 0.0]),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::Event;

    #[test]
    fn unnamed_beds_get_canonical_names_and_lfe_stays_direct() {
        let bed_ids = [2, 0, 1, 4, 5, 3, 6, 7];
        let mut events: Vec<Event> = bed_ids.iter().map(|&id| Event::with_id(id)).collect();
        events.push(Event::with_id(10));
        let conf = Configuration::new(events);
        let layout = SpeakerLayout::preset_7_1_4().expect("7.1.4 preset");

        let objects = build_object_metas(
            &conf,
            RCoordinateFormat::Cartesian,
            Some(&layout),
            &HashMap::new(),
        );

        let names: Vec<&str> = objects.iter().map(|object| object.name.as_str()).collect();
        assert_eq!(
            names,
            ["C", "L", "R", "Ls", "Rs", "LFE", "Lb", "Rb", "Obj_10"]
        );
        assert_eq!(objects[5].direct_speaker_index, Some(3));
        assert_eq!(objects[5].coord_mode, "cartesian");
        assert_eq!(
            [objects[5].x, objects[5].y, objects[5].z],
            [
                layout.speakers[3].x,
                layout.speakers[3].y,
                layout.speakers[3].z,
            ]
        );
        assert_eq!(objects[8].direct_speaker_index, None);
    }

    #[test]
    fn explicit_names_override_canonical_bed_names() {
        let conf = Configuration::new(vec![Event::with_id(3)]);
        let names = HashMap::from([(3, "Effects".to_string())]);

        let objects = build_object_metas(&conf, RCoordinateFormat::Cartesian, None, &names);

        assert_eq!(objects[0].name, "Effects");
    }
}

/// Resolve an event's position to ADM Cartesian `[x, y, z]`, converting from the
/// bridge's declared coordinate format. Returns `None` when the event carries no
/// position (e.g. a bed channel or a gain-only update).
pub fn event_pos_as_adm_cartesian(
    coordinate_format: RCoordinateFormat,
    event: &Event,
) -> Option<[f64; 3]> {
    let p = event.pos()?;
    if p.len() < 3 {
        return None;
    }

    match coordinate_format {
        RCoordinateFormat::Cartesian => Some([p[0], p[1], p[2]]),
        RCoordinateFormat::Polar => {
            let az = normalize_azimuth_deg(p[0] as f32);
            let el = (p[1] as f32).clamp(-90.0, 90.0);
            let dist = (p[2] as f32).max(0.0);
            let (x, y, z) = renderer::spatial_vbap::spherical_to_adm(az, el, dist);
            Some([x as f64, y as f64, z as f64])
        }
    }
}

/// Build the renderer's per-channel events from one metadata payload and append
/// them to `out`.
///
/// Object IDs `< 10` are bed channels: they map to a PCM channel index via
/// `bed_indices` (events for beds not present in the layout are skipped). IDs
/// `>= 10` are dynamic objects, placed after the beds in PCM order.
pub fn build_spatial_channel_events(
    conf: &Configuration,
    coordinate_format: RCoordinateFormat,
    bed_indices: &[usize],
    out: &mut Vec<SpatialChannelEvent>,
) {
    let bed_id_to_channel: HashMap<usize, usize> = bed_indices
        .iter()
        .enumerate()
        .map(|(idx, &bid)| (bid, idx))
        .collect();
    let num_beds = bed_indices.len();

    for event in &conf.events {
        let object_id = match event.id() {
            Some(id) => id as usize,
            None => continue,
        };
        let (channel_idx, is_bed) = if object_id < 10 {
            match bed_id_to_channel.get(&object_id) {
                Some(&ch) => (ch, true),
                None => continue,
            }
        } else {
            (num_beds + (object_id - 10), false)
        };
        out.push(SpatialChannelEvent {
            channel_idx,
            is_bed,
            gain_db: event.gain_db(),
            ramp_length: event.ramp_length(),
            size: event
                .size()
                .map(|s| [s[0] as f32, s[1] as f32, s[2] as f32]),
            position: event_pos_as_adm_cartesian(coordinate_format, event),
            sample_pos: event.sample_pos(),
        });
    }
}

//! Format-agnostic mapping from decoded spatial metadata to the renderer's
//! per-channel events. Shared by the engine and the CLI host.

use crate::events::{Configuration, Event};
use bridge_api::RCoordinateFormat;
use renderer::spatial_renderer::SpatialChannelEvent;
use std::collections::HashMap;

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

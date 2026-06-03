//! Per-crossover-band speaker gain table, sampled over the cartesian grid.
//!
//! Built once per topology (see [`crate::live_params::RendererControl::build_band_gaintable_full`])
//! and cached raw. Because a heatmap only ever shows **one** speaker, the wire
//! payload is serialized per speaker ([`BandGaintableFull::serialize_for_speaker`])
//! — one value per cell per band — which is `speaker_count`× smaller than shipping
//! the whole table. Format "OBGT": 16-byte header (magic + version + meta_len +
//! payload_len), metadata JSON, then zlib(x_pos, y_pos, z_pos, band0 gains, …).

use std::io::Write as _;

/// One crossover band's full field: `gains[cell * speaker_count + speaker]`.
pub struct BandField {
    pub low_hz: f32,
    pub high_hz: f32,
    pub gains: Vec<f32>,
}

/// All bands' fields over a shared cartesian grid (all speakers).
pub struct BandGaintableFull {
    pub x_positions: Vec<f32>,
    pub y_positions: Vec<f32>,
    pub z_positions: Vec<f32>,
    pub speaker_count: usize,
    pub bands: Vec<BandField>,
}

impl BandGaintableFull {
    /// Serialize just one speaker's per-band field (one f32 per cell per band) to
    /// the "OBGT" wire format. `speaker` out of range yields an all-zero field.
    pub fn serialize_for_speaker(&self, speaker: usize) -> Vec<u8> {
        let nx = self.x_positions.len();
        let ny = self.y_positions.len();
        let nz = self.z_positions.len();
        let cells = nx * ny * nz;
        let sc = self.speaker_count.max(1);

        let metadata = serde_json::json!({
            "domain": "cartesian_bands",
            "speaker_index": speaker,
            "x_count": nx,
            "y_count": ny,
            "z_count": nz,
            "band_count": self.bands.len(),
            "bands": self.bands
                .iter()
                .map(|b| {
                    serde_json::json!({
                        "low_hz": b.low_hz,
                        "high_hz": if b.high_hz.is_finite() {
                            serde_json::json!(b.high_hz)
                        } else {
                            serde_json::Value::Null
                        },
                    })
                })
                .collect::<Vec<_>>(),
        })
        .to_string();

        let mut raw: Vec<u8> = Vec::with_capacity((nx + ny + nz + cells * self.bands.len()) * 4);
        for &v in &self.x_positions {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &self.y_positions {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        for &v in &self.z_positions {
            raw.extend_from_slice(&v.to_le_bytes());
        }
        for band in &self.bands {
            for cell in 0..cells {
                let g = band.gains.get(cell * sc + speaker).copied().unwrap_or(0.0);
                raw.extend_from_slice(&g.to_le_bytes());
            }
        }

        let mut enc = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        let _ = enc.write_all(&raw);
        let payload = enc.finish().unwrap_or_default();

        let meta = metadata.as_bytes();
        let mut out = Vec::with_capacity(16 + meta.len() + payload.len());
        out.extend_from_slice(b"OBGT");
        out.push(1); // version
        out.extend_from_slice(&[0u8; 3]); // reserved
        out.extend_from_slice(&(meta.len() as u32).to_le_bytes());
        out.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        out.extend_from_slice(meta);
        out.extend_from_slice(&payload);
        out
    }
}

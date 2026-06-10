//! Measured HRIR sets (e.g. the embedded SAF KEMAR data, or a loaded SOFA file).
//!
//! A [`MeasuredHrirData`] holds scattered-direction impulse responses. It
//! implements [`HrirProvider`] by nearest-direction lookup followed by
//! onset-alignment and truncation to [`HRIR_LEN`], so it plugs straight into
//! [`HrirSet::new`](super::hrir::HrirSet::new) and reuses the regular-grid
//! bilinear interpolation. Time alignment (the interaural delay is supplied
//! analytically, see [`super::itd`]) keeps the grid interpolable without
//! comb-filtering.

use super::hrir::{HRIR_LEN, HrirPair, HrirProvider};

/// Embedded SAF default HRIRs: Genelec Aural ID of a KEMAR dummy head @48 kHz,
/// ISC-licensed (© 2020 Leo McCormack; data by Aki Mäkivirta & Jaan Johansson).
/// Pre-aligned and truncated to `HRIR_LEN` by `tools/gen_saf_hrir.py`.
static SAF_KEMAR_BLOB: &[u8] = include_bytes!("data/saf_kemar.bin");

const BLOB_MAGIC: u32 = 0x4F48_4952; // 'OHIR'

/// Onset detection / alignment parameters (mirror the generator's, used for any
/// not-yet-aligned source such as SOFA).
const PRE_SAMPLES: usize = 8;
const ONSET_FRAC: f32 = 0.15;

/// A scattered set of measured HRIR pairs with their directions (renderer
/// convention: az 0 = front, +az = right; el 0 = horizontal, +90 = up).
pub struct MeasuredHrirData {
    pub sample_rate: u32,
    /// `(azimuth_deg, elevation_deg)` per measurement.
    dirs: Vec<(f32, f32)>,
    /// Unit direction vectors, parallel to `dirs`, for nearest lookup.
    vecs: Vec<[f32; 3]>,
    /// Left/right impulse responses per measurement (arbitrary length).
    irs: Vec<(Vec<f32>, Vec<f32>)>,
}

impl MeasuredHrirData {
    /// Build from raw measurements. `dirs[i]` corresponds to `irs[i]`.
    pub fn new(sample_rate: u32, dirs: Vec<(f32, f32)>, irs: Vec<(Vec<f32>, Vec<f32>)>) -> Self {
        let vecs = dirs.iter().map(|&(az, el)| dir_vec(az, el)).collect();
        Self {
            sample_rate,
            dirs,
            vecs,
            irs,
        }
    }

    /// The embedded SAF KEMAR set.
    pub fn saf_kemar() -> Self {
        Self::from_blob(SAF_KEMAR_BLOB).expect("embedded SAF KEMAR blob is valid")
    }

    pub fn len(&self) -> usize {
        self.dirs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dirs.is_empty()
    }

    fn from_blob(blob: &[u8]) -> Option<Self> {
        let u32_at = |off: usize| -> Option<u32> {
            blob.get(off..off + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let f32_at =
            |off: usize| -> f32 { f32::from_le_bytes(blob[off..off + 4].try_into().unwrap()) };
        if u32_at(0)? != BLOB_MAGIC {
            return None;
        }
        let count = u32_at(8)? as usize;
        let ir_len = u32_at(12)? as usize;
        let fs = u32_at(16)?;
        let mut off = 20;
        let rec = 8 + ir_len * 2 * 4;
        let mut dirs = Vec::with_capacity(count);
        let mut irs = Vec::with_capacity(count);
        for _ in 0..count {
            if off + rec > blob.len() {
                return None;
            }
            let az = f32_at(off);
            let el = f32_at(off + 4);
            let mut p = off + 8;
            let mut left = Vec::with_capacity(ir_len);
            let mut right = Vec::with_capacity(ir_len);
            for _ in 0..ir_len {
                left.push(f32_at(p));
                p += 4;
            }
            for _ in 0..ir_len {
                right.push(f32_at(p));
                p += 4;
            }
            dirs.push((az, el));
            irs.push((left, right));
            off += rec;
        }
        Some(Self::new(fs, dirs, irs))
    }

    /// Index of the measurement nearest to a query direction (max dot product).
    fn nearest(&self, az_deg: f32, el_deg: f32) -> usize {
        let q = dir_vec(az_deg, el_deg);
        let mut best = 0usize;
        let mut best_dot = f32::NEG_INFINITY;
        for (i, v) in self.vecs.iter().enumerate() {
            let d = q[0] * v[0] + q[1] * v[1] + q[2] * v[2];
            if d > best_dot {
                best_dot = d;
                best = i;
            }
        }
        best
    }
}

impl HrirProvider for MeasuredHrirData {
    fn render(&self, az_deg: f32, el_deg: f32, _sample_rate: u32) -> HrirPair {
        let mut pair = HrirPair {
            left: [0.0; HRIR_LEN],
            right: [0.0; HRIR_LEN],
        };
        if self.irs.is_empty() {
            return pair;
        }
        let (l, r) = &self.irs[self.nearest(az_deg, el_deg)];
        align_into(l, &mut pair.left);
        align_into(r, &mut pair.right);
        pair
    }
}

/// Unit vector for a direction (az 0 = front/+Y, +az = right/+X; el up = +Z).
fn dir_vec(az_deg: f32, el_deg: f32) -> [f32; 3] {
    let az = az_deg.to_radians();
    let el = el_deg.to_radians();
    let ce = el.cos();
    [ce * az.sin(), ce * az.cos(), el.sin()]
}

/// Onset-align `ir` and copy `HRIR_LEN` taps into `out`. Idempotent for an
/// already-aligned IR (onset ≈ 0).
fn align_into(ir: &[f32], out: &mut [f32; HRIR_LEN]) {
    if ir.is_empty() {
        out.fill(0.0);
        return;
    }
    let peak = ir.iter().fold(0.0f32, |m, &x| m.max(x.abs())).max(1e-12);
    let thresh = ONSET_FRAC * peak;
    let onset = ir.iter().position(|&x| x.abs() >= thresh).unwrap_or(0);
    let start = onset.saturating_sub(PRE_SAMPLES);
    for (k, slot) in out.iter_mut().enumerate() {
        *slot = ir.get(start + k).copied().unwrap_or(0.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binaural::hrir::HrirSet;

    #[test]
    fn embedded_saf_loads() {
        let d = MeasuredHrirData::saf_kemar();
        assert_eq!(d.len(), 836);
        assert_eq!(d.sample_rate, 48_000);
    }

    fn energy(h: &[f32]) -> f32 {
        h.iter().map(|&x| x * x).sum()
    }

    #[test]
    fn measured_right_source_is_louder_in_right_ear() {
        // Validates the SAF→renderer azimuth handedness (+az = right).
        let set = HrirSet::new(&MeasuredHrirData::saf_kemar(), 48_000);
        let mut p = HrirPair {
            left: [0.0; HRIR_LEN],
            right: [0.0; HRIR_LEN],
        };
        set.at(90.0, 0.0, &mut p);
        assert!(
            energy(&p.right) > energy(&p.left),
            "L>R: handedness flipped?"
        );
    }

    #[test]
    fn measured_front_is_roughly_symmetric() {
        let set = HrirSet::new(&MeasuredHrirData::saf_kemar(), 48_000);
        let mut p = HrirPair {
            left: [0.0; HRIR_LEN],
            right: [0.0; HRIR_LEN],
        };
        set.at(0.0, 0.0, &mut p);
        let (el, er) = (energy(&p.left), energy(&p.right));
        let ratio = el / er;
        assert!(
            (0.5..2.0).contains(&ratio),
            "front asymmetric L={el} R={er}"
        );
    }
}

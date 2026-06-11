//! Interaural time difference (ITD) from source direction.
//!
//! Uses Woodworth's spherical-head approximation. The broadband ITD is applied
//! as a pure per-ear delay (via [`crate::delay_line::DelayLine`]) rather than
//! baked into the HRIR phase, so head-tracking can move it smoothly without
//! re-deriving filters.

/// Default effective head radius (m) — KEMAR-ish. Live-tunable via
/// `BinauralLiveParams::head_radius_m` for per-listener ITD fit.
pub const DEFAULT_HEAD_RADIUS_M: f32 = 0.0875;
/// Speed of sound (m/s).
const SPEED_OF_SOUND: f32 = 343.0;

/// Per-ear delays in seconds for a source at the given azimuth/elevation.
///
/// `azimuth_rad`: 0 = front, positive = source to the **right**.
/// `elevation_rad`: 0 = horizontal; the ITD shrinks toward the poles by
/// `cos(elevation)`.
///
/// Returns `(left_delay_s, right_delay_s)`, both ≥ 0: the ear nearer the source
/// gets 0, the far (contralateral) ear gets the positive ITD. A source on the
/// right therefore delays the **left** ear.
pub fn ear_delays_seconds(azimuth_rad: f32, elevation_rad: f32, head_radius_m: f32) -> (f32, f32) {
    // Woodworth: Δt = (r/c)(θ + sinθ) for the far ear, with θ measured from the
    // median plane and clamped to ±90° (rear hemisphere mirrors the front).
    let mut theta = azimuth_rad.rem_euclid(std::f32::consts::TAU);
    if theta > std::f32::consts::PI {
        theta -= std::f32::consts::TAU;
    }
    // Fold the rear hemisphere onto [-90°, 90°] (front/back ITD is ~symmetric).
    let folded = if theta > std::f32::consts::FRAC_PI_2 {
        std::f32::consts::PI - theta
    } else if theta < -std::f32::consts::FRAC_PI_2 {
        -std::f32::consts::PI - theta
    } else {
        theta
    };
    let mag = (head_radius_m / SPEED_OF_SOUND)
        * (folded.abs() + folded.abs().sin())
        * elevation_rad.cos().abs();
    if folded >= 0.0 {
        // Source on the right → left ear is the far ear.
        (mag, 0.0)
    } else {
        (0.0, mag)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn front_source_has_zero_itd() {
        let (l, r) = ear_delays_seconds(0.0, 0.0, DEFAULT_HEAD_RADIUS_M);
        assert!(l.abs() < 1e-9 && r.abs() < 1e-9);
    }

    #[test]
    fn right_source_delays_left_ear() {
        let (l, r) = ear_delays_seconds(std::f32::consts::FRAC_PI_2, 0.0, DEFAULT_HEAD_RADIUS_M);
        assert!(l > r);
        assert!(r.abs() < 1e-9);
        // Max ITD for a 0.0875 m head ≈ 0.66 ms.
        assert!((l - 0.00066).abs() < 0.0002, "itd={l}");
    }

    #[test]
    fn left_source_delays_right_ear() {
        let (l, r) = ear_delays_seconds(-std::f32::consts::FRAC_PI_2, 0.0, DEFAULT_HEAD_RADIUS_M);
        assert!(r > l);
        assert!(l.abs() < 1e-9);
    }

    #[test]
    fn elevated_source_has_smaller_itd_than_horizontal() {
        let (l_horiz, _) =
            ear_delays_seconds(std::f32::consts::FRAC_PI_2, 0.0, DEFAULT_HEAD_RADIUS_M);
        let (l_high, _) = ear_delays_seconds(
            std::f32::consts::FRAC_PI_2,
            std::f32::consts::FRAC_PI_4,
            DEFAULT_HEAD_RADIUS_M,
        );
        assert!(l_high < l_horiz && l_high > 0.0);
    }
}

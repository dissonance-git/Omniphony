//! Block-clock object orbit adapted from Omniphony v0.5's object-test tool.
//!
//! The useful primitive is the motion, not its Studio controls. A deterministic
//! orbit exposes panning seams between static sample points, advances on the
//! audio block clock instead of a UI timer, and keeps phase continuous when the
//! period changes. This dev-only fixture can drive renderer tests and synthetic
//! Windows smoke playthroughs without adding product UI or OSC surface.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OrbitAxis {
    X,
    Y,
    Z,
    Free {
        azimuth_deg: f32,
        elevation_deg: f32,
    },
}

impl Default for OrbitAxis {
    fn default() -> Self {
        Self::Z
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Orbit {
    pub axis: OrbitAxis,
    pub radius: f32,
    pub period_s: f32,
    /// Per-axis absolute room bound. `None` leaves the orbit unclamped.
    pub room_bound: Option<[f32; 3]>,
    phase_turns: f64,
}

impl Orbit {
    pub fn new(axis: OrbitAxis, radius: f32, period_s: f32) -> Self {
        Self {
            axis,
            radius: radius.max(0.0),
            period_s: period_s.max(1.0e-4),
            room_bound: None,
            phase_turns: 0.0,
        }
    }

    pub fn with_room_bound(mut self, bound: [f32; 3]) -> Self {
        self.room_bound = Some([bound[0].abs(), bound[1].abs(), bound[2].abs()]);
        self
    }

    pub fn phase_turns(&self) -> f64 {
        self.phase_turns
    }

    /// Change speed without reinterpreting elapsed time. The current phase is
    /// untouched, so changing the period changes only the next angular step.
    pub fn set_period_s(&mut self, period_s: f32) {
        self.period_s = period_s.max(1.0e-4);
    }

    /// Advance by one rendered block and return the new sounding position.
    pub fn advance(
        &mut self,
        centre: [f32; 3],
        block_samples: usize,
        sample_rate: u32,
    ) -> [f32; 3] {
        let dt = block_samples as f64 / sample_rate.max(1) as f64;
        self.phase_turns = (self.phase_turns + dt / self.period_s as f64).rem_euclid(1.0);
        self.position(centre)
    }

    pub fn position(&self, centre: [f32; 3]) -> [f32; 3] {
        if self.radius <= 0.0 {
            return self.clamp(centre);
        }
        let (_, u, v) = frame(self.axis);
        let angle = (self.phase_turns * std::f64::consts::TAU) as f32;
        let (sin, cos) = angle.sin_cos();
        let mut out = centre;
        for i in 0..3 {
            out[i] += self.radius * (u[i] * cos + v[i] * sin);
        }
        self.clamp(out)
    }

    fn clamp(&self, mut p: [f32; 3]) -> [f32; 3] {
        if let Some(bound) = self.room_bound {
            for i in 0..3 {
                p[i] = p[i].clamp(-bound[i], bound[i]);
            }
        }
        p
    }
}

/// Axis plus right-handed basis vectors spanning the orbit plane.
pub fn frame(axis: OrbitAxis) -> ([f32; 3], [f32; 3], [f32; 3]) {
    match axis {
        OrbitAxis::X => ([1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]),
        OrbitAxis::Y => ([0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [1.0, 0.0, 0.0]),
        OrbitAxis::Z => ([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]),
        OrbitAxis::Free {
            azimuth_deg,
            elevation_deg,
        } => {
            let az = azimuth_deg.to_radians();
            let el = elevation_deg.to_radians();
            let axis = [el.cos() * az.sin(), el.cos() * az.cos(), el.sin()];
            let seed = if axis[2].abs() < 0.9 {
                [0.0, 0.0, 1.0]
            } else {
                [1.0, 0.0, 0.0]
            };
            let u = normalize(cross(seed, axis));
            let v = normalize(cross(axis, u));
            (axis, u, v)
        }
    }
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let n = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if n < 1.0e-6 {
        [1.0, 0.0, 0.0]
    } else {
        [v[0] / n, v[1] / n, v[2] / n]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: u32 = 48_000;
    const BLOCK: usize = 40;

    fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2))
            .sqrt()
    }

    #[test]
    fn two_second_orbit_returns_to_start_on_the_block_clock() {
        let centre = [0.0, 0.0, 0.0];
        let mut orbit = Orbit::new(OrbitAxis::Z, 0.5, 2.0);
        let start = orbit.position(centre);
        let blocks = (2.0 * SR as f32 / BLOCK as f32).round() as usize;
        let mut end = start;
        for _ in 0..blocks {
            end = orbit.advance(centre, BLOCK, SR);
        }
        assert!(distance(start, end) < 1.0e-4, "period drift: {start:?} -> {end:?}");
        assert!(orbit.phase_turns() < 1.0e-4 || orbit.phase_turns() > 0.9999);
    }

    #[test]
    fn changing_period_does_not_teleport_phase() {
        let centre = [0.1, -0.2, 0.3];
        let mut orbit = Orbit::new(OrbitAxis::Z, 0.4, 4.0);
        for _ in 0..600 {
            orbit.advance(centre, BLOCK, SR);
        }
        let before = orbit.position(centre);
        let phase = orbit.phase_turns();
        orbit.set_period_s(1.0);
        let unchanged = orbit.position(centre);
        assert_eq!(phase, orbit.phase_turns());
        assert!(distance(before, unchanged) < 1.0e-7);
        let after = orbit.advance(centre, BLOCK, SR);
        assert!(distance(before, after) < 0.01, "speed change jumped: {before:?} -> {after:?}");
    }

    #[test]
    fn x_axis_orbit_spans_height() {
        let mut orbit = Orbit::new(OrbitAxis::X, 1.0, 1.0);
        let mut lo = f32::INFINITY;
        let mut hi = f32::NEG_INFINITY;
        for _ in 0..1200 {
            let p = orbit.advance([0.0; 3], BLOCK, SR);
            lo = lo.min(p[2]);
            hi = hi.max(p[2]);
        }
        assert!(lo < -0.99 && hi > 0.99, "height range {lo}..{hi}");
    }

    #[test]
    fn room_clamp_is_per_axis_and_never_freezes_the_whole_orbit() {
        let centre = [0.9, 0.0, 0.0];
        let mut orbit = Orbit::new(OrbitAxis::Z, 1.0, 1.0).with_room_bound([1.0; 3]);
        let mut wall_frames = 0usize;
        let mut moving_on_wall = 0usize;
        let mut previous = orbit.position(centre);
        for _ in 0..1200 {
            let p = orbit.advance(centre, BLOCK, SR);
            if (p[0] - 1.0).abs() < 1.0e-6 {
                wall_frames += 1;
                if distance(p, previous) > 1.0e-5 {
                    moving_on_wall += 1;
                }
            }
            previous = p;
        }
        assert!(wall_frames > 100, "test never exercised the wall clamp");
        assert!(moving_on_wall > wall_frames / 2, "per-axis clamp froze the orbit");
    }

    #[test]
    fn free_axis_frame_is_orthonormal() {
        let (axis, u, v) = frame(OrbitAxis::Free {
            azimuth_deg: 37.0,
            elevation_deg: 23.0,
        });
        let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
        let norm = |a: [f32; 3]| dot(a, a).sqrt();
        assert!((norm(axis) - 1.0).abs() < 1.0e-5);
        assert!((norm(u) - 1.0).abs() < 1.0e-5);
        assert!((norm(v) - 1.0).abs() < 1.0e-5);
        assert!(dot(axis, u).abs() < 1.0e-5);
        assert!(dot(axis, v).abs() < 1.0e-5);
        assert!(dot(u, v).abs() < 1.0e-5);
    }
}

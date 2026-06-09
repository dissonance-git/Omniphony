//! Live head-tracking input (e.g. SensorsOSC on Android).
//!
//! The orientation arrives on an arbitrary OSC address as a few floats. This
//! module owns the *pure* parts — value-format decoding, recenter reference and
//! exponential smoothing — so they are unit-testable without any OSC plumbing.
//! The transport glue (address match, locking) lives in the engine's OSC loop.

use super::HeadPose;

/// How to interpret the float payload of a head-tracking OSC message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeadTrackingFormat {
    /// Best-effort: 4 floats → `[x, y, z, w]` quaternion (Android order);
    /// 3 floats → rotation vector (`[x, y, z]`, `w` derived).
    #[default]
    Auto,
    /// Explicit quaternion `[x, y, z, w]`.
    Quat,
    /// Rotation vector `[x, y, z]`; `w = √(1 − x² − y² − z²)`.
    RotVec,
    /// Euler degrees `[yaw, pitch, roll]`.
    Euler,
}

impl HeadTrackingFormat {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "auto" => Some(Self::Auto),
            "quat" | "quaternion" => Some(Self::Quat),
            "rotvec" | "rotation_vector" | "rotationvector" => Some(Self::RotVec),
            "euler" | "orientation" | "ypr" => Some(Self::Euler),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Quat => "quat",
            Self::RotVec => "rotvec",
            Self::Euler => "euler",
        }
    }

    /// Decode a raw orientation from the message's float arguments.
    pub fn parse(self, args: &[f32]) -> Option<HeadPose> {
        match self {
            Self::Quat => {
                let [x, y, z, w] = first4(args)?;
                Some(HeadPose::from_quat(w, x, y, z))
            }
            Self::RotVec => {
                let [x, y, z] = first3(args)?;
                Some(rotvec_to_pose(x, y, z))
            }
            Self::Euler => {
                let [yaw, pitch, roll] = first3(args)?;
                Some(HeadPose::from_euler_deg(yaw, pitch, roll))
            }
            Self::Auto => {
                if args.len() >= 4 {
                    let [x, y, z, w] = first4(args)?;
                    Some(HeadPose::from_quat(w, x, y, z))
                } else if args.len() >= 3 {
                    let [x, y, z] = first3(args)?;
                    Some(rotvec_to_pose(x, y, z))
                } else {
                    None
                }
            }
        }
    }
}

fn first3(a: &[f32]) -> Option<[f32; 3]> {
    match a {
        [x, y, z, ..] => Some([*x, *y, *z]),
        _ => None,
    }
}

fn first4(a: &[f32]) -> Option<[f32; 4]> {
    match a {
        [x, y, z, w, ..] => Some([*x, *y, *z, *w]),
        _ => None,
    }
}

/// Android rotation-vector → quaternion: the three components are the vector part
/// `(x, y, z)`; the scalar `w` is recovered (clamped) from unit-norm.
fn rotvec_to_pose(x: f32, y: f32, z: f32) -> HeadPose {
    let w = (1.0 - (x * x + y * y + z * z)).max(0.0).sqrt();
    HeadPose::from_quat(w, x, y, z)
}

/// Head-tracking configuration *and* live recenter/smoothing state. Written by
/// the OSC listener thread; the smoothed result is mirrored into
/// `BinauralLiveParams::head_pose`, which the render thread reads.
#[derive(Debug, Clone)]
pub struct HeadTracking {
    /// OSC address carrying the orientation. `None`/empty → tracking disabled.
    pub address: Option<String>,
    /// Value format of the payload.
    pub format: HeadTrackingFormat,
    /// Exponential smoothing in [0, 1): 0 = instant, higher = smoother/laggier.
    pub smoothing: f32,
    /// Flip the applied rotation, for sensors whose motion comes out mirrored.
    pub invert: bool,
    /// Orientation captured at the last recenter — defines "looking forward".
    pub reference: HeadPose,
    /// Last raw orientation received (captured as the reference on recenter).
    pub last_raw: HeadPose,
}

impl Default for HeadTracking {
    fn default() -> Self {
        Self {
            address: None,
            format: HeadTrackingFormat::Auto,
            smoothing: 0.2,
            invert: false,
            reference: HeadPose::identity(),
            last_raw: HeadPose::identity(),
        }
    }
}

impl HeadTracking {
    /// Whether `addr` is the configured tracking address (non-empty match).
    pub fn matches(&self, addr: &str) -> bool {
        self.address
            .as_deref()
            .is_some_and(|a| !a.is_empty() && a == addr)
    }

    /// Convert a raw device orientation, relative to the recenter reference, into
    /// the world→head rotation the renderer applies to source positions. At the
    /// instant of recenter (`raw == reference`) this is the identity.
    fn world_to_head(&self, raw: HeadPose) -> HeadPose {
        if self.invert {
            self.reference.conjugate().mul(raw)
        } else {
            raw.conjugate().mul(self.reference)
        }
        .normalized()
    }

    /// Ingest a raw orientation and return the new smoothed pose to render with.
    /// `current` is the pose currently in use (for exponential smoothing).
    pub fn ingest(&mut self, raw: HeadPose, current: HeadPose) -> HeadPose {
        self.last_raw = raw;
        let target = self.world_to_head(raw);
        let t = (1.0 - self.smoothing.clamp(0.0, 0.999)).clamp(0.0, 1.0);
        current.nlerp(target, t)
    }

    /// Capture the current raw orientation as the new "forward" reference.
    pub fn recenter(&mut self) {
        self.reference = self.last_raw;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(p: HeadPose, q: HeadPose) {
        let d = (p.w - q.w).abs() + (p.x - q.x).abs() + (p.y - q.y).abs() + (p.z - q.z).abs();
        // q and −q are the same rotation; accept either sign.
        let d2 = (p.w + q.w).abs() + (p.x + q.x).abs() + (p.y + q.y).abs() + (p.z + q.z).abs();
        assert!(d.min(d2) < 1e-5, "{p:?} != {q:?}");
    }

    #[test]
    fn parse_euler_and_quat() {
        let e = HeadTrackingFormat::Euler.parse(&[90.0, 0.0, 0.0]).unwrap();
        approx(e, HeadPose::from_euler_deg(90.0, 0.0, 0.0));
        let q = HeadTrackingFormat::Quat
            .parse(&[0.0, 0.0, 0.7071, 0.7071])
            .unwrap();
        approx(q, HeadPose::from_quat(0.7071, 0.0, 0.0, 0.7071));
    }

    #[test]
    fn auto_picks_quat_for_four_floats_rotvec_for_three() {
        assert!(
            HeadTrackingFormat::Auto
                .parse(&[0.0, 0.0, 0.0, 1.0])
                .is_some()
        );
        assert!(HeadTrackingFormat::Auto.parse(&[0.0, 0.0, 0.0]).is_some());
        assert!(HeadTrackingFormat::Auto.parse(&[0.0]).is_none());
    }

    #[test]
    fn rotvec_recovers_unit_quaternion() {
        let p = rotvec_to_pose(0.0, 0.0, 0.0);
        approx(p, HeadPose::identity());
    }

    #[test]
    fn recenter_makes_current_orientation_forward() {
        let mut ht = HeadTracking {
            smoothing: 0.0,
            ..Default::default()
        };
        // Look 40° right, recenter: the rendered pose must become identity.
        let raw = HeadPose::from_euler_deg(40.0, 0.0, 0.0);
        ht.ingest(raw, HeadPose::identity());
        ht.recenter();
        let pose = ht.ingest(raw, HeadPose::identity());
        approx(pose, HeadPose::identity());
    }

    #[test]
    fn smoothing_moves_only_partway() {
        let mut ht = HeadTracking {
            smoothing: 0.5,
            ..Default::default()
        };
        let raw = HeadPose::from_euler_deg(90.0, 0.0, 0.0);
        let p1 = ht.ingest(raw, HeadPose::identity());
        // Halfway: not yet at the target, not still at identity.
        let target = ht.world_to_head(raw);
        approx_between(p1, HeadPose::identity(), target);
    }

    fn approx_between(p: HeadPose, a: HeadPose, b: HeadPose) {
        let da = (p.w - a.w).abs() + (p.x - a.x).abs() + (p.y - a.y).abs() + (p.z - a.z).abs();
        let db = (p.w - b.w).abs() + (p.x - b.x).abs() + (p.y - b.y).abs() + (p.z - b.z).abs();
        assert!(
            da > 1e-3 && db > 1e-3,
            "expected strictly between endpoints"
        );
    }
}

//! Rigid cooked-track deltas without model, gameplay or rendering ownership.
use crate::track::{Kind, Track};
use glam::{DQuat, DVec3};

/// Explicit admission bounds for this primitive, not new cooked-model limits.
pub const MAX_ROOT_MOTION_DURATION: f64 = 86_400.;
/// At most 33 exponentiation steps, independent of the absolute cycle counters.
pub const MAX_ROOT_MOTION_CYCLES: u64 = 1 << 32;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RootMotionError {
    #[error("invalid root-motion track: {0}")]
    Track(&'static str),
    #[error("root-motion capacity: {0}")]
    Capacity(&'static str),
    #[error("root-motion clip duration exceeds admission bound")]
    Duration,
    #[error("root-motion time must be finite")]
    Time,
    #[error("root-motion loop phase must be in [0, positive duration)")]
    Phase,
    #[error("root-motion cycle difference exceeds admission bound")]
    Cycles,
    #[error("nonfinite root-motion transform")]
    NonFinite,
}
type Result<T> = std::result::Result<T, RootMotionError>;

/// An exact signed cycle count and a finite phase in [0, clip duration).
/// Absolute counts may be large: only their checked difference is exponentiated.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootMotionTime {
    pub cycle: i64,
    pub seconds: f64,
}

/// Right-multiply this rigid transform onto the previously accepted root frame.
/// Translation is in that frame; rotation is a unit XYZW quaternion. This is
/// local node motion: parent, model, entity and physics spaces are not applied.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootMotionDelta {
    pub translation: [f64; 3],
    pub rotation: [f64; 4],
}
impl RootMotionDelta {
    pub const IDENTITY: Self = Self {
        translation: [0.; 3],
        rotation: [0., 0., 0., 1.],
    };

    /// Convert an admitted cooked TRS root/ancestor to a unit rigid transform.
    pub fn from_rest(translation: [f32; 3], rotation: [f32; 4]) -> Result<Self> {
        if translation.iter().any(|v| !v.is_finite()) || !crate::track::quaternion(rotation) {
            return Err(RootMotionError::Track("rigid rest transform"));
        }
        Ok(Rigid {
            translation: DVec3::from_array(translation.map(f64::from)),
            rotation: DQuat::from_array(rotation.map(f64::from)).normalize(),
        }
        .delta())
    }

    /// Ordered rigid composition, with bounded numerical admission only.
    pub fn compose(self, other: Self) -> Result<Self> {
        Ok(Rigid::checked(self)?
            .compose(Rigid::checked(other)?)?
            .delta())
    }

    pub fn inverse(self) -> Result<Self> {
        let value = Rigid::checked(self)?.inverse();
        if !value.translation.is_finite() {
            return Err(RootMotionError::NonFinite);
        }
        Ok(value.delta())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Rigid {
    translation: DVec3,
    rotation: DQuat,
}
impl Rigid {
    const IDENTITY: Self = Self {
        translation: DVec3::ZERO,
        rotation: DQuat::IDENTITY,
    };
    fn checked(value: RootMotionDelta) -> Result<Self> {
        let translation = DVec3::from_array(value.translation);
        let rotation = DQuat::from_array(value.rotation);
        if !translation.is_finite()
            || !rotation.is_finite()
            || (rotation.length_squared() - 1.).abs() > 1e-9
        {
            return Err(RootMotionError::NonFinite);
        }
        Ok(Self {
            translation,
            rotation: rotation.normalize(),
        })
    }
    fn compose(self, other: Self) -> Result<Self> {
        let value = Self {
            translation: self.translation + self.rotation * other.translation,
            rotation: (self.rotation * other.rotation).normalize(),
        };
        if !value.translation.is_finite() || !value.rotation.is_finite() {
            return Err(RootMotionError::NonFinite);
        }
        Ok(value)
    }
    fn inverse(self) -> Self {
        let rotation = self.rotation.conjugate();
        Self {
            translation: rotation * -self.translation,
            rotation,
        }
    }
    fn power(self, cycles: i64) -> Result<Self> {
        let mut power = cycles.unsigned_abs();
        let mut base = if cycles < 0 { self.inverse() } else { self };
        let mut result = Self::IDENTITY;
        while power != 0 {
            if power & 1 != 0 {
                result = result.compose(base)?;
            }
            power >>= 1;
            if power != 0 {
                base = base.compose(base)?;
            }
        }
        Ok(result)
    }
    fn delta(self) -> RootMotionDelta {
        RootMotionDelta {
            translation: self.translation.to_array(),
            rotation: self.rotation.to_array(),
        }
    }
}

/// Validated borrowed root channels. Construction scans retained keys once;
/// subsequent interval queries never allocate or validate whole tracks.
#[derive(Debug, Clone, Copy)]
pub struct RootMotionSampler<'a> {
    pub(crate) translation: Option<Track<'a>>,
    pub(crate) rotation: Option<Track<'a>>,
    pub(crate) prepared: PreparedRootMotion,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PreparedRootMotion {
    translation: [f32; 3],
    rotation: [f32; 4],
    origin: Rigid,
    cycle: Rigid,
    duration: f64,
}
impl<'a> RootMotionSampler<'a> {
    pub fn new(
        duration: f64,
        rest_translation: [f32; 3],
        rest_rotation: [f32; 4],
        translation: Option<Track<'a>>,
        rotation: Option<Track<'a>>,
    ) -> Result<Self> {
        if !duration.is_finite() || !(0. ..=MAX_ROOT_MOTION_DURATION).contains(&duration) {
            return Err(RootMotionError::Duration);
        }
        if rest_translation.iter().any(|v| !v.is_finite())
            || !crate::track::quaternion(rest_rotation)
            || translation.is_some_and(|t| t.kind != Kind::Vector)
            || rotation.is_some_and(|t| t.kind != Kind::Rotation)
        {
            return Err(RootMotionError::Track("rest pose or property"));
        }
        if [translation, rotation]
            .into_iter()
            .flatten()
            .any(|t| f64::from(*t.times.last().expect("admitted track")) > duration)
        {
            return Err(RootMotionError::Track("key exceeds clip duration"));
        }
        let mut out = Self {
            translation,
            rotation,
            prepared: PreparedRootMotion {
                translation: rest_translation,
                rotation: rest_rotation,
                origin: Rigid::IDENTITY,
                cycle: Rigid::IDENTITY,
                duration,
            },
        };
        out.prepared.origin = out.sample(0.);
        out.prepared.cycle = out
            .prepared
            .origin
            .inverse()
            .compose(out.sample(duration))?;
        Ok(out)
    }
    pub fn duration(&self) -> f64 {
        self.prepared.duration
    }
    fn sample(&self, seconds: f64) -> Rigid {
        let translation = self.translation.map_or(self.prepared.translation, |c| {
            let v = c.sample_finite(seconds);
            [v[0], v[1], v[2]]
        });
        let rotation = self
            .rotation
            .map_or(self.prepared.rotation, |c| c.sample_finite(seconds));
        Rigid {
            translation: DVec3::from_array(translation.map(f64::from)),
            rotation: DQuat::from_array(rotation.map(f64::from)).normalize(),
        }
    }
    /// Clamp finite endpoints independently to [0, duration]. Reverse intervals
    /// produce the inverse motion. A zero-duration clip has identity motion.
    pub fn clamped_delta(&self, from: f64, to: f64) -> Result<RootMotionDelta> {
        if !from.is_finite() || !to.is_finite() {
            return Err(RootMotionError::Time);
        }
        let from = from.clamp(0., self.prepared.duration);
        let to = to.clamp(0., self.prepared.duration);
        if from == to {
            return Ok(RootMotionDelta::IDENTITY);
        }
        Ok(self
            .sample(from)
            .inverse()
            .compose(self.sample(to))?
            .delta())
    }
    /// For phase transform F(t)=P(0)^-1 P(t) and cycle C=F(duration), the
    /// unwrapped trajectory is C^cycle F(t). Its relative interval is computed
    /// directly as F(from)^-1 C^(to.cycle-from.cycle) F(to), avoiding cancellation
    /// of large absolute positions. Cycle translation rotates with preceding
    /// cycles; adding translations or subtracting wrapped phases is incorrect.
    pub fn loop_delta(&self, from: RootMotionTime, to: RootMotionTime) -> Result<RootMotionDelta> {
        for time in [from, to] {
            if !time.seconds.is_finite() {
                return Err(RootMotionError::Time);
            }
            if self.prepared.duration <= 0.
                || time.seconds < 0.
                || time.seconds >= self.prepared.duration
            {
                return Err(RootMotionError::Phase);
            }
        }
        let cycles = to
            .cycle
            .checked_sub(from.cycle)
            .ok_or(RootMotionError::Cycles)?;
        if cycles.unsigned_abs() > MAX_ROOT_MOTION_CYCLES {
            return Err(RootMotionError::Cycles);
        }
        if cycles == 0 && from.seconds == to.seconds {
            return Ok(RootMotionDelta::IDENTITY);
        }
        let a = self
            .prepared
            .origin
            .inverse()
            .compose(self.sample(from.seconds))?;
        let b = self
            .prepared
            .origin
            .inverse()
            .compose(self.sample(to.seconds))?;
        Ok(a.inverse()
            .compose(self.prepared.cycle.power(cycles)?)?
            .compose(b)?
            .delta())
    }
}

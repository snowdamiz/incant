//! Rigid local-track deltas, independent of entity clocks, bodies and pose extraction.
use super::{Channel, DQuat, DVec3, NodeTrs, Property, sampling};
use crate::{AssetError, CookedModel};

/// Explicit admission bounds for this primitive, not new cooked-model limits.
pub const MAX_ROOT_MOTION_DURATION: f64 = 86_400.;
/// At most 33 exponentiation steps, independent of the absolute cycle counters.
pub const MAX_ROOT_MOTION_CYCLES: u64 = 1 << 32;

#[derive(Debug, thiserror::Error)]
pub enum RootMotionError {
    #[error("invalid root-motion model: {0}")]
    Model(#[from] AssetError),
    #[error("root motion requires animation data")]
    MissingAnimation,
    #[error("root-motion node index")]
    Node,
    #[error("root-motion clip index")]
    Clip,
    #[error("root motion requires a TRS node")]
    StaticMatrix,
    #[error("root motion requires unit rest scale and no selected-node scale track")]
    Scale,
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
}

#[derive(Clone, Copy)]
struct Rigid {
    translation: DVec3,
    rotation: DQuat,
}
impl Rigid {
    const IDENTITY: Self = Self {
        translation: DVec3::ZERO,
        rotation: DQuat::IDENTITY,
    };
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

/// Startup validation borrows immutable cooked tracks and copies only fixed-size
/// rest/endpoint transforms. No track copies, sampling allocations, source IO or
/// mutable gameplay state. The borrow prevents mutation after admission.
///
/// Motion comes from the chosen node's local translation/rotation channels.
/// Missing channels use rest values. Scale tracks (even constant ones), nonunit
/// rest scale and matrix-only selected nodes explicitly refuse. Animated parents
/// are not incorporated; this primitive does not strip motion from a pose or move
/// a body. The importer continues to refuse unsupported CUBICSPLINE tracks.
pub struct RootMotionSampler<'a> {
    translation: Option<&'a Channel>,
    rotation: Option<&'a Channel>,
    rest: NodeTrs,
    origin: Rigid,
    cycle: Rigid,
    duration: f64,
}
impl<'a> RootMotionSampler<'a> {
    pub fn new(model: &'a CookedModel, node: usize, clip: usize) -> Result<Self> {
        let data = model
            .animation
            .as_ref()
            .ok_or(RootMotionError::MissingAnimation)?;
        data.validate(&model.metadata.nodes, &model.meshes, &model.metadata.scenes)?;
        let rest = data.rest.get(node).ok_or(RootMotionError::Node)?;
        let rest = rest.as_ref().ok_or(RootMotionError::StaticMatrix)?;
        let clip = data.clips.get(clip).ok_or(RootMotionError::Clip)?;
        if rest.scale != [1.; 3]
            || clip
                .channels
                .iter()
                .any(|c| c.node == node && c.property == Property::Scale)
        {
            return Err(RootMotionError::Scale);
        }
        let duration = clip
            .channels
            .iter()
            .map(|c| f64::from(*c.times.last().expect("validated keys")))
            .fold(0., f64::max);
        if !duration.is_finite() || duration > MAX_ROOT_MOTION_DURATION {
            return Err(RootMotionError::Duration);
        }
        let mut out = Self {
            translation: clip
                .channels
                .iter()
                .find(|c| c.node == node && c.property == Property::Translation),
            rotation: clip
                .channels
                .iter()
                .find(|c| c.node == node && c.property == Property::Rotation),
            rest: rest.clone(),
            origin: Rigid::IDENTITY,
            cycle: Rigid::IDENTITY,
            duration,
        };
        out.origin = out.sample(0.);
        out.cycle = out.origin.inverse().compose(out.sample(duration))?;
        Ok(out)
    }
    pub fn duration(&self) -> f64 {
        self.duration
    }
    fn sample(&self, seconds: f64) -> Rigid {
        let translation = self.translation.map_or(self.rest.translation, |c| {
            let v = sampling::channel(c, seconds);
            [v[0], v[1], v[2]]
        });
        let rotation = self
            .rotation
            .map_or(self.rest.rotation, |c| sampling::channel(c, seconds));
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
        let from = from.clamp(0., self.duration);
        let to = to.clamp(0., self.duration);
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
            if self.duration <= 0. || time.seconds < 0. || time.seconds >= self.duration {
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
        let a = self.origin.inverse().compose(self.sample(from.seconds))?;
        let b = self.origin.inverse().compose(self.sample(to.seconds))?;
        Ok(a.inverse()
            .compose(self.cycle.power(cycles)?)?
            .compose(b)?
            .delta())
    }
}

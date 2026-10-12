//! Canonical cooked-channel arithmetic shared by pose and root-motion sampling.
use crate::RootMotionError;
use glam::DQuat;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    Linear,
    Step,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Vector,
    Rotation,
}

/// Immutable view admitted once, then sampled without scanning keys again.
#[derive(Debug, Clone, Copy)]
pub struct Track<'a> {
    pub(crate) times: &'a [f32],
    pub(crate) values: &'a [[f32; 4]],
    pub(crate) interpolation: Interpolation,
    pub(crate) kind: Kind,
}
impl<'a> Track<'a> {
    pub fn new(
        times: &'a [f32],
        values: &'a [[f32; 4]],
        interpolation: Interpolation,
        kind: Kind,
    ) -> Result<Self, RootMotionError> {
        if times.is_empty()
            || times.len() != values.len()
            || times.len() > crate::MAX_ROOT_MOTION_KEYS
            || times.iter().any(|t| !t.is_finite() || *t < 0.)
            || times.windows(2).any(|t| t[0] >= t[1])
        {
            return Err(RootMotionError::Track("key times"));
        }
        if values.iter().any(|v| {
            v.iter().any(|n| !n.is_finite())
                || match kind {
                    Kind::Vector => v[3] != 0.,
                    Kind::Rotation => !quaternion(*v),
                }
        }) {
            return Err(RootMotionError::Track("key values"));
        }
        Ok(Self {
            times,
            values,
            interpolation,
            kind,
        })
    }
    pub fn sample(self, seconds: f64) -> Result<[f32; 4], RootMotionError> {
        if !seconds.is_finite() {
            return Err(RootMotionError::Time);
        }
        Ok(self.sample_finite(seconds))
    }
    pub(crate) fn sample_finite(self, seconds: f64) -> [f32; 4] {
        sample_channel(
            self.times,
            self.values,
            self.interpolation,
            self.kind,
            seconds,
        )
    }
}

pub(crate) fn quaternion(value: [f32; 4]) -> bool {
    let length = value.iter().map(|&n| f64::from(n).powi(2)).sum::<f64>();
    length.is_finite() && (length - 1.).abs() <= 0.002
}

/// Low-level sampling for owners that already admit their complete cooked data.
/// Times must be finite, nonnegative, strictly increasing and nonempty; values
/// must have the same length, finite lanes, and unit rotation quaternions.
/// `seconds` must be finite. Use `Track::new` at untrusted data boundaries.
/// This function deliberately does not repeat channel validation per sample.
pub fn sample_channel(
    times: &[f32],
    values: &[[f32; 4]],
    interpolation: Interpolation,
    kind: Kind,
    seconds: f64,
) -> [f32; 4] {
    let upper = times.partition_point(|&t| f64::from(t) <= seconds);
    let a = upper.saturating_sub(1).min(times.len() - 1);
    let b = upper.min(times.len() - 1);
    let alpha = if a == b || interpolation == Interpolation::Step {
        0.
    } else {
        ((seconds - f64::from(times[a])) / f64::from(times[b] - times[a])).clamp(0., 1.)
    };
    let first = values[a];
    let next = values[b];
    if kind == Kind::Rotation {
        rotation(first, next, alpha)
    } else {
        [
            linear(first[0], next[0], alpha),
            linear(first[1], next[1], alpha),
            linear(first[2], next[2], alpha),
            0.,
        ]
    }
}
pub fn linear(a: f32, b: f32, alpha: f64) -> f32 {
    (f64::from(a) * (1. - alpha) + f64::from(b) * alpha) as f32
}
pub fn rotation(a: [f32; 4], b: [f32; 4], alpha: f64) -> [f32; 4] {
    let a = DQuat::from_array(a.map(f64::from)).normalize();
    let b = DQuat::from_array(b.map(f64::from)).normalize();
    a.slerp(b, alpha).normalize().to_array().map(|v| v as f32)
}

/// Canonical unit quaternion for already admitted finite, nonzero rotations.
/// Preserves the weighted pose sampler's sign choice at exact orthogonal ties.
pub fn canonical_quaternion(value: [f64; 4]) -> [f64; 4] {
    let mut q = DQuat::from_array(value).normalize().to_array();
    // q and -q are the same rotation. Canonicalize even the exact orthogonal
    // case, where a hemisphere dot product alone cannot choose between them.
    if [3, 0, 1, 2]
        .into_iter()
        .find_map(|i| (q[i] != 0.).then_some(q[i] < 0.))
        .unwrap_or(false)
    {
        q = q.map(|v| -v);
    }
    q
}

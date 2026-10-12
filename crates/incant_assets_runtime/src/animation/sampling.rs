use super::*;

/// One independent clip clock, or the authored rest pose. Time clamps to each
/// channel's key interval; looping and clock advancement belong to the caller.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClipSample {
    pub clip: Option<usize>,
    pub seconds: f64,
}
impl ClipSample {
    pub const REST: Self = Self {
        clip: None,
        seconds: 0.,
    };

    pub(super) fn validate(self, data: &AnimationData) -> Result<()> {
        if !self.seconds.is_finite() {
            return Err(invalid("nonfinite animation time"));
        }
        if self.clip.is_some_and(|clip| clip >= data.clips.len()) {
            return Err(invalid("clip index"));
        }
        Ok(())
    }
}

/// Caller validates the sample before entering. Each clip starts from rest, so
/// a missing channel never inherits another clip's value or a previous frame.
pub(super) fn sample(data: &AnimationData, sample: ClipSample, out: &mut [Option<NodeTrs>]) {
    out.clone_from_slice(&data.rest);
    let Some(clip) = sample.clip.map(|clip| &data.clips[clip]) else {
        return;
    };
    for ch in &clip.channels {
        let trs = out[ch.node].as_mut().unwrap();
        let value = channel(ch, sample.seconds);
        match ch.property {
            Property::Rotation => trs.rotation = value,
            Property::Translation | Property::Scale => {
                let value = [value[0], value[1], value[2]];
                if ch.property == Property::Translation {
                    trs.translation = value;
                } else {
                    trs.scale = value;
                }
            }
        }
    }
}
/// The same checked cooked channel arithmetic used by full-pose and root-track
/// sampling. Callers validate the retained data and finite time before entry.
pub(super) fn channel(ch: &Channel, seconds: f64) -> [f32; 4] {
    let upper = ch.times.partition_point(|&t| f64::from(t) <= seconds);
    let a = upper.saturating_sub(1).min(ch.times.len() - 1);
    let b = upper.min(ch.times.len() - 1);
    let alpha = if a == b || ch.interpolation == Interpolation::Step {
        0.
    } else {
        ((seconds - f64::from(ch.times[a])) / f64::from(ch.times[b] - ch.times[a])).clamp(0., 1.)
    };
    let first = ch.values[a];
    let next = ch.values[b];
    if ch.property == Property::Rotation {
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
fn linear(a: f32, b: f32, alpha: f64) -> f32 {
    (f64::from(a) * (1. - alpha) + f64::from(b) * alpha) as f32
}
fn rotation(a: [f32; 4], b: [f32; 4], alpha: f64) -> [f32; 4] {
    let a = DQuat::from_array(a.map(f64::from)).normalize();
    let b = DQuat::from_array(b.map(f64::from)).normalize();
    a.slerp(b, alpha).normalize().to_array().map(|v| v as f32)
}
pub(super) fn blend(from: &mut [Option<NodeTrs>], to: &[Option<NodeTrs>], alpha: f64) {
    for (from, to) in from.iter_mut().zip(to) {
        // Both samples have the same validated rest-pose mask. Static matrix
        // nodes cannot be channel targets and remain untouched by blending.
        if let (Some(from), Some(to)) = (from, to) {
            from.translation =
                std::array::from_fn(|i| linear(from.translation[i], to.translation[i], alpha));
            from.scale = std::array::from_fn(|i| linear(from.scale[i], to.scale[i], alpha));
            from.rotation = rotation(from.rotation, to.rotation, alpha);
        }
    }
}

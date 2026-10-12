//! Blend independently unwrapped interval deltas, never wrapped pose positions.
use crate::{SceneError, animation_step::ClockSpan};
use glam::{DQuat, DVec3};
use incant_anim::{RootMotionCatalog, RootMotionDelta, RootMotionTime};
use incant_types::PoseInterpolation;

pub(super) fn sample(
    catalog: &RootMotionCatalog,
    span: &ClockSpan,
    cycle: i64,
) -> Result<(RootMotionDelta, i64), SceneError> {
    let next = cycle
        .checked_add(span.wraps)
        .ok_or(SceneError::Invalid("root-motion absolute cycle overflow"))?;
    let from = span.before_pose.group(0).expect("admitted source pose");
    let to = span.after_pose.group(0).expect("admitted destination pose");
    if span.before_pose.outer_weight().is_some()
        || span.after_pose.outer_weight().is_some()
        || from.interpolation != to.interpolation
        || from.samples.len() != to.samples.len()
        || from.samples.len() > 8
    {
        return Err(SceneError::Invalid("root-motion interval pose topology"));
    }
    let mut deltas = [RootMotionDelta::IDENTITY; 8];
    for (index, (a, b)) in from.samples.iter().zip(to.samples).enumerate() {
        if a.sample.clip != b.sample.clip || a.weight != b.weight {
            return Err(SceneError::Invalid("root-motion interval contributions"));
        }
        let Some(clip) = a.sample.clip else { continue };
        let sampler = catalog
            .sampler(usize::from(clip))
            .ok_or(SceneError::Invalid("root-motion interval clip"))?;
        deltas[index] = if span.looping && sampler.duration() > 0.0 {
            sampler.loop_delta(
                canonical_time(cycle, a.sample.seconds, sampler.duration())?,
                canonical_time(next, b.sample.seconds, sampler.duration())?,
            )
        } else {
            sampler.clamped_delta(a.sample.seconds, b.sample.seconds)
        }
        .map_err(|_| SceneError::Invalid("root-motion interval numerical range"))?;
    }
    let result = match from.interpolation {
        PoseInterpolation::Clip => deltas[0],
        PoseInterpolation::SphericalPair => {
            spherical(deltas[0], deltas[1], from.samples[1].weight)?
        }
        PoseInterpolation::Weighted => {
            let mut anchor = 0;
            for (i, entry) in from.samples.iter().enumerate().skip(1) {
                let prior = &from.samples[anchor];
                if entry.weight > prior.weight
                    || (entry.weight == prior.weight
                        && entry
                            .sample
                            .clip
                            .cmp(&prior.sample.clip)
                            .then(entry.sample.seconds.total_cmp(&prior.sample.seconds))
                            .is_lt())
                {
                    anchor = i;
                }
            }
            if from.samples.len() == 1 {
                deltas[0]
            } else {
                let reference = incant_anim::track::canonical_quaternion(deltas[anchor].rotation);
                let mut translation = [0.; 3];
                let mut rotation = [0.; 4];
                // Preserve the pose sampler's relative-weight arithmetic and
                // canonical hemisphere/tie order, without narrowing deltas.
                let maximum = from.samples[anchor].weight;
                let total: f64 = from.samples.iter().map(|s| s.weight / maximum).sum();
                for (entry, delta) in from.samples.iter().zip(deltas) {
                    let weight = (entry.weight / maximum) / total;
                    for (t, v) in translation.iter_mut().zip(delta.translation) {
                        *t += v * weight;
                    }
                    let q = incant_anim::track::canonical_quaternion(delta.rotation);
                    let dot: f64 = q.iter().zip(reference).map(|(a, b)| a * b).sum();
                    let signed = if dot < 0. { -weight } else { weight };
                    for (sum, value) in rotation.iter_mut().zip(q) {
                        *sum += value * signed;
                    }
                }
                checked(translation, DQuat::from_array(rotation).normalize())?
            }
        }
    };
    Ok((result, next))
}

/// A seek may name the exact endpoint, unlike the loop sampler's half-open
/// phase. Represent that same point as phase zero in the following cycle.
fn canonical_time(cycle: i64, seconds: f64, duration: f64) -> Result<RootMotionTime, SceneError> {
    if seconds == duration {
        Ok(RootMotionTime {
            cycle: cycle
                .checked_add(1)
                .ok_or(SceneError::Invalid("root-motion endpoint cycle overflow"))?,
            seconds: 0.,
        })
    } else {
        Ok(RootMotionTime { cycle, seconds })
    }
}

pub(super) fn spherical(
    from: RootMotionDelta,
    to: RootMotionDelta,
    weight: f64,
) -> Result<RootMotionDelta, SceneError> {
    if weight == 0.0 {
        return Ok(from);
    }
    if weight == 1.0 {
        return Ok(to);
    }
    checked(
        std::array::from_fn(|i| from.translation[i] * (1.0 - weight) + to.translation[i] * weight),
        DQuat::from_array(from.rotation)
            .slerp(DQuat::from_array(to.rotation), weight)
            .normalize(),
    )
}
fn checked(translation: [f64; 3], rotation: DQuat) -> Result<RootMotionDelta, SceneError> {
    if !DVec3::from_array(translation).is_finite() || !rotation.is_finite() {
        return Err(SceneError::Invalid("root-motion blended numerical range"));
    }
    Ok(RootMotionDelta {
        translation,
        rotation: rotation.to_array(),
    })
}

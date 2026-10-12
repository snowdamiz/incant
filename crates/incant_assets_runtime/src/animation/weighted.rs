//! Bounded local-space pose mixing for multi-clip blend trees. Sampling has no
//! implicit clock, scene access, source IO or per-call allocation.
use super::*;

pub const MAX_POSE_SAMPLES: usize = 16;

/// A nonnegative relative contribution. Weights need not sum to one. The
/// complete list, including zero-weight samples, is checked before sampling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightedClipSample {
    pub sample: ClipSample,
    pub weight: f64,
}

fn quaternion(value: [f32; 4]) -> [f64; 4] {
    let mut q = DQuat::from_array(value.map(f64::from))
        .normalize()
        .to_array();
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

impl AnimationPlayer {
    /// Mix one to sixteen independently sampled local poses, then compose the
    /// hierarchy and skin palettes once. Translation and scale use a weighted
    /// arithmetic mean; rotation uses a normalized quaternion mean aligned to
    /// the highest-weight sample's hemisphere (clip/time order breaks ties).
    /// This is nlerp, not the two-pose `sample_blend` method's spherical blend.
    /// Missing channels use rest values. A single positive contribution exactly
    /// preserves `sample`; every invalid input or composed overflow preserves
    /// all published matrices and palettes. The existing unpublished local-pose
    /// buffer supplies scratch memory; no additional retained storage is needed.
    pub fn sample_weighted(&mut self, samples: &[WeightedClipSample]) -> Result<()> {
        self.sample_weighted_trs(samples)?;
        self.publish()
    }
    pub(super) fn validate_weighted(
        &self,
        samples: &[WeightedClipSample],
    ) -> Result<(usize, usize)> {
        if !(1..=MAX_POSE_SAMPLES).contains(&samples.len()) {
            return Err(AssetError::Limit("pose sample count"));
        }
        let mut anchor = None::<usize>;
        let mut active = 0;
        for (i, entry) in samples.iter().enumerate() {
            entry.sample.validate(&self.data)?;
            if !entry.weight.is_finite() || entry.weight < 0. {
                return Err(invalid("pose sample weight"));
            }
            if entry.weight == 0. {
                continue;
            }
            active += 1;
            if anchor.is_none_or(|old| {
                let prior = &samples[old];
                entry.weight > prior.weight
                    || (entry.weight == prior.weight
                        && entry
                            .sample
                            .clip
                            .cmp(&prior.sample.clip)
                            .then(entry.sample.seconds.total_cmp(&prior.sample.seconds))
                            .is_lt())
            }) {
                anchor = Some(i);
            }
        }
        let anchor = anchor.ok_or_else(|| invalid("pose weights are all zero"))?;
        Ok((anchor, active))
    }
    pub(super) fn sample_weighted_trs(&mut self, samples: &[WeightedClipSample]) -> Result<()> {
        let (anchor, active) = self.validate_weighted(samples)?;
        let reference = samples[anchor];
        if active == 1 {
            sampling::sample(&self.data, reference.sample, &mut self.trs);
            return Ok(());
        }
        // Scale before summing: even MAX weights and subnormal-only sets are
        // valid relative weights. The sum is bounded by MAX_POSE_SAMPLES.
        let total: f64 = samples.iter().map(|e| e.weight / reference.weight).sum();
        sampling::sample(&self.data, reference.sample, &mut self.trs);
        // Reuse the unpublished matrix's four columns as f64 accumulation
        // lanes: translation, scale, rotation, hemisphere anchor. `publish`
        // overwrites every matrix from finished TRS before swapping live poses.
        for (scratch, trs) in self.staged.local.iter_mut().zip(&self.trs) {
            *scratch = DMat4::ZERO;
            if let Some(trs) = trs {
                scratch.w_axis = glam::DVec4::from_array(quaternion(trs.rotation));
            }
        }
        for entry in samples.iter().filter(|e| e.weight > 0.) {
            let weight = (entry.weight / reference.weight) / total;
            sampling::sample(&self.data, entry.sample, &mut self.blend_trs);
            for (scratch, trs) in self.staged.local.iter_mut().zip(&self.blend_trs) {
                let Some(trs) = trs else { continue };
                for i in 0..3 {
                    scratch.x_axis[i] += f64::from(trs.translation[i]) * weight;
                    scratch.y_axis[i] += f64::from(trs.scale[i]) * weight;
                }
                let q = quaternion(trs.rotation);
                let dot: f64 = q
                    .iter()
                    .zip(scratch.w_axis.to_array())
                    .map(|(a, b)| a * b)
                    .sum();
                let signed = if dot < 0. { -weight } else { weight };
                for (i, value) in q.into_iter().enumerate() {
                    scratch.z_axis[i] += value * signed;
                }
            }
        }
        for (scratch, trs) in self.staged.local.iter().zip(&mut self.trs) {
            let Some(trs) = trs else { continue };
            trs.translation = scratch.x_axis.truncate().as_vec3().to_array();
            trs.scale = scratch.y_axis.truncate().as_vec3().to_array();
            let q = DQuat::from_array(scratch.z_axis.to_array()).normalize();
            if !q.is_finite() {
                return Err(invalid("nonfinite weighted rotation"));
            }
            trs.rotation = q.to_array().map(|v| v as f32);
        }
        Ok(())
    }
}

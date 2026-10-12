//! Preserve each state's local mixing rule before an outer transition SLERP.
use super::*;

/// One local pose, with its own interpolation rule. A weighted mean and a
/// spherical pair are deliberately distinct: flattening a pair into a weighted
/// list changes rotations, including at the start/end of a state transition.
#[derive(Debug, Clone, Copy)]
pub enum PoseSource<'a> {
    Clip(ClipSample),
    SphericalPair {
        from: ClipSample,
        to: ClipSample,
        weight: f64,
    },
    /// Same 1..=16 admission and relative-weight rules as `sample_weighted`.
    Weighted(&'a [WeightedClipSample]),
}

impl AnimationPlayer {
    fn validate_pose_source(&self, source: PoseSource<'_>) -> Result<()> {
        match source {
            PoseSource::Clip(sample) => sample.validate(&self.data),
            PoseSource::SphericalPair { from, to, weight } => {
                from.validate(&self.data)?;
                to.validate(&self.data)?;
                if !weight.is_finite() || !(0. ..=1.).contains(&weight) {
                    return Err(invalid("animation blend weight"));
                }
                Ok(())
            }
            PoseSource::Weighted(samples) => self.validate_weighted(samples).map(|_| ()),
        }
    }

    /// Write local TRS without publishing; caller validates the complete source.
    fn sample_pose_source(&mut self, source: PoseSource<'_>) -> Result<()> {
        match source {
            PoseSource::Clip(sample) => sampling::sample(&self.data, sample, &mut self.trs),
            PoseSource::SphericalPair { from, to, weight } => {
                if weight == 1. {
                    sampling::sample(&self.data, to, &mut self.trs);
                } else {
                    sampling::sample(&self.data, from, &mut self.trs);
                    if weight != 0. {
                        sampling::sample(&self.data, to, &mut self.blend_trs);
                        sampling::blend(&mut self.trs, &self.blend_trs, weight);
                    }
                }
            }
            PoseSource::Weighted(samples) => self.sample_weighted_trs(samples)?,
        }
        Ok(())
    }

    /// Sample two complete local poses, then crossfade their TRS values and
    /// compose the hierarchy/palettes once. The outer rotation uses shortest-arc
    /// SLERP; each input retains its own single, spherical-pair or weighted-mean
    /// rule. All inputs (even an inactive endpoint) are validated. Exact weights
    /// zero/one preserve the respective existing sampler result bit for bit.
    ///
    /// Accepted sampling allocates nothing; no live pose changes until the
    /// complete result succeeds. Refusals use the existing asset error type,
    /// whose diagnostic may allocate. One additional retained TRS array is charged by
    /// `retained_bytes`. There are at most 16 contributions per input; graph
    /// admission can impose a tighter total bound. This owns no transition clock.
    pub fn sample_pose_crossfade(
        &mut self,
        from: PoseSource<'_>,
        to: PoseSource<'_>,
        weight: f64,
    ) -> Result<()> {
        self.validate_pose_source(from)?;
        self.validate_pose_source(to)?;
        if !weight.is_finite() || !(0. ..=1.).contains(&weight) {
            return Err(invalid("animation pose crossfade weight"));
        }
        if weight == 0. || weight == 1. {
            self.sample_pose_source(if weight == 0. { from } else { to })?;
        } else {
            self.sample_pose_source(from)?;
            std::mem::swap(&mut self.trs, &mut self.group_trs);
            self.sample_pose_source(to)?;
            sampling::blend(&mut self.group_trs, &self.trs, weight);
            std::mem::swap(&mut self.trs, &mut self.group_trs);
        }
        self.publish()
    }
}

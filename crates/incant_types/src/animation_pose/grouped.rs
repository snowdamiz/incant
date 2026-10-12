//! Compact pose groups preserve inner interpolation and an independent outer fade.
use super::{AnimationPoseSamples, AnimationSample, WeightedAnimationSample};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PoseInterpolation {
    Clip = 0,
    SphericalPair = 1,
    Weighted = 2,
}

#[derive(Debug, Clone, Copy)]
pub struct AnimationPoseGroup<'a> {
    pub interpolation: PoseInterpolation,
    /// Clip: one sample with weight one. SphericalPair: ordered from/to,
    /// destination weight in the second entry. Weighted: a normalized mean.
    pub samples: &'a [WeightedAnimationSample],
}

impl AnimationPoseSamples {
    pub fn clip(sample: AnimationSample) -> Result<Self, &'static str> {
        let mut pose = Self::new(&[WeightedAnimationSample { sample, weight: 1. }])?;
        pose.kinds[0] = PoseInterpolation::Clip;
        Ok(pose)
    }

    pub fn spherical_pair(
        from: AnimationSample,
        to: AnimationSample,
        weight: f64,
    ) -> Result<Self, &'static str> {
        // Validate both endpoints before exposing either, even at zero/one.
        Self::clip(from)?;
        Self::clip(to)?;
        if !weight.is_finite() || !(0.0..=1.0).contains(&weight) {
            return Err("animation spherical pair weight");
        }
        let mut pose = Self::EMPTY;
        pose.len = 2;
        pose.kinds[0] = PoseInterpolation::SphericalPair;
        pose.samples[0] = WeightedAnimationSample {
            sample: from,
            weight: 1. - weight,
        };
        pose.samples[1] = WeightedAnimationSample { sample: to, weight };
        Ok(pose)
    }

    /// Two independently evaluated states followed by one TRS crossfade. Each
    /// state has at most eight contributions; nested active fades refuse. The
    /// inactive side is retained at endpoints so every consumer validates it.
    pub fn crossfade(from: &Self, to: &Self, weight: f64) -> Result<Self, &'static str> {
        if !weight.is_finite() || !(0.0..=1.0).contains(&weight) {
            return Err("animation pose crossfade weight");
        }
        if from.split != 0 || to.split != 0 || from.len > 8 || to.len > 8 {
            return Err("animation pose crossfade capacity or nesting");
        }
        let mut result = Self::EMPTY;
        let split = usize::from(from.len);
        let len = split + usize::from(to.len);
        result.samples[..split].copy_from_slice(from.samples());
        result.samples[split..len].copy_from_slice(to.samples());
        result.len = len as u8;
        result.split = from.len;
        result.kinds = [from.kinds[0], to.kinds[0]];
        result.outer_weight = weight;
        Ok(result)
    }

    pub fn is_weighted(&self) -> bool {
        self.split == 0 && self.kinds[0] == PoseInterpolation::Weighted
    }

    pub fn outer_weight(&self) -> Option<f64> {
        (self.split != 0).then_some(self.outer_weight)
    }

    pub fn group(&self, index: usize) -> Option<AnimationPoseGroup<'_>> {
        let split = usize::from(self.split);
        let samples = match index {
            0 if split == 0 => self.samples(),
            0 => &self.samples[..split],
            1 if split != 0 => &self.samples[split..usize::from(self.len)],
            _ => return None,
        };
        Some(AnimationPoseGroup {
            interpolation: self.kinds[index],
            samples,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn clip(n: u16) -> AnimationSample {
        AnimationSample {
            clip: Some(n),
            seconds: 0.25,
        }
    }
    #[test]
    fn groups_preserve_spherical_order_inactive_inputs_and_independent_weights() {
        let pair = AnimationPoseSamples::spherical_pair(clip(0), clip(1), 0.).unwrap();
        assert_eq!(pair.samples().len(), 2);
        assert_eq!(
            pair.group(0).unwrap().interpolation,
            PoseInterpolation::SphericalPair
        );
        let weighted = AnimationPoseSamples::new(&[
            WeightedAnimationSample {
                sample: clip(2),
                weight: 1.,
            },
            WeightedAnimationSample {
                sample: clip(3),
                weight: 3.,
            },
        ])
        .unwrap();
        for weight in [0., 0.25, 1.] {
            let both = AnimationPoseSamples::crossfade(&pair, &weighted, weight).unwrap();
            assert_eq!(both.outer_weight(), Some(weight));
            assert_eq!(both.group(0).unwrap().samples, pair.samples());
            assert_eq!(both.group(1).unwrap().samples, weighted.samples());
            assert!(both.group(2).is_none());
            assert!(both.blend(&weighted, 0.).is_err());
            assert!(AnimationPoseSamples::crossfade(&both, &pair, 1.).is_err());
        }
        assert!(
            AnimationPoseSamples::spherical_pair(
                clip(0),
                AnimationSample {
                    clip: None,
                    seconds: 1.
                },
                0.
            )
            .is_err()
        );
        assert!(AnimationPoseSamples::spherical_pair(clip(0), clip(1), f64::NAN).is_err());
        let entries: Vec<_> = (0..9)
            .map(|n| WeightedAnimationSample {
                sample: clip(n),
                weight: 1.,
            })
            .collect();
        let nine = AnimationPoseSamples::new(&entries).unwrap();
        assert!(AnimationPoseSamples::crossfade(&nine, &pair, 0.).is_err());
        let eight = AnimationPoseSamples::new(&entries[..8]).unwrap();
        assert_eq!(
            AnimationPoseSamples::crossfade(&eight, &eight, 0.5)
                .unwrap()
                .samples()
                .len(),
            16
        );
    }
}

//! Copied weighted local-pose observations shared by runtime and render hosts.
use crate::AnimationSample;
mod grouped;
pub use grouped::{AnimationPoseGroup, PoseInterpolation};
pub const MAX_ANIMATION_POSE_SAMPLES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightedAnimationSample {
    pub sample: AnimationSample,
    pub weight: f64,
}

/// Copyable normalized local-pose contributions. A state can contribute up to
/// eight samples and a fade can join two such states. This does not blend final
/// matrices; the retained model sampler blends TRS before building a hierarchy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationPoseSamples {
    samples: [WeightedAnimationSample; MAX_ANIMATION_POSE_SAMPLES],
    len: u8,
    split: u8,
    kinds: [PoseInterpolation; 2],
    outer_weight: f64,
}

impl AnimationPoseSamples {
    /// Construct a bounded owned pose from finite relative weights. Every input
    /// is validated, including zero-weight entries, before any output is returned.
    pub fn new(samples: &[WeightedAnimationSample]) -> Result<Self, &'static str> {
        if samples.is_empty() || samples.len() > MAX_ANIMATION_POSE_SAMPLES {
            return Err("animation pose sample capacity");
        }
        let mut maximum = 0.0f64;
        for entry in samples {
            if !entry.weight.is_finite()
                || entry.weight < 0.0
                || !entry.sample.seconds.is_finite()
                || entry.sample.seconds < 0.0
                || (entry.sample.clip.is_none() && entry.sample.seconds != 0.0)
                || entry
                    .sample
                    .clip
                    .is_some_and(|clip| usize::from(clip) >= crate::MAX_ANIMATION_CLIPS)
            {
                return Err("animation pose sample values");
            }
            maximum = maximum.max(entry.weight);
        }
        if maximum == 0.0 {
            return Err("animation pose zero total weight");
        }
        // Preserve already-normalized native weights without an extra rounding
        // cycle; only large relative weights need overflow-safe rescaling.
        let scale = maximum.max(1.0);
        let mut result = Self::EMPTY;
        for entry in samples {
            result.push(entry.sample, entry.weight / scale)?;
        }
        result.normalize();
        Ok(result)
    }
    const EMPTY: Self = Self {
        samples: [WeightedAnimationSample {
            sample: AnimationSample::REST,
            weight: 0.0,
        }; MAX_ANIMATION_POSE_SAMPLES],
        len: 0,
        split: 0,
        kinds: [PoseInterpolation::Weighted; 2],
        outer_weight: 0.0,
    };

    /// Packed contributions. A compound pose has independently normalized
    /// groups; use `group` and `outer_weight` rather than flattening them.
    pub fn samples(&self) -> &[WeightedAnimationSample] {
        &self.samples[..usize::from(self.len)]
    }

    /// `weight` is the destination contribution, matching ordinary crossfades.
    /// Endpoints preserve the input exactly. Equal clip/time entries coalesce;
    /// different phases of the same clip remain separate.
    pub fn blend(&self, destination: &Self, weight: f64) -> Result<Self, &'static str> {
        if !self.is_weighted() || !destination.is_weighted() {
            return Err("flattening grouped or spherical animation poses is unsupported");
        }
        if !weight.is_finite() || !(0.0..=1.0).contains(&weight) {
            return Err("animation pose blend weight");
        }
        if weight == 0.0 {
            return Ok(*self);
        }
        if weight == 1.0 {
            return Ok(*destination);
        }
        let mut result = Self::EMPTY;
        for (pose, factor) in [(self, 1.0 - weight), (destination, weight)] {
            for entry in pose.samples() {
                result.push(entry.sample, entry.weight * factor)?;
            }
        }
        result.normalize();
        Ok(result)
    }

    fn push(&mut self, sample: AnimationSample, weight: f64) -> Result<(), &'static str> {
        if weight == 0.0 {
            return Ok(());
        }
        if let Some(entry) = self.samples[..usize::from(self.len)]
            .iter_mut()
            .find(|entry| entry.sample == sample)
        {
            entry.weight += weight;
            return Ok(());
        }
        let entry = self
            .samples
            .get_mut(usize::from(self.len))
            .ok_or("animation pose sample capacity")?;
        *entry = WeightedAnimationSample { sample, weight };
        self.len += 1;
        Ok(())
    }

    fn normalize(&mut self) {
        let entries = &mut self.samples[..usize::from(self.len)];
        let sum: f64 = entries.iter().map(|entry| entry.weight).sum();
        let mut retained = 0;
        for index in 0..entries.len() {
            let mut entry = entries[index];
            entry.weight /= sum;
            // An admitted positive relative weight can underflow during
            // normalization. Keep the published pose canonical and nonzero,
            // just as push() does for zero input contributions.
            if entry.weight > 0.0 {
                entries[retained] = entry;
                retained += 1;
            }
        }
        entries[retained..].fill(Self::EMPTY.samples[0]);
        self.len = retained as u8;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(clip: Option<u16>, seconds: f64, weight: f64) -> WeightedAnimationSample {
        WeightedAnimationSample {
            sample: AnimationSample { clip, seconds },
            weight,
        }
    }

    #[test]
    fn copied_pose_preserves_phases_coalesces_equal_samples_and_normalizes_extreme_weights() {
        let pose = AnimationPoseSamples::new(&[
            entry(Some(0), 0.25, f64::MAX),
            entry(Some(0), 0.75, f64::MAX),
            entry(Some(0), 0.25, f64::MAX),
            entry(None, 0.0, 0.0),
        ])
        .unwrap();
        assert_eq!(pose.samples().len(), 2);
        assert_eq!(pose.samples()[0], entry(Some(0), 0.25, 2.0 / 3.0));
        assert_eq!(pose.samples()[1], entry(Some(0), 0.75, 1.0 / 3.0));
        let tiny = AnimationPoseSamples::new(&[
            entry(Some(0), 0.0, f64::from_bits(1)),
            entry(None, 0.0, f64::from_bits(1)),
        ])
        .unwrap();
        assert!(tiny.samples().iter().all(|v| v.weight == 0.5));
        let mut wide: Vec<_> = (0..15).map(|n| entry(Some(n), 0.0, 1.0)).collect();
        wide.push(entry(None, 0.0, f64::from_bits(1)));
        let pose = AnimationPoseSamples::new(&wide).unwrap();
        assert_eq!(pose.samples().len(), 15);
        assert!(pose.samples().iter().all(|v| v.weight > 0.0));
        assert_eq!(pose, AnimationPoseSamples::new(&wide[..15]).unwrap());
        wide.rotate_right(1);
        assert_eq!(pose, AnimationPoseSamples::new(&wide).unwrap());
    }

    #[test]
    fn copied_pose_validates_zero_weight_entries_and_refuses_invalid_capacity() {
        assert!(AnimationPoseSamples::new(&[]).is_err());
        assert!(AnimationPoseSamples::new(&[entry(None, 0.0, 0.0)]).is_err());
        let full: Vec<_> = (0..16).map(|n| entry(Some(n), 0.0, 1.0)).collect();
        assert_eq!(
            AnimationPoseSamples::new(&full).unwrap().samples().len(),
            16
        );
        let mut overflow = full;
        overflow.push(entry(Some(16), 0.0, 0.0));
        assert!(AnimationPoseSamples::new(&overflow).is_err());
        for bad in [
            entry(None, 1.0, 0.0),
            entry(Some(256), 0.0, 0.0),
            entry(Some(0), -1.0, 0.0),
            entry(Some(0), f64::NAN, 0.0),
            entry(Some(0), f64::INFINITY, 0.0),
            entry(None, 0.0, -1.0),
            entry(None, 0.0, f64::NAN),
            entry(None, 0.0, f64::INFINITY),
        ] {
            assert!(AnimationPoseSamples::new(&[entry(None, 0.0, 1.0), bad]).is_err());
        }
    }
}

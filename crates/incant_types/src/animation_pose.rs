//! Copied weighted local-pose observations shared by runtime and render hosts.
use crate::AnimationSample;
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
    };

    pub fn samples(&self) -> &[WeightedAnimationSample] {
        &self.samples[..usize::from(self.len)]
    }

    /// `weight` is the destination contribution, matching ordinary crossfades.
    /// Endpoints preserve the input exactly. Equal clip/time entries coalesce;
    /// different phases of the same clip remain separate.
    pub fn blend(&self, destination: &Self, weight: f64) -> Result<Self, &'static str> {
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
        for entry in entries {
            entry.weight /= sum;
        }
    }
}

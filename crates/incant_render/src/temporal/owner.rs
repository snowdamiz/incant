use super::{TemporalConfig, TemporalError, targets::Targets};
use crate::motion::{MotionFrame, TemporalInput};
use std::sync::Arc;
/// Metadata for a submitted accumulation. Neither texture handles nor staged state escape.
#[derive(Clone, Debug)]
pub struct TemporalFrame {
    motion: MotionFrame,
    // Number of accepted color samples since the latest owner reset. Saturates
    // once every allowed policy (<= 0.99) has reached its steady-state weight.
    samples: u8,
}
impl TemporalFrame {
    pub fn motion(&self) -> &MotionFrame {
        &self.motion
    }
    pub fn jitter_phase(&self) -> Option<u32> {
        self.motion.jitter_phase()
    }
    pub fn bypassed(&self) -> bool {
        self.motion.bypassed()
    }
    #[cfg(test)]
    pub(crate) fn accepted_samples(&self) -> u8 {
        self.samples
    }
}
pub(crate) struct TemporalState {
    pub config: TemporalConfig,
    targets: Option<Arc<Targets>>,
    last: Option<TemporalFrame>,
    slot: usize,
}
pub(crate) struct Pending {
    pub(super) targets: Option<Arc<Targets>>,
    pub(super) input: TemporalInput,
    pub(super) config: TemporalConfig,
    pub(super) jitter_delta_code: f32,
    samples: u8,
}

fn next_samples(previous: u8, has_previous: bool, bypassed: bool) -> u8 {
    if bypassed {
        0
    } else if !has_previous {
        1
    } else {
        previous.saturating_add(1).min(100)
    }
}

fn history_weight(samples: u8, maximum: f32) -> f32 {
    if samples <= 1 {
        0.
    } else {
        maximum.min(f32::from(samples - 1) / f32::from(samples))
    }
}
impl TemporalState {
    pub fn new(config: TemporalConfig) -> Result<Self, TemporalError> {
        config.validate()?;
        Ok(Self {
            config,
            targets: None,
            last: None,
            slot: 1,
        })
    }
    pub fn frame(&self) -> Option<TemporalFrame> {
        self.last.clone()
    }
    pub fn preflight(
        &self,
        device: &wgpu::Device,
        size: [u32; 2],
        motion_bytes: u64,
    ) -> Result<(), TemporalError> {
        super::targets::preflight(
            size,
            device.limits().max_texture_dimension_2d,
            self.targets.as_ref().map_or(0, |t| t.bytes),
            motion_bytes,
        )
        .map(|_| ())
    }
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        input: TemporalInput,
    ) -> Result<Pending, TemporalError> {
        if input.frame.bypassed() != input.surfaces.is_none()
            || input.surfaces.as_ref().is_some_and(|s| {
                s.slot > 1 || (input.frame.has_previous() && s.slot != 1 - self.slot)
            })
        {
            return Err(TemporalError::History);
        }
        if input.frame.has_previous()
            && self.last.as_ref().is_none_or(|last| {
                !last.motion.same_owner(&input.frame)
                    || last.motion.epoch() != input.frame.epoch()
                    || last.motion.sequence().checked_add(1) != Some(input.frame.sequence())
            })
        {
            return Err(TemporalError::History);
        }
        let targets = if input.frame.bypassed() {
            None
        } else {
            Some(Targets::prepare(
                device,
                input.frame.internal_size(),
                self.targets.as_ref(),
            ))
        };
        let jitter_delta_code = if input.frame.has_previous() {
            super::jitter::delta_code(
                self.last.as_ref().and_then(TemporalFrame::jitter_phase),
                input.frame.jitter_phase(),
            )
        } else {
            super::jitter::delta_code(None, None)
        };
        let samples = next_samples(
            self.last.as_ref().map_or(0, |last| last.samples),
            input.frame.has_previous(),
            input.frame.bypassed(),
        );
        Ok(Pending {
            targets,
            input,
            config: self.config,
            jitter_delta_code,
            samples,
        })
    }
    pub fn accept(&mut self, pending: Pending) {
        self.slot = pending.input.surfaces.as_ref().map_or(1, |s| s.slot);
        self.last = Some(TemporalFrame {
            motion: pending.input.frame,
            samples: pending.samples,
        });
        self.targets = pending.targets;
    }
}

#[cfg(test)]
impl TemporalState {
    pub(crate) fn accepted_color(&self) -> Option<wgpu::TextureView> {
        self.targets.as_ref().map(|t| t.color[self.slot].clone())
    }
}

impl Pending {
    pub(super) fn history_weight(&self) -> f32 {
        history_weight(self.samples, self.config.history_weight)
    }
    pub(crate) fn active(&self) -> bool {
        self.targets.is_some()
    }
}

#[cfg(test)]
mod warmup_tests {
    use super::*;

    #[test]
    fn reset_edge_bias_decays_as_sample_mean_before_steady_state() {
        // A pixel-aligned edge's first registered sample carries 3/8 of the
        // neighboring pixel. Later samples are exact. Independent arithmetic
        // mean predicts its residual; a constant 0.9 keeps > 0.2 after six.
        let mut samples = next_samples(0, false, false);
        let mut residual = 0.375_f32;
        for frame in 2..=10 {
            samples = next_samples(samples, true, false);
            residual *= history_weight(samples, 0.9);
            assert!((residual - 0.375 / frame as f32).abs() < 1e-7);
            if frame == 6 {
                assert!(residual < 0.1);
                assert!(0.375 * 0.9_f32.powi(5) > 0.2);
            }
        }
        assert_eq!(history_weight(next_samples(samples, true, false), 0.9), 0.9);
    }

    #[test]
    fn reset_and_bypass_restart_the_ramp() {
        for prior in [0, 1, 5, 99, 100, u8::MAX] {
            assert_eq!(next_samples(prior, false, false), 1);
            assert_eq!(next_samples(prior, true, true), 0);
        }
        assert_eq!(history_weight(1, 0.9), 0.);
        assert_eq!(history_weight(0, 0.9), 0.);
    }

    #[test]
    fn policy_maximum_and_bounded_age_hold_over_long_sessions() {
        for maximum in [0., 0.25, 0.5, 0.8, 0.9, 0.99] {
            let mut samples = next_samples(0, false, false);
            for _ in 0..1000 {
                samples = next_samples(samples, true, false);
                assert!(history_weight(samples, maximum) <= maximum);
            }
            assert_eq!(samples, 100);
            assert_eq!(history_weight(samples, maximum), maximum);
        }
    }
}

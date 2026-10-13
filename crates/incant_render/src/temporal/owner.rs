use super::{TemporalConfig, TemporalError, targets::Targets};
use crate::motion::{MotionFrame, TemporalInput};
use std::sync::Arc;
/// Metadata for a submitted accumulation. Neither texture handles nor staged state escape.
#[derive(Clone, Debug)]
pub struct TemporalFrame {
    motion: MotionFrame,
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
}
pub(crate) struct TemporalState {
    pub config: TemporalConfig,
    targets: Option<Arc<Targets>>,
    last: Option<TemporalFrame>,
}
pub(crate) struct Pending {
    pub(super) targets: Option<Arc<Targets>>,
    pub(super) input: TemporalInput,
    pub(super) config: TemporalConfig,
}
impl TemporalState {
    pub fn new(config: TemporalConfig) -> Result<Self, TemporalError> {
        config.validate()?;
        Ok(Self {
            config,
            targets: None,
            last: None,
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
        Ok(Pending {
            targets,
            input,
            config: self.config,
        })
    }
    pub fn accept(&mut self, pending: Pending) {
        self.last = Some(TemporalFrame {
            motion: pending.input.frame,
        });
        self.targets = pending.targets;
    }
}

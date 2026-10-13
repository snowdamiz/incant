use super::{
    Plan, UpscaleConfig, UpscaleError,
    targets::{Output, Overlay},
};
use crate::motion::MotionFrame;
use std::sync::Arc;
#[derive(Clone, Debug)]
pub struct UpscaleFrame {
    motion: MotionFrame,
    output: [u32; 2],
    slot: usize,
}
impl UpscaleFrame {
    pub fn motion(&self) -> &MotionFrame {
        &self.motion
    }
    pub fn output_size(&self) -> [u32; 2] {
        self.output
    }
}
pub(crate) struct Input {
    pub frame: MotionFrame,
    pub rect: [f32; 4],
    pub jitter: [f32; 2],
    pub previous_jitter: [f32; 2],
    pub surfaces: Option<(wgpu::TextureView, wgpu::TextureView)>,
    pub owner: Option<(wgpu::TextureView, wgpu::Buffer)>,
}
pub(crate) struct State {
    config: UpscaleConfig,
    output: Option<Arc<Output>>,
    overlay: Option<Arc<Overlay>>,
    last: Option<UpscaleFrame>,
}
pub(crate) struct Pending {
    pub(super) config: UpscaleConfig,
    pub(super) input: Input,
    pub(super) plan: Option<Plan>,
    pub(super) frame: UpscaleFrame,
    pub(super) output: Option<Arc<Output>>,
    pub(super) overlay: Option<Arc<Overlay>>,
}
impl State {
    pub fn new(config: UpscaleConfig) -> Result<Self, crate::temporal::TemporalError> {
        config.validate()?;
        Ok(Self {
            config,
            output: None,
            overlay: None,
            last: None,
        })
    }
    pub fn frame(&self) -> Option<UpscaleFrame> {
        self.last.clone()
    }
    pub fn preflight(
        &self,
        device: &wgpu::Device,
        plan: Plan,
        old_motion: u64,
    ) -> Result<(), UpscaleError> {
        plan.preflight(
            device.limits().max_texture_dimension_2d,
            self.output.as_ref().map_or(0, |o| o.bytes()),
            self.overlay.as_ref().map_or(0, |o| o.bytes()),
            old_motion,
        )
    }
    pub fn preflight_owner(
        &self,
        device: &wgpu::Device,
        plan: Plan,
        old_motion: Option<crate::motion::attachment::Cache>,
    ) -> Result<(), crate::motion::MotionError> {
        crate::motion::attachment::admit_records(
            plan.source,
            old_motion,
            plan.output,
            self.output
                .as_ref()
                .map(|o| crate::motion::attachment::Cache {
                    size: o.size,
                    owner: o.record.is_some(),
                }),
            self.overlay.as_ref().map(|o| o.size),
            device.limits().max_texture_dimension_2d,
        )
        .map(|_| ())
    }
    pub fn prepare(
        &self,
        d: &wgpu::Device,
        input: Input,
        plan: Option<Plan>,
        size: [u32; 2],
    ) -> Result<Pending, UpscaleError> {
        if input.frame.bypassed() != plan.is_none()
            || input.frame.bypassed() != input.surfaces.is_none()
        {
            return Err(UpscaleError::History);
        }
        if let Some(p) = plan
            && (input.rect != p.source_rect || input.frame.internal_size() != p.source)
        {
            return Err(UpscaleError::History);
        }
        if input.frame.has_previous()
            && self.last.as_ref().is_none_or(|last| {
                !last.motion.same_owner(&input.frame)
                    || last.motion.epoch() != input.frame.epoch()
                    || last.motion.sequence().checked_add(1) != Some(input.frame.sequence())
                    || last.output != size
            })
        {
            return Err(UpscaleError::History);
        }
        let owner = input.owner.is_some();
        if input.frame.has_previous()
            && self
                .output
                .as_ref()
                .is_some_and(|o| o.record.is_some() != owner)
        {
            return Err(UpscaleError::History);
        }
        let slot = self.last.as_ref().map_or(0, |f| 1 - f.slot);
        let frame = UpscaleFrame {
            motion: input.frame.clone(),
            output: size,
            slot,
        };
        Ok(Pending {
            config: self.config,
            input,
            plan,
            frame,
            output: plan.map(|p| Output::prepare(d, p.output, self.output.as_ref(), owner)),
            overlay: plan.map(|p| Overlay::prepare(d, p.source, self.overlay.as_ref())),
        })
    }
    pub fn accept(&mut self, p: Pending) {
        self.output = p.output;
        self.overlay = p.overlay;
        self.last = Some(p.frame);
    }
}
impl Pending {
    pub fn overlay(&self) -> Option<&wgpu::TextureView> {
        self.overlay.as_ref().map(|o| &o.view)
    }
    pub fn composite(&self) -> Option<&wgpu::TextureView> {
        self.output.as_ref().map(|o| &o.composite)
    }
    #[cfg(feature = "native-metalfx")]
    pub(crate) fn native_input(
        &self,
    ) -> Option<(&Input, Plan, &wgpu::TextureView, &wgpu::TextureView)> {
        let o = self.output.as_ref()?;
        Some((
            &self.input,
            self.plan?,
            &o.color[self.slot()],
            &o.depth[self.slot()],
        ))
    }
    #[cfg(feature = "native-metalfx")]
    pub(crate) fn native_config(&self) -> UpscaleConfig {
        self.config
    }
    #[cfg(feature = "native-metalfx")]
    pub(crate) fn discard_color_history(&mut self) {
        self.config.history_weight = 0.;
    }
    pub(super) fn slot(&self) -> usize {
        self.frame.slot
    }
}
#[cfg(test)]
impl State {
    pub(crate) fn accepted_color(&self) -> Option<wgpu::TextureView> {
        self.output
            .as_ref()
            .zip(self.last.as_ref())
            .map(|(o, l)| o.color[l.slot].clone())
    }
}

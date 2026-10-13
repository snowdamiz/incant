//! Native queue-completion latency feedback, never a GPU timestamp estimate.
//! One callback can be outstanding. No wait, queue drain or per-frame history.
use crate::resolution_mailbox::Mailbox;
use crate::{GpuRenderSettings, TargetProvider};
use incant_render::{
    Renderer,
    resolution::{ResolutionConfig, ResolutionError, ResolutionFrame},
};
use std::time::Instant;

/// Explicit opt-in wrapper. Ordinary target providers continue at full resolution.
pub struct ResolutionTargets<P> {
    inner: P,
    config: ResolutionConfig,
}
pub(crate) fn validate(config: ResolutionConfig) -> Result<(), ResolutionError> {
    config.validate()?;
    // std::time::Instant has no browser clock. The portable renderer controller
    // remains usable with explicit host measurements; do not panic on opt-in.
    if cfg!(target_arch = "wasm32") {
        return Err(ResolutionError::ClockUnavailable);
    }
    Ok(())
}
impl<P: TargetProvider> GpuRenderSettings<P> {
    /// Use wall-clock render-preparation-to-queue-completion-callback latency.
    /// Includes queue backlog, CPU preparation/presentation, and callback dispatch
    /// delay. Excludes earlier simulation and is NOT isolated GPU execution time
    /// or a refresh interval. No timestamp feature or synchronous wait is enabled.
    pub fn with_dynamic_resolution(
        self,
        config: ResolutionConfig,
    ) -> Result<GpuRenderSettings<ResolutionTargets<P>>, ResolutionError> {
        validate(config)?;
        Ok(GpuRenderSettings {
            renderer: self.renderer,
            targets: ResolutionTargets {
                inner: self.targets,
                config,
            },
        })
    }
}
impl<P: TargetProvider> TargetProvider for ResolutionTargets<P> {
    type Error = P::Error;
    type Frame = P::Frame;
    fn acquire(&mut self, renderer: &Renderer) -> Result<Self::Frame, Self::Error> {
        self.inner.acquire(renderer)
    }
    fn resolution_config(&self) -> Option<ResolutionConfig> {
        Some(self.config)
    }
}
#[derive(Default)]
pub(crate) struct Completion {
    slot: Mailbox<(ResolutionFrame, f64)>,
    pub discarded: u64,
}
impl Completion {
    pub fn take(&mut self) -> Result<Option<(ResolutionFrame, f64)>, ResolutionError> {
        self.slot.take()
    }
    pub fn register(
        &self,
        renderer: &Renderer,
        start: Instant,
        frame: ResolutionFrame,
    ) -> Result<(), ResolutionError> {
        if frame.pixel_perfect() {
            return Ok(());
        }
        let Some(ticket) = self.slot.reserve()? else {
            return Ok(());
        };
        renderer.queue.on_submitted_work_done(move || {
            ticket.complete((frame, start.elapsed().as_secs_f64() * 1000.));
        });
        Ok(())
    }
}

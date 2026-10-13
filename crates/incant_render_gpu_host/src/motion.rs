//! Trusted native presentation controls. No authored/serialized history or ticks.
use crate::{GpuRenderSettings, TargetProvider};
use incant_render::{Renderer, motion::MotionError, resolution::ResolutionConfig};
use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

/// Opaque cut/reset control bound to one target-provider lifetime. Reusing it with
/// a replacement host is harmless: that host still creates a fresh render owner.
#[derive(Clone)]
pub struct MotionReset(Arc<AtomicU64>);
impl MotionReset {
    pub fn camera_cut(&self) -> Result<u64, MotionError> {
        self.0
            .try_update(Ordering::AcqRel, Ordering::Acquire, |n| n.checked_add(1))
            .map(|old| old + 1)
            .map_err(|_| MotionError::Sequence)
    }
}
pub struct MotionTargets<P> {
    inner: P,
    epoch: Arc<AtomicU64>,
}
impl<P: TargetProvider> GpuRenderSettings<P> {
    /// Renderer::enable_motion_vectors must have prepared the fixed pipelines.
    /// This captures only opaque/MASK surface motion, not final HDR color history.
    pub fn with_motion_history(self) -> (GpuRenderSettings<MotionTargets<P>>, MotionReset) {
        let epoch = Arc::new(AtomicU64::new(0));
        (
            GpuRenderSettings {
                renderer: self.renderer,
                targets: MotionTargets {
                    inner: self.targets,
                    epoch: Arc::clone(&epoch),
                },
            },
            MotionReset(epoch),
        )
    }
}
impl<P: TargetProvider> TargetProvider for MotionTargets<P> {
    type Error = P::Error;
    type Frame = P::Frame;
    fn acquire(&mut self, renderer: &Renderer) -> Result<Self::Frame, Self::Error> {
        self.inner.acquire(renderer)
    }
    fn resolution_config(&self) -> Option<ResolutionConfig> {
        self.inner.resolution_config()
    }
    fn motion_epoch(&self) -> Option<u64> {
        Some(self.epoch.load(Ordering::Acquire))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cuts_are_shared_monotonic_and_exhaustion_preserves_epoch() {
        let a = MotionReset(Arc::new(AtomicU64::new(0)));
        let b = a.clone();
        assert_eq!(a.camera_cut().unwrap(), 1);
        assert_eq!(b.camera_cut().unwrap(), 2);
        a.0.store(u64::MAX, Ordering::Release);
        assert_eq!(b.camera_cut(), Err(MotionError::Sequence));
        assert_eq!(a.0.load(Ordering::Acquire), u64::MAX);
    }
}

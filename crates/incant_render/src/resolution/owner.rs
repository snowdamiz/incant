use super::{FramePlan, ResolutionConfig, ResolutionController, ResolutionError, ResolutionScale};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Extent {
    output: [u32; 2],
    rect: [u32; 4],
    pixel: Option<[u32; 2]>,
}
/// An opaque receipt for a successfully submitted frame of one retained owner.
/// Receipts cannot be constructed by external callers or reused after reconfiguration.
#[derive(Clone, Debug)]
pub struct ResolutionFrame {
    owner: Arc<()>,
    serial: u64,
    epoch: u64,
    extent: Extent,
    internal: [u32; 2],
    scale: ResolutionScale,
}
impl ResolutionFrame {
    pub fn sequence(&self) -> u64 {
        self.serial
    }
    pub fn output_size(&self) -> [u32; 2] {
        self.extent.output
    }
    pub fn internal_size(&self) -> [u32; 2] {
        self.internal
    }
    pub fn pixel_perfect(&self) -> bool {
        self.extent.pixel.is_some()
    }
    pub fn scale(&self) -> ResolutionScale {
        self.scale
    }
}
pub(crate) struct ResolutionOwner {
    identity: Arc<()>,
    control: ResolutionController,
    last: Option<ResolutionFrame>,
    observed: u64,
}
impl ResolutionOwner {
    pub fn new(config: ResolutionConfig) -> Result<Self, ResolutionError> {
        Ok(Self {
            control: ResolutionController::new(config)?,
            identity: Arc::new(()),
            last: None,
            observed: 0,
        })
    }
    pub fn scale(&self) -> ResolutionScale {
        self.control.scale()
    }
    pub fn last(&self) -> Option<ResolutionFrame> {
        self.last.clone()
    }
    pub fn prepare(
        &self,
        output: [u32; 2],
        rect: [f32; 4],
        pixel: Option<[u32; 2]>,
    ) -> crate::Result<ResolutionFrame> {
        let extent = Extent {
            output,
            rect: rect.map(f32::to_bits),
            pixel,
        };
        let serial = self
            .last
            .as_ref()
            .map_or(0, |f| f.serial)
            .checked_add(1)
            .ok_or(ResolutionError::Sequence)?;
        let epoch = match &self.last {
            Some(last) if last.extent != extent => {
                last.epoch.checked_add(1).ok_or(ResolutionError::Sequence)?
            }
            Some(last) => last.epoch,
            None => 0,
        };
        let scale = if pixel.is_some() {
            ResolutionScale::FULL
        } else {
            self.scale()
        };
        let plan = FramePlan::new(output, rect, pixel, scale)?;
        Ok(ResolutionFrame {
            owner: Arc::clone(&self.identity),
            serial,
            epoch,
            extent,
            internal: plan.size,
            scale,
        })
    }
    pub fn accept(&mut self, frame: ResolutionFrame) {
        if self.last.as_ref().is_some_and(|f| f.epoch != frame.epoch) {
            self.control.reset_history();
        }
        self.last = Some(frame);
    }
    /// Stale/foreign/repeated receipts are discarded before payload admission.
    pub fn observe(
        &mut self,
        frame: &ResolutionFrame,
        milliseconds: f64,
    ) -> Result<bool, ResolutionError> {
        let Some(last) = &self.last else {
            return Ok(false);
        };
        if !Arc::ptr_eq(&frame.owner, &self.identity)
            || frame.epoch != last.epoch
            || frame.serial <= self.observed
            || frame.serial > last.serial
            || frame.scale != self.scale()
            || frame.pixel_perfect()
        {
            return Ok(false);
        }
        self.control.observe(milliseconds)?;
        self.observed = frame.serial;
        Ok(true)
    }
}

//! macOS-only adapter. Never retains the drawable in its own callback.
use super::{
    Surface,
    presentation::{Error, Observation, Ticket},
};
use core::ptr::NonNull;
use objc2::{available, runtime::ProtocolObject};
use objc2_metal::MTLDrawable;

impl Surface {
    /// Diagnostic-only binding for subsequent successful acquisitions. Replacing
    /// it does not relabel any already acquired drawable or late callback.
    /// Set context before get_current_texture; clear before final snapshot.
    pub fn set_presentation_observation(
        &self,
        observation: Option<Observation>,
    ) -> Result<(), Error> {
        if observation.is_some() && !available!(macos = 10.15.4) {
            return Err(Error::Unavailable);
        }
        *self.presentation_observation.lock() = observation;
        Ok(())
    }
}
pub(super) fn attach(ticket: &Ticket, drawable: &ProtocolObject<dyn MTLDrawable>) {
    let Some(completion) = ticket.request() else {
        return;
    };
    let block = block2::RcBlock::new(move |raw: NonNull<ProtocolObject<dyn MTLDrawable>>| {
        // SAFETY: Metal invokes the registered handler with its live drawable.
        // Read the supported timestamp while it is valid; never save raw/retain.
        let time = unsafe { raw.as_ref() }.presentedTime();
        completion.presented(time);
    });
    // SAFETY: Metal copies this valid block. Captures contain only Arc<Collector>
    // and an index into its preallocated rows; neither owns the drawable/window.
    unsafe {
        drawable.addPresentedHandler(block2::RcBlock::as_ptr(&block));
    }
}

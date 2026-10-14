//! Host adapter to qualify for a future native-player diagnostic feature.
//! Does not create a surface, acquire a drawable, present, wait, or run a window.
extern crate alloc;

#[cfg(all(feature = "observer", target_os = "macos"))]
pub use wgpu_hal::metal::presentation::{
    Collector, Context, Coverage, Error, Observation, Outcome, Snapshot,
};

#[cfg(all(feature = "observer", target_os = "macos"))]
pub fn bind(surface: &wgpu::Surface<'_>, observation: Option<Observation>) -> Result<(), Error> {
    // SAFETY: borrow the existing Metal surface only; never destroy/replace it,
    // touch a raw resource, acquire a second drawable or modify layer scheduling.
    let hal = unsafe { surface.as_hal::<wgpu::hal::api::Metal>() }.ok_or(Error::Unavailable)?;
    hal.set_presentation_observation(observation)
}

// Compile the exact copied collector source as the local CPU test model. The
// real public API above separately compiles against the patched HAL dependency.
#[cfg(all(test, feature = "observer", target_os = "macos"))]
#[allow(
    dead_code,
    reason = "The CPU model has no native adapter for Error::Unavailable"
)]
#[path = "../wgpu-hal/src/metal/presentation.rs"]
mod collector_model;

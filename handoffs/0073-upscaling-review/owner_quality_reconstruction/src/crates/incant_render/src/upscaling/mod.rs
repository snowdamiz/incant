//! Explicit portable output-resolution temporal reconstruction. Opaque/MASK
//! history is separate from current-only transparent/sprite/world-text overlays.
//! No quality preset, sharpening, MetalFX, or device budget claim.
#[cfg(test)]
mod confidence_oracle;
#[cfg(test)]
mod kernel_oracle;
#[cfg(test)]
mod kernel_patterns;
#[cfg(test)]
mod kernel_tests;
#[cfg(test)]
mod oracle;
mod owner;
mod plan;
pub(crate) mod system;
mod targets;
#[cfg(test)]
mod tests;
pub use crate::temporal::TemporalConfig as UpscaleConfig;
pub use owner::UpscaleFrame;
pub(crate) use owner::{Input, Pending, State};
pub use plan::MAX_UPSCALE_TRANSITION_BYTES;
pub(crate) use plan::Plan;
pub(crate) use system::{Draw, System};
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum UpscaleError {
    #[error("upscaling source/output footprint is invalid")]
    Footprint,
    #[error("upscaling combined old/candidate allocation limit")]
    Capacity,
    #[error("upscaling motion and output history have different owners")]
    History,
    #[error("upscaling pipelines were not enabled at startup")]
    Disabled,
    #[error("upscaling HDR/depth format is unsupported")]
    Format,
}

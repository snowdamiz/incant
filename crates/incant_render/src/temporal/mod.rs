//! Explicit opt-in, fixed-internal-resolution temporal accumulation. This is not
//! temporal upscaling or MetalFX. Only opaque/MASK model radiance enters history.
mod config;
#[cfg(test)]
mod coverage;
#[cfg(test)]
mod coverage_tests;
pub(crate) mod jitter;
#[cfg(test)]
mod oracle;
mod owner;
pub(crate) mod system;
mod targets;
#[cfg(test)]
mod tests;
pub use config::TemporalConfig;
pub use owner::TemporalFrame;
pub(crate) use owner::{Pending, TemporalState};
pub(crate) use system::{TemporalDraw, TemporalSystem};
pub use targets::{MAX_TEMPORAL_COLOR_BYTES, MAX_TEMPORAL_TRANSITION_BYTES};
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TemporalError {
    #[error("invalid temporal accumulation configuration")]
    Config,
    #[error("temporal pipelines were not enabled at renderer startup")]
    Disabled,
    #[error("temporal color, motion and depth have different submitted owners")]
    History,
    #[error("temporal combined history allocation limit")]
    Capacity,
    #[error("temporal HDR format is unsupported")]
    Format,
    #[error("temporal cluster source contract changed")]
    ClusterSource,
}

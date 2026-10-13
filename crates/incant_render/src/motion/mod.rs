//! Opt-in opaque-surface motion and depth, not temporal color reconstruction.
//! History belongs to one immediately submitted retained scene, never world ticks.
mod draw;
mod owner;
mod plan;
mod provenance;
mod retained;
pub(crate) mod system;
mod targets;
#[cfg(test)]
mod tests;

pub(crate) use draw::MotionDraw;
pub use owner::MotionFrame;
pub use plan::{MAX_MOTION_INSTANCE_BYTES, MAX_MOTION_PIXELS, MAX_MOTION_TARGET_BYTES};
pub use provenance::MotionChanges;
pub(crate) use retained::{MotionState, MotionView, TemporalInput};
pub(crate) use system::MotionSystem;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MotionError {
    #[error(
        "motion history requires the complete cooked-temporal feature when native cooking is enabled"
    )]
    Cooked,
    #[error("motion pipelines were not enabled at renderer startup")]
    Disabled,
    #[error("motion history belongs to another renderer")]
    Owner,
    #[error("motion history sequence exhausted")]
    Sequence,
    #[error("motion attachment or instance allocation limit")]
    Capacity,
    #[error("motion history topology differs from the retained model binding")]
    Topology,
    #[error("motion projection is nonfinite or invalid")]
    Projection,
    #[error("motion texture formats are unsupported by this device")]
    Format,
}
pub(crate) fn admission() -> Result<(), MotionError> {
    if cfg!(all(
        not(target_arch = "wasm32"),
        not(feature = "cooked-temporal"),
        any(
            feature = "cooked-output",
            feature = "cooked-geometry",
            feature = "cooked-sprites",
            feature = "cooked-ui",
            feature = "cooked-text",
            feature = "cooked-materials",
            feature = "cooked-clusters"
        )
    )) {
        return Err(MotionError::Cooked);
    }
    Ok(())
}

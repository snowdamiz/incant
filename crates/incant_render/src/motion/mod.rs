//! Opt-in opaque-surface motion and depth, not temporal color reconstruction.
//! History belongs to one immediately submitted retained scene, never world ticks.
mod draw;
mod owner;
mod plan;
mod retained;
mod system;
mod targets;
#[cfg(test)]
mod tests;

pub(crate) use draw::MotionDraw;
pub use owner::MotionFrame;
pub use plan::{MAX_MOTION_INSTANCE_BYTES, MAX_MOTION_PIXELS, MAX_MOTION_TARGET_BYTES};
pub(crate) use retained::MotionState;
pub(crate) use system::MotionSystem;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum MotionError {
    #[error(
        "motion history requires its uncooked portable pipeline; native cooked features are unsupported"
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

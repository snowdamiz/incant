//! Opt-in spatial resolution control. This is not a temporal upscaler.
mod controller;
mod owner;
mod plan;
pub use controller::{ResolutionConfig, ResolutionController, ResolutionError, ResolutionScale};
pub use owner::ResolutionFrame;
pub(crate) use owner::ResolutionOwner;
pub(crate) use plan::FramePlan;
#[cfg(test)]
mod tests;

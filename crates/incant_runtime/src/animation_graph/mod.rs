//! Compiled animation state machines and bounded 1D blend trees.
pub(crate) mod binary;
mod definition;
pub use definition::*;
mod playback;
pub use playback::{AnimationGraphControl, AnimationGraphSnapshot, AnimationGraphState};

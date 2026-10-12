//! Compiled animation state machines and bounded 1D blend trees.
pub(crate) mod binary;
mod definition;
pub use definition::*;
pub(crate) mod fade;
mod motion;
mod playback;
pub use fade::AnimationGraphFadeSnapshot;
pub use playback::{AnimationGraphControl, AnimationGraphSnapshot, AnimationGraphState};
mod tree;
pub use tree::*;

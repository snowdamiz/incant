//! Experimental GPU-only color TAAU. Explicit startup/retained opt-in; not a
//! shipping policy or visual/performance acceptance. See gpu-color-taau.md.
#[cfg(all(test, feature = "native-metalfx", target_os = "macos"))]
mod comparison;
mod owner;
pub(crate) mod render;
pub(crate) mod system;
#[cfg(test)]
mod tests;
pub(crate) use owner::{Pending, State};
pub(crate) use system::System;
pub const MAX_HISTORY_BYTES: u64 = 64 * 1024 * 1024;

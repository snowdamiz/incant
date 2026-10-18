//! Experimental GPU-only color TAAU. Explicit startup/retained opt-in; not a
//! shipping policy or visual/performance acceptance. See gpu-color-taau.md.
#[cfg(all(test, feature = "native-metalfx", target_os = "macos"))]
mod comparison;
mod continuity;
mod owner;
#[cfg(test)]
mod raster_geometry;
#[cfg(test)]
mod raster_readback;
#[cfg(test)]
mod raster_tests;
pub(crate) mod render;
#[cfg(test)]
mod static_tests;
pub(crate) mod system;
#[cfg(test)]
mod tests;
pub(crate) use owner::{Pending, State};
pub(crate) use system::System;
pub const MAX_HISTORY_BYTES: u64 = 64 * 1024 * 1024;

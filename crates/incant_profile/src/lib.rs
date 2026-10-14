//! Opt-in, process-scoped native diagnostics. Default hooks are zero-sized no-ops.
//! Fixed phase labels and at most64 numeric script identities; no authored strings.
//! Nested wall/thread-CPU durations are inclusive and must not be summed as frame time.
mod frame_model;
mod heap_model;
mod model;
pub use frame_model::*;
pub use heap_model::*;
#[cfg(feature = "summary")]
mod frame;
pub use model::*;
#[cfg(feature = "summary")]
mod enabled;
#[cfg(feature = "summary")]
pub use enabled::*;
#[cfg(not(feature = "summary"))]
mod disabled;
#[cfg(not(feature = "summary"))]
pub use disabled::*;
#[cfg(feature = "summary")]
mod allocation;
#[cfg(feature = "summary")]
pub use allocation::CountingAllocator;
#[cfg(all(feature = "tracy", any(unix, windows)))]
mod tracy;

//! QuickJS runtime related types.

mod base;
mod exotic;
pub(crate) mod opaque;
pub(crate) mod raw;
mod userdata;

#[cfg(feature = "futures")]
mod r#async;
#[cfg(feature = "futures")]
pub(crate) mod schedular;
#[cfg(feature = "futures")]
mod spawner;
#[cfg(feature = "futures")]
pub use spawner::DriveFuture;

use alloc::boxed::Box;
pub use base::{Runtime, WeakRuntime};
pub use userdata::{UserDataError, UserDataGuard};

#[cfg(feature = "futures")]
pub(crate) use r#async::InnerRuntime;
#[cfg(feature = "futures")]
pub use r#async::{AsyncRuntime, AsyncWeakRuntime};

use crate::value::promise::PromiseHookType;
use crate::{Ctx, Value};

/// The type of the promise hook.
#[cfg(not(feature = "parallel"))]
pub type PromiseHook =
    Box<dyn for<'a> Fn(Ctx<'a>, PromiseHookType, Value<'a>, Value<'a>) + 'static>;
/// The type of the promise hook.
#[cfg(feature = "parallel")]
pub type PromiseHook =
    Box<dyn for<'a> Fn(Ctx<'a>, PromiseHookType, Value<'a>, Value<'a>) + Send + 'static>;

/// The type of the promise rejection tracker.
#[cfg(not(feature = "parallel"))]
pub type RejectionTracker = Box<dyn for<'a> Fn(Ctx<'a>, Value<'a>, Value<'a>, bool) + 'static>;
/// The type of the promise rejection tracker.
#[cfg(feature = "parallel")]
pub type RejectionTracker =
    Box<dyn for<'a> Fn(Ctx<'a>, Value<'a>, Value<'a>, bool) + Send + 'static>;

/// The type of the interrupt handler.
#[cfg(not(feature = "parallel"))]
pub type InterruptHandler = Box<dyn FnMut() -> bool + 'static>;
/// The type of the interrupt handler.
#[cfg(feature = "parallel")]
pub type InterruptHandler = Box<dyn FnMut() -> bool + Send + 'static>;

/// Exact current QuickJS logical allocation counters, not backing/RSS/used heap.
/// Values are instantaneous observations; this does not track a continuous peak.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MallocState {
    pub allocated_bytes: usize,
    pub allocation_count: usize,
    /// Zero means unlimited, matching the pinned native API.
    pub limit_bytes: usize,
}

/// A struct with information about the runtimes memory usage.
pub type MemoryUsage = crate::qjs::JSMemoryUsage;

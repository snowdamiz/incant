//! Rust guest SDK for Incant's implemented native Wasm capabilities.
//!
//! This `no_std`, allocation-free API uses caller-owned bounded scratch. Numeric
//! columns and snapshots are copies, not native pointers. Commands publish only
//! after a successful callback. Returning an error, or ignoring an error from an
//! active Tick operation, traps the callback and rejects all its staged writes.
#![no_std]
pub mod animation;
pub mod flipbook;
pub mod input;
pub mod localization;
mod raw;
pub mod steering;
mod tick;
pub use animation::{AnimationControl, AnimationPlayback, AnimationSnapshot, AnimationTransition};
pub use flipbook::{FlipbookSnapshot, Playback};
pub use incant_wasm_abi::{ActionKind, GamepadButton, Key, LogLevel, MouseButton, TouchPhase};
pub use input::InputSnapshot;
pub use tick::{Chunk, Tick};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Capacity,
    Bounds,
    Malformed,
    Failed,
    MissingVelocity,
    Callback,
}
pub type Result<T> = core::result::Result<T, Error>;
/// Canonical big-endian 128-bit stable entity identity; never a native handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub struct EntityId(pub [u8; incant_wasm_abi::ID_BYTES]);
impl EntityId {
    pub const fn from_u128(value: u128) -> Self {
        Self(value.to_be_bytes())
    }
}
/// translation xyz, quaternion xyzw, scale xyz; no padding or native pointers.
pub type Transform = [f64; incant_wasm_abi::TRANSFORM_STRIDE];
pub type Velocity = [f64; incant_wasm_abi::VELOCITY_STRIDE];
pub const IDENTITY: Transform = [0., 0., 0., 0., 0., 0., 1., 1., 1., 1.];
pub use incant_wasm_abi::{ANIMATION_MAX_BYTES, FLIPBOOK_MAX_BYTES, INPUT_MAX_BYTES};

/// Export the versioned entry points. The callback receives a non-escapable Tick
/// lifetime; errors trap before the host can publish its queued transaction.
/// The embedding application owns the panic handler.
#[macro_export]
macro_rules! module {
    ($callback:path) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn incant_abi() -> i32 {
            $crate::__private::ABI_VERSION
        }
        #[unsafe(no_mangle)]
        pub extern "C" fn update(epoch: i64, dt: f64) {
            $crate::__private::run(epoch, dt, $callback);
        }
    };
}
#[doc(hidden)]
pub mod __private {
    pub use incant_wasm_abi::ABI_VERSION;
    pub fn run(
        epoch: i64,
        dt: f64,
        callback: impl for<'tick> FnOnce(&mut crate::Tick<'tick>, f64) -> crate::Result<()>,
    ) {
        let mut tick = crate::Tick::new(epoch);
        if !dt.is_finite() || dt <= 0. || callback(&mut tick, dt).is_err() || tick.failed() {
            trap();
        }
    }
    pub fn trap() -> ! {
        #[cfg(target_arch = "wasm32")]
        {
            core::arch::wasm32::unreachable()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            panic!("guest callback rejected")
        }
    }
}

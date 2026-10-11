//! Explicitly enabled desktop development Wasm host. Native shipping uses
//! wasm2c; this crate is not a dependency of any shipped runtime/player.
#[cfg(all(
    feature = "wasmtime-host",
    not(any(target_os = "macos", target_os = "linux", target_os = "windows"))
))]
compile_error!("the Wasmtime development host supports desktop targets only");

#[cfg(feature = "wasmtime-host")]
mod abi;
#[cfg(feature = "wasmtime-host")]
mod bounds;
#[cfg(feature = "wasmtime-host")]
mod host;
#[cfg(feature = "wasmtime-host")]
pub use host::{DevModuleHost, ModuleError};

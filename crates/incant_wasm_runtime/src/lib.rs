//! Shared copied `incant_v1` capability and native transaction boundary.
//! Contains no compiler, parser, JIT, OS or authoring capability.
mod animations;
mod capability;
mod flipbooks;
mod generated;
pub use flipbooks::{
    MAGIC as FLIPBOOK_MAGIC, MAX_BYTES as MAX_FLIPBOOK_BYTES, ROW_BYTES as FLIPBOOK_ROW_BYTES,
};
mod input;
mod localization;
mod logging;
pub use incant_runtime::logging::{LogLevel, NativeLog, NativeLogs};
pub use input::{MAGIC as INPUT_MAGIC, MAX_BYTES as MAX_INPUT_BYTES};
mod compiled;
pub use compiled::{Import, ModuleExports, StaticError, StaticModule};
pub mod limits;
mod transaction;
pub use capability::{Call, Capability};
use incant_runtime::RuntimeError;
use thiserror::Error;
#[derive(Debug, Error)]
pub enum BoundaryError {
    #[error("Wasm ABI: {0}")]
    Abi(&'static str),
    #[error("Wasm quota: {0}")]
    Limit(&'static str),
    #[error(transparent)]
    Runtime(#[from] RuntimeError),
    #[error("module failed; construct or reload a new instance")]
    Failed,
}

#[cfg(feature = "embedded-fixture")]
pub mod embedded {
    include!(concat!(env!("OUT_DIR"), "/linked.rs"));
}

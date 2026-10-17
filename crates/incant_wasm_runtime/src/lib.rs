//! Shared copied `incant_v1` capability and native transaction boundary.
//! Contains no compiler, parser, JIT, OS or authoring capability.
mod animation_graphs;
mod animation_graphs_v2;
mod animation_ik;
mod animation_ik_v2;
mod animation_poses;
mod animation_poses_v5;
mod animations;
mod audio;
mod capability;
mod flipbooks;
mod generated;
mod navigation_paths;
mod physics_queries;
mod root_motion;
mod steering;
pub use flipbooks::{
    MAGIC as FLIPBOOK_MAGIC, MAX_BYTES as MAX_FLIPBOOK_BYTES, ROW_BYTES as FLIPBOOK_ROW_BYTES,
};
mod haptics;
mod hud;
mod input;
mod localization;
mod logging;
pub use incant_runtime::logging::{LogLevel, NativeLog, NativeLogs};
pub use input::{MAGIC as INPUT_MAGIC, MAX_BYTES as MAX_INPUT_BYTES};
mod compiled;
pub use compiled::{Import, ModuleExports, StaticError, StaticModule};
pub mod limits;
pub mod persistent;
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
    #[error(transparent)]
    Query(#[from] incant_runtime::QueryError),
    #[error(transparent)]
    Haptics(#[from] incant_input::haptics::HapticError),
    #[error("module failed; construct or reload a new instance")]
    Failed,
}

#[cfg(feature = "embedded-fixture")]
pub mod embedded {
    include!(concat!(env!("OUT_DIR"), "/linked.rs"));
}

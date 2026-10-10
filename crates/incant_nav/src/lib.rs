//! Bounded, deterministic navigation geometry. No filesystem, editor or network capability.
mod build;
mod grid;
pub use grid::{GridPath, GridPathRequest, MAX_GRID_CELLS, NavigationGrid};
mod path;
mod resources;
mod search;
pub use resources::{NavigationAsset, NavigationResources};
mod shortcut;
mod surface;
mod types;
pub use build::{NavigationMesh, RebuildReport};
pub use path::{NavigationPath, PathRequest};
use thiserror::Error;
pub use types::*;

#[derive(Debug, Clone, Error)]
pub enum NavigationError {
    #[error("invalid navigation input: {0}")]
    Invalid(String),
    #[error("navigation build failed: {0}")]
    Build(String),
    #[error("navigation resource limit exceeded: {0}")]
    Limit(String),
}
pub(crate) fn invalid(message: impl Into<String>) -> NavigationError {
    NavigationError::Invalid(message.into())
}
pub(crate) fn limit(message: impl Into<String>) -> NavigationError {
    NavigationError::Limit(message.into())
}

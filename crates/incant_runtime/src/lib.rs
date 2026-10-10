//! Native simulation loaded from checked binary scenes. This first migration
//! slice supports spatial entities and linear velocity; authoring and all other
//! gameplay systems remain outside this crate until explicitly migrated.
mod binary;
mod scene;
mod world;

pub use scene::{CookedEntity, CookedScene, SceneError, StableId};
pub use world::{EntitySnapshot, NativeWorld, RuntimeError};

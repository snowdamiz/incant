//! Native simulation loaded from checked binary scenes. This first migration
//! slice supports spatial entities and linear velocity; authoring and all other
//! gameplay systems remain outside this crate until explicitly migrated.
mod binary;
mod commands;
mod hierarchy;
mod scene;
mod world;

pub use commands::{FrameCommands, MAX_FRAME_COMMANDS, StructuralCommand};
pub use scene::{CookedEntity, CookedScene, SceneError, StableId};
pub use world::{EntitySnapshot, NativeWorld, RuntimeError};

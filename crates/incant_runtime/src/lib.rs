//! Native simulation loaded from checked binary scenes. This first migration
//! slice supports spatial motion and typed 3D rigid-body/collider physics.
//! Authoring and other gameplay systems remain outside until explicitly migrated.
mod audio;
mod audio_binary;
mod binary;
mod columns;
mod commands;
mod hierarchy;
mod physics;
mod physics_binary;
mod save;
mod scene;
mod world;

pub use audio::{
    CookedAudio, MAX_AUDIO_BUSES, MAX_AUDIO_NODES, MAX_AUDIO_SOURCES, NativeAudioBus,
    NativeAudioSource,
};
pub use columns::{NumericChunk, NumericColumns, TRANSFORM_STRIDE, VELOCITY_STRIDE};
pub use commands::{FrameCommands, MAX_FRAME_COMMANDS, StructuralCommand};
pub use incant_physics_runtime::{BodyState, MAX_TRIGGER_EVENTS, MAX_TRIGGER_PAIRS, TriggerEvent};
pub use physics::CookedPhysics;
pub use save::{MAX_SAVE_BYTES, SaveError};
pub use scene::{CookedEntity, CookedScene, SceneError, StableId};
pub use world::{EntitySnapshot, NativeWorld, RuntimeError};

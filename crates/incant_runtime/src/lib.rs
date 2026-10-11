//! Native simulation loaded from checked binary scenes. This first migration
//! slice supports spatial motion, typed physics/audio/render declarations and
//! committed spatial extraction. Mixer/render hosts and authoring stay separate.
mod audio;
mod audio_binary;
mod binary;
mod columns;
mod commands;
mod flipbook;
mod flipbook_binary;
mod hierarchy;
pub use flipbook::{
    FlipbookControl, FlipbookSnapshot, FlipbookState, FrameRate, MAX_FLIPBOOK_FRAMES,
    MAX_SCENE_FLIPBOOK_FRAMES, NativeFlipbook,
};
mod physics;
mod physics_2d;
mod physics_2d_binary;
mod physics_binary;
mod physics_queries;
mod save;
mod scene;
mod spatial;
mod sprite;
mod sprite_binary;
mod tilemap;
mod tilemap_binary;
pub use sprite::{
    MAX_SPRITE_ENTITIES, NativeSpriteRegion, NativeSpriteRenderer, SpriteMode, SpriteSampling,
};
pub use tilemap::{
    MAX_SCENE_TILE_SLOTS, MAX_TILEMAP_ENTITIES, NativeTilemap, NativeTilemapLayer, PreparedTilemap,
};
mod entity_identity;
mod spatial_binary;
mod spatial_commands;
mod spatial_view;
mod world;
mod world_identity;

pub use audio::{
    CookedAudio, MAX_AUDIO_BUSES, MAX_AUDIO_NODES, MAX_AUDIO_SOURCES, NativeAudioBus,
    NativeAudioSource,
};
pub use columns::{NumericChunk, NumericColumns, TRANSFORM_STRIDE, VELOCITY_STRIDE};
pub use commands::{FrameCommands, MAX_FRAME_COMMANDS, StructuralCommand};
pub use entity_identity::{EntityIdentity, WorldIncarnation};
pub use incant_physics_2d_runtime::{
    BodyState as BodyState2D, RayHit as RayHit2D, RayQuery as RayQuery2D,
    TriggerEvent as TriggerEvent2D,
};
pub use incant_physics_runtime::{BodyState, MAX_TRIGGER_EVENTS, MAX_TRIGGER_PAIRS, TriggerEvent};
pub use incant_physics_runtime::{
    CharacterMovement, CharacterOptions, CharacterQuery, CharacterStep,
};
pub use incant_physics_runtime::{RayHit, RayQuery};
pub use physics::CookedPhysics;
pub use physics_2d::CookedPhysics2D;
pub use physics_queries::{NativeQueries, QueryError};
pub use save::{MAX_SAVE_BYTES, SaveError};
pub use scene::{CookedEntity, CookedScene, SceneError, StableId};
pub use world::{EntitySnapshot, NativeWorld, RuntimeError};
pub use world_identity::WorldIdentity;

pub use spatial::{
    CookedSpatial, MAX_DIRECTIONAL_LIGHTS, MAX_LOCAL_LIGHTS, MAX_MESH_ENTITIES, MAX_SHADOW_LIGHTS,
    MAX_VISUAL_ASSETS, NativeEnvironmentLight, NativeMeshRenderer, VisualAssetKind, VisualAssetRef,
};
pub use spatial_view::{SpatialNode, SpatialView};

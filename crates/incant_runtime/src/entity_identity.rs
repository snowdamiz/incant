//! Lifetime identity for retained native attachments. No raw ECS access leaks.
use bevy_ecs::{entity::Entity, world::WorldId};

/// Comparable only; callers cannot construct a token or use it to access an ECS
/// table. A plain Spawn reusing a stable ID has a different entity generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EntityIdentity {
    pub(crate) world: WorldId,
    pub(crate) entity: Entity,
}

/// Identifies a particular ECS allocation; unlike the retained host identity,
/// it changes when an in-place restore replaces the simulation world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldIncarnation(pub(crate) WorldId);

//! Runtime-only structural changes, staged and checked before a sync point.
//! This is not the authoring command bus: no journal, provenance or JSON path.
use crate::scene::{MAX_ENTITIES, matrix, validate_values};
use crate::world::{Global, Hierarchy, Local, Motion, spawn};
use crate::{CookedEntity, NativeWorld, RuntimeError, SceneError, StableId};
use incant_types::Velocity;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_FRAME_COMMANDS: usize = 65_536;

#[derive(Debug, Clone, PartialEq)]
pub enum StructuralCommand {
    Spawn(CookedEntity),
    /// Children must also be removed or reparented in this batch. No implicit
    /// cascading deletion, and no recycling an existing ID within one batch.
    Despawn(StableId),
    /// Keeps the local transform. A parent may be spawned later in this batch.
    SetParent {
        entity: StableId,
        parent: Option<StableId>,
    },
    /// Adds, replaces or removes the actual Bevy motion component.
    SetVelocity {
        entity: StableId,
        velocity: Option<Velocity>,
    },
}

/// Reusable, bounded queue. Collect it while borrowing component views, then
/// apply between ticks. Failure retains both world state and the submitted queue.
#[derive(Debug, Default)]
pub struct FrameCommands {
    commands: Vec<StructuralCommand>,
}
impl FrameCommands {
    pub fn push(&mut self, command: StructuralCommand) -> Result<(), RuntimeError> {
        if self.commands.len() == MAX_FRAME_COMMANDS {
            return Err(RuntimeError::BufferFull(MAX_FRAME_COMMANDS));
        }
        self.commands.push(command);
        Ok(())
    }
    pub fn len(&self) -> usize {
        self.commands.len()
    }
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
    pub fn clear(&mut self) {
        self.commands.clear();
    }
    pub fn iter(&self) -> impl ExactSizeIterator<Item = &StructuralCommand> {
        self.commands.iter()
    }
}

struct Staged {
    parents: BTreeMap<StableId, Option<StableId>>,
    spawns: BTreeMap<StableId, CookedEntity>,
    removed: BTreeSet<StableId>,
    velocities: BTreeMap<StableId, Option<Velocity>>,
    hierarchy: Vec<crate::hierarchy::Node>,
}

impl NativeWorld {
    /// Atomically applies one batch at a safe boundary. Borrowing `&mut self`
    /// excludes active native component views and running schedule execution.
    /// Validation reads native values only; it never constructs a Project.
    pub fn apply(&mut self, commands: &mut FrameCommands) -> Result<(), RuntimeError> {
        if commands.is_empty() {
            return Ok(());
        }
        // Component-only batches don't copy hierarchy metadata or inspect every
        // entity. In particular, toggling one mover scales with this batch.
        if commands
            .iter()
            .all(|c| matches!(c, StructuralCommand::SetVelocity { .. }))
        {
            let mut velocities = BTreeMap::new();
            for command in commands.iter() {
                let StructuralCommand::SetVelocity { entity, velocity } = command else {
                    unreachable!()
                };
                if !self.entities.contains_key(entity) {
                    return Err(RuntimeError::MissingEntity(*entity));
                }
                validate_velocity(*entity, velocity)?;
                velocities.insert(*entity, velocity.clone());
            }
            self.install_velocities(velocities);
        } else {
            let staged = self.stage(commands)?;
            // Everything that can reject external data completed above. These
            // handles/IDs are exclusively owned by this world until commit ends.
            for id in staged.removed {
                let entity = self.entities.remove(&id).expect("staged entity");
                self.world.despawn(entity);
            }
            for (id, data) in staged.spawns {
                self.entities.insert(id, spawn(&mut self.world, data));
            }
            self.install_velocities(staged.velocities);
            let mut hierarchy = Hierarchy {
                nodes: Vec::with_capacity(staged.hierarchy.len()),
                globals: Vec::with_capacity(staged.hierarchy.len()),
            };
            for node in staged.hierarchy {
                let entity = self.entities[&node.id];
                self.world
                    .get_mut::<Global>(entity)
                    .expect("runtime spatial component")
                    .0 = node.world;
                hierarchy.nodes.push((entity, node.parent));
                hierarchy.globals.push(node.world);
            }
            self.parents = staged.parents;
            self.world.insert_resource(hierarchy);
        }
        commands.clear();
        Ok(())
    }

    fn install_velocities(&mut self, velocities: BTreeMap<StableId, Option<Velocity>>) {
        for (id, velocity) in velocities {
            let mut entity = self.world.entity_mut(self.entities[&id]);
            if let Some(velocity) = velocity {
                entity.insert(Motion(velocity));
            } else {
                entity.remove::<Motion>();
            }
        }
    }

    fn stage(&self, commands: &FrameCommands) -> Result<Staged, RuntimeError> {
        // Rebuild topology only on structural topology changes. This copies IDs
        // and parent links, not component data or the authoring document.
        let mut parents = self.parents.clone();
        let mut spawns = BTreeMap::new();
        let mut seen_spawns = BTreeSet::new();
        let mut removed = BTreeSet::new();
        let mut velocities = BTreeMap::new();
        for command in commands.iter() {
            match command {
                StructuralCommand::Spawn(data) => {
                    if data.id == self.scene_id()
                        || self.entities.contains_key(&data.id)
                        || !seen_spawns.insert(data.id)
                    {
                        return Err(SceneError::Duplicate(data.id).into());
                    }
                    validate_values(data)?;
                    parents.insert(data.id, data.parent);
                    spawns.insert(data.id, data.clone());
                }
                StructuralCommand::Despawn(id) => {
                    parents.remove(id).ok_or(RuntimeError::MissingEntity(*id))?;
                    if spawns.remove(id).is_none() {
                        removed.insert(*id);
                    }
                    velocities.remove(id);
                }
                StructuralCommand::SetParent { entity, parent } => {
                    *parents
                        .get_mut(entity)
                        .ok_or(RuntimeError::MissingEntity(*entity))? = *parent;
                }
                StructuralCommand::SetVelocity { entity, velocity } => {
                    if !parents.contains_key(entity) {
                        return Err(RuntimeError::MissingEntity(*entity));
                    }
                    validate_velocity(*entity, velocity)?;
                    velocities.insert(*entity, velocity.clone());
                }
            }
        }
        if parents.len() > MAX_ENTITIES {
            return Err(SceneError::Invalid("entity limit").into());
        }
        let hierarchy = crate::hierarchy::prepare(&parents, |id| {
            if let Some(data) = spawns.get(&id) {
                matrix(&data.transform)
            } else {
                matrix(
                    &self
                        .world
                        .get::<Local>(self.entities[&id])
                        .expect("runtime spatial component")
                        .0,
                )
            }
        })?;
        Ok(Staged {
            parents,
            spawns,
            removed,
            velocities,
            hierarchy,
        })
    }
}

fn validate_velocity(id: StableId, value: &Option<Velocity>) -> Result<(), SceneError> {
    if value
        .as_ref()
        .is_some_and(|v| !v.linear.iter().all(|x| x.is_finite()))
    {
        return Err(SceneError::Component(id));
    }
    Ok(())
}

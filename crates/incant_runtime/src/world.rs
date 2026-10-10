use crate::scene::{matrix, validate_components};
use crate::{CookedEntity, CookedScene, SceneError, StableId};
use bevy_ecs::prelude::*;
use glam::DMat4;
use incant_types::{Transform, Velocity};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Component)]
#[repr(transparent)]
pub(crate) struct Identity(pub StableId);
#[derive(Component)]
#[repr(transparent)]
pub(crate) struct Local(pub Transform);
#[derive(Component)]
#[repr(transparent)]
pub(crate) struct Motion(pub Velocity);
#[derive(Component)]
pub(crate) struct Global(pub DMat4);
#[derive(Resource)]
struct Step(f64);
#[derive(Resource)]
pub(crate) struct Hierarchy {
    pub nodes: Vec<(Entity, Option<usize>)>,
    pub globals: Vec<DMat4>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntitySnapshot {
    pub id: StableId,
    pub parent: Option<StableId>,
    pub transform: Transform,
    pub velocity: Option<Velocity>,
    pub world_transform: [[f64; 4]; 4],
}

#[derive(Debug, Error, PartialEq)]
pub enum RuntimeError {
    #[error(transparent)]
    Scene(#[from] SceneError),
    #[error("runtime tick counter exhausted")]
    TickOverflow,
    #[error("runtime entity {0:?} does not exist")]
    MissingEntity(StableId),
    #[error("frame command buffer is full (maximum {0})")]
    BufferFull(usize),
}

/// A disposable Bevy world; no project, command bus, authoring validator or
/// serialization in the tick path. Snapshot conversion is explicitly requested.
pub struct NativeWorld {
    pub(crate) world: World,
    schedule: Schedule,
    pub(crate) entities: BTreeMap<StableId, Entity>,
    pub(crate) parents: BTreeMap<StableId, Option<StableId>>,
    scene_id: StableId,
    pub(crate) tick: u64,
    pub(crate) tick_rate: u32,
}

impl NativeWorld {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RuntimeError> {
        Ok(Self::new(CookedScene::from_bytes(bytes)?))
    }

    pub fn new(scene: CookedScene) -> Self {
        let mut world = World::new();
        world.insert_resource(Step(1. / f64::from(scene.tick_rate)));
        let mut entities = BTreeMap::new();
        let parents = scene.entities.iter().map(|e| (e.id, e.parent)).collect();
        let handles: Vec<_> = scene
            .entities
            .into_iter()
            .map(|entity| {
                let id = entity.id;
                let handle = spawn(&mut world, entity);
                entities.insert(id, handle);
                handle
            })
            .collect();
        world.insert_resource(Hierarchy {
            nodes: scene
                .topology
                .into_iter()
                .map(|(index, parent)| (handles[index], parent))
                .collect(),
            globals: vec![DMat4::IDENTITY; handles.len()],
        });
        let mut initialize = Schedule::default();
        initialize.add_systems(propagate);
        initialize.run(&mut world);
        let mut schedule = Schedule::default();
        // Native builds enable Bevy's multi_threaded executor and parallel
        // queries. Browser builds remain serial until their worker host exists.
        schedule.add_systems((integrate, publish_globals).chain());
        Self {
            world,
            schedule,
            entities,
            parents,
            scene_id: scene.id,
            tick: 0,
            tick_rate: scene.tick_rate,
        }
    }

    pub fn scene_id(&self) -> StableId {
        self.scene_id
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn tick_rate(&self) -> u32 {
        self.tick_rate
    }
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn step(&mut self) -> Result<(), RuntimeError> {
        let next = self.tick.checked_add(1).ok_or(RuntimeError::TickOverflow)?;
        // Prepare the complete candidate hierarchy in its existing scratch
        // buffer before changing any live component or the simulation clock.
        // This is typed numeric work, not authoring validation or a world copy.
        self.prepare_globals(Some(self.world.resource::<Step>().0))?;
        self.schedule.run(&mut self.world);
        self.tick = next;
        Ok(())
    }

    /// Native bulk access borrows ECS component values directly. Structural
    /// mutations cannot occur during the borrow. Queue them in FrameCommands and
    /// apply after this callback returns; foreign script bindings remain open.
    pub fn for_each_moving_mut(
        &mut self,
        mut visit: impl FnMut(StableId, &mut Transform, &mut Velocity),
    ) {
        let mut query = self.world.query::<(&Identity, &mut Local, &mut Motion)>();
        for (id, mut local, mut motion) in query.iter_mut(&mut self.world) {
            visit(id.0, &mut local.0, &mut motion.0);
        }
        self.refresh_globals();
    }

    pub(crate) fn refresh_globals(&mut self) {
        // Update derived hierarchy once for this bulk operation, without a
        // document projection or per-entity command/serialization round-trip.
        self.world
            .resource_scope(|world, mut hierarchy: Mut<Hierarchy>| {
                let mut query = world.query::<(&Local, &mut Global)>();
                propagate_values(&mut hierarchy, |entity, global| {
                    let (local, mut destination) =
                        query.get_mut(world, entity).expect("fixed topology");
                    let value = global * matrix(&local.0);
                    destination.0 = value;
                    value
                });
            });
    }

    /// Check immediate numeric writes or the next integration step without
    /// publishing derived state. A failed pass may overwrite scratch entries;
    /// every later pass fills them parent-first before reading them again.
    pub(crate) fn prepare_globals(&mut self, step: Option<f64>) -> Result<(), SceneError> {
        self.world
            .resource_scope(|world, mut hierarchy: Mut<Hierarchy>| {
                let mut query = world.query::<(&Identity, &Local, Option<&Motion>)>();
                for index in 0..hierarchy.nodes.len() {
                    let (entity, parent) = hierarchy.nodes[index];
                    let (id, local, motion) = query.get(world, entity).expect("fixed topology");
                    let velocity = motion.map(|v| &v.0);
                    validate_components(id.0, &local.0, velocity)?;
                    let local_matrix = if let (Some(step), Some(velocity)) = (step, velocity) {
                        let mut candidate = local.0.clone();
                        advance_translation(&mut candidate, velocity, step);
                        matrix(&candidate)
                    } else {
                        matrix(&local.0)
                    };
                    let value = parent.map_or(local_matrix, |parent| {
                        hierarchy.globals[parent] * local_matrix
                    });
                    if !value.is_finite() {
                        return Err(SceneError::Component(id.0));
                    }
                    hierarchy.globals[index] = value;
                }
                Ok(())
            })
    }

    pub fn inspect(&self, id: StableId) -> Option<EntitySnapshot> {
        let entity = self.world.get_entity(*self.entities.get(&id)?).ok()?;
        Some(EntitySnapshot {
            id,
            parent: self.parents[&id],
            transform: entity.get::<Local>()?.0.clone(),
            velocity: entity.get::<Motion>().map(|m| m.0.clone()),
            world_transform: entity.get::<Global>()?.0.to_cols_array_2d(),
        })
    }

    /// Explicit inspection operation. Stepping never creates this vector.
    pub fn snapshot(&self) -> Vec<EntitySnapshot> {
        self.entities
            .keys()
            .map(|id| self.inspect(*id).expect("fixed topology"))
            .collect()
    }
}

fn integrate(step: Res<Step>, mut query: Query<(&mut Local, &Motion)>) {
    let advance = |(mut local, motion): (Mut<Local>, &Motion)| {
        advance_translation(&mut local.0, &motion.0, step.0);
    };
    #[cfg(not(target_arch = "wasm32"))]
    query.par_iter_mut().for_each(advance);
    #[cfg(target_arch = "wasm32")]
    query.iter_mut().for_each(advance);
}

fn advance_translation(transform: &mut Transform, velocity: &Velocity, step: f64) {
    for axis in 0..3 {
        transform.translation[axis] += velocity.linear[axis] * step;
    }
}

fn publish_globals(hierarchy: Res<Hierarchy>, mut query: Query<&mut Global>) {
    for (index, &(entity, _)) in hierarchy.nodes.iter().enumerate() {
        query.get_mut(entity).expect("fixed topology").0 = hierarchy.globals[index];
    }
}

pub(crate) fn spawn(world: &mut World, entity: CookedEntity) -> Entity {
    let mut target = world.spawn((
        Identity(entity.id),
        Local(entity.transform),
        Global(DMat4::IDENTITY),
    ));
    if let Some(velocity) = entity.velocity {
        target.insert(Motion(velocity));
    }
    target.id()
}

fn propagate(mut hierarchy: ResMut<Hierarchy>, mut query: Query<(&Local, &mut Global)>) {
    propagate_values(&mut hierarchy, |entity, parent| {
        let (local, mut global) = query.get_mut(entity).expect("fixed topology");
        let value = parent * matrix(&local.0);
        global.0 = value;
        value
    });
}

fn propagate_values(hierarchy: &mut Hierarchy, mut update: impl FnMut(Entity, DMat4) -> DMat4) {
    for index in 0..hierarchy.nodes.len() {
        let (entity, parent) = hierarchy.nodes[index];
        let global = parent.map_or(DMat4::IDENTITY, |parent| hierarchy.globals[parent]);
        hierarchy.globals[index] = update(entity, global);
    }
}

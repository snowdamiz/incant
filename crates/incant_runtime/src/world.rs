use crate::scene::matrix;
use crate::{CookedScene, SceneError, StableId};
use bevy_ecs::prelude::*;
use glam::DMat4;
use incant_types::{Transform, Velocity};
use std::collections::BTreeMap;
use thiserror::Error;

#[derive(Component)]
struct Identity(StableId);
#[derive(Component)]
struct Local(Transform);
#[derive(Component)]
struct Motion(Velocity);
#[derive(Component)]
struct Global(DMat4);
#[derive(Resource)]
struct Step(f64);
#[derive(Resource)]
struct Hierarchy {
    nodes: Vec<(Entity, Option<usize>)>,
    globals: Vec<DMat4>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntitySnapshot {
    pub id: StableId,
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
}

/// A disposable Bevy world; no project, command bus, authoring validator or
/// serialization in the tick path. Snapshot conversion is explicitly requested.
pub struct NativeWorld {
    world: World,
    schedule: Schedule,
    entities: BTreeMap<StableId, Entity>,
    scene_id: StableId,
    tick: u64,
}

impl NativeWorld {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, RuntimeError> {
        Ok(Self::new(CookedScene::from_bytes(bytes)?))
    }

    pub fn new(scene: CookedScene) -> Self {
        let mut world = World::new();
        world.insert_resource(Step(1. / f64::from(scene.tick_rate)));
        let mut entities = BTreeMap::new();
        let handles: Vec<_> = scene
            .entities
            .into_iter()
            .map(|entity| {
                let mut target = world.spawn((
                    Identity(entity.id),
                    Local(entity.transform),
                    Global(DMat4::IDENTITY),
                ));
                if let Some(velocity) = entity.velocity {
                    target.insert(Motion(velocity));
                }
                let handle = target.id();
                entities.insert(entity.id, handle);
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
        schedule.add_systems((integrate, propagate).chain());
        Self {
            world,
            schedule,
            entities,
            scene_id: scene.id,
            tick: 0,
        }
    }

    pub fn scene_id(&self) -> StableId {
        self.scene_id
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn entity_count(&self) -> usize {
        self.entities.len()
    }

    pub fn step(&mut self) -> Result<(), RuntimeError> {
        let next = self.tick.checked_add(1).ok_or(RuntimeError::TickOverflow)?;
        self.schedule.run(&mut self.world);
        self.tick = next;
        Ok(())
    }

    /// Native bulk access borrows ECS component values directly. Structural
    /// mutations cannot occur during the borrow; script-host bindings and their
    /// structural command buffer are separate migration work.
    pub fn for_each_moving_mut(
        &mut self,
        mut visit: impl FnMut(StableId, &mut Transform, &mut Velocity),
    ) {
        let mut query = self.world.query::<(&Identity, &mut Local, &mut Motion)>();
        for (id, mut local, mut motion) in query.iter_mut(&mut self.world) {
            visit(id.0, &mut local.0, &mut motion.0);
        }
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

    pub fn inspect(&self, id: StableId) -> Option<EntitySnapshot> {
        let entity = self.world.get_entity(*self.entities.get(&id)?).ok()?;
        Some(EntitySnapshot {
            id,
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
        for axis in 0..3 {
            local.0.translation[axis] += motion.0.linear[axis] * step.0;
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    query.par_iter_mut().for_each(advance);
    #[cfg(target_arch = "wasm32")]
    query.iter_mut().for_each(advance);
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

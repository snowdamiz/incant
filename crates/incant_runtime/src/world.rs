use crate::physics::{Binding, Spin};
use crate::physics_2d::{Binding2D, Spin2D};
use crate::scene::{matrix, validate_components};
use crate::{CookedEntity, CookedScene, SceneError, StableId};
use bevy_ecs::prelude::*;
use glam::DMat4;
use incant_types::{Transform, Velocity};
use std::collections::{BTreeMap, BTreeSet};
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
    #[error("runtime spatial generation exhausted")]
    GenerationOverflow,
    #[error(transparent)]
    Physics(#[from] incant_physics_runtime::PhysicsError),
    #[error(transparent)]
    Physics2D(#[from] incant_physics_2d_runtime::PhysicsError),
    #[error("native physics world failed; construct a new session")]
    PhysicsFailed,
    #[error(
        "solid tilemap collision requires a native Rapier2D consumer; simulation is unsupported"
    )]
    TilemapPhysicsUnsupported,
    #[error("runtime entity {0:?} does not exist")]
    MissingEntity(StableId),
    #[error("frame command buffer is full (maximum {0})")]
    BufferFull(usize),
}

/// A disposable Bevy world; no project, command bus, authoring validator or
/// serialization in the tick path. Snapshot conversion is explicitly requested.
pub struct NativeWorld {
    pub(crate) identity: crate::WorldIdentity,
    pub(crate) world: World,
    schedule: Schedule,
    pub(crate) physics: incant_physics_runtime::PhysicsRuntime<StableId>,
    pub(crate) physics_2d: incant_physics_2d_runtime::PhysicsWorld2D,
    pub(crate) physics_2d_used: bool,
    pub(crate) physics_failed: bool,
    pub(crate) physics_used: bool,
    pub(crate) spatial_used: bool,
    pub(crate) spatial_ids: BTreeSet<StableId>,
    pub(crate) tilemap_collision_ids: BTreeSet<StableId>,
    pub(crate) visual_assets: Vec<crate::VisualAssetRef>,
    pub(crate) topology_generation: u64,
    pub(crate) binding_generation: u64,
    pub(crate) audio_used: bool,
    pub(crate) audio_ids: BTreeSet<StableId>,
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
        let physics = scene.physics.clone();
        let physics_2d = scene.physics_2d.clone();
        let audio = scene.audio.clone();
        let spatial = scene.spatial;
        let visual_assets = scene.visual_assets;
        let mut solver = incant_physics_runtime::PhysicsRuntime::default();
        let by_id: BTreeMap<_, _> = scene.entities.iter().map(|e| (e.id, e)).collect();
        for declaration in &physics {
            solver.install(
                scene.id,
                declaration.entity,
                declaration.prepare(by_id[&declaration.entity]),
            );
        }
        let mut planar_changes: Vec<_> = physics_2d
            .iter()
            .map(|p| (p.entity, Some(p.prepare(by_id[&p.entity]))))
            .collect();
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
        schedule.add_systems((integrate, publish_globals, crate::flipbook::advance).chain());
        let mut result = Self {
            identity: crate::WorldIdentity::new(),
            world,
            physics: solver,
            physics_2d: Default::default(),
            physics_2d_used: false,
            physics_failed: false,
            physics_used: !physics.is_empty(),
            spatial_used: !spatial.is_empty() || !visual_assets.is_empty(),
            spatial_ids: BTreeSet::new(),
            tilemap_collision_ids: BTreeSet::new(),
            visual_assets,
            topology_generation: 0,
            binding_generation: 0,
            audio_used: !audio.is_empty(),
            audio_ids: BTreeSet::new(),
            schedule,
            entities,
            parents,
            scene_id: scene.id,
            tick: 0,
            tick_rate: scene.tick_rate,
        };
        for node in spatial {
            result.install_spatial(node.entity, Some(node));
        }
        for node in audio {
            result.install_audio(node.entity, Some(node));
        }
        for declaration in physics {
            result.install_physics(declaration);
        }
        for declaration in physics_2d {
            result.install_physics_2d(declaration.entity, Some(declaration), false);
        }
        for id in result.tilemap_collision_ids.clone() {
            let entity = result.world.entity(result.entities[&id]);
            let map = entity.get::<crate::tilemap::Prepared>().unwrap();
            let state = crate::physics_2d::state(
                &entity.get::<Local>().unwrap().0,
                entity.get::<Motion>().map(|m| &m.0),
                None,
                None,
            );
            planar_changes.push((
                id,
                Some(
                    incant_physics_2d_runtime::PreparedBody::tilemap(&map.0, state)
                        .expect("validated map"),
                ),
            ));
            result.install_physics_2d(id, None, true);
        }
        result
            .physics_2d
            .apply(planar_changes)
            .expect("validated planar scene");
        result
    }

    pub(crate) fn install_spatial(&mut self, id: StableId, node: Option<crate::CookedSpatial>) {
        if self.world.get::<crate::CookedSpatial>(self.entities[&id]) == node.as_ref() {
            return;
        }
        let previous_flipbook = self
            .world
            .get::<crate::CookedSpatial>(self.entities[&id])
            .and_then(|n| n.flipbook.as_ref());
        let next_flipbook = node.as_ref().and_then(|n| n.flipbook.as_ref());
        let playback_changed = previous_flipbook != next_flipbook;
        let next_playback = next_flipbook.map(|f| crate::flipbook::Playback {
            clock: f.clock(self.tick_rate),
            state: crate::FlipbookState::new(f.playing),
        });
        let prepared = node
            .as_ref()
            .and_then(|n| n.tilemap.as_ref())
            .map(|map| crate::tilemap::Prepared(map.prepare().expect("validated tilemap")));
        if prepared
            .as_ref()
            .is_some_and(|map| !map.0.collision().is_empty())
        {
            self.tilemap_collision_ids.insert(id);
        } else {
            self.tilemap_collision_ids.remove(&id);
        }
        let mut entity = self.world.entity_mut(self.entities[&id]);
        if playback_changed {
            if let Some(playback) = next_playback {
                entity.insert(playback);
            } else {
                entity.remove::<crate::flipbook::Playback>();
            }
        }
        if let Some(prepared) = prepared {
            entity.insert(prepared);
        } else {
            entity.remove::<crate::tilemap::Prepared>();
        }
        if let Some(node) = node {
            self.spatial_used = true;
            self.spatial_ids.insert(id);
            entity.insert(node);
        } else {
            self.spatial_ids.remove(&id);
            entity.remove::<crate::CookedSpatial>();
        }
    }

    /// Bounded typed audio view over committed ECS values. No document, JSON,
    /// project copy or mixer handle; listeners/sources see derived world poses.
    pub fn audio_nodes(
        &self,
    ) -> impl ExactSizeIterator<Item = (&crate::CookedAudio, [[f64; 4]; 4])> {
        self.audio_ids.iter().map(|id| {
            let handle = self.entities[id];
            (
                self.world
                    .get::<crate::CookedAudio>(handle)
                    .expect("audio node"),
                self.world
                    .get::<Global>(handle)
                    .expect("audio pose")
                    .0
                    .to_cols_array_2d(),
            )
        })
    }

    /// Compose local rotations without inherited scale/shear. Intended for the
    /// single listener: visits its ancestor chain without allocation. Positions
    /// still use the committed world matrix from audio_nodes.
    pub fn world_rotation(&self, id: StableId) -> Option<[f64; 4]> {
        let mut next = Some(id);
        let mut rotation = glam::DQuat::IDENTITY;
        while let Some(id) = next {
            let local = self.world.get::<Local>(*self.entities.get(&id)?)?;
            rotation =
                (glam::DQuat::from_array(local.0.rotation).normalize() * rotation).normalize();
            next = self.parents[&id];
        }
        Some(rotation.to_array())
    }

    pub(crate) fn install_audio(&mut self, id: StableId, node: Option<crate::CookedAudio>) {
        let mut entity = self.world.entity_mut(self.entities[&id]);
        if let Some(node) = node {
            self.audio_used = true;
            self.audio_ids.insert(id);
            entity.insert(node);
        } else {
            self.audio_ids.remove(&id);
            entity.remove::<crate::CookedAudio>();
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
        self.ensure_usable()?;
        let next = self.tick.checked_add(1).ok_or(RuntimeError::TickOverflow)?;
        let dt = self.world.resource::<Step>().0;
        // Validate the whole numeric input set and ordinary integration before
        // the first solver write. No cloned configuration or document exists.
        self.prepare_globals(Some(dt))?;
        if self.physics_used || self.physics_2d_used {
            self.sync_physics_2d()?;
            let mut query = self
                .world
                .query::<(&Identity, &Local, Option<&Motion>, Option<&Spin>, &Binding)>();
            for (id, local, velocity, spin, _) in query.iter(&self.world) {
                let state =
                    crate::physics::state(&local.0, velocity.map(|v| &v.0), spin.map(|s| &s.0));
                // The full set was validated above; this repeats only bounded
                // numeric checks in the standalone solver API, never authoring.
                if let Err(error) = self.physics.sync_motion(&self.scene_id, &id.0, &state) {
                    self.physics_failed = true;
                    return Err(error.into());
                }
            }
            // Every post-advance error or caught unwind makes this world terminal.
            self.physics_failed = true;
            if let Err(error) = self.physics_2d.step(dt) {
                return Err(error.into());
            }
            if let Err(error) = self.physics.step(dt) {
                self.physics_failed = true;
                return Err(error.into());
            }
            // Solver advancement cannot be undone. If candidate hierarchy
            // publication fails, the ECS/clock stay unchanged and this world is
            // terminal. NativeSession also closes access to the failed session.
            if let Err(error) = self.prepare_candidate_globals(Some(dt), true) {
                self.physics_failed = true;
                return Err(error.into());
            }
            for (id, state) in self.physics_2d.states() {
                let mut entity = self.world.entity_mut(self.entities[&id]);
                let rotation = crate::physics_2d::rotation(state.rotation);
                let mut local = entity.get_mut::<Local>().expect("planar transform");
                local.0.translation[..2].copy_from_slice(&state.translation);
                local.0.rotation = rotation;
                if let Some(mut velocity) = entity.get_mut::<Motion>() {
                    velocity.0.linear[..2].copy_from_slice(&state.velocity);
                }
                if let Some(mut spin) = entity.get_mut::<Spin2D>() {
                    spin.0.angular = state.angular_velocity;
                }
                entity.get_mut::<Binding2D>().unwrap().rotation = rotation;
            }
            for (id, state) in self.physics.state_iter() {
                let mut entity = self.world.entity_mut(self.entities[id]);
                let mut local = entity.get_mut::<Local>().expect("physics transform");
                local.0.translation = state.translation;
                local.0.rotation = state.rotation;
                if let Some(mut velocity) = entity.get_mut::<Motion>() {
                    velocity.0.linear = state.velocity;
                }
                if let Some(mut spin) = entity.get_mut::<Spin>() {
                    spin.0.angular = state.angular_velocity;
                }
            }
        }
        self.schedule.run(&mut self.world);
        self.tick = next;
        self.physics_failed = false;
        Ok(())
    }

    pub(crate) fn ensure_usable(&self) -> Result<(), RuntimeError> {
        if self.physics_failed {
            Err(RuntimeError::PhysicsFailed)
        } else {
            Ok(())
        }
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
        self.prepare_candidate_globals(step, false)
    }

    fn prepare_candidate_globals(
        &mut self,
        step: Option<f64>,
        solved: bool,
    ) -> Result<(), SceneError> {
        let physics = &self.physics;
        let physics_2d = &self.physics_2d;
        let scene_id = self.scene_id;
        self.world
            .resource_scope(|world, mut hierarchy: Mut<Hierarchy>| {
                let mut query = world.query::<(
                    &Identity,
                    &Local,
                    Option<&Motion>,
                    Option<&Spin>,
                    Option<&Binding>,
                    Option<&Spin2D>,
                    Option<&Binding2D>,
                )>();
                for index in 0..hierarchy.nodes.len() {
                    let (entity, parent) = hierarchy.nodes[index];
                    let (id, local, motion, spin, binding, spin2d, binding2d) =
                        query.get(world, entity).expect("fixed topology");
                    let velocity = motion.map(|v| &v.0);
                    validate_components(id.0, &local.0, velocity)?;
                    if let Some(binding) = binding {
                        crate::physics::validate_motion(
                            id.0,
                            parent.is_some(),
                            &local.0,
                            velocity,
                            spin.map(|s| &s.0),
                            binding.body.as_ref(),
                        )?;
                    }
                    if let Some(binding) = binding2d {
                        crate::physics_2d::validate_motion(
                            id.0,
                            parent.is_some(),
                            &local.0,
                            velocity,
                            spin2d.map(|s| &s.0),
                            binding.body.as_ref(),
                        )?;
                    }
                    let local_matrix = if solved && binding2d.is_some() {
                        let state = physics_2d.state(id.0).expect("planar binding");
                        let mut candidate = local.0.clone();
                        candidate.translation[..2].copy_from_slice(&state.translation);
                        candidate.rotation = crate::physics_2d::rotation(state.rotation);
                        validate_components(id.0, &candidate, None)?;
                        matrix(&candidate)
                    } else if solved && binding.is_some() {
                        let state = physics.state(&scene_id, &id.0).expect("physics binding");
                        let mut candidate = local.0.clone();
                        candidate.translation = state.translation;
                        candidate.rotation = state.rotation;
                        validate_components(id.0, &candidate, None)?;
                        matrix(&candidate)
                    } else if let (Some(step), Some(velocity), None, None) =
                        (step, velocity, binding, binding2d)
                    {
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

type NonPhysics = (Without<Binding>, Without<Binding2D>);

fn integrate(step: Res<Step>, mut query: Query<(&mut Local, &Motion), NonPhysics>) {
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

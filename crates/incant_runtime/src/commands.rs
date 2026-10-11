//! Runtime-only structural changes, staged and checked before a sync point.
//! This is not the authoring command bus: no journal, provenance or JSON path.
use crate::CookedPhysics;
use crate::physics::{Binding, Spin};
use crate::physics_2d::{Binding2D, Spin2D};
use crate::scene::{MAX_ENTITIES, matrix, validate_values};
use crate::world::{Global, Hierarchy, Local, Motion, spawn};
use crate::{CookedEntity, NativeWorld, RuntimeError, SceneError, StableId};
use incant_physics_runtime::PreparedBody;
use incant_types::{AngularVelocity, AngularVelocity2D, Velocity};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_FRAME_COMMANDS: usize = 65_536;

#[derive(Debug, Clone, PartialEq)]
pub enum StructuralCommand {
    /// Replace committed runtime settings atomically; the catalog stays immutable.
    SetLocale(crate::LocaleSettings),
    /// Replace/remove checked navigation configuration at a committed boundary.
    SetNavigation {
        entity: StableId,
        navigation: Option<Box<crate::CookedNavigation>>,
    },
    Spawn(CookedEntity),
    /// Children must also be removed or reparented in this batch. No implicit
    /// cascading deletion, and no recycling an existing ID within one batch.
    Despawn(StableId),
    /// Keeps the local transform. A parent may be spawned later in this batch.
    SetParent {
        entity: StableId,
        parent: Option<StableId>,
    },
    /// Replace/remove typed audio state; the final bus/listener graph is checked
    /// before any component changes. Mixer/device state belongs to the audio host.
    SetAudio {
        entity: StableId,
        audio: Option<crate::CookedAudio>,
    },
    /// Scalar controls use the final audio declaration in submitted order.
    /// Publication is atomic with all other commands; mixer sync follows later.
    ControlAudio {
        entity: StableId,
        control: crate::AudioControl,
    },
    /// Controls use the final declaration of this batch, in submitted order.
    /// Pause/resume preserves phase; restart/seek resets it.
    ControlFlipbook {
        entity: StableId,
        control: crate::FlipbookControl,
    },
    /// Replace/remove render configuration using the immutable startup asset
    /// table. The final bounded declarations are checked before publication.
    SetSpatial {
        entity: StableId,
        spatial: Option<crate::CookedSpatial>,
    },
    /// Add/replace/remove a checked physics declaration at this boundary. The
    /// nested entity ID must match. Removal retains linear motion as an ordinary
    /// native mover; body installation supplies missing zero motion components.
    SetPhysics {
        entity: StableId,
        physics: Option<CookedPhysics>,
    },
    /// Add/replace/remove planar configuration. Cannot share an entity with
    /// a 3D collider or solid tilemap; final-state validation precedes writes.
    SetPhysics2D {
        entity: StableId,
        physics: Option<crate::CookedPhysics2D>,
    },
    /// Requires a planar rigid body, including when None resets it to zero.
    SetAngularVelocity2D {
        entity: StableId,
        velocity: Option<AngularVelocity2D>,
    },
    /// Physics angular motion is typed and never included in Transform/Velocity
    /// foreign numeric buffers. Requires a rigid body; None resets to zero.
    SetAngularVelocity {
        entity: StableId,
        velocity: Option<AngularVelocity>,
    },
    /// Adds, replaces or removes the actual Bevy motion component. A rigid body
    /// requires Velocity; use a zero vector instead of removing it.
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
    navigation: crate::navigation_runtime::NavigationRuntime,
    parents: BTreeMap<StableId, Option<StableId>>,
    spawns: BTreeMap<StableId, CookedEntity>,
    removed: BTreeSet<StableId>,
    velocities: BTreeMap<StableId, Option<Velocity>>,
    hierarchy: Vec<crate::hierarchy::Node>,
    physics: BTreeMap<StableId, Option<CookedPhysics>>,
    spins: BTreeMap<StableId, Option<AngularVelocity>>,
    prepared: BTreeMap<StableId, Option<PreparedBody>>,
    spins_2d: BTreeMap<StableId, Option<AngularVelocity2D>>,
    prepared_2d: Vec<(StableId, Option<incant_physics_2d_runtime::PreparedBody>)>,
    bindings_2d: BTreeMap<StableId, (Option<crate::CookedPhysics2D>, bool)>,
    audio: BTreeMap<StableId, Option<crate::CookedAudio>>,
    spatial: BTreeMap<StableId, Option<crate::CookedSpatial>>,
    topology_generation: u64,
    binding_generation: u64,
    flipbook_states: BTreeMap<StableId, crate::FlipbookState>,
}

impl NativeWorld {
    /// Atomically applies one batch at a safe boundary. Borrowing `&mut self`
    /// excludes active native component views and running schedule execution.
    /// Validation reads native values only; it never constructs a Project.
    pub fn apply(&mut self, commands: &mut FrameCommands) -> Result<(), RuntimeError> {
        self.ensure_usable()?;
        let localization = self.prepare_locale(commands)?;
        if commands
            .iter()
            .all(|c| matches!(c, StructuralCommand::SetLocale(_)))
            && !commands.is_empty()
        {
            let navigation = self.navigation_motion()?;
            self.localization = localization.expect("locale command");
            if let Some(next) = navigation {
                self.navigation = next;
            }
            commands.clear();
            return Ok(());
        }
        if commands.is_empty() {
            if let Some(next) = self.navigation_motion()? {
                self.navigation = next;
            }
            return Ok(());
        }
        if self.navigation.nodes.is_empty() && commands
            .iter()
            .all(|c| matches!(c, StructuralCommand::SetSpatial { .. }))
            && !commands.iter().any(|c| matches!(c, StructuralCommand::SetSpatial{entity,spatial} if self.tilemap_collision_ids.contains(entity) || spatial.as_ref().is_some_and(|n|n.tilemap.as_ref().is_some_and(|m|m.layers.iter().any(|l|l.solid)))))
        {
            self.apply_spatial(commands)?;
            return Ok(());
        }
        if commands
            .iter()
            .all(|c| matches!(c, StructuralCommand::ControlFlipbook { .. }))
        {
            let states = self.prepare_flipbook_controls(commands, &BTreeMap::new(), |id| {
                self.entities.contains_key(&id)
            })?;
            let navigation = self.navigation_motion()?;
            self.install_flipbook_states(states);
            if let Some(next) = navigation {
                self.navigation = next;
            }
            commands.clear();
            return Ok(());
        }
        // Audio-only changes inspect at most 161 audio nodes; no hierarchy copy.
        if commands.iter().all(|c| {
            matches!(
                c,
                StructuralCommand::SetAudio { .. } | StructuralCommand::ControlAudio { .. }
            )
        }) {
            let mut changes = BTreeMap::new();
            for command in commands.iter() {
                let StructuralCommand::SetAudio { entity, audio } = command else {
                    continue;
                };
                if !self.entities.contains_key(entity) {
                    return Err(RuntimeError::MissingEntity(*entity));
                }
                if audio.as_ref().is_some_and(|a| a.entity != *entity) {
                    return Err(SceneError::Invalid("audio entity reference").into());
                }
                changes.insert(*entity, audio.clone());
            }
            let changes = self
                .prepare_audio_controls(commands, changes, |id| self.entities.contains_key(&id))?;
            self.validate_audio_changes(&changes, &self.parents)?;
            let navigation = self.navigation_motion()?;
            for (id, node) in changes {
                self.install_audio(id, node);
            }
            if let Some(next) = navigation {
                self.navigation = next;
            }
            commands.clear();
            return Ok(());
        }
        // Component-only batches don't copy hierarchy metadata or inspect every
        // entity. In particular, toggling one mover scales with this batch.
        if commands.iter().all(|c| {
            matches!(
                c,
                StructuralCommand::SetLocale(_)
                    | StructuralCommand::SetVelocity { .. }
                    | StructuralCommand::SetAngularVelocity { .. }
                    | StructuralCommand::SetAngularVelocity2D { .. }
                    | StructuralCommand::ControlFlipbook { .. }
                    | StructuralCommand::ControlAudio { .. }
            )
        }) {
            let mut velocities = BTreeMap::new();
            let mut spins = BTreeMap::new();
            let mut spins_2d = BTreeMap::new();
            let mut touched = BTreeSet::new();
            for command in commands.iter() {
                let id = match command {
                    StructuralCommand::SetVelocity { entity, velocity } => {
                        validate_velocity(*entity, velocity)?;
                        velocities.insert(*entity, velocity.clone());
                        *entity
                    }
                    StructuralCommand::SetAngularVelocity { entity, velocity } => {
                        if let Some(velocity) = velocity {
                            incant_types::physics_validation::angular(velocity)
                                .map_err(|_| SceneError::Component(*entity))?;
                        }
                        spins.insert(*entity, velocity.clone());
                        *entity
                    }
                    StructuralCommand::SetAngularVelocity2D { entity, velocity } => {
                        if let Some(v) = velocity {
                            incant_types::physics_2d_validation::angular(v)
                                .map_err(|_| SceneError::Component(*entity))?;
                        }
                        spins_2d.insert(*entity, velocity.clone());
                        *entity
                    }
                    StructuralCommand::SetLocale(_)
                    | StructuralCommand::ControlFlipbook { .. }
                    | StructuralCommand::ControlAudio { .. } => continue,
                    _ => unreachable!(),
                };
                if !self.entities.contains_key(&id) {
                    return Err(RuntimeError::MissingEntity(id));
                }
                touched.insert(id);
            }
            // Resolve all final numeric values before changing a single table.
            // This work scales with touched IDs, not the world's parent graph.
            for id in touched {
                let entity = self.world.entity(self.entities[&id]);
                let binding = entity.get::<Binding>();
                let body = binding.and_then(|b| b.body.as_ref());
                if spins.contains_key(&id) && body.is_none() {
                    return Err(SceneError::Component(id).into());
                }
                let planar = entity.get::<Binding2D>();
                let planar_body = planar.and_then(|p| p.body.as_ref());
                if spins_2d.contains_key(&id) && planar_body.is_none() {
                    return Err(SceneError::Component(id).into());
                }
                if let Some(planar) = planar {
                    let velocity = velocities
                        .get(&id)
                        .map(|v| v.as_ref())
                        .unwrap_or_else(|| entity.get::<Motion>().map(|v| &v.0));
                    let spin = spins_2d
                        .get(&id)
                        .map(|v| v.as_ref())
                        .unwrap_or_else(|| entity.get::<Spin2D>().map(|s| &s.0));
                    if planar_body.is_some() && velocity.is_none() {
                        return Err(SceneError::Component(id).into());
                    }
                    crate::physics_2d::validate_motion(
                        id,
                        self.parents[&id].is_some(),
                        &entity.get::<Local>().unwrap().0,
                        velocity,
                        spin,
                        planar.body.as_ref(),
                    )?;
                }
                if let Some(binding) = binding {
                    let velocity = velocities
                        .get(&id)
                        .map(|v| v.as_ref())
                        .unwrap_or_else(|| entity.get::<Motion>().map(|v| &v.0));
                    let spin = spins
                        .get(&id)
                        .map(|v| v.as_ref())
                        .unwrap_or_else(|| entity.get::<Spin>().map(|s| &s.0));
                    if body.is_some() && velocity.is_none() {
                        return Err(SceneError::Component(id).into());
                    }
                    crate::physics::validate_motion(
                        id,
                        self.parents[&id].is_some(),
                        &entity.get::<Local>().unwrap().0,
                        velocity,
                        spin,
                        binding.body.as_ref(),
                    )?;
                }
            }
            let playback = self.prepare_flipbook_controls(commands, &BTreeMap::new(), |id| {
                self.entities.contains_key(&id)
            })?;
            let audio = self.prepare_audio_controls(commands, BTreeMap::new(), |id| {
                self.entities.contains_key(&id)
            })?;
            if !audio.is_empty() {
                self.validate_audio_changes(&audio, &self.parents)?;
            }
            let navigation = self.navigation.motion(
                |id| self.world.get::<Global>(self.entities[&id]).unwrap().0,
                |id| {
                    velocities
                        .get(&id)
                        .map(|v| v.as_ref())
                        .unwrap_or_else(|| {
                            self.world.get::<Motion>(self.entities[&id]).map(|v| &v.0)
                        })
                        .is_none_or(|v| v.linear == [0.; 3])
                },
            )?;
            for (id, node) in audio {
                self.install_audio(id, node);
            }
            self.install_flipbook_states(playback);
            self.install_velocities(velocities);
            if let Some(next) = navigation {
                self.navigation = next;
            }
            for (id, spin) in spins_2d {
                self.world
                    .entity_mut(self.entities[&id])
                    .insert(Spin2D(spin.unwrap_or(AngularVelocity2D { angular: 0. })));
            }
            for (id, spin) in spins {
                self.world
                    .entity_mut(self.entities[&id])
                    .insert(Spin(spin.unwrap_or(AngularVelocity { angular: [0.; 3] })));
            }
        } else {
            let staged = self.stage(commands)?;
            // The prepared final collection was checked during staging, while
            // all native numeric undo was still owned by any script adapter.
            self.physics_2d
                .apply(staged.prepared_2d)
                .expect("staged planar collection");
            // Everything that can reject external data completed above. These
            // handles/IDs are exclusively owned by this world until commit ends.
            for id in staged.removed {
                self.audio_ids.remove(&id);
                self.spatial_ids.remove(&id);
                self.tilemap_collision_ids.remove(&id);
                self.physics.remove(&self.scene_id(), &id);
                let entity = self.entities.remove(&id).expect("staged entity");
                self.world.despawn(entity);
            }
            for (id, data) in staged.spawns {
                self.entities.insert(id, spawn(&mut self.world, data));
            }
            self.install_velocities(staged.velocities);
            for (id, node) in staged.spatial {
                self.install_spatial(id, node);
            }
            self.install_flipbook_states(staged.flipbook_states);
            for (id, node) in staged.audio {
                self.install_audio(id, node);
            }
            for (id, (declaration, tilemap)) in staged.bindings_2d {
                self.install_physics_2d(id, declaration, tilemap);
            }
            for (id, spin) in staged.spins_2d {
                let mut entity = self.world.entity_mut(self.entities[&id]);
                if entity.get::<Binding2D>().is_some_and(|b| b.body.is_some()) {
                    entity.insert(Spin2D(spin.unwrap_or(AngularVelocity2D { angular: 0. })));
                }
            }
            for (id, declaration) in staged.physics {
                if let Some(declaration) = declaration {
                    self.install_physics(declaration);
                } else {
                    self.world
                        .entity_mut(self.entities[&id])
                        .remove::<(Binding, Spin)>();
                }
            }
            for (id, spin) in staged.spins {
                let mut target = self.world.entity_mut(self.entities[&id]);
                if target.get::<Binding>().is_some_and(|b| b.body.is_some()) {
                    target.insert(Spin(spin.unwrap_or(AngularVelocity { angular: [0.; 3] })));
                }
            }
            for (id, prepared) in staged.prepared {
                if let Some(prepared) = prepared {
                    self.physics.install(self.scene_id(), id, prepared);
                } else {
                    self.physics.remove(&self.scene_id(), &id);
                }
            }
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
            self.navigation_used |= !staged.navigation.nodes.is_empty();
            self.navigation = staged.navigation;
            self.topology_generation = staged.topology_generation;
            self.binding_generation = staged.binding_generation;
            self.parents = staged.parents;
            self.world.insert_resource(hierarchy);
        }
        if let Some(localization) = localization {
            self.localization = localization;
        }
        commands.clear();
        Ok(())
    }

    fn validate_audio_changes(
        &self,
        changes: &BTreeMap<StableId, Option<crate::CookedAudio>>,
        parents: &BTreeMap<StableId, Option<StableId>>,
    ) -> Result<(), SceneError> {
        let mut final_nodes: BTreeMap<_, _> = self
            .audio_nodes()
            .filter(|(node, _)| parents.contains_key(&node.entity))
            .map(|(node, _)| (node.entity, node.clone()))
            .collect();
        for (&id, node) in changes {
            if let Some(node) = node {
                final_nodes.insert(id, node.clone());
            } else {
                final_nodes.remove(&id);
            }
        }
        crate::audio::validate(&final_nodes.into_values().collect::<Vec<_>>(), |id| {
            parents.contains_key(&id)
        })
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
        let mut physics = BTreeMap::new();
        let mut physics_2d = BTreeMap::new();
        let mut spins_2d = BTreeMap::new();
        let mut spins = BTreeMap::new();
        let mut audio = BTreeMap::new();
        let mut spatial = BTreeMap::new();
        let mut navigation_changes = BTreeMap::new();
        for command in commands.iter() {
            match command {
                StructuralCommand::SetNavigation { entity, navigation } => {
                    if !parents.contains_key(entity) {
                        return Err(RuntimeError::MissingEntity(*entity));
                    }
                    if navigation.as_ref().is_some_and(|n| n.entity != *entity) {
                        return Err(SceneError::Invalid("navigation entity reference").into());
                    }
                    navigation_changes.insert(*entity, navigation.as_deref().cloned());
                }
                StructuralCommand::Spawn(data) => {
                    if data.id == self.scene_id()
                        || self
                            .visual_assets
                            .binary_search_by_key(&data.id, |a| a.id)
                            .is_ok()
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
                    physics.remove(id);
                    physics_2d.remove(id);
                    spins_2d.remove(id);
                    spins.remove(id);
                    audio.remove(id);
                    spatial.remove(id);
                    navigation_changes.remove(id);
                }
                StructuralCommand::SetLocale(_)
                | StructuralCommand::ControlFlipbook { .. }
                | StructuralCommand::ControlAudio { .. } => {}
                StructuralCommand::SetParent { entity, parent } => {
                    *parents
                        .get_mut(entity)
                        .ok_or(RuntimeError::MissingEntity(*entity))? = *parent;
                }
                StructuralCommand::SetAudio {
                    entity,
                    audio: node,
                } => {
                    if !parents.contains_key(entity) {
                        return Err(RuntimeError::MissingEntity(*entity));
                    }
                    if node.as_ref().is_some_and(|a| a.entity != *entity) {
                        return Err(SceneError::Invalid("audio entity reference").into());
                    }
                    audio.insert(*entity, node.clone());
                }
                StructuralCommand::SetSpatial {
                    entity,
                    spatial: node,
                } => {
                    if !parents.contains_key(entity) {
                        return Err(RuntimeError::MissingEntity(*entity));
                    }
                    if node.as_ref().is_some_and(|n| n.entity != *entity) {
                        return Err(SceneError::Invalid("spatial entity reference").into());
                    }
                    spatial.insert(*entity, node.clone());
                }
                StructuralCommand::SetPhysics {
                    entity,
                    physics: declaration,
                } => {
                    if !parents.contains_key(entity) {
                        return Err(RuntimeError::MissingEntity(*entity));
                    }
                    if declaration.as_ref().is_some_and(|p| p.entity != *entity) {
                        return Err(SceneError::Invalid("physics entity reference").into());
                    }
                    // Later changes are checked as one final state. No partial
                    // declaration is installed, and no undo is discarded here.
                    spins.remove(entity);
                    physics.insert(*entity, declaration.clone());
                }
                StructuralCommand::SetPhysics2D {
                    entity,
                    physics: declaration,
                } => {
                    if !parents.contains_key(entity) {
                        return Err(RuntimeError::MissingEntity(*entity));
                    }
                    if declaration.as_ref().is_some_and(|p| p.entity != *entity) {
                        return Err(SceneError::Invalid("2D entity reference").into());
                    }
                    spins_2d.remove(entity);
                    physics_2d.insert(*entity, declaration.clone());
                }
                StructuralCommand::SetAngularVelocity2D { entity, velocity } => {
                    if !parents.contains_key(entity) {
                        return Err(RuntimeError::MissingEntity(*entity));
                    }
                    if let Some(v) = velocity {
                        incant_types::physics_2d_validation::angular(v)
                            .map_err(|_| SceneError::Component(*entity))?;
                    }
                    spins_2d.insert(*entity, velocity.clone());
                }
                StructuralCommand::SetAngularVelocity { entity, velocity } => {
                    if !parents.contains_key(entity) {
                        return Err(RuntimeError::MissingEntity(*entity));
                    }
                    if let Some(velocity) = velocity {
                        incant_types::physics_validation::angular(velocity)
                            .map_err(|_| SceneError::Component(*entity))?;
                    }
                    spins.insert(*entity, velocity.clone());
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
        let audio = self.prepare_audio_controls(commands, audio, |id| parents.contains_key(&id))?;
        self.validate_audio_changes(&audio, &parents)?;
        let binding_generation = self.validate_spatial_changes(&spatial, &parents)?;
        let flipbook_states =
            self.prepare_flipbook_controls(commands, &spatial, |id| parents.contains_key(&id))?;
        let topology_generation = if parents != self.parents {
            self.topology_generation
                .checked_add(1)
                .ok_or(RuntimeError::GenerationOverflow)?
        } else {
            self.topology_generation
        };
        let mut prepared = BTreeMap::new();
        let mut prepared_2d: Vec<_> = removed.iter().map(|&id| (id, None)).collect();
        let mut bindings_2d = BTreeMap::new();
        for (&id, parent) in &parents {
            let live = self
                .entities
                .get(&id)
                .map(|&handle| self.world.entity(handle));
            let data = spawns.get(&id);
            let transform = data
                .map(|e| &e.transform)
                .unwrap_or_else(|| &live.as_ref().unwrap().get::<Local>().unwrap().0);
            let velocity = velocities.get(&id).map(|v| v.as_ref()).unwrap_or_else(|| {
                data.map(|e| e.velocity.as_ref())
                    .unwrap_or_else(|| live.as_ref().unwrap().get::<Motion>().map(|v| &v.0))
            });
            let binding = live.as_ref().and_then(|e| e.get::<Binding>());
            let changed = physics.get(&id);
            let body = match changed {
                Some(Some(p)) => p.body.as_ref(),
                Some(None) => None,
                None => binding.and_then(|b| b.body.as_ref()),
            };
            let has_physics = changed.map_or(binding.is_some(), |p| p.is_some());
            let spin = spins
                .get(&id)
                .map(|v| v.as_ref())
                .unwrap_or_else(|| match changed {
                    Some(Some(p)) => p.angular_velocity.as_ref(),
                    Some(None) => None,
                    None => live.as_ref().and_then(|e| e.get::<Spin>()).map(|s| &s.0),
                });
            let old_planar = live.as_ref().and_then(|e| e.get::<Binding2D>());
            let changed_planar = physics_2d.get(&id);
            let has_planar = changed_planar
                .map_or(old_planar.is_some_and(|b| b.collider.is_some()), |p| {
                    p.is_some()
                });
            let planar_body = match changed_planar {
                Some(Some(p)) => p.body.as_ref(),
                Some(None) => None,
                None => old_planar.and_then(|b| b.body.as_ref()),
            };
            let planar_spin =
                spins_2d
                    .get(&id)
                    .map(|v| v.as_ref())
                    .unwrap_or_else(|| match changed_planar {
                        Some(Some(p)) => p.angular_velocity.as_ref(),
                        Some(None) => None,
                        None => live.as_ref().and_then(|e| e.get::<Spin2D>()).map(|s| &s.0),
                    });
            let changed_map = spatial.get(&id);
            let next_node = changed_map
                .map(|n| n.as_ref())
                .unwrap_or_else(|| live.as_ref().and_then(|e| e.get::<crate::CookedSpatial>()));
            let prepared_map = if changed_map.is_some() {
                next_node
                    .and_then(|n| n.tilemap.as_ref())
                    .map(|m| m.prepare().expect("validated spatial graph"))
            } else {
                None
            };
            let map = if changed_map.is_some() {
                prepared_map.as_ref()
            } else {
                live.as_ref()
                    .and_then(|e| e.get::<crate::tilemap::Prepared>())
                    .map(|p| &p.0)
            };
            let solid = map.is_some_and(|m| !m.collision().is_empty());
            if (has_planar && solid) || (has_physics && (has_planar || solid)) {
                return Err(SceneError::Invalid("mixed 2D/3D/tile collision bindings").into());
            }
            if spins_2d.contains_key(&id) && planar_body.is_none() {
                return Err(SceneError::Component(id).into());
            }
            if has_planar || solid {
                if planar_body.is_some() && velocities.get(&id).is_some_and(Option::is_none) {
                    return Err(SceneError::Component(id).into());
                }
                crate::physics_2d::validate_motion(
                    id,
                    parent.is_some(),
                    transform,
                    velocity,
                    planar_spin,
                    planar_body,
                )?;
            }
            if changed_planar.is_some()
                || (changed_map.is_some()
                    && (solid || old_planar.is_some_and(|b| b.collider.is_none())))
            {
                let entity = CookedEntity {
                    id,
                    parent: *parent,
                    transform: transform.clone(),
                    velocity: velocity.cloned(),
                };
                let mut declaration = changed_planar.cloned().flatten();
                let value = if let Some(declaration) = &mut declaration {
                    declaration.angular_velocity = planar_spin.cloned();
                    declaration.validate(&entity)?;
                    let previous = old_planar.and_then(|binding| {
                        self.physics_2d.state(id).map(|state| (binding, state))
                    });
                    let state =
                        crate::physics_2d::state(transform, velocity, planar_spin, previous);
                    Some(
                        incant_physics_2d_runtime::PreparedBody::new(
                            declaration.body.clone(),
                            declaration.collider.clone(),
                            state,
                        )
                        .expect("validated planar declaration"),
                    )
                } else if solid {
                    let previous = old_planar.and_then(|binding| {
                        self.physics_2d.state(id).map(|state| (binding, state))
                    });
                    let state = crate::physics_2d::state(transform, velocity, None, previous);
                    Some(incant_physics_2d_runtime::PreparedBody::tilemap(
                        map.unwrap(),
                        state,
                    )?)
                } else {
                    None
                };
                bindings_2d.insert(id, (declaration, solid));
                prepared_2d.push((id, value));
            }
            if spins.contains_key(&id) && body.is_none() {
                return Err(SceneError::Component(id).into());
            }
            if has_physics {
                if body.is_some() && velocities.get(&id).is_some_and(|v| v.is_none()) {
                    return Err(SceneError::Component(id).into());
                }
                crate::scene::validate_components(id, transform, velocity)?;
                crate::physics::validate_motion(
                    id,
                    parent.is_some(),
                    transform,
                    velocity,
                    spin,
                    body,
                )?;
            }
            if let Some(change) = changed {
                let value = if let Some(declaration) = change {
                    let mut declaration = declaration.clone();
                    declaration.angular_velocity = spin.cloned();
                    let entity = CookedEntity {
                        id,
                        parent: *parent,
                        transform: transform.clone(),
                        velocity: velocity.cloned(),
                    };
                    declaration.validate(&entity)?;
                    Some(declaration.prepare(&entity))
                } else {
                    None
                };
                prepared.insert(id, value);
            }
        }
        self.physics_2d.validate_changes(&prepared_2d)?;
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
        let mut navigation_nodes: Vec<_> = self
            .navigation
            .nodes
            .iter()
            .filter(|n| {
                parents.contains_key(&n.entity) && !navigation_changes.contains_key(&n.entity)
            })
            .map(|n| n.as_ref().clone())
            .chain(navigation_changes.values().filter_map(|n| n.clone()))
            .collect();
        crate::navigation::canonicalize(&mut navigation_nodes)?;
        let globals: BTreeMap<_, _> = if navigation_nodes.is_empty() {
            BTreeMap::new()
        } else {
            hierarchy.iter().map(|n| (n.id, n.world)).collect()
        };
        let navigation = self.navigation.prepare(
            navigation_nodes
                .into_iter()
                .map(std::sync::Arc::new)
                .collect(),
            |id| {
                let parent = *parents.get(&id)?;
                let live = self.entities.get(&id).map(|&e| self.world.entity(e));
                let spawned = spawns.get(&id);
                let velocity = velocities.get(&id).map(|v| v.as_ref()).unwrap_or_else(|| {
                    spawned.map(|e| e.velocity.as_ref()).unwrap_or_else(|| {
                        live.as_ref().and_then(|e| e.get::<Motion>()).map(|m| &m.0)
                    })
                });
                let previous = live.as_ref().and_then(|e| e.get::<Binding>());
                let collider = physics
                    .get(&id)
                    .map(|v| v.as_ref().map(|p| &p.collider))
                    .unwrap_or_else(|| previous.map(|p| &p.collider));
                let body = physics
                    .get(&id)
                    .map(|v| v.as_ref().and_then(|p| p.body.as_ref()))
                    .unwrap_or_else(|| previous.and_then(|p| p.body.as_ref()));
                let planar = physics_2d
                    .get(&id)
                    .map(|v| v.as_ref().and_then(|p| p.body.as_ref()))
                    .unwrap_or_else(|| {
                        live.as_ref()
                            .and_then(|e| e.get::<Binding2D>())
                            .and_then(|b| b.body.as_ref())
                    });
                let node = spatial
                    .get(&id)
                    .map(|v| v.as_ref())
                    .unwrap_or_else(|| live.as_ref().and_then(|e| e.get::<crate::CookedSpatial>()));
                let mesh = node
                    .and_then(|n| n.mesh.as_ref())
                    .and_then(|m| {
                        self.visual_assets
                            .binary_search_by_key(&m.mesh, |a| a.id)
                            .ok()
                    })
                    .map(|i| self.visual_assets[i]);
                Some(crate::navigation_runtime::Input {
                    parent,
                    world: globals[&id],
                    stationary: velocity.is_none_or(|v| v.linear == [0.; 3])
                        && body.is_none_or(|b| b.motion == incant_types::BodyMotion::Fixed)
                        && planar.is_none_or(|b| b.motion == incant_types::BodyMotion::Fixed),
                    collider,
                    mesh,
                })
            },
        )?;
        Ok(Staged {
            navigation,
            parents,
            spawns,
            removed,
            velocities,
            hierarchy,
            physics,
            spins,
            prepared,
            prepared_2d,
            bindings_2d,
            spins_2d,
            audio,
            spatial,
            topology_generation,
            binding_generation,
            flipbook_states,
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

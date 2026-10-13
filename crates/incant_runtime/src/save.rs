//! Explicit bounded world checkpoints, never the tick path. Hashes detect
//! corruption, not origin or authorization. VM/host state is a separate boundary.
#[cfg(feature = "native-checkpoints")]
mod checkpoint;
mod codec;
mod graph_fade;
mod ik;
mod root_motion;
use crate::flipbook::Playback;
use crate::world::{Local, Motion};
use crate::{CookedEntity, CookedScene, CookedSpatial, NativeWorld, SceneError};
#[cfg(feature = "native-checkpoints")]
pub use checkpoint::{WorldCheckpoint, WorldCheckpointError};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub use codec::MAX_SAVE_BYTES;
#[derive(Debug, Error, PartialEq)]
pub enum SaveError {
    #[error("UI declarations require a bound v7 save and a separate hosted widget checkpoint")]
    UiUnsupported,
    #[error("text bindings are unsupported in an unbound legacy save")]
    TextUnsupported,
    #[error("localization is unsupported in a legacy v1/v2 save")]
    LocalizationUnsupported,
    #[error("invalid native world save: {0}")]
    Invalid(&'static str),
    #[error("unsupported native world save version {0}")]
    Version(u32),
    #[error("native world save integrity check failed")]
    Integrity,
    #[error(transparent)]
    Scene(#[from] SceneError),
    #[error("native world save belongs to a different scene, tick rate or original cooked binding")]
    WorldMismatch,
    #[error("physics world save/restore is not supported")]
    PhysicsUnsupported,
    #[error("audio playback cursor save/restore is not supported")]
    AudioUnsupported,
    #[error("navigation is unsupported in a legacy v1 save")]
    NavigationUnsupported,
    #[error("spatial configuration is unsupported in a legacy v1 save")]
    SpatialUnsupported,
    #[error("navigation has uncommitted numeric changes; apply or step before saving")]
    UncommittedNavigation,
}

impl NativeWorld {
    /// Checkpoint committed components, hierarchy, spatial/asset declarations,
    /// exact flipbook fractions, animation clocks/flags, locale settings/catalog and
    /// the fixed-step clock. Navigation geometry is
    /// saved; bounded query caches are rebuilt on load. No asset files are read.
    ///
    /// V7 includes UI declarations, not mutable hosted widget state. Composed
    /// sessions must separately include every widget buffer, selection and focus.
    /// This is NOT a game-session/VM checkpoint: input, timers, callbacks, script
    /// heap and external FrameCommands are not included. Hosts must discard old
    /// pending commands/path results and explicitly restore their own state.
    /// Physics-used worlds refuse, even after removing their declarations.
    /// V8 audio declarations are data only: a composed session must additionally
    /// save its exact retained mixer owner; world bytes never resume audio cursors.
    /// Dirty navigation refuses until committed apply/step. Numeric loans cannot
    /// overlap this shared borrow. Saving never mutates the running world.
    pub fn save_bytes(&self) -> Result<Vec<u8>, SaveError> {
        self.check_save_capabilities()?;
        self.encode_components()
    }

    // The opaque native checkpoint calls this only after checking its separate
    // solver owners. Public byte saves retain the physics refusal above.
    fn encode_components(&self) -> Result<Vec<u8>, SaveError> {
        self.ensure_navigation_current()
            .map_err(|_| SaveError::UncommittedNavigation)?;
        let entities = self
            .entities
            .iter()
            .map(|(&id, &handle)| CookedEntity {
                id,
                parent: self.parents[&id],
                transform: self
                    .world
                    .get::<Local>(handle)
                    .expect("live entity")
                    .0
                    .clone(),
                velocity: self.world.get::<Motion>(handle).map(|v| v.0.clone()),
            })
            .collect();
        let spatial = self
            .spatial_ids
            .iter()
            .map(|id| {
                self.world
                    .get::<CookedSpatial>(self.entities[id])
                    .expect("spatial index")
                    .clone()
            })
            .collect();
        let scene = CookedScene::new(self.scene_id(), self.tick_rate, entities)?
            .with_physics(
                self.entities
                    .keys()
                    .filter_map(|&id| self.physics_configuration(id))
                    .collect(),
            )?
            .with_physics_2d(
                self.entities
                    .keys()
                    .filter_map(|&id| self.physics_2d_configuration(id))
                    .collect(),
            )?
            .with_audio(self.audio_nodes().map(|(node, _)| node.clone()).collect())?
            .with_localization(self.localization.clone())?
            .with_spatial(self.visual_assets.clone(), spatial)?
            .with_navigation(
                self.navigation
                    .nodes
                    .iter()
                    .map(|n| (**n).clone())
                    .collect(),
            )?
            .with_text(
                self.font_assets.clone(),
                self.text_ids
                    .iter()
                    .map(|&id| crate::CookedText {
                        entity: id,
                        text: self
                            .world
                            .get::<crate::text::TextComponent>(self.entities[&id])
                            .expect("text index")
                            .0
                            .clone(),
                    })
                    .collect(),
            )?
            .with_ui(
                self.ui
                    .iter()
                    .map(|(&entity, node)| crate::CookedUi {
                        entity,
                        node: std::sync::Arc::clone(node),
                    })
                    .collect(),
            )?;
        // Stable-ID order is independent of ECS table/archetype insertion order.
        let states = self
            .entities
            .iter()
            .filter_map(|(&id, &handle)| {
                self.world
                    .get::<Playback>(handle)
                    .map(|p| (id, p.state.checkpoint()))
            })
            .collect::<Vec<_>>();
        let animations = self
            .entities
            .iter()
            .filter_map(|(&id, &handle)| {
                self.world
                    .get::<crate::animation::Playback>(handle)
                    .map(|p| (id, p.state))
            })
            .collect::<Vec<_>>();
        let graphs = self
            .entities
            .iter()
            .filter_map(|(&id, &handle)| {
                self.world
                    .get::<crate::animation::Playback>(handle)
                    .and_then(|p| p.graph.map(|g| (id, g)))
            })
            .collect::<Vec<_>>();
        self.validate_ik_catalog(&self.ik_catalog)?;
        self.validate_root_catalog(&self.root_catalog)?;
        let roots = self
            .root_ids
            .iter()
            .map(|&id| {
                let handle = self.entities[&id];
                let state = *self
                    .world
                    .get::<crate::root_motion::RootMotionState>(handle)
                    .expect("root state");
                state.validate(
                    self.world
                        .get::<crate::animation::Playback>(handle)
                        .expect("root animation"),
                    self.tick,
                )?;
                Ok((id, state))
            })
            .collect::<Result<Vec<_>, SceneError>>()?;
        let mut ik = Vec::new();
        self.snapshot_ik(&mut ik)
            .map_err(|_| SaveError::Invalid("IK snapshot"))?;
        codec::encode(
            self.tick,
            self.source_scene_fingerprint,
            &scene.to_bytes(),
            &states,
            &animations,
            &graphs,
            self.ui_used,
            self.audio_used,
            &ik,
            &roots,
        )
    }

    /// Load validated data without source/project/asset-cache access. V2–v7 retain
    /// the original cooked-scene fingerprint for later program pairing; it is not
    /// authentication and does not restore a VM. Legacy v1 has no original binding
    /// field: its fingerprint describes only the saved plain scene. V3 adds localized
    /// scenes, v4 adds animation playback, v5 adds active crossfades, and v6 adds exact graph state; older saves cannot silently clear
    /// an existing locale/catalog. V7 additionally retains committed UI
    /// declarations and sticky UI history; widget buffers/focus are separate.
    pub fn from_save_bytes(bytes: &[u8]) -> Result<Self, SaveError> {
        let decoded = codec::decode(bytes)?;
        let scene = CookedScene::from_bytes(decoded.scene)?;
        Self::from_decoded_components(decoded, scene, None)
    }

    // This is not a byte admission API. The other caller owns an authenticated
    // in-memory checkpoint and supplies both complete native owners.
    fn from_decoded_components(
        decoded: codec::Decoded<'_>,
        scene: CookedScene,
        owners: Option<crate::world::NativePhysicsOwners>,
    ) -> Result<Self, SaveError> {
        if decoded.version < 3 && !scene.localization().is_default() {
            return Err(SaveError::LocalizationUnsupported);
        }
        if decoded.origin.is_none() {
            if scene.has_text() {
                return Err(SaveError::TextUnsupported);
            }
            if !scene.navigation().is_empty() {
                return Err(SaveError::NavigationUnsupported);
            }
            if !scene.spatial().is_empty() || !scene.visual_assets().is_empty() {
                return Err(SaveError::SpatialUnsupported);
            }
        }
        if owners.is_none() {
            check_scene_capabilities(&scene)?;
        }
        if decoded.version < 8 && !scene.audio().is_empty() {
            return Err(SaveError::AudioUnsupported);
        }
        if decoded.version < 7 && scene.has_ui() {
            return Err(SaveError::UiUnsupported);
        }
        // Exact membership is required: no missing/extra/duplicate playback row
        // may silently install a default cursor after a valid nested scene load.
        let expected = scene
            .spatial()
            .iter()
            .filter(|n| n.flipbook.is_some())
            .count();
        if decoded.states.len() != expected * codec::STATE_BYTES {
            return Err(SaveError::Invalid("flipbook membership"));
        }
        let expected_animations = scene
            .spatial()
            .iter()
            .filter(|n| n.animation.is_some())
            .count();
        if decoded.animations.len()
            != expected_animations * codec::animation_stride(decoded.version)
        {
            return Err(SaveError::Invalid("animation membership"));
        }
        let expected_graphs = scene
            .spatial()
            .iter()
            .filter(|n| n.animation.as_ref().is_some_and(|a| a.graph.is_some()))
            .count();
        if decoded.graphs.len() != expected_graphs * codec::graph_stride(decoded.version) {
            return Err(SaveError::Invalid("animation graph membership"));
        }
        let ik_states = ik::decode(&scene, decoded.ik)?;
        let mut world = Self::with_physics_owners(scene, owners);
        world.install_ik_states(ik_states);
        for row in decoded.states.as_chunks::<{ codec::STATE_BYTES }>().0 {
            let (id, checkpoint) = codec::state(row)?;
            let entity = *world
                .entities
                .get(&id)
                .ok_or(SaveError::Invalid("flipbook entity"))?;
            let mut playback = world
                .world
                .get_mut::<Playback>(entity)
                .ok_or(SaveError::Invalid("flipbook component"))?;
            playback.state = playback
                .clock
                .restore(checkpoint)
                .map_err(|_| SaveError::Invalid("flipbook checkpoint"))?;
        }
        for row in decoded
            .animations
            .chunks_exact(codec::animation_stride(decoded.version))
        {
            let (id, state) = codec::animation(row)?;
            let entity = *world
                .entities
                .get(&id)
                .ok_or(SaveError::Invalid("animation entity"))?;
            let mut playback = world
                .world
                .get_mut::<crate::animation::Playback>(entity)
                .ok_or(SaveError::Invalid("animation component"))?;
            playback
                .definition
                .validate_state(state)
                .map_err(|_| SaveError::Invalid("animation checkpoint"))?;
            playback.state = state;
        }
        for row in decoded
            .graphs
            .chunks_exact(codec::graph_stride(decoded.version))
        {
            let (id, graph) = codec::graph(row)?;
            let entity = *world
                .entities
                .get(&id)
                .ok_or(SaveError::Invalid("animation graph entity"))?;
            let mut playback = world
                .world
                .get_mut::<crate::animation::Playback>(entity)
                .ok_or(SaveError::Invalid("animation graph component"))?;
            let definition = playback
                .definition
                .graph
                .as_ref()
                .ok_or(SaveError::Invalid("animation graph definition"))?;
            definition
                .validate_state(&graph, playback.state, &playback.definition)
                .map_err(|_| SaveError::Invalid("animation graph checkpoint"))?;
            playback.graph = Some(graph);
        }
        root_motion::install(&mut world, decoded.roots, decoded.tick)?;
        world.tick = decoded.tick;
        world.ui_used |= decoded.ui_used;
        world.audio_used |= decoded.audio_used;
        if let Some(origin) = decoded.origin {
            world.source_scene_fingerprint = origin;
        }
        Ok(world)
    }

    /// Load against the trusted original cooked scene without constructing a
    /// second ECS world. Origin, scene/rate and immutable localization tables
    /// must match; saved mutable locale settings and gameplay state may differ.
    /// These are compatibility checks, not authentication of save provenance.
    pub fn from_save_bytes_for_scene(
        bytes: &[u8],
        initial: &CookedScene,
    ) -> Result<Self, SaveError> {
        check_scene_capabilities(initial)?;
        let mut candidate = Self::from_save_bytes(bytes)?;
        if !initial.audio().is_empty() && !candidate.audio_used {
            return Err(SaveError::AudioUnsupported);
        }
        if initial.has_ui() && !candidate.ui_used {
            return Err(SaveError::UiUnsupported);
        }
        if !codec::is_bound(bytes) {
            return Err(SaveError::Invalid("original scene binding required"));
        }
        candidate.check_save_binding(
            Some(Sha256::digest(initial.to_bytes()).into()),
            initial.id(),
            initial.tick_rate(),
            initial.localization(),
            initial.font_assets(),
        )?;
        let catalog: std::sync::Arc<[_]> = initial
            .spatial()
            .iter()
            .filter_map(|n| n.ik.clone())
            .collect();
        candidate.validate_ik_catalog(&catalog)?;
        candidate.ik_catalog = catalog;
        let roots: std::sync::Arc<[_]> = initial
            .spatial()
            .iter()
            .filter_map(|n| n.root_motion.as_ref().map(|r| r.catalog.clone()))
            .collect();
        candidate.validate_root_catalog(&roots)?;
        candidate.root_catalog = roots;
        Ok(candidate)
    }

    /// Read-only compatibility check for a separately loaded candidate. Checks
    /// sticky unsupported capabilities on both worlds and exact immutable
    /// bindings, without copying a world or sharing its host identity.
    pub fn validate_save_compatibility(&self, candidate: &Self) -> Result<(), SaveError> {
        self.check_save_capabilities()?;
        candidate.check_save_capabilities()?;
        candidate.validate_ik_catalog(&self.ik_catalog)?;
        candidate.validate_root_catalog(&self.root_catalog)?;
        if self.audio_used && !candidate.audio_used {
            return Err(SaveError::AudioUnsupported);
        }
        if self.ui_used && !candidate.ui_used {
            return Err(SaveError::UiUnsupported);
        }
        candidate.check_save_binding(
            Some(self.source_scene_fingerprint),
            self.scene_id(),
            self.tick_rate,
            &self.localization,
            &self.font_assets,
        )
    }

    fn check_save_binding(
        &self,
        origin: Option<[u8; 32]>,
        scene: crate::StableId,
        rate: u32,
        localization: &crate::NativeLocalization,
        font_assets: &[crate::FontAssetRef],
    ) -> Result<(), SaveError> {
        if self.font_assets != font_assets
            || self.localization.tables() != localization.tables()
            || self.scene_id() != scene
            || self.tick_rate != rate
            || origin.is_some_and(|origin| self.source_scene_fingerprint != origin)
        {
            return Err(SaveError::WorldMismatch);
        }
        Ok(())
    }

    /// Atomic replacement after complete decoding/validation. Retains the host
    /// WorldIdentity, changes WorldIncarnation/entity identities, and advances
    /// spatial/navigation generations. Previous borrowed views cannot overlap
    /// this exclusive borrow; owned path results and external command queues must
    /// be discarded by their host. No callback/VM/solver state is resumed here.
    pub fn restore_bytes(&mut self, bytes: &[u8]) -> Result<(), SaveError> {
        self.check_save_capabilities()?;
        let mut candidate = Self::from_save_bytes(bytes)?;
        // The complete candidate decode validated framing and the supported
        // version. Both v3 and v4 carry the localization contract.
        let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        if self.audio_used && version < 8 {
            return Err(SaveError::AudioUnsupported);
        }
        if self.ui_used && version < 7 {
            return Err(SaveError::UiUnsupported);
        }
        if version < 3 && !self.localization.is_default() {
            return Err(SaveError::LocalizationUnsupported);
        }
        // Legacy data has no extended-state/origin contract. Do not let a plain
        // v1 payload silently clear a world's spatial/navigation capability.
        if !codec::is_bound(bytes) {
            if !self.font_assets.is_empty() || !self.text_ids.is_empty() {
                return Err(SaveError::TextUnsupported);
            }
            if self.navigation_used {
                return Err(SaveError::NavigationUnsupported);
            }
            if self.spatial_used {
                return Err(SaveError::SpatialUnsupported);
            }
        }
        candidate.check_save_binding(
            codec::is_bound(bytes).then_some(self.source_scene_fingerprint),
            self.scene_id(),
            self.tick_rate,
            &self.localization,
            &self.font_assets,
        )?;
        candidate.validate_ik_catalog(&self.ik_catalog)?;
        candidate.validate_root_catalog(&self.root_catalog)?;
        candidate.ik_catalog = self.ik_catalog.clone();
        candidate.root_catalog = self.root_catalog.clone();
        // V1 never carried the origin fingerprint. A matching live destination
        // retains its own known origin; the restored data cannot replace it.
        candidate.source_scene_fingerprint = self.source_scene_fingerprint;
        self.publish_restored_components(candidate)
    }

    // All compatibility and owner checks precede this final atomic publication.
    fn publish_restored_components(&mut self, mut candidate: Self) -> Result<(), SaveError> {
        candidate.topology_generation = self
            .topology_generation
            .checked_add(1)
            .ok_or(SaveError::Invalid("spatial generation exhausted"))?;
        candidate.binding_generation = self
            .binding_generation
            .checked_add(1)
            .ok_or(SaveError::Invalid("spatial generation exhausted"))?;
        candidate
            .navigation
            .invalidate_after_restore(&self.navigation)?;
        candidate.text_generation = self
            .text_generation
            .checked_add(1)
            .ok_or(SaveError::Invalid("text generation exhausted"))?;
        candidate.ui_generation = self
            .ui_generation
            .checked_add(1)
            .ok_or(SaveError::Invalid("UI generation exhausted"))?;
        candidate.identity = self.identity.clone();
        *self = candidate;
        Ok(())
    }

    /// A composed save must also capture its retained UI widget owner, even
    /// after the last UI declaration was removed. World bytes alone do not
    /// represent text edits, selection, focus or any native input context.
    pub fn requires_ui_checkpoint(&self) -> bool {
        self.ui_used
    }

    /// Audio declarations/history require a separate retained mixer checkpoint.
    pub fn requires_audio_checkpoint(&self) -> bool {
        self.audio_used
    }
    fn check_save_capabilities(&self) -> Result<(), SaveError> {
        if self.physics_failed
            || self.physics_used
            || self.physics_2d_used
            || self.physics_save_required
        {
            return Err(SaveError::PhysicsUnsupported);
        }
        Ok(())
    }
}

fn check_scene_capabilities(scene: &CookedScene) -> Result<(), SaveError> {
    if !scene.physics().is_empty()
        || !scene.physics_2d().is_empty()
        || scene
            .spatial()
            .iter()
            .any(|node| node.tilemap.as_ref().is_some_and(|m| m.has_collision()))
    {
        return Err(SaveError::PhysicsUnsupported);
    }
    Ok(())
}

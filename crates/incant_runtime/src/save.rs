//! Explicit bounded world checkpoints, never the tick path. Hashes detect
//! corruption, not origin or authorization. VM/host state is a separate boundary.
mod codec;
use crate::flipbook::Playback;
use crate::world::{Local, Motion};
use crate::{CookedEntity, CookedScene, CookedSpatial, NativeWorld, SceneError};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub use codec::MAX_SAVE_BYTES;
#[derive(Debug, Error, PartialEq)]
pub enum SaveError {
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
    /// This is NOT a game-session/VM checkpoint: input, timers, callbacks, script
    /// heap and external FrameCommands are not included. Hosts must discard old
    /// pending commands/path results and explicitly restore their own state.
    /// Physics/audio-used worlds refuse, even after removing their declarations;
    /// contacts, sleep, trigger history and audio cursors must never reset silently.
    /// Dirty navigation refuses until committed apply/step. Numeric loans cannot
    /// overlap this shared borrow. Saving never mutates the running world.
    pub fn save_bytes(&self) -> Result<Vec<u8>, SaveError> {
        self.check_save_capabilities()?;
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
            .with_localization(self.localization.clone())?
            .with_spatial(self.visual_assets.clone(), spatial)?
            .with_navigation(
                self.navigation
                    .nodes
                    .iter()
                    .map(|n| (**n).clone())
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
        codec::encode(
            self.tick,
            self.source_scene_fingerprint,
            &scene.to_bytes(),
            &states,
            &animations,
        )
    }

    /// Load validated data without source/project/asset-cache access. V2/v3 retain
    /// the original cooked-scene fingerprint for later program pairing; it is not
    /// authentication and does not restore a VM. Legacy v1 has no original binding
    /// field: its fingerprint describes only the saved plain scene. V3 adds localized
    /// scenes; older saves cannot silently clear an existing locale/catalog.
    pub fn from_save_bytes(bytes: &[u8]) -> Result<Self, SaveError> {
        let decoded = codec::decode(bytes)?;
        let scene = CookedScene::from_bytes(decoded.scene)?;
        if decoded.version < 3 && !scene.localization().is_default() {
            return Err(SaveError::LocalizationUnsupported);
        }
        if decoded.origin.is_none() {
            if !scene.navigation().is_empty() {
                return Err(SaveError::NavigationUnsupported);
            }
            if !scene.spatial().is_empty() || !scene.visual_assets().is_empty() {
                return Err(SaveError::SpatialUnsupported);
            }
        }
        check_scene_capabilities(&scene)?;
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
        if decoded.animations.len() != expected_animations * codec::ANIMATION_BYTES {
            return Err(SaveError::Invalid("animation membership"));
        }
        let mut world = Self::new(scene);
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
            .as_chunks::<{ codec::ANIMATION_BYTES }>()
            .0
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
        world.tick = decoded.tick;
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
        let candidate = Self::from_save_bytes(bytes)?;
        if !codec::is_bound(bytes) {
            return Err(SaveError::Invalid("original scene binding required"));
        }
        candidate.check_save_binding(
            Some(Sha256::digest(initial.to_bytes()).into()),
            initial.id(),
            initial.tick_rate(),
            initial.localization(),
        )?;
        Ok(candidate)
    }

    /// Read-only compatibility check for a separately loaded candidate. Checks
    /// sticky unsupported capabilities on both worlds and exact immutable
    /// bindings, without copying a world or sharing its host identity.
    pub fn validate_save_compatibility(&self, candidate: &Self) -> Result<(), SaveError> {
        self.check_save_capabilities()?;
        candidate.check_save_capabilities()?;
        candidate.check_save_binding(
            Some(self.source_scene_fingerprint),
            self.scene_id(),
            self.tick_rate,
            &self.localization,
        )
    }

    fn check_save_binding(
        &self,
        origin: Option<[u8; 32]>,
        scene: crate::StableId,
        rate: u32,
        localization: &crate::NativeLocalization,
    ) -> Result<(), SaveError> {
        if self.localization.tables() != localization.tables()
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
        if bytes[8..12] != 3u32.to_le_bytes() && !self.localization.is_default() {
            return Err(SaveError::LocalizationUnsupported);
        }
        // Legacy data has no extended-state/origin contract. Do not let a plain
        // v1 payload silently clear a world's spatial/navigation capability.
        if !codec::is_bound(bytes) {
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
        )?;
        // V1 never carried the origin fingerprint. A matching live destination
        // retains its own known origin; the restored data cannot replace it.
        candidate.source_scene_fingerprint = self.source_scene_fingerprint;
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
        candidate.identity = self.identity.clone();
        *self = candidate;
        Ok(())
    }

    fn check_save_capabilities(&self) -> Result<(), SaveError> {
        if self.audio_used {
            return Err(SaveError::AudioUnsupported);
        }
        if self.physics_failed || self.physics_used || self.physics_2d_used {
            return Err(SaveError::PhysicsUnsupported);
        }
        Ok(())
    }
}

fn check_scene_capabilities(scene: &CookedScene) -> Result<(), SaveError> {
    if !scene.audio().is_empty() {
        return Err(SaveError::AudioUnsupported);
    }
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

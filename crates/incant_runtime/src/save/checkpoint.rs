//! Trusted in-memory composition of ECS components and complete native solvers.
//! This module never admits native physics from caller-provided serialized bytes.
mod validation;
use super::{SaveError, codec};
use crate::{CookedScene, NativeWorld, StableId, physics_2d::Binding2D};
use incant_physics_2d_runtime::CheckpointWorkBudget as PlanarWork;
use incant_physics_runtime::CheckpointWorkBudget as SpatialWork;
use incant_physics_snapshot::{Budget, construction::AllocationBudget};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum WorldCheckpointError {
    #[error("world checkpoint owner mismatch: {0}")]
    Owner(&'static str),
    #[error(transparent)]
    Components(#[from] SaveError),
    #[error(transparent)]
    Spatial(#[from] incant_physics_runtime::CheckpointError),
    #[error(transparent)]
    Planar(#[from] incant_physics_2d_runtime::CheckpointError),
}

/// An opaque checkpoint of this runtime world, including completed native
/// physics and separately retained pending ECS numeric inputs. It cannot be
/// constructed from bytes, cloned, or used as a whole game-session save.
/// Script heaps, callbacks, external commands, input, mixer cursors and hosted
/// widget state still belong to their hosts and must be checkpointed separately.
pub struct WorldCheckpoint {
    components: Vec<u8>,
    spatial: Option<incant_physics_runtime::PhysicsCheckpoint>,
    planar: Option<incant_physics_2d_runtime::PhysicsCheckpoint>,
    planar_rotations: Vec<(StableId, [f64; 4])>,
    physics_used: bool,
    physics_2d_used: bool,
    physics_save_required: bool,
    spatial_used: bool,
    navigation_used: bool,
    ik_catalog: Arc<[Arc<crate::NativeIk>]>,
    root_catalog: Arc<[Arc<incant_anim::RootMotionCatalog>]>,
}

impl NativeWorld {
    /// Capture without stepping, flushing pending physics inputs or changing
    /// published observations. Native solvers must be at admitted completed
    /// boundaries; post-step structural/query mutations may therefore refuse.
    /// Valid numeric ECS changes remain pending in the restored world.
    ///
    /// The supplied budgets cover native physics and its stable-ID owners, not
    /// the ECS serializer or allocator overhead. Component bytes retain the
    /// existing MAX_SAVE_BYTES/scene limits. Dirty navigation still refuses.
    pub fn checkpoint(
        &mut self,
        payload: &mut Budget,
        planar_work: &mut PlanarWork,
        spatial_work: &mut SpatialWork,
    ) -> Result<WorldCheckpoint, WorldCheckpointError> {
        if self.physics_failed {
            return Err(WorldCheckpointError::Owner("failed world"));
        }
        validation::bindings(self, planar_work, spatial_work)?;
        // A physics tick advances both dimensions, including an empty planar
        // owner in a 3D-only world. Declaration history is a separate flag.
        let stepped = self.physics_used || self.physics_2d_used;
        let spatial = stepped
            .then(|| self.physics.checkpoint(payload, spatial_work))
            .transpose()?;
        let planar = stepped
            .then(|| self.physics_2d.checkpoint(payload, planar_work))
            .transpose()?;
        let components = self.encode_components()?;
        let mut planar_rotations = Vec::new();
        planar_rotations
            .try_reserve_exact(self.physics_2d.states().count())
            .map_err(|_| WorldCheckpointError::Owner("planar cache allocation"))?;
        for (&id, &handle) in &self.entities {
            if let Some(binding) = self.world.get::<Binding2D>(handle) {
                planar_rotations.push((id, binding.rotation));
            }
        }
        Ok(WorldCheckpoint {
            components,
            spatial,
            planar,
            planar_rotations,
            physics_used: self.physics_used,
            physics_2d_used: self.physics_2d_used,
            physics_save_required: self.physics_save_required,
            spatial_used: self.spatial_used,
            navigation_used: self.navigation_used,
            ik_catalog: Arc::clone(&self.ik_catalog),
            root_catalog: Arc::clone(&self.root_catalog),
        })
    }

    /// Atomically replace this world only after complete native reconstruction,
    /// ECS/physics joins and original-scene compatibility checks. The host world
    /// identity is retained; ECS/entity incarnations and cache generations change.
    /// Host owners and queued commands must be restored/discarded by their host.
    pub fn restore_checkpoint(
        &mut self,
        checkpoint: &WorldCheckpoint,
        payload: &mut Budget,
        planar_work: &mut PlanarWork,
        spatial_work: &mut SpatialWork,
        allocations: &mut AllocationBudget,
    ) -> Result<(), WorldCheckpointError> {
        let candidate =
            checkpoint.reconstruct_for(self, payload, planar_work, spatial_work, allocations)?;
        self.publish_restored_components(candidate)?;
        Ok(())
    }
}

impl WorldCheckpoint {
    /// Stage a compatible fresh world without changing `current`. This allows a
    /// session host to bind a fresh VM and validate its other owners before one
    /// atomic session replacement. Host identity is deliberately fresh here;
    /// only NativeWorld::restore_checkpoint retains its destination identity.
    /// Historical host/save obligations survive without changing native stepping
    /// history. This does not restore any script, input, audio or UI host itself.
    pub fn reconstruct_for(
        &self,
        current: &NativeWorld,
        payload: &mut Budget,
        planar_work: &mut PlanarWork,
        spatial_work: &mut SpatialWork,
        allocations: &mut AllocationBudget,
    ) -> Result<NativeWorld, WorldCheckpointError> {
        if current.physics_failed {
            return Err(WorldCheckpointError::Owner("failed world"));
        }
        let mut candidate = self.reconstruct(payload, planar_work, spatial_work, allocations)?;
        candidate.check_save_binding(
            Some(current.source_scene_fingerprint),
            current.scene_id(),
            current.tick_rate,
            &current.localization,
            &current.font_assets,
        )?;
        candidate.validate_ik_catalog(&current.ik_catalog)?;
        candidate.validate_root_catalog(&current.root_catalog)?;
        candidate.ik_catalog = Arc::clone(&current.ik_catalog);
        candidate.root_catalog = Arc::clone(&current.root_catalog);
        // Rewinding does not erase host obligations or capability history.
        candidate.physics_save_required |=
            current.physics_save_required || current.physics_used || current.physics_2d_used;
        candidate.spatial_used |= current.spatial_used;
        candidate.navigation_used |= current.navigation_used;
        candidate.ui_used |= current.ui_used;
        candidate.audio_used |= current.audio_used;
        Ok(candidate)
    }

    /// Reconstruct a fresh, separately identified world. Any failed stage drops
    /// the private candidate; the checkpoint and original world stay unchanged.
    pub fn reconstruct(
        &self,
        payload: &mut Budget,
        planar_work: &mut PlanarWork,
        spatial_work: &mut SpatialWork,
        allocations: &mut AllocationBudget,
    ) -> Result<NativeWorld, WorldCheckpointError> {
        let decoded = codec::decode(&self.components)?;
        let scene = CookedScene::from_bytes(decoded.scene).map_err(SaveError::from)?;
        if (!scene.physics().is_empty() && self.spatial.is_none())
            || (!scene.physics_2d().is_empty() && self.planar.is_none())
            || (self.planar.is_none()
                && scene
                    .spatial()
                    .iter()
                    .any(|n| n.tilemap.as_ref().is_some_and(|m| m.has_collision())))
        {
            return Err(WorldCheckpointError::Owner("missing native owner"));
        }
        let spatial = self
            .spatial
            .as_ref()
            .map(|p| p.reconstruct(payload, spatial_work, allocations))
            .transpose()?;
        let planar = self
            .planar
            .as_ref()
            .map(|p| p.reconstruct(payload, planar_work, allocations))
            .transpose()?;
        let mut candidate = NativeWorld::from_decoded_components(
            decoded,
            scene,
            Some((spatial.unwrap_or_default(), planar.unwrap_or_default())),
        )?;
        candidate.physics_used = self.physics_used;
        candidate.physics_2d_used = self.physics_2d_used;
        candidate.physics_save_required = self.physics_save_required;
        candidate.spatial_used |= self.spatial_used;
        candidate.navigation_used |= self.navigation_used;
        candidate.validate_ik_catalog(&self.ik_catalog)?;
        candidate.validate_root_catalog(&self.root_catalog)?;
        candidate.ik_catalog = Arc::clone(&self.ik_catalog);
        candidate.root_catalog = Arc::clone(&self.root_catalog);
        validation::restore_planar_cache(&mut candidate, &self.planar_rotations)?;
        validation::bindings(&candidate, planar_work, spatial_work)?;
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests;

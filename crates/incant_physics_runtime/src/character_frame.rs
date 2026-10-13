//! Retained query-only collision storage. Geometry is copied at cold structural
//! boundaries; every fixed frame updates poses and refits the same broad phase.
use crate::{BodyState, PhysicsError, PhysicsRuntime, pose};
use rapier3d::prelude::*;
pub const MAX_CHARACTER_QUERY_COLLIDERS: usize = 65_536;
pub(crate) struct CharacterFrame {
    pub colliders: ColliderSet,
    pub bvh: BroadPhaseBvh,
    pub ready: bool,
}
impl<I: Ord + Clone> PhysicsRuntime<I> {
    /// Cold startup/structural hook. Calling this on an unchanged collection is
    /// constant work and preserves geometry, broad-phase and modified-list capacity.
    pub fn enable_character_queries(&mut self, scene: &I) -> Result<(), PhysicsError> {
        if self.failed {
            return Err(PhysicsError::NumericRange);
        }
        let scene = self
            .scenes
            .get_mut(scene)
            .ok_or_else(|| PhysicsError::Query("scene does not exist".into()))?;
        if scene.entries.len() > MAX_CHARACTER_QUERY_COLLIDERS {
            return Err(PhysicsError::Query(
                "character query collider capacity".into(),
            ));
        }
        if scene.character.is_none() {
            let mut cache = CharacterFrame {
                colliders: scene.world.read().colliders.clone(),
                bvh: BroadPhaseBvh::new(),
                ready: false,
            };
            for entry in scene.entries.values() {
                let c = &mut cache.colliders[entry.collider];
                c.set_position(pose(&entry.config.state));
                cache.bvh.set_aabb(
                    &scene.world.read().integration_parameters,
                    entry.collider,
                    c.compute_aabb(),
                );
            }
            scene.character = Some(cache);
        }
        Ok(())
    }
    /// End a private candidate-query interval even when later world preflight
    /// refuses. Geometry stays retained; only explicit refresh can reactivate it.
    pub fn clear_character_query_frame(&mut self, scene: &I) {
        if let Some(cache) = self
            .scenes
            .get_mut(scene)
            .and_then(|s| s.character.as_mut())
        {
            cache.ready = false;
        }
    }
    /// Refresh one complete scene frame from borrowed current native numeric
    /// inputs. This never writes solver bodies, contacts, clock or entry config.
    /// A refusal leaves the frame unavailable; no old reply can be consumed.
    pub fn prepare_character_queries(
        &mut self,
        scene: &I,
        mut state: impl FnMut(&I) -> Option<BodyState>,
    ) -> Result<(), PhysicsError> {
        if self.failed {
            return Err(PhysicsError::NumericRange);
        }
        let scene = self
            .scenes
            .get_mut(scene)
            .ok_or_else(|| PhysicsError::Query("scene does not exist".into()))?;
        let cache = scene
            .character
            .as_mut()
            .ok_or_else(|| PhysicsError::Query("character query storage is not enabled".into()))?;
        cache.ready = false;
        for (id, entry) in &scene.entries {
            let state = state(id).ok_or_else(|| {
                PhysicsError::Query("character query frame is missing a collider".into())
            })?;
            entry.config.validate_state(&state)?;
            let collider = &mut cache.colliders[entry.collider];
            collider.set_position(pose(&state));
            cache.bvh.set_aabb(
                &scene.world.read().integration_parameters,
                entry.collider,
                collider.compute_aabb(),
            );
        }
        cache.ready = true;
        Ok(())
    }
}

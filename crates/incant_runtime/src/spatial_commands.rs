//! Final-state spatial validation at explicit structural boundaries only.
use crate::{
    CookedSpatial, FrameCommands, NativeWorld, RuntimeError, SceneError, StableId,
    StructuralCommand,
};
use std::collections::BTreeMap;

impl NativeWorld {
    pub(crate) fn apply_spatial(
        &mut self,
        commands: &mut FrameCommands,
    ) -> Result<(), RuntimeError> {
        let mut changes = BTreeMap::new();
        for command in commands.iter() {
            let StructuralCommand::SetSpatial { entity, spatial } = command else {
                unreachable!()
            };
            if !self.entities.contains_key(entity) {
                return Err(RuntimeError::MissingEntity(*entity));
            }
            if spatial.as_ref().is_some_and(|s| s.entity != *entity) {
                return Err(SceneError::Invalid("spatial entity reference").into());
            }
            changes.insert(*entity, spatial.clone());
        }
        let generation = self.validate_spatial_changes(&changes, &self.parents)?;
        for (id, node) in changes {
            self.install_spatial(id, node);
        }
        self.binding_generation = generation;
        commands.clear();
        Ok(())
    }

    pub(crate) fn validate_spatial_changes(
        &self,
        changes: &BTreeMap<StableId, Option<CookedSpatial>>,
        parents: &BTreeMap<StableId, Option<StableId>>,
    ) -> Result<u64, RuntimeError> {
        let live = |id: &StableId| self.world.get::<CookedSpatial>(self.entities[id]);
        // Lend unchanged declarations. No scene or ancestor/component cloning;
        // only the bounded submitted configuration values are owned by staging.
        let unchanged = self
            .spatial_ids
            .iter()
            .filter(|id| parents.contains_key(id) && !changes.contains_key(id))
            .map(|id| live(id).expect("spatial declaration"));
        crate::spatial::validate(
            unchanged.chain(changes.values().filter_map(Option::as_ref)),
            &self.visual_assets,
            |id| parents.contains_key(&id),
        )?;
        let changed = self.spatial_ids.iter().any(|id| !parents.contains_key(id))
            || changes
                .iter()
                .any(|(id, node)| self.entities.get(id).and_then(|_| live(id)) != node.as_ref());
        if changed {
            self.binding_generation
                .checked_add(1)
                .ok_or(RuntimeError::GenerationOverflow)
        } else {
            Ok(self.binding_generation)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CookedEntity, CookedScene};
    use incant_types::{PointLight, Transform};
    #[test]
    fn generation_exhaustion_never_publishes_a_prefix() {
        let id = StableId([1; 16]);
        let mut world = NativeWorld::new(
            CookedScene::new(
                StableId([0; 16]),
                60,
                vec![CookedEntity {
                    id,
                    parent: None,
                    transform: Transform::default(),
                    velocity: None,
                }],
            )
            .unwrap(),
        );
        world.binding_generation = u64::MAX;
        let mut node = CookedSpatial::empty(id);
        node.point = Some(PointLight {
            color: [1.; 3],
            intensity: 1.,
            range: 1.,
        });
        let mut commands = FrameCommands::default();
        commands
            .push(StructuralCommand::SetSpatial {
                entity: id,
                spatial: Some(node),
            })
            .unwrap();
        assert_eq!(
            world.apply(&mut commands),
            Err(RuntimeError::GenerationOverflow)
        );
        assert!(
            world
                .spatial_view()
                .unwrap()
                .nodes()
                .all(|n| n.declaration.is_none())
        );
        assert_eq!(commands.len(), 1);
        world.topology_generation = u64::MAX;
        commands.clear();
        commands
            .push(StructuralCommand::Spawn(CookedEntity {
                id: StableId([2; 16]),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            }))
            .unwrap();
        assert_eq!(
            world.apply(&mut commands),
            Err(RuntimeError::GenerationOverflow)
        );
        assert_eq!(world.entity_count(), 1);
    }
}

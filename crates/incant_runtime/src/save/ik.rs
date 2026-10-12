use super::{SaveError, codec::IK_BYTES};
use crate::{AnimationIkSample, CookedScene, StableId, binary::Reader};
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn decode(
    scene: &CookedScene,
    bytes: &[u8],
) -> Result<BTreeMap<StableId, Arc<[AnimationIkSample]>>, SaveError> {
    let expected = scene
        .spatial()
        .iter()
        .filter_map(|n| n.ik.as_ref())
        .map(|n| n.initial.len())
        .sum::<usize>();
    if expected > crate::MAX_SCENE_ANIMATION_IK || bytes.len() != expected * IK_BYTES {
        return Err(SaveError::Invalid("IK membership"));
    }
    let mut reader = Reader::new(bytes);
    let mut result = BTreeMap::new();
    for node in scene.spatial() {
        let Some(definition) = &node.ik else { continue };
        let mut state = Vec::with_capacity(definition.initial.len());
        for _ in definition.initial.iter() {
            if StableId(reader.array()?) != node.entity {
                return Err(SaveError::Invalid("IK owner order"));
            }
            state.push(crate::ik_binary::read_sample(&mut reader)?);
        }
        definition.validate_state(&state)?;
        result.insert(node.entity, state.into());
    }
    Ok(result)
}

impl crate::NativeWorld {
    pub(super) fn validate_ik_catalog(
        &self,
        catalog: &[Arc<crate::NativeIk>],
    ) -> Result<(), SaveError> {
        for id in &self.spatial_ids {
            let Some(ik) = self
                .world
                .get::<crate::CookedSpatial>(self.entities[id])
                .and_then(|n| n.ik.as_ref())
            else {
                continue;
            };
            if !catalog.iter().any(|entry| {
                entry.mesh_fingerprint == ik.mesh_fingerprint
                    && entry.initial.len() == ik.initial.len()
                    && entry
                        .initial
                        .iter()
                        .zip(ik.initial.iter())
                        .all(|(a, b)| a.id == b.id && a.chain == b.chain)
            }) {
                return Err(SaveError::Invalid("IK original constraint binding"));
            }
        }
        Ok(())
    }
}

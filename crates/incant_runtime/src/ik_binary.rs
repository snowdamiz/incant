//! Version 16 suffix: ordered numeric constraints bound to the model fingerprint.
use crate::{AnimationIkSample, CookedSpatial, NativeIk, SceneError, StableId, binary::Reader};
use std::sync::Arc;
pub(crate) const ROW: usize = 78;
const PREFIX: usize = 52;
pub(crate) fn encoded_size(ik: &NativeIk) -> usize {
    PREFIX + ik.initial.len() * ROW
}
pub(crate) fn write(bytes: &mut Vec<u8>, nodes: &[CookedSpatial]) {
    bytes.extend((nodes.iter().filter(|n| n.ik.is_some()).count() as u32).to_le_bytes());
    for node in nodes {
        if let Some(ik) = &node.ik {
            bytes.extend(node.entity.0);
            bytes.extend(ik.mesh_fingerprint);
            bytes.extend((ik.initial.len() as u32).to_le_bytes());
            for sample in ik.initial.iter() {
                write_sample(bytes, sample);
            }
        }
    }
}
pub(crate) fn write_sample(bytes: &mut Vec<u8>, sample: &AnimationIkSample) {
    bytes.extend(sample.id.0);
    for n in sample.chain {
        bytes.extend(n.to_le_bytes());
    }
    for n in sample
        .target
        .into_iter()
        .chain(sample.pole)
        .chain([sample.weight])
    {
        bytes.extend(n.to_le_bytes());
    }
}
pub(crate) fn read_sample(r: &mut Reader<'_>) -> Result<AnimationIkSample, SceneError> {
    let id = StableId(r.array()?);
    let chain = [
        u16::from_le_bytes(r.array()?),
        u16::from_le_bytes(r.array()?),
        u16::from_le_bytes(r.array()?),
    ];
    let target = r.floats()?;
    let pole = r.floats()?;
    let [weight] = r.floats()?;
    let sample = AnimationIkSample {
        id,
        chain,
        target,
        pole,
        weight,
    };
    sample.validate().map_err(SceneError::Invalid)?;
    Ok(sample)
}
pub(crate) fn read(
    r: &mut Reader<'_>,
    nodes: &mut [CookedSpatial],
    allow_empty: bool,
) -> Result<(), SceneError> {
    let n = r.u32()? as usize;
    if (n == 0 && !allow_empty)
        || n > nodes.len()
        || n > crate::MAX_SCENE_ANIMATION_IK
        || n > r.remaining_len() / (PREFIX + ROW)
    {
        return Err(SceneError::Invalid("IK declaration count"));
    }
    let mut last = None;
    let mut budget = 0usize;
    for _ in 0..n {
        let entity = StableId(r.array()?);
        if last.is_some_and(|old| old >= entity) {
            return Err(SceneError::Invalid("noncanonical IK order"));
        }
        last = Some(entity);
        let index = nodes
            .binary_search_by_key(&entity, |n| n.entity)
            .map_err(|_| SceneError::Invalid("IK entity"))?;
        let mesh_fingerprint = r.array()?;
        let count = r.u32()? as usize;
        if count == 0
            || count > crate::MAX_ANIMATION_IK
            || count > r.remaining_len() / ROW
            || count > crate::MAX_SCENE_ANIMATION_IK.saturating_sub(budget)
        {
            return Err(SceneError::Invalid("IK aggregate capacity"));
        }
        budget += count;
        let mut initial = Vec::with_capacity(count);
        for _ in 0..count {
            initial.push(read_sample(r)?);
        }
        let ik = NativeIk {
            mesh_fingerprint,
            initial: initial.into(),
        };
        ik.validate()?;
        nodes[index].ik = Some(Arc::new(ik));
    }
    Ok(())
}

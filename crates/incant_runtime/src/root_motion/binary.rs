//! Canonical v17 suffix: sorted root catalogs followed by sorted bindings.
use super::*;
use crate::{StableId, binary::Reader};
pub(crate) const BINDING_BYTES: usize = 24;
type Key = ([u8; 32], u16);

fn catalogs(nodes: &[CookedSpatial]) -> BTreeMap<Key, &RootMotionCatalog> {
    nodes
        .iter()
        .filter_map(|n| n.root_motion.as_ref())
        .map(|r| {
            (
                (r.catalog.fingerprint(), r.catalog.root()),
                r.catalog.as_ref(),
            )
        })
        .collect()
}
pub(crate) fn encoded_size(nodes: &[CookedSpatial]) -> usize {
    8 + nodes.iter().filter(|n| n.root_motion.is_some()).count() * BINDING_BYTES
        + catalogs(nodes)
            .values()
            .map(|c| 4 + c.encoded_len().expect("admitted root catalog"))
            .sum::<usize>()
}
pub(crate) fn write(bytes: &mut Vec<u8>, nodes: &[CookedSpatial]) {
    let catalogs = catalogs(nodes);
    bytes.extend((catalogs.len() as u32).to_le_bytes());
    let mut indices = BTreeMap::new();
    for (i, (key, catalog)) in catalogs.into_iter().enumerate() {
        indices.insert(key, i as u32);
        let encoded = catalog.to_bytes().expect("admitted root catalog");
        bytes.extend((encoded.len() as u32).to_le_bytes());
        bytes.extend(encoded);
    }
    bytes.extend((nodes.iter().filter(|n| n.root_motion.is_some()).count() as u32).to_le_bytes());
    for node in nodes {
        let Some(root) = &node.root_motion else {
            continue;
        };
        bytes.extend(node.entity.0);
        bytes.extend(indices[&(root.catalog.fingerprint(), root.catalog.root())].to_le_bytes());
        bytes.extend([
            match root.application {
                RootMotionApplication::ExtractOnly => 0,
                RootMotionApplication::Transform => 1,
                RootMotionApplication::Kinematic3d => 2,
                RootMotionApplication::Character3d => 3,
            },
            u8::from(root.applying),
            0,
            0,
        ]);
    }
}
pub(crate) fn read(
    reader: &mut Reader<'_>,
    nodes: &mut [CookedSpatial],
    allow_empty: bool,
) -> Result<(), SceneError> {
    let count = reader.u32()? as usize;
    if (!allow_empty && count == 0)
        || count > MAX_ROOT_MOTION_INSTANCES
        || count > reader.remaining_len() / 140
    {
        return Err(SceneError::Invalid("root-motion catalog count"));
    }
    let mut catalogs = Vec::with_capacity(count);
    let mut last = None;
    let mut budget = 0usize;
    for _ in 0..count {
        let length = reader.u32()? as usize;
        let catalog = RootMotionCatalog::from_bytes(reader.bytes(length)?)
            .map_err(|_| SceneError::Invalid("root-motion catalog payload"))?;
        let key = (catalog.fingerprint(), catalog.root());
        if last.is_some_and(|old| old >= key) {
            return Err(SceneError::Invalid("root-motion catalog order"));
        }
        last = Some(key);
        budget = budget
            .checked_add(catalog.retained_bytes())
            .filter(|n| *n <= MAX_SCENE_ROOT_MOTION_BYTES)
            .ok_or(SceneError::Invalid("root-motion retained scene capacity"))?;
        catalogs.push(Arc::new(catalog));
    }
    let bindings = reader.u32()? as usize;
    if (bindings == 0) != (count == 0)
        || bindings > MAX_ROOT_MOTION_INSTANCES
        || bindings > nodes.len()
        || bindings > reader.remaining_len() / BINDING_BYTES
    {
        return Err(SceneError::Invalid("root-motion binding count"));
    }
    let mut used = vec![false; count];
    let mut last = None;
    for _ in 0..bindings {
        let entity = StableId(reader.array()?);
        if last.is_some_and(|old| old >= entity) {
            return Err(SceneError::Invalid("root-motion binding order"));
        }
        last = Some(entity);
        let index = nodes
            .binary_search_by_key(&entity, |n| n.entity)
            .map_err(|_| SceneError::Invalid("root-motion binding owner"))?;
        let catalog = reader.u32()? as usize;
        let value = catalogs
            .get(catalog)
            .ok_or(SceneError::Invalid("root-motion catalog index"))?;
        let [policy, enabled, r0, r1] = reader.array()?;
        if enabled > 1 || r0 != 0 || r1 != 0 {
            return Err(SceneError::Invalid("root-motion binding flags"));
        }
        let root = NativeRootMotion {
            catalog: Arc::clone(value),
            applying: enabled != 0,
            application: match policy {
                0 => RootMotionApplication::ExtractOnly,
                1 => RootMotionApplication::Transform,
                2 => RootMotionApplication::Kinematic3d,
                3 => RootMotionApplication::Character3d,
                _ => return Err(SceneError::Invalid("root-motion application policy")),
            },
        };
        root.validate(
            nodes[index]
                .animation
                .as_deref()
                .ok_or(SceneError::Invalid("root motion requires animation"))?,
        )?;
        nodes[index].root_motion = Some(Arc::new(root));
        used[catalog] = true;
    }
    if used.contains(&false) {
        return Err(SceneError::Invalid("unreferenced root-motion catalog"));
    }
    Ok(())
}

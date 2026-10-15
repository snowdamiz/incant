//! Ordinary same-scene command preparation and copies; no file or runtime authority.
mod references;

use super::{CommandError, prefab::Budget};
use incant_doc::{Entity, Id, Project, Provenance, Scene};
use std::collections::{BTreeMap, BTreeSet};

fn invalid(message: &str) -> CommandError {
    CommandError::Invalid(format!("duplicate subtree: {message}"))
}

/// Prepare ordinary command data without editing the document. The eventual
/// execute still validates the exact subtree, fresh identities and revision.
pub(super) fn prepare(
    project: &Project,
    scene_id: &str,
    root: &str,
) -> Result<(super::Command, Id), CommandError> {
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| invalid("scene absent"))?;
    if !scene.entities.contains_key(root) {
        return Err(invalid("source root absent"));
    }
    // One scene scan, then a bounded traversal; do not rescan all entities for
    // each level of a deep subtree. This is authoring preparation, never a tick.
    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for (id, entity) in &scene.entities {
        if let Some(parent) = entity.parent.as_deref() {
            children.entry(parent).or_default().push(id);
        }
    }
    let mut mapping = BTreeMap::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if mapping.contains_key(id) {
            return Err(invalid("cyclic source hierarchy"));
        }
        if mapping.len() == incant_doc::prefab::MAX_PREFAB_NODES {
            return Err(invalid("identity mapping requires 1..1024 nodes"));
        }
        mapping.insert(id.to_owned(), incant_doc::new_id());
        if let Some(next) = children.get(id) {
            // Bound the pending frontier as well as the eventual mapping.
            if next.len() > incant_doc::prefab::MAX_PREFAB_NODES - mapping.len() - pending.len() {
                return Err(invalid("identity mapping requires 1..1024 nodes"));
            }
            pending.extend(next.iter().copied());
        }
    }
    let created = mapping[root].clone();
    Ok((
        super::Command::DuplicateSubtree {
            scene_id: scene_id.into(),
            root_entity_id: root.into(),
            entities: mapping,
        },
        created,
    ))
}

pub(super) fn duplicate(
    project: &mut Project,
    scene_id: &str,
    root: &str,
    mapping: &BTreeMap<Id, Id>,
    provenance: &Provenance,
    budget: &mut Budget,
) -> Result<(), CommandError> {
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| invalid("scene absent"))?;
    complete_subtree(scene, root, mapping)?;
    fresh_ids(project, mapping)?;
    // Charge every copy before cloning components or inserting any output node.
    for (source, target) in mapping {
        let entity = &scene.entities[source];
        if entity.prefab.is_some() {
            return Err(invalid(
                "detach linked instances explicitly before duplication",
            ));
        }
        if entity.components.contains_key("AudioListener")
            || entity.components.contains_key("EnvironmentLight")
            || entity
                .components
                .get("UiNode")
                .is_some_and(|v| v["kind"]["role"] == "root")
        {
            return Err(invalid(
                "a subtree containing a singleton cannot be duplicated",
            ));
        }
        budget.copy_entity(entity, target, provenance)?;
    }
    let mut copies = Vec::with_capacity(mapping.len());
    for (source, target) in mapping {
        let original = &scene.entities[source];
        copies.push(Entity {
            id: target.clone(),
            name: original.name.clone(),
            parent: if source == root {
                original.parent.clone()
            } else {
                original.parent.as_ref().map(|id| mapping[id].clone())
            },
            components: references::remap(&original.components, mapping)?,
            provenance: Some(provenance.clone()),
            prefab: None,
        });
    }
    let scene = project.scenes.get_mut(scene_id).expect("admitted scene");
    for entity in copies {
        scene.entities.insert(entity.id.clone(), entity);
    }
    Ok(())
}

/// Use the bounded caller map to validate closure and reachability. This scans
/// the scene once and retains only <=1024 mapped-node edges, not the whole scene.
fn complete_subtree(
    scene: &Scene,
    root: &str,
    mapping: &BTreeMap<Id, Id>,
) -> Result<(), CommandError> {
    if mapping.is_empty() || mapping.len() > incant_doc::prefab::MAX_PREFAB_NODES {
        return Err(invalid("identity mapping requires 1..1024 nodes"));
    }
    if !mapping.contains_key(root) {
        return Err(invalid("identity mapping omits the source root"));
    }
    let mut children: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for source in mapping.keys() {
        let entity = scene
            .entities
            .get(source)
            .ok_or_else(|| invalid("mapped source absent"))?;
        if source == root {
            if entity
                .parent
                .as_ref()
                .is_some_and(|id| mapping.contains_key(id))
            {
                return Err(invalid("cyclic source hierarchy"));
            }
        } else {
            let parent = entity
                .parent
                .as_deref()
                .filter(|id| mapping.contains_key(*id))
                .ok_or_else(|| invalid("mapped source is outside the subtree"))?;
            children.entry(parent).or_default().push(source);
        }
    }
    let mut seen = BTreeSet::new();
    let mut pending = vec![root];
    while let Some(id) = pending.pop() {
        if !seen.insert(id) {
            return Err(invalid("cyclic source hierarchy"));
        }
        if let Some(children) = children.get(id) {
            pending.extend(children.iter().copied());
        }
    }
    if seen.len() != mapping.len()
        || scene.entities.iter().any(|(id, e)| {
            e.parent.as_ref().is_some_and(|p| mapping.contains_key(p)) && !mapping.contains_key(id)
        })
    {
        return Err(invalid(
            "identity mapping must cover exactly the complete subtree",
        ));
    }
    Ok(())
}

fn fresh_ids(project: &Project, mapping: &BTreeMap<Id, Id>) -> Result<(), CommandError> {
    let mut targets = BTreeSet::new();
    for id in mapping.values() {
        let parsed = id
            .parse::<ulid::Ulid>()
            .map_err(|_| invalid("mapped identity is not a ULID"))?;
        if parsed.to_string() != *id || !targets.insert(parsed) {
            return Err(invalid(
                "mapped identities must be distinct canonical ULIDs",
            ));
        }
    }
    let used = |id: &Id| {
        id.parse::<ulid::Ulid>()
            .is_ok_and(|id| targets.contains(&id))
    };
    let occupied = used(&project.id)
        || project
            .scenes
            .iter()
            .any(|(id, s)| used(id) || s.entities.keys().any(&used))
        || project
            .assets
            .keys()
            .chain(project.scripts.keys())
            .chain(project.string_tables.keys())
            .any(&used)
        || project
            .prefabs
            .iter()
            .any(|(id, p)| used(id) || p.nodes.keys().any(&used));
    if occupied {
        return Err(invalid("mapped identities must be globally fresh"));
    }
    Ok(())
}

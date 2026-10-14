use super::{PayloadBudget, Prefab, PrefabError, check_budget, invalid, remap_components};
use crate::{Entity, Id, Project, Scene};
use std::collections::{BTreeMap, BTreeSet};

/// Validate a bounded standalone subtree using the ordinary component/scene validators.
pub fn validate_prototype(project: &Project, prefab: &Prefab) -> Result<(), PrefabError> {
    PayloadBudget::default().prototype(prefab)?;
    if prefab.name.trim().is_empty() || !prefab.nodes.contains_key(&prefab.root) {
        return Err(invalid("prototype name/root"));
    }
    for id in std::iter::once(&prefab.id).chain(prefab.nodes.keys()) {
        if id.parse::<ulid::Ulid>().is_err() {
            return Err(invalid("invalid prototype ULID"));
        }
    }
    let identity: BTreeMap<_, _> = prefab
        .nodes
        .keys()
        .map(|id| (id.clone(), id.clone()))
        .collect();
    let mut entities = BTreeMap::new();
    for (id, node) in &prefab.nodes {
        let mut at = id.as_str();
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(at) {
                return Err(invalid("prototype hierarchy cycle"));
            }
            let n = prefab
                .nodes
                .get(at)
                .ok_or_else(|| invalid("prototype parent absent"))?;
            match &n.parent {
                None if at == prefab.root => break,
                None => return Err(invalid("prototype must have exactly one root")),
                Some(_) if at == prefab.root => return Err(invalid("prototype root has a parent")),
                Some(parent) => at = parent,
            }
        }
        let components = remap_components(&node.components, &identity)?;
        entities.insert(
            id.clone(),
            Entity {
                id: id.clone(),
                name: node.name.clone(),
                parent: node.parent.clone(),
                components,
                prefab: None,
                provenance: if id == &prefab.root {
                    prefab.provenance.clone()
                } else {
                    None
                },
            },
        );
    }
    let scene = Scene {
        id: prefab.id.clone(),
        name: prefab.name.clone(),
        entities,
        program: None,
    };
    let mut errors = Vec::new();
    project.scene_diagnostics(&scene.id, &scene, "/prototype", &mut |p, m| {
        errors.push(format!("{p}: {m}"))
    });
    if !errors.is_empty() {
        return Err(invalid(errors.join("; ")));
    }
    crate::navigation::validate_scenes(std::iter::once(&scene)).map_err(invalid)?;
    crate::audio::validate_entities(scene.entities.values()).map_err(invalid)?;
    if scene
        .entities
        .values()
        .filter(|e| e.components.contains_key("EnvironmentLight"))
        .count()
        > 1
    {
        return Err(invalid("prototype has multiple environment lights"));
    }
    Ok(())
}
/// Complete node-to-entity bijection for one instance; does not invent missing nodes.
pub fn instance_map(
    scene: &Scene,
    root: &str,
    prefab: &Prefab,
) -> Result<BTreeMap<Id, Id>, PrefabError> {
    let mut map = BTreeMap::new();
    for e in scene.entities.values() {
        let Some(link) = &e.prefab else {
            continue;
        };
        if link.root != root {
            continue;
        }
        if link.source != prefab.id
            || !prefab.nodes.contains_key(&link.node)
            || map.insert(link.node.clone(), e.id.clone()).is_some()
        {
            return Err(invalid("mixed or duplicate instance nodes"));
        }
    }
    if map.len() != prefab.nodes.len() || map.get(&prefab.root).map(String::as_str) != Some(root) {
        return Err(invalid("incomplete instance or wrong root mapping"));
    }
    Ok(map)
}
pub fn validate(project: &Project) -> Result<(), PrefabError> {
    check_budget(project)?;
    for (id, p) in &project.prefabs {
        if id != &p.id {
            return Err(invalid("prototype key and ID differ"));
        }
        validate_prototype(project, p)?;
    }
    for scene in project.scenes.values() {
        let mut roots: BTreeMap<&str, BTreeMap<Id, Id>> = BTreeMap::new();
        for e in scene.entities.values() {
            if let Some(link) = &e.prefab {
                let map = roots.entry(&link.root).or_default();
                if map.insert(link.node.clone(), e.id.clone()).is_some() {
                    return Err(invalid("duplicate instance nodes"));
                }
            }
            if let Some(parent) = e.parent.as_ref().and_then(|p| scene.entities.get(p))
                && let Some(parent_link) = &parent.prefab
                && e.prefab.as_ref().is_none_or(|l| l.root != parent_link.root)
            {
                return Err(invalid(
                    "linked subtree cannot contain foreign children or nested instances",
                ));
            }
        }
        for (root, map) in roots {
            let entity = scene
                .entities
                .get(root)
                .ok_or_else(|| invalid("instance root absent"))?;
            let link = entity
                .prefab
                .as_ref()
                .ok_or_else(|| invalid("root missing link"))?;
            let p = project
                .prefabs
                .get(&link.source)
                .ok_or_else(|| invalid("prototype absent"))?;
            if map.len() != p.nodes.len() || map.get(&p.root).map(String::as_str) != Some(root) {
                return Err(invalid("incomplete instance or wrong root mapping"));
            }
            for (node_id, eid) in &map {
                let node = p
                    .nodes
                    .get(node_id)
                    .ok_or_else(|| invalid("unknown instance node"))?;
                let e = &scene.entities[eid];
                let link = e
                    .prefab
                    .as_ref()
                    .ok_or_else(|| invalid("node missing link"))?;
                if link.source != p.id {
                    return Err(invalid("mixed instance prototypes"));
                }
                if e.name != node.name {
                    return Err(invalid("linked name diverges from prototype; detach first"));
                }
                if node_id != &p.root
                    && e.parent.as_ref() != node.parent.as_ref().and_then(|p| map.get(p))
                {
                    return Err(invalid("linked topology differs"));
                }
                let expected = remap_components(&node.components, &map)?;
                for kind in expected.keys().chain(e.components.keys()) {
                    if !link.overrides.contains(kind)
                        && expected.get(kind) != e.components.get(kind)
                    {
                        return Err(invalid("unrecorded component divergence"));
                    }
                }
                for kind in &link.overrides {
                    if !crate::is_component_kind(kind) {
                        return Err(invalid("unregistered override component"));
                    }
                }
            }
        }
    }
    Ok(())
}

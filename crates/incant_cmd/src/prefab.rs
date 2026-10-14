//! Linked prefab commands resolve into ordinary scene entities on the staged project.
use super::{Command, CommandError, entity_mut, scene_mut};
use incant_doc::prefab::{
    MAX_PREFAB_NODES, PayloadBudget, PrefabError, instance_map, remap_components,
    validate_prototype,
};
use incant_doc::{Entity, Id, Prefab, PrefabLink, PrefabNode, Project, Provenance};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(super) struct Budget {
    nodes: usize,
    payload: PayloadBudget,
}
impl Budget {
    fn charge(&mut self, count: usize) -> Result<(), CommandError> {
        self.nodes = self
            .nodes
            .checked_add(count)
            .filter(|n| *n <= 10_000)
            .ok_or(PrefabError::Limit("10000 affected nodes per transaction"))?;
        Ok(())
    }
    pub(super) fn remove_scene(&mut self, scene: &incant_doc::Scene) -> Result<(), CommandError> {
        self.charge(
            scene
                .entities
                .values()
                .filter(|e| e.prefab.is_some())
                .count(),
        )
    }
    pub(super) fn override_value(
        &mut self,
        entity: &Entity,
        kind: &str,
        value: Option<&serde_json::Value>,
    ) -> Result<(), CommandError> {
        if entity.prefab.is_some() {
            self.edit(entity)?;
            if let Some(value) = value {
                self.payload.component(kind, value)?;
            }
        }
        Ok(())
    }
    pub(super) fn edit(&mut self, entity: &Entity) -> Result<(), CommandError> {
        if entity.prefab.is_some() {
            self.charge(1)?;
            self.payload.components(&entity.components)?;
        }
        Ok(())
    }
}
fn invalid(message: &str) -> CommandError {
    PrefabError::Invalid(message.into()).into()
}
/// Bound incoming prefab values/maps before the transaction's ordinary project clone.
pub(super) fn preflight(commands: &[Command]) -> Result<(), CommandError> {
    let mut payload = PayloadBudget::default();
    for c in commands {
        match c {
            Command::UpdatePrefab { prefab } => payload.prototype(prefab)?,
            Command::CreatePrefab { nodes, .. }
            | Command::InstantiatePrefab {
                entities: nodes, ..
            } => {
                if nodes.is_empty() || nodes.len() > MAX_PREFAB_NODES {
                    return Err(PrefabError::Limit("identity mapping 1..1024").into());
                }
                for (a, b) in nodes {
                    payload.text(a)?;
                    payload.text(b)?;
                }
                if let Command::CreatePrefab { name, .. } = c {
                    payload.text(name)?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
fn mapping(
    nodes: &BTreeMap<Id, Id>,
    expected: impl Iterator<Item = Id>,
) -> Result<(), CommandError> {
    if nodes.keys().cloned().collect::<BTreeSet<_>>() != expected.collect() {
        return Err(invalid(
            "identity mapping must cover exactly the complete subtree",
        ));
    }
    let mut unique = BTreeSet::new();
    for id in nodes.values() {
        if id.parse::<ulid::Ulid>().is_err() || !unique.insert(id) {
            return Err(invalid("mapped identities must be distinct ULIDs"));
        }
    }
    Ok(())
}
fn source<'a>(project: &'a Project, id: &str) -> Result<&'a Prefab, CommandError> {
    project
        .prefabs
        .get(id)
        .ok_or_else(|| invalid("prototype does not exist"))
}
fn roots(project: &Project, prefab: &str) -> Vec<(Id, Id)> {
    project
        .scenes
        .iter()
        .flat_map(|(sid, s)| {
            s.entities.values().filter_map(move |e| {
                e.prefab
                    .as_ref()
                    .filter(|l| l.source == prefab && l.root == e.id)
                    .map(|_| (sid.clone(), e.id.clone()))
            })
        })
        .collect()
}
#[expect(
    clippy::too_many_arguments,
    reason = "Mirrors CreatePrefab fields plus its transaction context"
)]
pub(super) fn create(
    project: &mut Project,
    scene_id: &str,
    root: &str,
    prefab_id: &str,
    name: &str,
    nodes: &BTreeMap<Id, Id>,
    provenance: &Provenance,
    budget: &mut Budget,
) -> Result<(), CommandError> {
    if project.prefabs.contains_key(prefab_id) {
        return Err(invalid("prototype already exists"));
    }
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| invalid("scene absent"))?;
    if !scene.entities.contains_key(root) {
        return Err(invalid("source root absent"));
    }
    let mut subtree = BTreeSet::from([root.to_owned()]);
    loop {
        let before = subtree.len();
        for e in scene.entities.values() {
            if e.parent.as_ref().is_some_and(|p| subtree.contains(p)) {
                subtree.insert(e.id.clone());
            }
        }
        if subtree.len() > MAX_PREFAB_NODES {
            return Err(PrefabError::Limit("subtree 1024 nodes").into());
        }
        if subtree.len() == before {
            break;
        }
    }
    mapping(nodes, subtree.iter().cloned())?;
    budget.charge(subtree.len())?;
    for id in &subtree {
        let e = &scene.entities[id];
        if e.prefab.is_some() {
            return Err(invalid(
                "detach linked instances before capturing a prototype",
            ));
        }
        budget.payload.components(&e.components)?;
        budget.payload.text(&e.name)?;
    }
    let mut output = BTreeMap::new();
    for id in &subtree {
        let e = &scene.entities[id];
        output.insert(
            nodes[id].clone(),
            PrefabNode {
                name: e.name.clone(),
                parent: if id == root {
                    None
                } else {
                    e.parent.as_ref().map(|p| nodes[p].clone())
                },
                components: remap_components(&e.components, nodes)?,
            },
        );
    }
    let p = Prefab {
        id: prefab_id.into(),
        name: name.into(),
        root: nodes[root].clone(),
        nodes: output,
        provenance: Some(provenance.clone()),
    };
    validate_prototype(project, &p)?;
    project.prefabs.insert(prefab_id.into(), p);
    Ok(())
}
pub(super) fn instantiate(
    project: &mut Project,
    source_id: &str,
    scene_id: &str,
    entities: &BTreeMap<Id, Id>,
    parent: &Option<Id>,
    provenance: &Provenance,
    budget: &mut Budget,
) -> Result<(), CommandError> {
    let p = source(project, source_id)?;
    mapping(entities, p.nodes.keys().cloned())?;
    budget.charge(p.nodes.len())?;
    budget.payload.prototype(p)?;
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| invalid("scene absent"))?;
    if entities.values().any(|id| scene.entities.contains_key(id)) {
        return Err(invalid("instance entity already exists"));
    }
    let root = &entities[&p.root];
    let mut prepared = Vec::with_capacity(p.nodes.len());
    for (id, n) in &p.nodes {
        prepared.push(Entity {
            id: entities[id].clone(),
            name: n.name.clone(),
            parent: if id == &p.root {
                parent.clone()
            } else {
                n.parent.as_ref().map(|id| entities[id].clone())
            },
            components: remap_components(&n.components, entities)?,
            provenance: Some(provenance.clone()),
            prefab: Some(PrefabLink {
                source: source_id.into(),
                root: root.clone(),
                node: id.clone(),
                overrides: BTreeSet::new(),
            }),
        });
    }
    let scene = scene_mut(project, scene_id)?;
    for e in prepared {
        scene.entities.insert(e.id.clone(), e);
    }
    Ok(())
}
pub(super) fn update(
    project: &mut Project,
    replacement: &Prefab,
    provenance: &Provenance,
    budget: &mut Budget,
) -> Result<(), CommandError> {
    let old = source(project, &replacement.id)?;
    if old.root != replacement.root
        || old.nodes.len() != replacement.nodes.len()
        || old.nodes.iter().any(|(id, n)| {
            replacement
                .nodes
                .get(id)
                .is_none_or(|r| r.parent != n.parent)
        })
    {
        return Err(invalid(
            "prototype update must preserve every node ID and parent",
        ));
    }
    budget.charge(replacement.nodes.len())?;
    budget.payload.prototype(replacement)?;
    validate_prototype(project, replacement)?;
    let mut instances: BTreeMap<(Id, Id), BTreeMap<Id, Id>> = BTreeMap::new();
    for (sid, scene) in &project.scenes {
        for e in scene.entities.values() {
            if let Some(l) = &e.prefab
                && l.source == replacement.id
            {
                budget.charge(1)?;
                let n = replacement
                    .nodes
                    .get(&l.node)
                    .ok_or_else(|| invalid("unknown prototype node"))?;
                budget.payload.components(&n.components)?;
                budget.payload.text(&n.name)?;
                if instances
                    .entry((sid.clone(), l.root.clone()))
                    .or_default()
                    .insert(l.node.clone(), e.id.clone())
                    .is_some()
                {
                    return Err(invalid("duplicate instance node"));
                }
            }
        }
    }
    // All fan-out charges precede entity replacement. Input linkage was validated;
    // explicit size/root checks also keep staged multi-command input fallible.
    for ((sid, root), map) in instances {
        if map.len() != replacement.nodes.len() || map.get(&replacement.root) != Some(&root) {
            return Err(invalid("incomplete instance"));
        }
        let scene = scene_mut(project, &sid)?;
        for (id, eid) in &map {
            let n = &replacement.nodes[id];
            let resolved = remap_components(&n.components, &map)?;
            let e = scene
                .entities
                .get_mut(eid)
                .ok_or_else(|| invalid("instance disappeared"))?;
            let overrides = &e
                .prefab
                .as_ref()
                .ok_or_else(|| invalid("link disappeared"))?
                .overrides;
            e.components.retain(|kind, _| overrides.contains(kind));
            for (kind, value) in resolved {
                if !overrides.contains(&kind) {
                    e.components.insert(kind, value);
                }
            }
            e.name = n.name.clone();
            e.provenance = Some(provenance.clone());
        }
    }
    let mut p = replacement.clone();
    p.provenance = Some(provenance.clone());
    project.prefabs.insert(p.id.clone(), p);
    Ok(())
}
pub(super) fn revert(
    project: &mut Project,
    scene_id: &str,
    entity_id: &str,
    component: &str,
    provenance: &Provenance,
    budget: &mut Budget,
) -> Result<(), CommandError> {
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| invalid("scene absent"))?;
    let e = scene
        .entities
        .get(entity_id)
        .ok_or_else(|| invalid("entity absent"))?;
    let l = e
        .prefab
        .as_ref()
        .ok_or_else(|| invalid("entity is not linked"))?;
    if !l.overrides.contains(component) {
        return Err(invalid("component is not overridden"));
    }
    let p = source(project, &l.source)?;
    budget.edit(e)?;
    budget.payload.components(&p.nodes[&l.node].components)?;
    let map = instance_map(scene, &l.root, p)?;
    let mut resolved = remap_components(&p.nodes[&l.node].components, &map)?;
    let e = entity_mut(project, scene_id, entity_id)?;
    if let Some(v) = resolved.remove(component) {
        e.components.insert(component.into(), v);
    } else {
        e.components.remove(component);
    }
    e.prefab
        .as_mut()
        .ok_or_else(|| invalid("link disappeared"))?
        .overrides
        .remove(component);
    e.provenance = Some(provenance.clone());
    Ok(())
}
pub(super) fn detach_or_delete(
    project: &mut Project,
    scene_id: &str,
    root: &str,
    delete: bool,
    provenance: &Provenance,
    budget: &mut Budget,
) -> Result<(), CommandError> {
    let scene = project
        .scenes
        .get(scene_id)
        .ok_or_else(|| invalid("scene absent"))?;
    let e = scene
        .entities
        .get(root)
        .ok_or_else(|| invalid("root absent"))?;
    let l = e
        .prefab
        .as_ref()
        .filter(|l| l.root == root)
        .ok_or_else(|| invalid("expected instance root"))?;
    let map = instance_map(scene, root, source(project, &l.source)?)?;
    budget.charge(map.len())?;
    let scene = scene_mut(project, scene_id)?;
    for eid in map.values() {
        if delete {
            scene.entities.remove(eid);
        } else {
            let e = scene
                .entities
                .get_mut(eid)
                .ok_or_else(|| invalid("entity absent"))?;
            e.prefab = None;
            e.provenance = Some(provenance.clone());
        }
    }
    Ok(())
}
pub(super) fn delete(
    project: &mut Project,
    id: &str,
    budget: &mut Budget,
) -> Result<(), CommandError> {
    if !roots(project, id).is_empty() {
        return Err(invalid("detach or delete instances first"));
    }
    budget.charge(source(project, id)?.nodes.len())?;
    project
        .prefabs
        .remove(id)
        .ok_or_else(|| invalid("prototype absent"))?;
    Ok(())
}

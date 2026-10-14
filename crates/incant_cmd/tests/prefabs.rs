#[path = "support/prefab.rs"]
mod support;
use incant_cmd::{Command, CommandBus, CommandError};
use incant_doc::{Entity, Project, new_id};
use serde_json::json;
use std::collections::BTreeMap;
use support::Fixture;
#[test]
fn two_instances_override_remove_revert_update_and_detach() {
    let mut f = Fixture::new();
    f.create();
    let a = f.instance();
    let b = f.instance();
    let node = f.nodes[&f.source].clone();
    let ar = a[&node].clone();
    let br = b[&node].clone();
    assert_ne!(ar, br);
    assert_eq!(
        f.bus.project().scenes[&f.scene].entities[&a[&f.nodes[&f.child]]].parent,
        Some(ar.clone())
    );
    f.run(vec![Command::SetComponent {
        scene_id: f.scene.clone(),
        entity_id: ar.clone(),
        component: "Velocity".into(),
        value: json!({"linear":[4,0,0]}),
    }])
    .unwrap();
    f.run(vec![Command::RemoveComponent {
        scene_id: f.scene.clone(),
        entity_id: br.clone(),
        component: "Velocity".into(),
    }])
    .unwrap();
    let mut p = f.bus.project().prefabs[&f.prefab].clone();
    p.nodes.get_mut(&node).unwrap().name = "Updated".into();
    p.nodes
        .get_mut(&node)
        .unwrap()
        .components
        .insert("Velocity".into(), json!({"linear":[2,0,0]}));
    p.nodes
        .get_mut(&f.nodes[&f.child])
        .unwrap()
        .components
        .insert("Velocity".into(), json!({"linear":[0,2,0]}));
    f.run(vec![Command::UpdatePrefab { prefab: p }]).unwrap();
    let s = &f.bus.project().scenes[&f.scene];
    for map in [&a, &b] {
        assert_eq!(
            s.entities[&map[&f.nodes[&f.child]]].components["Velocity"]["linear"],
            json!([0, 2, 0])
        );
    }

    assert_eq!(
        s.entities[&ar].components["Velocity"]["linear"],
        json!([4, 0, 0])
    );
    assert!(!s.entities[&br].components.contains_key("Velocity"));
    assert_eq!(s.entities[&ar].name, "Updated");
    for id in [&ar, &br] {
        f.run(vec![Command::RevertPrefabComponent {
            scene_id: f.scene.clone(),
            entity_id: id.clone(),
            component: "Velocity".into(),
        }])
        .unwrap();
    }
    assert_eq!(
        f.bus.project().scenes[&f.scene].entities[&ar].components["Velocity"]["linear"],
        json!([2, 0, 0])
    );
    let before = f.bus.project().scenes[&f.scene].entities[&ar]
        .components
        .clone();
    f.run(vec![Command::DetachPrefab {
        scene_id: f.scene.clone(),
        root_entity_id: ar.clone(),
    }])
    .unwrap();
    assert_eq!(
        f.bus.project().scenes[&f.scene].entities[&ar].components,
        before
    );
    assert!(a.values().all(|id| {
        f.bus.project().scenes[&f.scene].entities[id]
            .prefab
            .is_none()
    }));
    f.run(vec![Command::RenameEntity {
        scene_id: f.scene.clone(),
        entity_id: ar,
        name: "Detached".into(),
    }])
    .unwrap();
    f.run(vec![Command::DeletePrefabInstance {
        scene_id: f.scene.clone(),
        root_entity_id: br,
    }])
    .unwrap();
    f.run(vec![Command::DeletePrefab {
        prefab_id: f.prefab.clone(),
    }])
    .unwrap();
}
#[test]
fn invalid_commands_roll_back_document_crdt_history_revision_and_journal() {
    let mut f = Fixture::new();
    f.create();
    let a = f.instance();
    let node = f.nodes[&f.source].clone();
    let ar = a[&node].clone();
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("journal");
    let initial = f.bus.project().clone();
    f.bus = CommandBus::persistent(&journal, initial.clone()).unwrap();
    let mut bad = f.bus.project().prefabs[&f.prefab].clone();
    bad.nodes.remove(&node);
    let mut alias = a.clone();
    for e in alias.values_mut() {
        *e = ar.clone();
    }
    let mut foreign = Entity::new("Foreign");
    foreign.parent = Some(ar.clone());
    let invalid = vec![
        Command::UpdatePrefab { prefab: bad },
        Command::InstantiatePrefab {
            source: f.prefab.clone(),
            scene_id: f.scene.clone(),
            parent: None,
            entities: alias,
        },
        Command::RenameEntity {
            scene_id: f.scene.clone(),
            entity_id: ar.clone(),
            name: "Bad".into(),
        },
        Command::ReparentEntity {
            scene_id: f.scene.clone(),
            entity_id: a[&f.nodes[&f.child]].clone(),
            parent: None,
        },
        Command::CreateEntity {
            scene_id: f.scene.clone(),
            entity: foreign,
        },
        Command::DeleteEntity {
            scene_id: f.scene.clone(),
            entity_id: a[&f.nodes[&f.child]].clone(),
        },
        Command::DeletePrefab {
            prefab_id: f.prefab.clone(),
        },
        Command::SetComponent {
            scene_id: f.scene.clone(),
            entity_id: ar,
            component: "Transform".into(),
            value: json!({"translation":[1,2]}),
        },
    ];
    for command in invalid {
        let before = f.bus.project().clone();
        let crdt = f.bus.export_crdt().unwrap();
        let revision = f.bus.revision();
        let history = f.bus.history().to_vec();
        let count = history.len();
        // Windows enforces the journal lock for reads as well. No ephemeral
        // project-file review is involved in this ordinary command fixture.
        drop(f.bus);
        let log = std::fs::read(&journal).unwrap();
        f.bus = CommandBus::persistent(&journal, initial.clone()).unwrap();
        assert_eq!(f.bus.project(), &before);
        assert_eq!(f.bus.export_crdt().unwrap(), crdt);
        assert_eq!(f.bus.revision(), revision);
        assert_eq!(f.bus.history(), history);
        assert!(
            f.run(vec![
                Command::SetMemory {
                    section: "would change".into(),
                    text: "never".into()
                },
                command
            ])
            .is_err()
        );
        assert_eq!(f.bus.project(), &before);
        assert_eq!(f.bus.export_crdt().unwrap(), crdt);
        assert_eq!(f.bus.revision(), revision);
        assert_eq!(f.bus.history().len(), count);
        assert_eq!(f.bus.history(), history);
        drop(f.bus);
        assert_eq!(std::fs::read(&journal).unwrap(), log);
        f.bus = CommandBus::persistent(&journal, initial.clone()).unwrap();
        assert_eq!(f.bus.project(), &before);
        assert_eq!(f.bus.export_crdt().unwrap(), crdt);
        assert_eq!(f.bus.revision(), revision);
        assert_eq!(f.bus.history(), history);
    }
}
#[test]
fn identity_replay_reopen_undo_redo_preserve_exact_links() {
    let mut f = Fixture::new();
    let initial = f.bus.project().clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("journal");
    f.bus = CommandBus::persistent(&path, initial.clone()).unwrap();
    f.create();
    f.instance();
    f.instance();
    let final_state = f.bus.project().clone();
    let transactions = f.bus.history().to_vec();
    let text = final_state.canonical_text().unwrap();
    assert_eq!(Project::from_text(&text).unwrap(), final_state);
    for _ in 0..3 {
        f.bus.undo().unwrap();
    }
    assert_eq!(f.bus.project(), &initial);
    for _ in 0..3 {
        f.bus.redo().unwrap();
    }
    assert_eq!(f.bus.project(), &final_state);
    drop(f.bus);
    let mut reopened = CommandBus::persistent(&path, initial.clone()).unwrap();
    assert_eq!(reopened.project(), &final_state);
    reopened.undo().unwrap();
    reopened.redo().unwrap();
    assert_eq!(reopened.project(), &final_state);
    // Re-execution uses exactly caller-supplied IDs; provenance transaction IDs differ.
    let mut replay = CommandBus::new(initial).unwrap();
    for tx in transactions {
        replay
            .execute(tx.commands, tx.actor, tx.description, None)
            .unwrap();
    }
    let mut a = replay.project().clone();
    let mut b = final_state;
    for p in [&mut a, &mut b] {
        for x in p.prefabs.values_mut() {
            x.provenance = None;
        }
        for e in p.scenes.values_mut().flat_map(|s| s.entities.values_mut()) {
            e.provenance = None;
        }
    }
    assert_eq!(a, b);
}
#[test]
fn expansion_budget_is_cumulative_across_commands_and_rolls_back() {
    let mut f = Fixture::new();
    f.create();
    let before = f.bus.project().clone();
    let commands = (0..5001)
        .map(|_| Command::InstantiatePrefab {
            source: f.prefab.clone(),
            scene_id: f.scene.clone(),
            parent: None,
            entities: f.nodes.values().map(|id| (id.clone(), new_id())).collect(),
        })
        .collect();
    assert!(matches!(
        f.run(commands),
        Err(CommandError::Prefab(
            incant_doc::prefab::PrefabError::Limit(_)
        ))
    ));
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.history().len(), 1);
}
#[test]
fn component_removal_is_an_override_even_if_prototype_later_adds_it() {
    let mut f = Fixture::new();
    f.create();
    let a = f.instance();
    let node = f.nodes[&f.source].clone();
    let eid = a[&node].clone();
    f.run(vec![Command::RemoveComponent {
        scene_id: f.scene.clone(),
        entity_id: eid.clone(),
        component: "Velocity".into(),
    }])
    .unwrap();
    let mut p = f.bus.project().prefabs[&f.prefab].clone();
    p.nodes
        .get_mut(&node)
        .unwrap()
        .components
        .remove("Velocity");
    f.run(vec![Command::UpdatePrefab { prefab: p.clone() }])
        .unwrap();
    p.nodes
        .get_mut(&node)
        .unwrap()
        .components
        .insert("Velocity".into(), json!({"linear":[7,0,0]}));
    f.run(vec![Command::UpdatePrefab { prefab: p }]).unwrap();
    assert!(
        !f.bus.project().scenes[&f.scene].entities[&eid]
            .components
            .contains_key("Velocity")
    );
}
#[test]
fn typed_audio_references_remap_and_external_reference_refuses() {
    let mut f = Fixture::new();
    let mut p = f.bus.project().clone();
    let s = p.scenes.get_mut(&f.scene).unwrap();
    s.entities
        .get_mut(&f.source)
        .unwrap()
        .components
        .insert("AudioBus".into(), json!({"parent":null,"gain_db":0}));
    s.entities
        .get_mut(&f.child)
        .unwrap()
        .components
        .insert("AudioBus".into(), json!({"parent":f.source,"gain_db":0}));
    f.bus = CommandBus::new(p).unwrap();
    f.create();
    let a = f.instance();
    let b = f.instance();
    for map in [&a, &b] {
        let child = &f.bus.project().scenes[&f.scene].entities[&map[&f.nodes[&f.child]]];
        assert_eq!(
            child.components["AudioBus"]["parent"],
            map[&f.nodes[&f.source]]
        );
    }
    let mut g = Fixture::new();
    let mut p = g.bus.project().clone();
    let mut outside = Entity::new("Outside bus");
    outside
        .components
        .insert("AudioBus".into(), json!({"parent":null,"gain_db":0}));
    let s = p.scenes.get_mut(&g.scene).unwrap();
    s.entities
        .get_mut(&g.source)
        .unwrap()
        .components
        .insert("AudioBus".into(), json!({"parent":outside.id,"gain_db":0}));
    s.entities.insert(outside.id.clone(), outside);
    g.bus = CommandBus::new(p).unwrap();
    let before = g.bus.project().clone();
    assert!(g.run(vec![g.create_command()]).is_err());
    assert_eq!(g.bus.project(), &before);
}
#[test]
fn oversized_mapping_refuses_before_project_mutation() {
    let mut f = Fixture::new();
    let before = f.bus.project().clone();
    let nodes: BTreeMap<_, _> = (0..1025).map(|_| (new_id(), new_id())).collect();
    assert!(
        f.run(vec![Command::CreatePrefab {
            scene_id: f.scene.clone(),
            root_entity_id: f.source.clone(),
            prefab_id: f.prefab.clone(),
            name: "Too big".into(),
            nodes
        }])
        .is_err()
    );
    assert_eq!(f.bus.project(), &before);
}

#[test]
fn payload_limit_and_global_identity_alias_refuse_atomically() {
    let mut f = Fixture::new();
    let before = f.bus.project().clone();
    let mut alias = f.nodes.clone();
    alias.insert(f.source.clone(), f.source.clone());
    assert!(
        f.run(vec![Command::CreatePrefab {
            scene_id: f.scene.clone(),
            root_entity_id: f.source.clone(),
            prefab_id: f.prefab.clone(),
            name: "Alias".into(),
            nodes: alias
        }])
        .is_err()
    );
    assert_eq!(f.bus.project(), &before);
    let name = "x".repeat(incant_doc::prefab::MAX_PREFAB_BYTES / 6 + 1);
    assert!(matches!(
        f.run(vec![Command::CreatePrefab {
            scene_id: f.scene.clone(),
            root_entity_id: f.source.clone(),
            prefab_id: f.prefab.clone(),
            name,
            nodes: f.nodes.clone()
        }]),
        Err(CommandError::Prefab(
            incant_doc::prefab::PrefabError::Limit(_)
        ))
    ));
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.revision(), 0);
}

#[test]
fn deleting_prototypes_charges_cumulative_node_budget() {
    use incant_doc::{Prefab, PrefabNode};
    let mut p = Project::empty("Deletion budget");
    let mut commands = Vec::new();
    for _ in 0..11 {
        let id = new_id();
        let root = new_id();
        let mut nodes = BTreeMap::new();
        nodes.insert(
            root.clone(),
            PrefabNode {
                name: "Root".into(),
                parent: None,
                components: BTreeMap::new(),
            },
        );
        for _ in 1..1000 {
            nodes.insert(
                new_id(),
                PrefabNode {
                    name: "Child".into(),
                    parent: Some(root.clone()),
                    components: BTreeMap::new(),
                },
            );
        }
        p.prefabs.insert(
            id.clone(),
            Prefab {
                id: id.clone(),
                name: "Prototype".into(),
                root,
                nodes,
                provenance: None,
            },
        );
        commands.push(Command::DeletePrefab { prefab_id: id });
    }
    let mut bus = CommandBus::new(p.clone()).unwrap();
    assert!(matches!(
        bus.execute(
            commands,
            incant_cmd::Actor::user("test"),
            "Refuse over budget",
            Some(0)
        ),
        Err(CommandError::Prefab(
            incant_doc::prefab::PrefabError::Limit(_)
        ))
    ));
    assert_eq!(bus.project(), &p);
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
}
#[test]
fn deleting_scenes_charges_linked_nodes_but_leaves_unlinked_behavior_unchanged() {
    use incant_doc::{Prefab, PrefabLink, PrefabNode, Scene};
    use std::collections::BTreeSet;
    let mut p = Project::empty("Scene deletion budget");
    let id = new_id();
    let node = new_id();
    p.prefabs.insert(
        id.clone(),
        Prefab {
            id: id.clone(),
            name: "Prototype".into(),
            root: node.clone(),
            provenance: None,
            nodes: BTreeMap::from([(
                node.clone(),
                PrefabNode {
                    name: "Node".into(),
                    parent: None,
                    components: BTreeMap::new(),
                },
            )]),
        },
    );
    let mut commands = Vec::new();
    for _ in 0..3 {
        let mut s = Scene::new("World");
        for _ in 0..4000 {
            let mut e = Entity::new("Node");
            e.prefab = Some(PrefabLink {
                source: id.clone(),
                root: e.id.clone(),
                node: node.clone(),
                overrides: BTreeSet::new(),
            });
            s.entities.insert(e.id.clone(), e);
        }
        commands.push(Command::DeleteScene {
            scene_id: s.id.clone(),
        });
        p.scenes.insert(s.id.clone(), s);
    }
    let mut bus = CommandBus::new(p.clone()).unwrap();
    assert!(matches!(
        bus.execute(
            commands.clone(),
            incant_cmd::Actor::user("test"),
            "Refuse over budget",
            Some(0)
        ),
        Err(CommandError::Prefab(
            incant_doc::prefab::PrefabError::Limit(_)
        ))
    ));
    assert_eq!(bus.project(), &p);
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
    for e in p.scenes.values_mut().flat_map(|s| s.entities.values_mut()) {
        e.prefab = None;
    }
    p.prefabs.clear();
    let mut plain = CommandBus::new(p).unwrap();
    plain
        .execute(
            commands,
            incant_cmd::Actor::user("test"),
            "Ordinary deletion",
            Some(0),
        )
        .unwrap();
    assert!(plain.project().scenes.is_empty());
}

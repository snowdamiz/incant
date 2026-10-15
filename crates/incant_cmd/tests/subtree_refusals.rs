#[path = "support/prefab.rs"]
mod prefabs;
#[path = "support/subtree.rs"]
mod support;
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Scene, new_id};
use serde_json::json;
use std::collections::BTreeMap;
use support::Fixture;

fn refused(f: &mut Fixture, commands: Vec<Command>) -> String {
    let before = f.bus.project().clone();
    let revision = f.bus.revision();
    let history = f.bus.history().to_vec();
    let redo: Vec<_> = f.bus.redo_history().cloned().collect();
    let crdt = f.bus.export_crdt().unwrap();
    let error = f.run(commands).unwrap_err().to_string();
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.revision(), revision);
    assert_eq!(f.bus.history(), history);
    assert_eq!(f.bus.redo_history().cloned().collect::<Vec<_>>(), redo);
    assert_eq!(f.bus.export_crdt().unwrap(), crdt);
    error
}

#[test]
fn exact_map_and_global_identity_refusals_do_not_commit_any_prefix() {
    let mut f = Fixture::new();
    let mut variants = vec![BTreeMap::new()];
    let mut missing = f.mapping.clone();
    missing.remove(&f.leaf);
    variants.push(missing);
    let mut missing_root = f.mapping.clone();
    missing_root.remove(&f.root);
    variants.push(missing_root);
    let mut extra = f.mapping.clone();
    extra.insert(f.outside.clone(), new_id());
    variants.push(extra);
    let mut absent = f.mapping.clone();
    absent.insert(new_id(), new_id());
    variants.push(absent);
    for target in [
        f.mapping[&f.child].clone(),
        "not-an-id".into(),
        "01aaaaaaaaaaaaaaaaaaaaaaaa".into(),
        f.root.clone(),
        f.parent.clone(),
        f.scene.clone(),
        f.bus.project().id.clone(),
    ] {
        let mut map = f.mapping.clone();
        map.insert(f.root.clone(), target);
        variants.push(map);
    }
    for map in variants {
        let command = f.with_map(map);
        refused(
            &mut f,
            vec![
                Command::SetMemory {
                    section: "temporary".into(),
                    text: "must roll back".into(),
                },
                command,
            ],
        );
    }
    let bad = Command::DuplicateSubtree {
        scene_id: new_id(),
        root_entity_id: f.root.clone(),
        entities: f.mapping.clone(),
    };
    refused(&mut f, vec![bad]);
    let bad = Command::DuplicateSubtree {
        scene_id: f.scene.clone(),
        root_entity_id: new_id(),
        entities: f.mapping.clone(),
    };
    refused(&mut f, vec![bad]);
    // Freshness is checked at this operation, not only against the final document.
    let mut another = Scene::new("Other");
    let other = Entity::new("Other scene entity");
    let oid = other.id.clone();
    another.entities.insert(oid.clone(), other);
    f.run(vec![Command::CreateScene {
        scene: another.clone(),
    }])
    .unwrap();
    let mut collision = f.mapping.clone();
    collision.insert(f.root.clone(), oid.clone());
    let duplicate = f.with_map(collision);
    refused(
        &mut f,
        vec![
            duplicate,
            Command::DeleteEntity {
                scene_id: another.id,
                entity_id: oid,
            },
        ],
    );
}

#[test]
fn cycles_and_disconnected_maps_are_refused_before_final_validation() {
    let mut f = Fixture::new();
    let duplicate = f.command();
    let cycle = Command::ReparentEntity {
        scene_id: f.scene.clone(),
        entity_id: f.root.clone(),
        parent: Some(f.leaf.clone()),
    };
    refused(&mut f, vec![cycle, duplicate]);
    let duplicate = f.command();
    let a = Command::ReparentEntity {
        scene_id: f.scene.clone(),
        entity_id: f.child.clone(),
        parent: Some(f.leaf.clone()),
    };
    refused(&mut f, vec![a, duplicate]);
}

#[test]
fn complete_1024_node_subtree_is_admitted_but_omitted_descendant_is_not() {
    let mut f = Fixture::new();
    let mut project = f.bus.project().clone();
    let scene = project.scenes.get_mut(&f.scene).unwrap();
    for _ in 3..1024 {
        let mut entity = Entity::new("Child");
        entity.parent = Some(f.root.clone());
        f.mapping.insert(entity.id.clone(), new_id());
        scene.entities.insert(entity.id.clone(), entity);
    }
    f.bus = CommandBus::new(project).unwrap();
    f.run(vec![f.command()]).unwrap();
    assert_eq!(f.bus.project().scenes[&f.scene].entities.len(), 2050);
    f.bus.undo().unwrap();
    let mut extra = Entity::new("Omitted descendant");
    extra.parent = Some(f.child.clone());
    f.run(vec![Command::CreateEntity {
        scene_id: f.scene.clone(),
        entity: extra,
    }])
    .unwrap();
    let command = f.command();
    assert!(refused(&mut f, vec![command]).contains("complete subtree"));
}

#[test]
fn singleton_listener_environment_and_ui_root_refuse_atomically() {
    let mut f = Fixture::new();
    f.run(vec![f.set(&f.root, "AudioListener", json!({}))])
        .unwrap();
    let command = f.command();
    assert!(refused(&mut f, vec![command]).contains("singleton"));
    let mut f = Fixture::new();
    let texture = new_id();
    f.run(vec![
        Command::UpsertAsset {
            asset: incant_doc::Asset {
                id: texture.clone(),
                name: "Sky".into(),
                path: "assets/sky.png".into(),
                kind: "texture".into(),
                sha256: "0".repeat(64),
                import_settings: None,
            },
        },
        f.set(
            &f.root,
            "EnvironmentLight",
            json!({"texture":texture,"intensity":1,"rotation_degrees":0}),
        ),
    ])
    .unwrap();
    let command = f.command();
    assert!(refused(&mut f, vec![command]).contains("singleton"));
    let mut f = Fixture::new();
    let ui = |role| {
        json!({"kind":{"role":role},"order":0,"disabled":false,"style":{
        "size":["auto","auto"],"min_size":[0,0],"max_size":[null,null],"padding":[0,0,0,0],
        "gap":[0,0],"direction":"column","wrap":false,"align":"stretch","justify":"start",
        "grow":0,"shrink":1,"basis":"auto"}})
    };
    f.run(vec![
        Command::ReparentEntity {
            scene_id: f.scene.clone(),
            entity_id: f.root.clone(),
            parent: None,
        },
        Command::RemoveComponent {
            scene_id: f.scene.clone(),
            entity_id: f.root.clone(),
            component: "Velocity".into(),
        },
        f.set(&f.root, "UiNode", ui("root")),
        f.set(&f.child, "UiNode", ui("container")),
        f.set(&f.leaf, "UiNode", ui("container")),
    ])
    .unwrap();
    let command = f.command();
    assert!(refused(&mut f, vec![command]).contains("singleton"));
}

#[test]
fn asset_script_table_and_noncanonical_existing_id_collisions_refuse() {
    let mut f = Fixture::new();
    let asset = new_id();
    let script = new_id();
    let table = new_id();
    f.run(vec![
        Command::UpsertAsset {
            asset: incant_doc::Asset {
                id: asset.clone(),
                name: "Image".into(),
                path: "assets/image.png".into(),
                kind: "texture".into(),
                sha256: "0".repeat(64),
                import_settings: None,
            },
        },
        Command::UpsertScript {
            script: incant_doc::ScriptSource {
                id: script.clone(),
                name: "Code".into(),
                path: "scripts/code.ts".into(),
            },
        },
        Command::UpsertStringTable {
            table: incant_doc::StringTable {
                id: table.clone(),
                name: "Words".into(),
                source_locale: "en".into(),
                messages: BTreeMap::new(),
            },
        },
    ])
    .unwrap();
    for target in [asset, script, table] {
        let mut map = f.mapping.clone();
        map.insert(f.root.clone(), target);
        let command = f.with_map(map);
        assert!(refused(&mut f, vec![command]).contains("globally fresh"));
    }
    let mut project = f.bus.project().clone();
    let canonical = project.id.clone();
    project.id = project.id.to_lowercase();
    f.bus = CommandBus::new(project).unwrap();
    let mut map = f.mapping.clone();
    map.insert(f.root.clone(), canonical);
    let command = f.with_map(map);
    assert!(refused(&mut f, vec![command]).contains("globally fresh"));
}

#[test]
fn linked_subtree_requires_explicit_detachment_and_cannot_alias_prototype_ids() {
    let mut f = prefabs::Fixture::new();
    f.create();
    let instance = f.instance();
    let root = instance[&f.nodes[&f.source]].clone();
    let mapping: BTreeMap<_, _> = instance.values().map(|id| (id.clone(), new_id())).collect();
    let duplicate = Command::DuplicateSubtree {
        scene_id: f.scene.clone(),
        root_entity_id: root.clone(),
        entities: mapping.clone(),
    };
    let before = f.bus.project().clone();
    assert!(
        f.run(vec![duplicate.clone()])
            .unwrap_err()
            .to_string()
            .contains("detach")
    );
    assert_eq!(f.bus.project(), &before);
    for target in [f.prefab.clone(), f.nodes[&f.source].clone()] {
        let mut ids = BTreeMap::from([(f.source.clone(), target), (f.child.clone(), new_id())]);
        let command = Command::DuplicateSubtree {
            scene_id: f.scene.clone(),
            root_entity_id: f.source.clone(),
            entities: std::mem::take(&mut ids),
        };
        assert!(
            f.run(vec![command])
                .unwrap_err()
                .to_string()
                .contains("globally fresh")
        );
    }
    f.run(vec![
        Command::DetachPrefab {
            scene_id: f.scene.clone(),
            root_entity_id: root,
        },
        duplicate,
    ])
    .unwrap();
    assert!(mapping.values().all(|id| {
        f.bus.project().scenes[&f.scene].entities[id]
            .prefab
            .is_none()
    }));
}

#[test]
fn duplication_and_prefab_expansion_share_the_cumulative_node_limit() {
    let mut f = prefabs::Fixture::new();
    f.create();
    let initial = f.bus.project().clone();
    let mut commands = Vec::new();
    // 5,000 two-node operations reach 10,000; the extra ordinary copy refuses
    // the whole transaction even though each expansion family alone is below it.
    for index in 0..5001 {
        commands.push(if index % 2 == 0 {
            Command::DuplicateSubtree {
                scene_id: f.scene.clone(),
                root_entity_id: f.source.clone(),
                entities: BTreeMap::from([
                    (f.source.clone(), new_id()),
                    (f.child.clone(), new_id()),
                ]),
            }
        } else {
            Command::InstantiatePrefab {
                source: f.prefab.clone(),
                scene_id: f.scene.clone(),
                parent: None,
                entities: f.nodes.values().map(|id| (id.clone(), new_id())).collect(),
            }
        });
    }
    let revision = f.bus.revision();
    assert!(
        f.run(commands)
            .unwrap_err()
            .to_string()
            .contains("10000 affected nodes")
    );
    assert_eq!(f.bus.project(), &initial);
    assert_eq!(f.bus.revision(), revision);
}

#[test]
fn payload_and_mapping_bounds_refuse_without_output() {
    let mut f = Fixture::new();
    let mut project = f.bus.project().clone();
    project
        .scenes
        .get_mut(&f.scene)
        .unwrap()
        .entities
        .get_mut(&f.root)
        .unwrap()
        .name = "x".repeat(3 * 1024 * 1024);
    f.bus = CommandBus::new(project).unwrap();
    let other = f.mapping.keys().map(|id| (id.clone(), new_id())).collect();
    let commands = vec![f.command(), f.with_map(other)];
    assert!(refused(&mut f, commands).contains("32 MiB payload"));
    let mut f = Fixture::new();
    let map = (0..1025).map(|_| (new_id(), new_id())).collect();
    let command = f.with_map(map);
    assert!(refused(&mut f, vec![command]).contains("1..1024"));
}

#[test]
fn refusal_keeps_persistent_journal_and_redo_history_unchanged() {
    let mut f = Fixture::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history");
    let initial = f.bus.project().clone();
    f.bus = CommandBus::persistent(&path, initial.clone()).unwrap();
    f.run(vec![Command::SetMemory {
        section: "undo me".into(),
        text: "text".into(),
    }])
    .unwrap();
    f.bus.undo().unwrap();
    drop(f.bus);
    let bytes = std::fs::read(&path).unwrap();
    f.bus = CommandBus::persistent(&path, initial.clone()).unwrap();
    let command = f.command();
    let failure = Command::RenameEntity {
        scene_id: f.scene.clone(),
        entity_id: new_id(),
        name: "absent".into(),
    };
    refused(&mut f, vec![command, failure]);
    drop(f.bus);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let mut bus = CommandBus::persistent(&path, initial).unwrap();
    bus.redo().unwrap();
    assert_eq!(bus.project().memory["undo me"], "text");
    let command = Command::DuplicateSubtree {
        scene_id: f.scene,
        root_entity_id: f.root,
        entities: f.mapping,
    };
    assert!(
        bus.execute(vec![command], Actor::user("stale"), "stale", Some(0))
            .is_err()
    );
}

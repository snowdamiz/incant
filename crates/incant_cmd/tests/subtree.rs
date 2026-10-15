#[path = "support/subtree.rs"]
mod support;
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Asset, Collider, Entity, Project, ScriptSource, new_id};
use serde_json::json;
use support::Fixture;

#[test]
fn prepared_identity_map_is_fresh_complete_and_does_not_edit_until_execute() {
    let mut f = Fixture::new();
    let before = f.bus.project().clone();
    let (command, created) = f.bus.prepare_duplicate_subtree(&f.scene, &f.root).unwrap();
    let (again, second) = f.bus.prepare_duplicate_subtree(&f.scene, &f.root).unwrap();
    assert_ne!(created, second);
    assert_ne!(command, again);
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.revision(), 0);
    let Command::DuplicateSubtree { entities, .. } = &command else {
        panic!("wrong command")
    };
    assert_eq!(
        entities.keys().collect::<Vec<_>>(),
        f.mapping.keys().collect::<Vec<_>>()
    );
    assert_eq!(entities[&f.root], created);
    assert!(
        f.bus
            .prepare_duplicate_subtree(&f.scene, &new_id())
            .is_err()
    );
    f.run(vec![command]).unwrap();
    assert_eq!(f.bus.project().scenes[&f.scene].entities.len(), 8);
    f.bus.undo().unwrap();
    assert_eq!(f.bus.project(), &before);
}

#[test]
fn preparation_refuses_an_oversized_frontier_without_editing() {
    let f = Fixture::new();
    let mut project = f.bus.project().clone();
    for _ in 0..1024 {
        let mut child = Entity::new("Extra child");
        child.parent = Some(f.root.clone());
        project
            .scenes
            .get_mut(&f.scene)
            .unwrap()
            .entities
            .insert(child.id.clone(), child);
    }
    let bus = CommandBus::new(project.clone()).unwrap();
    assert!(bus.prepare_duplicate_subtree(&f.scene, &f.root).is_err());
    assert_eq!(bus.project(), &project);
    assert_eq!(bus.revision(), 0);
}

#[test]
fn nested_copy_preserves_sources_reparents_children_and_records_provenance() {
    let mut f = Fixture::new();
    let before = f.bus.project().clone();
    f.run(vec![f.command()]).unwrap();
    let tx = &f.bus.history()[0];
    let scene = &f.bus.project().scenes[&f.scene];
    assert_eq!(scene.entities.len(), 8);
    for (source, target) in &f.mapping {
        assert_eq!(
            scene.entities[source],
            before.scenes[&f.scene].entities[source]
        );
        let mut expected = scene.entities[source].clone();
        expected.id = target.clone();
        if source != &f.root {
            expected.parent = Some(f.mapping[expected.parent.as_ref().unwrap()].clone());
        }
        expected.provenance = scene.entities[target].provenance.clone();
        assert_eq!(scene.entities[target], expected);
        let p = expected.provenance.unwrap();
        assert_eq!(p.transaction, tx.id);
        assert_eq!(p.actor, tx.actor.actor);
        assert_eq!(p.model.as_deref(), Some("subtree-test"));
    }
    assert_eq!(
        scene.entities[&f.mapping[&f.root]].parent.as_ref(),
        Some(&f.parent)
    );
    assert_eq!(
        scene.entities[&f.outside],
        before.scenes[&f.scene].entities[&f.outside]
    );
    let after = f.bus.project().clone();
    f.bus.undo().unwrap();
    assert_eq!(f.bus.project(), &before);
    f.bus.redo().unwrap();
    assert_eq!(f.bus.project(), &after);
}

#[test]
fn actual_typed_audio_and_navigation_edges_remap_internal_only() {
    let mut f = Fixture::new();
    let clip = new_id();
    let mesh = new_id();
    let script = new_id();
    let mut internal_source = Entity::new("Internal bus source");
    internal_source.parent = Some(f.root.clone());
    let mut external_source = Entity::new("External bus source");
    external_source.parent = Some(f.root.clone());
    let audio = |bus: &str| {
        json!({"clip":clip,"bus":bus,"gain_db":0,"pan":0,"rate":1,
        "start_seconds":0,"playing":false,"looping":false,"streaming":false})
    };
    internal_source
        .components
        .insert("AudioSource".into(), audio(&f.child));
    external_source
        .components
        .insert("AudioSource".into(), audio(&f.outside));
    for entity in [&internal_source, &external_source] {
        f.mapping.insert(entity.id.clone(), new_id());
    }
    f.run(vec![
        Command::RemoveComponent {
            scene_id: f.scene.clone(),
            entity_id: f.root.clone(),
            component: "Velocity".into(),
        },
        Command::UpsertAsset {
            asset: Asset {
                id: clip.clone(),
                name: "Clip".into(),
                path: "assets/clip.wav".into(),
                kind: "audio".into(),
                sha256: "0".repeat(64),
                import_settings: None,
            },
        },
        Command::UpsertScript {
            script: ScriptSource {
                id: script.clone(),
                name: "Behavior".into(),
                path: "scripts/behavior.ts".into(),
            },
        },
        Command::UpsertAsset {
            asset: Asset {
                id: mesh.clone(),
                name: "Navigation mesh source".into(),
                path: "assets/source.glb".into(),
                kind: "model".into(),
                sha256: "0".repeat(64),
                import_settings: None,
            },
        },
        f.set(&f.outside, "AudioBus", json!({"gain_db":0})),
        f.set(&f.root, "AudioBus", json!({"parent":f.outside,"gain_db":0})),
        f.set(&f.child, "AudioBus", json!({"parent":f.root,"gain_db":0})),
        f.set(
            &f.child,
            "MeshRenderer",
            json!({"mesh":mesh,"materials":[],"cast_shadows":true}),
        ),
        f.set(
            &f.outside,
            "Transform",
            json!(incant_doc::Transform::default()),
        ),
        f.set(&f.outside, "Collider", json!(Collider::default())),
        f.set(
            &f.leaf,
            "Script",
            json!({"source":script,"props":{"target":f.child,"nested":[f.root]}}),
        ),
        f.set(
            &f.leaf,
            "NavigationMesh",
            json!({"settings":{"min":[-10,-10,-10],"max":[10,10,10],
            "cell_size":0.25,"cell_height":0.25,"tile_cells":32,"agent_radius":0.25,
            "agent_height":1,"max_climb":0.25,"max_slope_degrees":45},
            "sources":[{"entity":f.child,"geometry":"mesh"},
                {"entity":f.outside,"geometry":"collider"}]}),
        ),
        Command::CreateEntity {
            scene_id: f.scene.clone(),
            entity: internal_source.clone(),
        },
        Command::CreateEntity {
            scene_id: f.scene.clone(),
            entity: external_source.clone(),
        },
    ])
    .unwrap();
    let before = f.bus.project().clone();
    f.run(vec![f.command()]).unwrap();
    let entities = &f.bus.project().scenes[&f.scene].entities;
    assert_eq!(
        entities[&f.mapping[&f.root]].components["AudioBus"]["parent"],
        f.outside
    );
    assert_eq!(
        entities[&f.mapping[&f.child]].components["AudioBus"]["parent"],
        f.mapping[&f.root]
    );
    assert_eq!(
        entities[&f.mapping[&internal_source.id]].components["AudioSource"]["bus"],
        f.mapping[&f.child]
    );
    assert_eq!(
        entities[&f.mapping[&external_source.id]].components["AudioSource"]["bus"],
        f.outside
    );
    let copied = &entities[&f.mapping[&f.leaf]].components;
    assert_eq!(
        copied["NavigationMesh"]["sources"][0]["entity"],
        f.mapping[&f.child]
    );
    assert_eq!(copied["NavigationMesh"]["sources"][1]["entity"], f.outside);
    assert!(copied["NavigationMesh"].get("links").is_none());
    assert_eq!(
        copied["Script"],
        before.scenes[&f.scene].entities[&f.leaf].components["Script"]
    );
    assert_eq!(f.bus.project().assets, before.assets);
    assert_eq!(f.bus.project().scripts, before.scripts);
}

#[test]
fn commands_apply_in_order_and_reexecution_uses_only_supplied_entity_ids() {
    let mut f = Fixture::new();
    let before = f.bus.project().clone();
    let final_id = f.mapping[&f.leaf].clone();
    let commands = vec![
        f.set(&f.root, "Velocity", json!({"linear":[2,0,0]})),
        f.command(),
        Command::RenameEntity {
            scene_id: f.scene.clone(),
            entity_id: final_id.clone(),
            name: "Copy leaf".into(),
        },
    ];
    let serialized = serde_json::to_value(&commands).unwrap();
    f.run(serde_json::from_value(serialized).unwrap()).unwrap();
    assert_eq!(
        f.bus.project().scenes[&f.scene].entities[&final_id].name,
        "Copy leaf"
    );
    assert_eq!(
        f.bus.project().scenes[&f.scene].entities[&f.mapping[&f.root]].components["Velocity"]["linear"],
        json!([2, 0, 0])
    );
    let mut replay = CommandBus::new(before).unwrap();
    replay
        .execute(
            commands,
            Actor::agent("subtree-test", "conversation"),
            "Replay",
            Some(0),
        )
        .unwrap();
    let mut projects = [f.bus.project().clone(), replay.project().clone()];
    for project in &mut projects {
        for entity in project
            .scenes
            .values_mut()
            .flat_map(|s| s.entities.values_mut())
        {
            entity.provenance = None;
        }
    }
    assert_eq!(projects[0], projects[1]);
}

#[test]
fn host_save_reopen_undo_redo_and_journal_replay_keep_exact_entities() {
    let mut f = Fixture::new();
    let initial = f.bus.project().clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("world.incant");
    let journal = dir.path().join("custom-history.jsonl");
    std::fs::write(&path, initial.canonical_text().unwrap()).unwrap();
    f.bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    f.run(vec![f.command()]).unwrap();
    let after = f.bus.project().clone();
    let transactions = f.bus.history().to_vec();
    f.bus.save_project_file(1, Actor::user("Save")).unwrap();
    assert_eq!(f.bus.history(), transactions);
    assert_eq!(f.bus.revision(), 1);
    drop(f.bus);
    let mut bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    assert_eq!(bus.project(), &after);
    assert_eq!(bus.history(), transactions);
    bus.undo().unwrap();
    assert_eq!(bus.project(), &initial);
    bus.redo().unwrap();
    assert_eq!(bus.project(), &after);
    drop(bus);
    assert_eq!(
        Project::from_text(&std::fs::read_to_string(path).unwrap()).unwrap(),
        after
    );
}

use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Project, Scene, SceneProgram, ScriptSource, new_id};
use incant_import::{ImportRequest, ImportSnapshot};
use incant_wasm_cook::{CapturedModule, project_cache};
use std::{fs, path::Path};
fn wasm(value: i32) -> Vec<u8> {
    wat::parse_str(format!(r#"(module (memory (export "memory") 1) (func (export "incant_abi") (result i32) i32.const 1) (func (export "update") (param i64 f64) i32.const {value} drop))"#)).unwrap()
}
fn prepare(bus: &CommandBus, root: &Path) -> incant_import::PreparedImports {
    ImportSnapshot::capture(bus)
        .prepare(
            root,
            None,
            &[ImportRequest {
                source: "guest.wasm".into(),
                texture_usage: None,
            }],
        )
        .unwrap()
}
#[test]
fn public_import_select_reimport_undo_clear_and_journal_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("guest.wasm"), wasm(1)).unwrap();
    let initial = Project::empty("Module");
    let journal = root.join("project.journal.jsonl");
    let mut bus = CommandBus::persistent(&journal, initial.clone()).unwrap();
    let mut scene = Scene::new("Main");
    let scene_id = scene.id.clone();
    let entity = Entity::new("Mover");
    scene.entities.insert(entity.id.clone(), entity);
    bus.execute(
        vec![Command::CreateScene { scene }],
        Actor::user("test"),
        "Scene",
        None,
    )
    .unwrap();
    let first = prepare(&bus, root)
        .commit(&mut bus, Actor::import("wasm"), "Import")
        .unwrap()
        .imports
        .remove(0);
    let module = first.asset.id.clone();
    let program = Some(SceneProgram::Wasm {
        module: module.clone(),
        instances: Default::default(),
    });
    let command = |program| Command::SetSceneProgram {
        scene_id: scene_id.clone(),
        program,
    };
    bus.execute(
        vec![command(program.clone())],
        Actor::agent("test-model", "conversation"),
        "Select",
        None,
    )
    .unwrap();
    assert_eq!(
        bus.history().last().unwrap().actor.model.as_deref(),
        Some("test-model")
    );
    let selected = bus.project().clone();
    bus.undo().unwrap();
    assert!(bus.project().scenes[&scene_id].program.is_none());
    bus.redo().unwrap();
    assert_eq!(bus.project(), &selected);
    fs::write(root.join("guest.wasm"), wasm(2)).unwrap();
    let changed = prepare(&bus, root)
        .commit(&mut bus, Actor::import("wasm"), "Reimport")
        .unwrap()
        .imports
        .remove(0);
    assert_eq!(module, changed.asset.id);
    assert_ne!(first.asset.sha256, changed.asset.sha256);
    bus.undo().unwrap();
    assert_eq!(bus.project(), &selected);
    bus.redo().unwrap();
    let updated = bus.project().clone();
    bus.execute(vec![command(None)], Actor::user("test"), "Clear", None)
        .unwrap();
    assert!(
        !bus.project()
            .canonical_text()
            .unwrap()
            .contains("\"program\"")
    );
    bus.undo().unwrap();
    assert_eq!(bus.project(), &updated);
    drop(bus);
    let reopened = CommandBus::persistent(&journal, initial).unwrap();
    assert_eq!(reopened.project(), &updated);
    fs::remove_file(root.join("guest.wasm")).unwrap();
    let loaded =
        CapturedModule::load(&project_cache(root, false).unwrap(), &changed.asset.sha256).unwrap();
    assert_eq!(loaded.original(), wasm(2));
}
#[test]
fn stale_malformed_oversized_and_mixed_inputs_leave_document_and_history_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::write(root.join("guest.wasm"), wasm(1)).unwrap();
    let mut bus = CommandBus::new(Project::empty("Module")).unwrap();
    let stale = prepare(&bus, root);
    let scene = Scene::new("Main");
    let scene_id = scene.id.clone();
    bus.execute(
        vec![Command::CreateScene { scene }],
        Actor::user("test"),
        "Scene",
        None,
    )
    .unwrap();
    assert!(
        stale
            .commit(&mut bus, Actor::import("wasm"), "Stale")
            .is_err()
    );
    let first = prepare(&bus, root)
        .commit(&mut bus, Actor::import("wasm"), "Import")
        .unwrap()
        .imports
        .remove(0);
    let before = bus.project().clone();
    let count = bus.history().len();
    let script = ScriptSource {
        id: new_id(),
        name: "Mixed".into(),
        path: "mixed.ts".into(),
    };
    let mut entity = Entity::new("Mixed");
    entity.components.insert(
        "Script".into(),
        serde_json::json!({"source":script.id,"props":{}}),
    );
    assert!(
        bus.execute(
            vec![
                Command::SetSceneProgram {
                    scene_id: scene_id.clone(),
                    program: Some(SceneProgram::Wasm {
                        module: first.asset.id,
                        instances: Default::default()
                    })
                },
                Command::UpsertScript { script },
                Command::CreateEntity { scene_id, entity }
            ],
            Actor::user("test"),
            "Mixed",
            None
        )
        .is_err()
    );
    for bytes in [b"not wasm".to_vec(), vec![0; 1024 * 1024 + 1]] {
        fs::write(root.join("guest.wasm"), bytes).unwrap();
        assert!(
            ImportSnapshot::capture(&bus)
                .prepare(
                    root,
                    None,
                    &[ImportRequest {
                        source: "guest.wasm".into(),
                        texture_usage: None
                    }]
                )
                .is_err()
        );
    }
    assert_eq!(bus.project(), &before);
    assert_eq!(bus.history().len(), count);
}
#[test]
fn cache_identity_metering_and_symlink_tampering_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let captured = CapturedModule::capture(&wasm(1)).unwrap();
    let cache = project_cache(root, true).unwrap();
    captured.store(&cache).unwrap();
    let key = captured.fingerprint();
    let path = cache.join(format!("{key}.wasm.json"));
    let clean = fs::read(&path).unwrap();
    for field in [
        "policy",
        "registry_sha256",
        "input_sha256",
        "prepared_sha256",
    ] {
        let mut value: serde_json::Value = serde_json::from_slice(&clean).unwrap();
        value[field] = "altered".into();
        fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(CapturedModule::load(&cache, &key).is_err());
    }
    let mut value: serde_json::Value = serde_json::from_slice(&clean).unwrap();
    value["prepared"] = serde_json::json!([0, 97, 115, 109, 1, 0, 0, 0]);
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(CapturedModule::load(&cache, &key).is_err());
    // An attacker may recompute untrusted envelope hashes: regenerated metering
    // still has to match the exact original module, not merely its declarations.
    let mut forged: serde_json::Value = serde_json::from_slice(&clean).unwrap();
    let original: Vec<u8> = serde_json::from_value(forged["original"].clone()).unwrap();
    forged["prepared"] = serde_json::to_value(&original).unwrap();
    forged["prepared_sha256"] = incant_assets::sha256(&original).into();
    let forged_key = incant_assets::sha256(
        format!(
            "{}\0{}\0{}\0{}",
            forged["policy"].as_str().unwrap(),
            forged["registry_sha256"].as_str().unwrap(),
            forged["input_sha256"].as_str().unwrap(),
            forged["prepared_sha256"].as_str().unwrap()
        )
        .as_bytes(),
    );
    fs::write(
        cache.join(format!("{forged_key}.wasm.json")),
        serde_json::to_vec(&forged).unwrap(),
    )
    .unwrap();
    let error = CapturedModule::load(&cache, &forged_key).unwrap_err();
    assert!(error.to_string().contains("prepared input mismatch"));
    fs::write(&path, &clean).unwrap();
    assert!(CapturedModule::load(&cache, &"0".repeat(64)).is_err());
    #[cfg(unix)]
    {
        fs::remove_file(&path).unwrap();
        let outside = root.join("outside");
        fs::write(&outside, clean).unwrap();
        std::os::unix::fs::symlink(outside, &path).unwrap();
        assert!(CapturedModule::load(&cache, &key).is_err());
    }
}

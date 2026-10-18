//! Actual CLI publication/reopen boundary. Geometry is covered by import tests.
use base64::{Engine, engine::general_purpose::STANDARD};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Origin, Project, Scene, Transform};
use incant_import::{ImportRequest, ImportSnapshot};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, process::Command as Process};

#[test]
fn alpha_cli_publishes_saved_json_and_reversible_history_and_refuses_without_writes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("game.incant.json");
    let journal = path.with_extension("journal.jsonl");
    let initial = Project::empty("CLI alpha");
    fs::write(&path, initial.canonical_text().unwrap()).unwrap();
    fs::write(root.path().join("mask.png"), STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAYAAAD0In+KAAAADklEQVR4nGP439DwHwQBGngF/R0pXpQAAAAASUVORK5CYII=").unwrap()).unwrap();
    let mut bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    let imported = ImportSnapshot::capture(&bus)
        .prepare(
            root.path(),
            None,
            &[ImportRequest {
                source: "mask.png".into(),
                texture_usage: Some(incant_assets::TextureUsage::Color),
            }],
        )
        .unwrap()
        .commit(&mut bus, Actor::import("fixture"), "Import fixture")
        .unwrap();
    let asset = imported.imports[0].asset.clone();
    let scene = Scene::new("Main");
    let scene_id = scene.id.clone();
    let mut entity = Entity::new("Source sprite");
    let entity_id = entity.id.clone();
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity.components.insert(
        "SpriteRenderer".into(),
        json!({"texture":asset.id,"region":[0,0,2,1],"size":[2,1],"pivot":[0,0],"tint":[1,1,1,1]}),
    );
    bus.execute(
        vec![
            Command::CreateScene { scene },
            Command::CreateEntity {
                scene_id: scene_id.clone(),
                entity,
            },
        ],
        Actor::user("fixture"),
        "Author fixture",
        None,
    )
    .unwrap();
    bus.save_project_file(bus.revision(), Actor::user("fixture"))
        .unwrap();
    let before = bus.project().clone();
    let history_count = bus.history().len();
    let revision = bus.revision();
    drop(bus); // The CLI must own the normal project/journal lock itself.
    fs::remove_file(root.path().join("mask.png")).unwrap();
    let invoke = |selected: &str| {
        Process::new(env!("CARGO_BIN_EXE_incant_headless"))
            .current_dir(root.path())
            .args([
                "collider-from-alpha",
                path.to_str().unwrap(),
                &scene_id,
                selected,
                "--threshold",
                "128",
            ])
            .output()
            .unwrap()
    };
    let output = invoke(&entity_id);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let receipt: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(receipt["revision"], revision + 1);
    let derivation = &receipt["derivation"];
    assert_eq!(derivation["prepared_revision"], revision);
    assert_eq!(derivation["project_id"], before.id);
    assert_eq!(derivation["scene_id"], scene_id);
    assert_eq!(derivation["source_entity"], entity_id);
    assert_eq!(derivation["texture_id"], asset.id);
    assert_eq!(derivation["texture_key"], asset.sha256);
    assert_eq!(derivation["threshold"], 128);
    let ids: BTreeSet<_> = derivation["rectangles"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["entity_id"].as_str().unwrap().to_owned())
        .collect();
    assert!(!ids.is_empty());
    assert_eq!(
        ids.len(),
        derivation["rectangles"].as_array().unwrap().len()
    );
    let saved = Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap();
    let added: BTreeSet<_> = saved.scenes[&scene_id]
        .entities
        .keys()
        .filter(|id| !before.scenes[&scene_id].entities.contains_key(*id))
        .cloned()
        .collect();
    assert_eq!(ids, added);
    assert_eq!(saved.assets, before.assets);
    assert_eq!(
        saved.scenes[&scene_id].entities[&entity_id],
        before.scenes[&scene_id].entities[&entity_id]
    );
    let mut bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    assert_eq!(bus.project(), &saved);
    assert_eq!(bus.revision(), revision + 1);
    assert_eq!(bus.history().len(), history_count + 1);
    let transaction = bus.history().last().unwrap();
    assert_eq!(transaction.actor, Actor::user("cli"));
    assert_eq!(transaction.commands.len(), ids.len());
    for id in &ids {
        let collider = &saved.scenes[&scene_id].entities[id];
        assert!(collider.components.contains_key("Collider2D"));
        let provenance = collider.provenance.as_ref().unwrap();
        assert_eq!(provenance.origin, Origin::User);
        assert_eq!(provenance.actor, "cli");
        assert_eq!(provenance.transaction, transaction.id);
    }
    bus.undo().unwrap();
    assert_eq!(bus.project(), &before);
    bus.redo().unwrap();
    assert_eq!(bus.project(), &saved);
    let accepted_revision = bus.revision();
    let accepted_history = bus.history().to_vec();
    drop(bus);
    let project_bytes = fs::read(&path).unwrap();
    let journal_bytes = fs::read(&journal).unwrap();
    // A generated Collider2D entity is not an admissible source sprite. This
    // reaches the real CLI/bus refusal after opening the same saved project.
    let refused = invoke(ids.first().unwrap());
    assert!(!refused.status.success());
    assert!(refused.stdout.is_empty());
    assert!(String::from_utf8_lossy(&refused.stderr).contains("only Transform and SpriteRenderer"));
    assert_eq!(fs::read(&path).unwrap(), project_bytes);
    assert_eq!(fs::read(&journal).unwrap(), journal_bytes);
    let reopened = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    assert_eq!(reopened.project(), &saved);
    assert_eq!(reopened.revision(), accepted_revision);
    assert_eq!(reopened.history(), accepted_history);
}

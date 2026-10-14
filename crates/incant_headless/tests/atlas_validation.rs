use base64::{Engine, engine::general_purpose::STANDARD};
use incant_cmd::{Actor, Command as Edit, CommandBus};
use incant_doc::{Entity, Project, Scene, Transform};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

const FRAME: &str = "00000000000000000000000001";
fn cli(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_incant_headless"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}
fn success(root: &Path, args: &[&str]) -> Value {
    let output = cli(root, args);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn descriptor() -> Value {
    json!({"version":1,"id":"00000000000000000000000009","maximum_size":[64,64],"padding":2,
        "frames":{(FRAME):{"name":"First","path":"source.png"},
                  "00000000000000000000000002":{"name":"Second","path":"source.png"}}})
}

#[test]
fn cli_reports_human_import_errors_and_structured_source_free_sprite_diagnostics() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    success(root, &["init", "game.incant.json", "--entities", "0"]);
    fs::write(root.join("source.png"), STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAYAAAD0In+KAAAADklEQVR4nGP439DwHwQBGngF/R0pXpQAAAAASUVORK5CYII=").unwrap()).unwrap();
    let source = root.join("sheet.atlas.json");
    let mut bad = descriptor();
    bad["padding"] = json!(0);
    fs::write(&source, bad.to_string()).unwrap();
    let failed = cli(root, &["import", "game.incant.json", "sheet.atlas.json"]);
    assert!(!failed.status.success());
    let error = String::from_utf8_lossy(&failed.stderr);
    assert!(
        error.contains("could not cook sheet.atlas.json: atlas padding must be in 1..16"),
        "{error}"
    );
    assert!(!error.contains("Cook {") && !error.contains("Atlas(Padding)"));
    fs::write(&source, descriptor().to_string()).unwrap();
    let imported = success(root, &["import", "game.incant.json", "sheet.atlas.json"]);
    let texture = imported["asset"]["id"].as_str().unwrap();
    let path = root.join("game.incant.json");
    let journal = path.with_extension("journal.jsonl");
    let mut bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    let mut scene = Scene::new("Sprites");
    let sid = scene.id.clone();
    let mut sprite = Entity::new("Stable frame");
    let eid = sprite.id.clone();
    sprite
        .components
        .insert("Transform".into(), json!(Transform::default()));
    sprite.components.insert("SpriteRenderer".into(), json!({"texture":texture,"region":{"frame":FRAME},"size":[1,1],"pivot":[0.5,0.5],"tint":[1,1,1,1]}));
    let mut raw = Entity::new("Raw atlas address");
    raw.components = sprite.components.clone();
    raw.components.get_mut("SpriteRenderer").unwrap()["region"] = json!([0, 0, 1, 1]);
    scene.entities.insert(raw.id.clone(), raw);
    scene.entities.insert(eid.clone(), sprite);
    bus.execute(
        vec![Edit::CreateScene { scene }],
        Actor::user("cli-test"),
        "Author sprites",
        None,
    )
    .unwrap();
    bus.save_project_file(bus.revision(), Actor::user("cli-test"))
        .unwrap();
    drop(bus);
    let before = fs::read(&path).unwrap();
    let before_journal = fs::read(&journal).unwrap();
    let checked = success(root, &["validate", "game.incant.json"]);
    assert_eq!(checked["valid"], true);
    assert_eq!(checked["diagnostics"][0]["code"], "raw_atlas_region");
    let unchanged = success(root, &["import", "game.incant.json", "sheet.atlas.json"]);
    assert_eq!(unchanged["changed"], false);
    assert_eq!(unchanged["diagnostics"][0]["severity"], "warning");
    let mut dropped = descriptor();
    dropped["frames"].as_object_mut().unwrap().remove(FRAME);
    fs::write(&source, dropped.to_string()).unwrap();
    let failed = cli(root, &["import", "game.incant.json", "sheet.atlas.json"]);
    assert!(!failed.status.success());
    let error = String::from_utf8_lossy(&failed.stderr);
    assert!(
        error.contains("import rejected")
            && error.contains(&eid)
            && error.contains(FRAME)
            && error.contains(texture),
        "{error}"
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    assert_eq!(fs::read(&journal).unwrap(), before_journal);
    fs::remove_file(source).unwrap();
    fs::remove_file(root.join("source.png")).unwrap();
    assert_eq!(success(root, &["validate", "game.incant.json"]), checked);

    // Pure document commands have no filesystem authority. Cached validation must
    // expose their unresolved binding as an error rather than claiming full validity.
    let document = Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap();
    let mut bad_sprite = document.scenes[&sid].entities[&eid].components["SpriteRenderer"].clone();
    bad_sprite["region"] = json!({"frame":"00000000000000000000000003"});
    let mut bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    bus.execute(
        vec![Edit::SetComponent {
            scene_id: sid,
            entity_id: eid.clone(),
            component: "SpriteRenderer".into(),
            value: bad_sprite,
        }],
        Actor::user("cli-test"),
        "Author unresolved reference",
        None,
    )
    .unwrap();
    bus.save_project_file(bus.revision(), Actor::user("cli-test"))
        .unwrap();
    drop(bus);
    let failed = cli(root, &["validate", "game.incant.json"]);
    assert!(!failed.status.success());
    let response: Value = serde_json::from_slice(&failed.stdout).unwrap();
    assert_eq!(response["valid"], false);
    assert!(
        response["diagnostics"]
            .as_array()
            .unwrap()
            .iter()
            .any(|d| d["code"] == "missing_frame" && d["entity_id"] == eid)
    );
}

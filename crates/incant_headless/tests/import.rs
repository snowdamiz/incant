use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::{fs, process::Command};

#[test]
fn cli_import_is_idempotent_and_persists_reimport_history() {
    let temp = tempfile::tempdir().unwrap();
    let project = temp.path().join("game.incant.json");
    let run = |args: &[&str]| -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_incant_headless"))
            .current_dir(temp.path())
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    };
    run(&["init", project.to_str().unwrap(), "--entities", "0"]);
    let write_source = |size: f32| {
        let bytes: Vec<_> = [0_f32, 0., 0., size, 0., 0., 0., 1., 0.]
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect();
        let source = json!({"asset":{"version":"2.0"},"buffers":[{"byteLength":36,"uri":format!("data:application/octet-stream;base64,{}",STANDARD.encode(bytes))}],"bufferViews":[{"buffer":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[size,1,0]}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0});
        fs::write(temp.path().join("triangle.gltf"), source.to_string()).unwrap();
    };
    write_source(1.);
    let args = ["import", project.to_str().unwrap(), "triangle.gltf"];
    let first = run(&args);
    assert_eq!(first["changed"], true);
    assert_eq!(first["cache_hit"], false);
    let repeated = run(&args);
    assert_eq!(repeated["changed"], false);
    assert_eq!(repeated["cache_hit"], true);
    assert_eq!(repeated["revision"], first["revision"]);
    write_source(2.);
    let changed = run(&args);
    assert_eq!(changed["asset"]["id"], first["asset"]["id"]);
    assert_ne!(changed["asset"]["sha256"], first["asset"]["sha256"]);
    let loaded = incant_doc::Project::from_text(&fs::read_to_string(&project).unwrap()).unwrap();
    assert_eq!(loaded.assets.len(), 1);
    let mut bus =
        incant_cmd::CommandBus::persistent(project.with_extension("journal.jsonl"), loaded)
            .unwrap();
    assert_eq!(bus.history().len(), 2);
    bus.undo().unwrap();
    assert_eq!(
        bus.project().assets.values().next().unwrap().sha256,
        first["asset"]["sha256"].as_str().unwrap()
    );
}

#[test]
fn texture_settings_survive_restart_and_cache_rebuild() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let run = |args: &[&str]| -> Value {
        let output = Command::new(env!("CARGO_BIN_EXE_incant_headless"))
            .current_dir(root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        serde_json::from_slice(&output.stdout).unwrap()
    };
    run(&["init", "game.incant.json", "--entities", "0"]);
    fs::write(root.join("normal.png"),STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAYAAAD0In+KAAAADklEQVR4nGP439DwHwQBGngF/R0pXpQAAAAASUVORK5CYII=").unwrap()).unwrap();
    let first = run(&[
        "import",
        "game.incant.json",
        "normal.png",
        "--texture-usage",
        "normal",
    ]);
    assert_eq!(first["asset"]["kind"], "texture");
    assert_eq!(first["asset"]["import_settings"]["usage"], "normal");
    fs::remove_dir_all(root.join(".incant/cache/textures")).unwrap();
    let rebuilt = run(&["import", "game.incant.json", "normal.png"]);
    assert_eq!(rebuilt["changed"], false);
    assert_eq!(rebuilt["cache_hit"], false);
    assert_eq!(rebuilt["details"]["usage"], "normal");
    assert_eq!(rebuilt["asset"]["sha256"], first["asset"]["sha256"]);
    let changed = run(&[
        "import",
        "game.incant.json",
        "normal.png",
        "--texture-usage",
        "color",
    ]);
    assert_eq!(changed["asset"]["id"], first["asset"]["id"]);
    assert_ne!(changed["asset"]["sha256"], first["asset"]["sha256"]);
    let project = root.join("game.incant.json");
    let loaded = incant_doc::Project::from_text(&fs::read_to_string(&project).unwrap()).unwrap();
    let mut bus =
        incant_cmd::CommandBus::persistent(project.with_extension("journal.jsonl"), loaded)
            .unwrap();
    assert_eq!(bus.history().len(), 2);
    bus.undo().unwrap();
    assert_eq!(
        bus.project()
            .assets
            .values()
            .next()
            .unwrap()
            .import_settings,
        Some(incant_doc::AssetImportSettings::Texture {
            usage: incant_doc::TextureUsage::Normal
        })
    );
}

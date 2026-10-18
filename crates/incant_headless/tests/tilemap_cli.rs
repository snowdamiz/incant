//! Real CLI/journal/save boundary. No renderer, window, device or network work.
use base64::{Engine, engine::general_purpose::STANDARD};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Project, Scene, Tilemap, Transform};
use incant_import::{ImportRequest, ImportSnapshot};
use serde_json::{Value, json};
use std::{fs, process::Command as Process};

#[test]
fn tilemap_cli_saves_one_reversible_stroke_noops_and_refuses_without_writes() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("game.incant");
    let journal = path.with_extension("journal.jsonl");
    fs::write(&path, Project::empty("CLI paint").canonical_text().unwrap()).unwrap();
    fs::write(root.path().join("tile.png"),STANDARD.decode("iVBORw0KGgoAAAANSUhEUgAAAAIAAAABCAYAAAD0In+KAAAADklEQVR4nGP439DwHwQBGngF/R0pXpQAAAAASUVORK5CYII=").unwrap()).unwrap();
    let frame = incant_doc::new_id();
    let atlas = incant_doc::new_id();
    fs::write(root.path().join("sheet.atlas.json"),json!({"version":1,"id":atlas,"maximum_size":[32,32],"padding":1,"frames":{frame.clone():{"name":"Tile","path":"tile.png"}}}).to_string()).unwrap();
    let mut bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    let imported = ImportSnapshot::capture(&bus)
        .prepare(
            root.path(),
            None,
            &[ImportRequest {
                source: "sheet.atlas.json".into(),
                texture_usage: None,
            }],
        )
        .unwrap()
        .commit(&mut bus, Actor::import("fixture"), "Atlas")
        .unwrap();
    let texture = imported.imports[0].asset.id.clone();
    let scene = Scene::new("Main");
    let sid = scene.id.clone();
    let mut entity = Entity::new("Map");
    let eid = entity.id.clone();
    let layer = incant_doc::new_id();
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity.components.insert("Tilemap".into(),json!({"tileset":texture,"size":[2,1],"cell_size":[1,1],"layers":[{"id":layer,"cells":[null,null],"solid":true}]}));
    bus.execute(
        vec![
            Command::CreateScene { scene },
            Command::CreateEntity {
                scene_id: sid.clone(),
                entity,
            },
        ],
        Actor::user("fixture"),
        "Map",
        None,
    )
    .unwrap();
    bus.save_project_file(bus.revision(), Actor::user("fixture"))
        .unwrap();
    let before = bus.project().clone();
    let rev = bus.revision();
    let history = bus.history().len();
    drop(bus);
    for name in ["tile.png", "sheet.atlas.json"] {
        fs::remove_file(root.path().join(name)).unwrap();
    }
    let request = root.path().join("stroke.json");
    let value = json!({"scene_id":sid,"entity_id":eid,"layer_id":layer,"expected_revision":rev,"cells":[[1,0],[0,0]],"brush":{"kind":"autotile","action":"paint","frames":vec![frame.clone();16]}});
    fs::write(&request, value.to_string()).unwrap();
    let invoke = || {
        Process::new(env!("CARGO_BIN_EXE_incant_headless"))
            .current_dir(root.path())
            .args([
                "tilemap-paint",
                path.to_str().unwrap(),
                request.to_str().unwrap(),
            ])
            .output()
            .unwrap()
    };
    let out = invoke();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let response: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(response["revision"], rev + 1);
    assert_eq!(response["changed"], true);
    assert_eq!(response["changes"].as_array().unwrap().len(), 2);
    assert_eq!(response["atlas_id"], atlas);
    let after = Project::from_text(&fs::read_to_string(&path).unwrap()).unwrap();
    let map: Tilemap =
        serde_json::from_value(after.scenes[&sid].entities[&eid].components["Tilemap"].clone())
            .unwrap();
    assert_eq!(map.layers[0].cells, vec![Some(frame.clone()); 2]);
    let mut bus = CommandBus::open_project_file_with_journal(&path, &journal).unwrap();
    assert_eq!(bus.project(), &after);
    assert_eq!(bus.history().len(), history + 1);
    assert_eq!(bus.history().last().unwrap().commands.len(), 1);
    assert_eq!(bus.history().last().unwrap().actor, Actor::user("cli"));
    assert_eq!(
        after.scenes[&sid].entities[&eid]
            .provenance
            .as_ref()
            .unwrap()
            .transaction,
        bus.history().last().unwrap().id
    );
    bus.undo().unwrap();
    assert_eq!(bus.project(), &before);
    bus.redo().unwrap();
    assert_eq!(bus.project(), &after);
    let current = bus.revision();
    drop(bus);
    let project_bytes = fs::read(&path).unwrap();
    let journal_bytes = fs::read(&journal).unwrap();
    let mut same = value.clone();
    same["expected_revision"] = json!(current);
    fs::write(&request, same.to_string()).unwrap();
    let out = invoke();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let result: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(result["changed"], false);
    assert_eq!(result["revision"], current);
    assert_eq!(fs::read(&path).unwrap(), project_bytes);
    assert_eq!(fs::read(&journal).unwrap(), journal_bytes);
    for bad in [
        value,
        {
            let mut x = same.clone();
            x["cells"] = json!([[0, 0], [0, 0]]);
            x
        },
        {
            let mut x = same.clone();
            x["brush"]["frames"] = json!([frame]);
            x
        },
        {
            let mut x = same;
            x["root"] = json!("elsewhere");
            x
        },
    ] {
        fs::write(&request, bad.to_string()).unwrap();
        let out = invoke();
        assert!(!out.status.success());
        assert!(out.stdout.is_empty());
        assert_eq!(fs::read(&path).unwrap(), project_bytes);
        assert_eq!(fs::read(&journal).unwrap(), journal_bytes);
    }
    fs::write(&request, vec![b' '; 1024 * 1024 + 1]).unwrap();
    let out = invoke();
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("exceeds 1 MiB"));
    assert_eq!(fs::read(&path).unwrap(), project_bytes);
    assert_eq!(fs::read(&journal).unwrap(), journal_bytes);
}

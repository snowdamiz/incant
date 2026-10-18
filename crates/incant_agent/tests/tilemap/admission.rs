use super::support::{Fixture, TOOL, actor, frame};
use incant_agent::{NoViewport, dispatch, tools};
use incant_cmd::CommandBus;
use incant_doc::Project;
use serde_json::{Value, json};
use std::fs;

#[test]
fn strict_shared_schema_and_invalid_strokes_refuse_before_any_publication() {
    let mut f = Fixture::new();
    let mut host = f.host();
    let before = f.bus.project().clone();
    let history = f.bus.history().to_vec();
    let revision = f.bus.revision();
    let journal = f.journal_bytes();
    let valid = f.args();
    let mut candidates = vec![Value::Null, json!({})];
    for key in [
        "root",
        "cache",
        "source",
        "actor",
        "description",
        "commands",
    ] {
        let mut bad = valid.clone();
        bad[key] = json!("untrusted");
        candidates.push(bad);
        let mut bad = valid.clone();
        bad["brush"][key] = json!("untrusted");
        candidates.push(bad);
    }
    for key in [
        "scene_id",
        "entity_id",
        "layer_id",
        "expected_revision",
        "cells",
        "brush",
    ] {
        let mut bad = valid.clone();
        bad.as_object_mut().unwrap().remove(key);
        candidates.push(bad);
    }
    for key in ["scene_id", "entity_id", "layer_id"] {
        let mut bad = valid.clone();
        bad[key] = json!(incant_doc::new_id());
        candidates.push(bad);
    }
    for cells in [
        json!([]),
        json!([[0, 0], [0, 0]]),
        json!([[3, 0]]),
        json!([[-1, 0]]),
        json!(vec![[0, 0]; 4097]),
    ] {
        let mut bad = valid.clone();
        bad["cells"] = cells;
        candidates.push(bad);
    }
    for brush in [
        json!({"kind":"unknown"}),
        json!({"kind":"paint","frame":frame(17)}),
        json!({"kind":"autotile","action":"paint","frames":vec![frame(0);15]}),
        json!({"kind":"autotile","action":"paint","frames":vec![frame(17);16]}),
    ] {
        let mut bad = valid.clone();
        bad["brush"] = brush;
        candidates.push(bad);
    }
    let mut stale = valid;
    stale["expected_revision"] = json!(revision + 1);
    candidates.push(stale);
    for args in candidates {
        assert!(
            dispatch(&mut f.bus, &mut host, TOOL, args.clone(), actor()).is_err(),
            "{args}"
        );
        assert_eq!(f.bus.project(), &before);
        assert_eq!(f.bus.history(), history);
        assert_eq!(f.bus.revision(), revision);
        assert_eq!(f.journal_bytes(), journal);
    }
    let definition = tools().into_iter().find(|v| v["name"] == TOOL).unwrap();
    assert_eq!(
        definition["parameters"],
        json!(schemars::schema_for!(incant_import::TilemapPaintRequest))
    );
    assert_eq!(definition["parameters"]["additionalProperties"], false);
}

#[test]
fn absent_or_wrong_host_and_missing_cooked_atlas_cannot_publish() {
    let mut f = Fixture::new();
    let args = f.args();
    assert!(dispatch(&mut f.bus, &mut NoViewport, TOOL, args, actor()).is_err());
    let mut host = f.host();
    let mut other = CommandBus::new(Project::empty("Other")).unwrap();
    let mut args = f.args();
    args["expected_revision"] = json!(0);
    assert!(dispatch(&mut other, &mut host, TOOL, args, actor()).is_err());
    assert!(other.history().is_empty());
    fs::remove_dir_all(f.root.path().join(".incant/cache/textures")).unwrap();
    let before = f.bus.project().clone();
    let history = f.bus.history().to_vec();
    let args = f.args();
    assert!(dispatch(&mut f.bus, &mut host, TOOL, args, actor()).is_err());
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.history(), history);
}

#[test]
fn foreign_terrain_erase_refuses_but_explicit_ordinary_erase_is_undoable() {
    let mut f = Fixture::new();
    let mut host = f.host();
    let mut args = f.args();
    args["brush"] = json!({"kind":"paint","frame":frame(16)});
    dispatch(&mut f.bus, &mut host, TOOL, args, actor()).unwrap();
    let before = f.bus.project().clone();
    let history = f.bus.history().to_vec();
    let mut args = f.args();
    args["brush"]["action"] = json!("erase");
    let error = dispatch(&mut f.bus, &mut host, TOOL, args, actor()).unwrap_err();
    assert!(error.to_string().contains("foreign terrain"));
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.history(), history);
    let mut args = f.args();
    args["brush"] = json!({"kind":"erase"});
    assert_eq!(
        dispatch(&mut f.bus, &mut host, TOOL, args, actor()).unwrap()["untrusted_project_data"]["changed"],
        true
    );
    assert!(f.map().layers[0].cells.iter().all(Option::is_none));
    f.bus.undo().unwrap();
    assert_eq!(f.bus.project(), &before);
}

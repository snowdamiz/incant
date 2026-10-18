use super::support::{Fixture, Mock, TOOL, actor, agent, frame};
use incant_agent::{ApprovalMode, dispatch};
use incant_cmd::Actor;
use incant_doc::Origin;
use incant_import::{ImportSnapshot, TilemapPaintRequest};
use serde_json::json;
use std::{fs, sync::atomic::AtomicBool};

#[test]
fn agent_uses_shared_preparation_cooked_atlas_and_exact_durable_history() {
    let mut f = Fixture::new();
    let mut host = f.host();
    for path in ["mask.png", "tiles.atlas.json"] {
        fs::remove_file(f.root.path().join(path)).unwrap();
    }
    let args = f.args();
    let request: TilemapPaintRequest = serde_json::from_value(args.clone()).unwrap();
    let prepared = ImportSnapshot::capture(&f.bus)
        .prepare_tilemap_paint(f.root.path(), None, &request)
        .unwrap();
    let expected = prepared.commands().to_vec();
    let before = f.bus.project().clone();
    let revision = f.bus.revision();
    let history = f.bus.history().len();
    let result = dispatch(&mut f.bus, &mut host, TOOL, args, actor()).unwrap();
    assert_eq!(result["revision"], revision + 1);
    let tx = f.bus.history().last().unwrap();
    assert_eq!(tx.commands, expected);
    assert_eq!(tx.commands.len(), 1);
    assert_eq!(tx.actor, actor());
    assert_eq!(result["transaction_id"], tx.id);
    assert_eq!(f.bus.history().len(), history + 1);
    let data = &result["untrusted_project_data"];
    assert_eq!(data["prepared_revision"], revision);
    assert_eq!(data["scene_id"], f.scene);
    assert_eq!(data["entity_id"], f.entity);
    assert_eq!(data["layer_id"], f.layer);
    assert_eq!(data["changed"], true);
    assert_eq!(
        data["changes"],
        json!([
            {"cell":[0,0],"before":null,"after":frame(2)},
            {"cell":[1,0],"before":null,"after":frame(8)},
        ])
    );
    let map = f.map();
    assert_eq!(
        map.layers[0].cells,
        vec![Some(frame(2)), Some(frame(8)), None, None, None, None]
    );
    assert_eq!(map.layers[1].cells, vec![Some(frame(16)); 6]);
    let after = f.bus.project().clone();
    let entity = &after.scenes[&f.scene].entities[&f.entity];
    assert_eq!(
        entity.components["Transform"],
        before.scenes[&f.scene].entities[&f.entity].components["Transform"]
    );
    assert_eq!(after.assets, before.assets);
    let provenance = entity.provenance.as_ref().unwrap();
    assert_eq!(provenance.origin, Origin::Agent);
    assert_eq!(provenance.model.as_deref(), Some("mock-tilemap-model"));
    assert_eq!(provenance.conversation_id.as_deref(), Some("tilemap-turn"));
    assert_eq!(provenance.transaction, tx.id);
    f.bus.undo().unwrap();
    assert_eq!(f.bus.project(), &before);
    f.bus.redo().unwrap();
    assert_eq!(f.bus.project(), &after);
    f.reopen();
    assert_eq!(f.bus.project(), &after);
    f.bus
        .save_project_file(f.bus.revision(), Actor::user("save"))
        .unwrap();
    f.reopen();
    assert_eq!(f.bus.project(), &after);
    f.bus.undo().unwrap();
    assert_eq!(f.bus.project(), &before);
    f.bus.redo().unwrap();
    assert_eq!(f.bus.project(), &after);
}

#[test]
fn no_op_agent_turn_returns_no_transaction_and_preserves_redo_and_journal() {
    let mut f = Fixture::new();
    let mut host = f.host();
    let args = f.args();
    dispatch(&mut f.bus, &mut host, TOOL, args, actor()).unwrap();
    let painted = f.bus.project().clone();
    f.bus.undo().unwrap();
    let before = f.bus.project().clone();
    let history = f.bus.history().to_vec();
    let redo: Vec<_> = f.bus.redo_history().cloned().collect();
    let revision = f.bus.revision();
    let journal = f.journal_bytes();
    let mut args = f.args();
    args["brush"] = json!({"kind":"erase"});
    let mut provider = Mock::new(args.clone());
    let report = agent(ApprovalMode::Auto)
        .run(
            &mut provider,
            &mut f.bus,
            &mut host,
            "Erase empty cells",
            "no-op",
            &AtomicBool::new(false),
            &mut |_, _| panic!("Auto"),
            &mut |_| {},
        )
        .unwrap();
    assert!(report.tool_calls[0].succeeded);
    assert!(report.transaction_ids.is_empty());
    let result = provider.result.unwrap();
    assert!(result["transaction_id"].is_null());
    assert_eq!(result["revision"], revision);
    assert_eq!(result["untrusted_project_data"]["changed"], false);
    assert_eq!(result["untrusted_project_data"]["changes"], json!([]));
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.history(), history);
    assert_eq!(f.bus.redo_history().cloned().collect::<Vec<_>>(), redo);
    assert_eq!(f.journal_bytes(), journal);
    f.bus.redo().unwrap();
    assert_eq!(f.bus.project(), &painted);
    assert!(
        dispatch(&mut f.bus, &mut host, TOOL, args, actor()).is_err(),
        "a stale no-op must still refuse"
    );
    assert_eq!(f.bus.project(), &painted);
}

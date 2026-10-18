use super::support::{Fixture, TOOL, actor};
use incant_agent::dispatch;
use incant_cmd::CommandBus;
use incant_doc::{Collider2D, ColliderShape2D, Origin, Transform};
use serde_json::json;
use std::fs;

#[test]
fn alpha_tool_uses_cooked_texels_shared_history_provenance_and_durable_undo() {
    let mut f = Fixture::new();
    let mut host = f.host();
    let args = f.args();
    // Unlike import, derivation reads no original source, only the checked cache.
    fs::remove_file(f.root.path().join("mask.png")).unwrap();
    let before = f.bus.project().clone();
    let revision = f.bus.revision();
    let history = f.bus.history().len();
    let result = dispatch(&mut f.bus, &mut host, TOOL, args, actor()).unwrap();
    assert_eq!(result["revision"], revision + 1);
    let tx = f.bus.history().last().unwrap();
    assert_eq!(result["transaction_id"], tx.id);
    assert_eq!(f.bus.history().len(), history + 1);
    assert_eq!(tx.actor, actor());
    assert_eq!(tx.commands.len(), 2);
    let data = &result["untrusted_project_data"];
    assert_eq!(data["source_entity"], f.entity);
    assert_eq!(data["prepared_revision"], revision);
    assert_eq!(data["solid_pixels"], 3);
    assert_eq!(data["region"], json!([0, 0, 2, 2]));
    let rectangles = data["rectangles"].as_array().unwrap();
    assert_eq!(rectangles.len(), 2);
    for (index, (pixels, half_extents, translation)) in [
        ([0, 0, 1, 2], [0.5, 1.0], [0.5, 1.0, 0.0]),
        ([1, 1, 1, 1], [0.5, 0.5], [1.5, 0.5, 0.0]),
    ]
    .into_iter()
    .enumerate()
    {
        assert_eq!(rectangles[index]["pixels"], json!(pixels));
        let id = rectangles[index]["entity_id"].as_str().unwrap();
        let entity = &f.bus.project().scenes[&f.scene].entities[id];
        let collider: Collider2D =
            serde_json::from_value(entity.components["Collider2D"].clone()).unwrap();
        assert_eq!(collider.shape, ColliderShape2D::Box { half_extents });
        let transform: Transform =
            serde_json::from_value(entity.components["Transform"].clone()).unwrap();
        assert_eq!(transform.translation, translation);
        let provenance = entity.provenance.as_ref().unwrap();
        assert_eq!(provenance.origin, Origin::Agent);
        assert_eq!(provenance.transaction, tx.id);
        assert_eq!(provenance.model.as_deref(), Some("mock-alpha-model"));
        assert_eq!(provenance.conversation_id.as_deref(), Some("alpha-turn"));
    }
    let after = f.bus.project().clone();
    assert_eq!(
        after.scenes[&f.scene].entities[&f.entity],
        before.scenes[&f.scene].entities[&f.entity]
    );
    assert_eq!(after.assets, before.assets);
    f.bus.undo().unwrap();
    assert_eq!(f.bus.project(), &before);
    f.bus.redo().unwrap();
    assert_eq!(f.bus.project(), &after);
    drop(f.bus);
    let mut reopened =
        CommandBus::persistent(f.root.path().join("journal"), before.clone()).unwrap();
    assert_eq!(reopened.project(), &after);
    reopened.undo().unwrap();
    assert_eq!(reopened.project(), &before);
}

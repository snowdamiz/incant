#[path = "support/prefab.rs"]
mod support;

use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::EngineRequirement;
use serde_json::json;
use std::fs;
use support::Fixture;

#[test]
fn host_file_pin_preserves_prefab_overrides_through_reopen_undo_and_redo() {
    let mut fixture = Fixture::new();
    fixture.create();
    let instance = fixture.instance();
    let entity = instance[&fixture.nodes[&fixture.source]].clone();
    fixture
        .run(vec![Command::SetComponent {
            scene_id: fixture.scene.clone(),
            entity_id: entity.clone(),
            component: "Velocity".into(),
            value: json!({"linear":[9,0,0]}),
        }])
        .unwrap();
    let mut legacy = fixture.bus.project().clone();
    legacy.engine_version = None;
    let source = format!("\n{}\n  ", serde_json::to_string(&legacy).unwrap());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("linked.incant");
    fs::write(&path, &source).unwrap();
    let candidate = {
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        let review = bus.review_project_file_pin().unwrap();
        bus.apply_project_file_pin(&review.review_id, Actor::user("host-review"))
            .unwrap();
        let mut expected = legacy.clone();
        expected.engine_version = Some(EngineRequirement::exact_current());
        assert_eq!(bus.project(), &expected);
        assert_eq!(fs::read_to_string(&path).unwrap(), review.candidate_text);
        review.candidate_text
    };
    {
        let mut bus = CommandBus::open_project_file(&path).unwrap();
        assert_eq!(bus.history().len(), 1);
        bus.undo().unwrap();
        assert_eq!(bus.project(), &legacy);
        assert_eq!(fs::read_to_string(&path).unwrap(), source);
    }
    let mut bus = CommandBus::open_project_file(&path).unwrap();
    assert_eq!(bus.project(), &legacy);
    bus.redo().unwrap();
    assert_eq!(fs::read_to_string(&path).unwrap(), candidate);
    assert_eq!(bus.project().prefabs, legacy.prefabs);
    assert_eq!(bus.project().scenes, legacy.scenes);
    bus.execute(
        vec![Command::RevertPrefabComponent {
            scene_id: fixture.scene.clone(),
            entity_id: entity.clone(),
            component: "Velocity".into(),
        }],
        Actor::user("editor"),
        "Revert still uses the shared command bus",
        Some(bus.revision()),
    )
    .unwrap();
    let linked = &bus.project().scenes[&fixture.scene].entities[&entity];
    assert_eq!(linked.components["Velocity"]["linear"], json!([1, 0, 0]));
    assert!(linked.prefab.as_ref().unwrap().overrides.is_empty());
    // Ordinary commands do not silently acquire authority to rewrite the file.
    assert_eq!(fs::read_to_string(&path).unwrap(), candidate);
}

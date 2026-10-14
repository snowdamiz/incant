#[path = "support/prefab.rs"]
mod support;
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::Project;

#[test]
fn pin_review_and_history_preserve_linked_prefab_overrides() {
    let mut fixture = support::Fixture::new();
    fixture.create();
    let instance = fixture.instance();
    let root = instance[&fixture.nodes[&fixture.source]].clone();
    fixture
        .run(vec![Command::RemoveComponent {
            scene_id: fixture.scene.clone(),
            entity_id: root.clone(),
            component: "Velocity".into(),
        }])
        .unwrap();
    let mut legacy = fixture.bus.project().clone();
    legacy.engine_version = None;
    let before = legacy.canonical_text().unwrap();
    let mut bus = CommandBus::new(legacy.clone()).unwrap();
    let review = bus.review_engine_pin(&before, 0).unwrap();
    let candidate = Project::from_text(&review.candidate_text).unwrap();
    assert_eq!(candidate.prefabs, legacy.prefabs);
    assert_eq!(candidate.scenes, legacy.scenes);
    assert_eq!(bus.project(), &legacy);
    bus.execute(
        review.commands,
        Actor::user("reviewer"),
        "Pin linked project",
        Some(0),
    )
    .unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), review.candidate_text);
    bus.undo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), review.candidate_text);
    bus.execute(
        vec![Command::RevertPrefabComponent {
            scene_id: fixture.scene.clone(),
            entity_id: root.clone(),
            component: "Velocity".into(),
        }],
        Actor::user("reviewer"),
        "Revert component after pin",
        Some(bus.revision()),
    )
    .unwrap();
    let entity = &bus.project().scenes[&fixture.scene].entities[&root];
    assert_eq!(entity.components["Velocity"]["linear"], serde_json::json!([1, 0, 0]));
    assert!(!entity.prefab.as_ref().unwrap().overrides.contains("Velocity"));
    assert_eq!(bus.project().engine_version, candidate.engine_version);
}

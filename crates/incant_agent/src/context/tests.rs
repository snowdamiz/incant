use super::*;
use incant_cmd::{Actor, Command};
use incant_doc::{Entity, Project, Scene, Transform};

fn data(message: &Value) -> Value {
    serde_json::from_str(
        message["content"]
            .as_str()
            .unwrap()
            .split_once('\n')
            .unwrap()
            .1,
    )
    .unwrap()
}

#[test]
fn snapshot_is_read_only_and_supplies_complete_sampled_schemas() {
    let mut project = Project::empty("Project");
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Player");
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    scene.entities.insert(entity.id.clone(), entity.clone());
    project.scenes.insert(scene.id.clone(), scene.clone());
    let bus = CommandBus::new(project.clone()).unwrap();
    let message = Snapshot::capture(&bus).message().unwrap();
    let value = data(&message);
    assert_eq!(message["role"], "user");
    assert_eq!(value["revision"], 0);
    assert_eq!(value["scenes"]["total"], 1);
    assert_eq!(value["scenes"]["items"][0]["entities"][0]["id"], entity.id);
    assert_eq!(
        value["component_schemas"]["items"][0]["schema"],
        incant_doc::schema_registry()["Transform"]
    );
    assert_eq!(bus.project(), &project);
    assert!(bus.history().is_empty());
}

#[test]
fn large_unicode_and_escaped_project_data_is_bounded_with_explicit_omissions() {
    let text = "\"\\\n\0ع😀".repeat(1000);
    let mut project = Project::empty(text.clone());
    for n in 0..10 {
        let mut scene = Scene::new(text.clone());
        for _ in 0..10 {
            let entity = Entity::new(text.clone());
            scene.entities.insert(entity.id.clone(), entity);
        }
        project.scenes.insert(scene.id.clone(), scene);
        project.memory.insert(format!("memory-{n}"), text.clone());
    }
    let bus = CommandBus::new(project).unwrap();
    let message = Snapshot::capture(&bus).message().unwrap();
    assert!(encoded_len(&message, MESSAGE_BYTES).is_some());
    let value = data(&message);
    assert_eq!(value["project"]["name"]["truncated"], true);
    assert_eq!(value["scenes"]["total"], 10);
    assert!(value["scenes"]["items"].as_array().unwrap().len() <= 8);
    for scene in value["scenes"]["items"].as_array().unwrap() {
        assert_eq!(scene["entity_count"], 10);
        assert!(scene["entities"].as_array().unwrap().len() <= 8);
    }
    // Even dense scene data cannot consume the memory/history allowance.
    assert_eq!(value["memory"]["total"], 10);
    assert!(!value["memory"]["items"].as_array().unwrap().is_empty());
    assert_eq!(value["memory"]["items"][0]["value"]["truncated"], true);
    assert!(
        value["coverage"]
            .as_str()
            .unwrap()
            .contains("not evidence of absence")
    );
}

#[test]
fn recent_edits_follow_undo_and_do_not_include_retired_documents() {
    let mut bus = CommandBus::new(Project::empty("History")).unwrap();
    for n in 0..10 {
        bus.execute(
            vec![Command::SetMemory {
                section: "state".into(),
                text: n.to_string(),
            }],
            Actor::user("editor"),
            format!("Edit {n}"),
            Some(bus.revision()),
        )
        .unwrap();
    }
    bus.undo().unwrap();
    let revision = bus.revision();
    let message = Snapshot::capture(&bus).message().unwrap();
    let value = data(&message);
    let recent = value["recent_transactions"]["newest_first"]
        .as_array()
        .unwrap();
    assert_eq!(recent.len(), 8);
    assert_eq!(recent[0]["description"]["text"], "Edit 8");
    assert_eq!(value["memory"]["items"][0]["value"]["text"], "8");
    assert_eq!(value["recent_transactions"]["total_applied"], 9);
    assert_eq!(value["revision"], revision);
    assert!(!message["content"].as_str().unwrap().contains("Edit 9"));
    assert!(
        recent
            .iter()
            .all(|row| row.get("before").is_none() && row.get("after").is_none())
    );
    assert_eq!(bus.revision(), revision);
}

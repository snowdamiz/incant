#![cfg(feature = "typescript")]
use incant_cook::{TypeScriptCookError, compile_project_sources, prepare_script_scene};
use incant_doc::{Entity, Project, Scene, ScriptSource, new_id};
use serde_json::json;

#[test]
fn interrupted_source_transaction_refuses_before_opening_the_source_graph() {
    let root = tempfile::tempdir().unwrap();
    let state = root.path().join(".incant-source-transactions");
    std::fs::create_dir(&state).unwrap();
    std::fs::write(state.join("pending"), "interrupted").unwrap();
    let mut project = Project::empty("Recovery refusal");
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Actor");
    let source = new_id();
    entity
        .components
        .insert("Script".into(), json!({"source":source,"props":{}}));
    scene.entities.insert(entity.id.clone(), entity);
    let scene_id = scene.id.clone();
    project.scenes.insert(scene_id.clone(), scene);
    project.scripts.insert(
        source.clone(),
        ScriptSource {
            id: source,
            name: "Actor".into(),
            path: "absent.ts".into(),
        },
    );
    let prepared = prepare_script_scene(&project, &scene_id).unwrap();
    let result = compile_project_sources(prepared, root.path());
    assert!(matches!(
        result,
        Err(TypeScriptCookError::Source(
            incant_source::SourceError::RecoveryRequired
        ))
    ));
    assert_eq!(
        std::fs::read_to_string(state.join("pending")).unwrap(),
        "interrupted"
    );
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

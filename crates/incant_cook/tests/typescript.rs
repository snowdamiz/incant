#![cfg(feature = "typescript")]
use incant_cook::{TypeScriptCookError, compile_project_sources, prepare_script_scene};
use incant_doc::{Entity, Project, Scene, ScriptSource, new_id};
use serde_json::json;

#[test]
fn interrupted_source_transaction_refuses_before_opening_the_source_graph() {
    let root = tempfile::tempdir().unwrap();
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
            id: source.clone(),
            name: "Actor".into(),
            path: "main.ts".into(),
        },
    );
    let journal = root.path().join("journal");
    std::fs::write(&journal, "").unwrap();
    let before = "export default {update(){}};";
    std::fs::write(root.path().join("main.ts"), before).unwrap();
    let operation = new_id();
    let edit = incant_source::SourceEdit {
        script_id: source,
        path: "main.ts".into(),
        before: Some(before.into()),
        after: Some("export default {update(){throw new Error('uncommitted');}};".into()),
    };
    let store = incant_source::SourceStore::open(root.path(), &journal, &project.id).unwrap();
    store.begin(&operation, &edit).unwrap();
    drop(store); // Simulate interrupted publication: the durable marker survives.
    let state = root.path().join(".incant-source-transactions");
    let marker = std::fs::read(state.join("pending")).unwrap();
    let prepared = prepare_script_scene(&project, &scene_id).unwrap();
    let result = compile_project_sources(prepared, root.path());
    assert!(matches!(
        result,
        Err(TypeScriptCookError::Source(
            incant_source::SourceError::RecoveryRequired
        ))
    ));
    assert_eq!(std::fs::read(state.join("pending")).unwrap(), marker);
    assert_eq!(
        std::fs::read_to_string(root.path().join("main.ts")).unwrap(),
        before
    );
    let store = incant_source::SourceStore::open(root.path(), &journal, &project.id).unwrap();
    store.rollback(&operation, &edit).unwrap();
    store.finish(&operation).unwrap();
    assert!(!state.join("pending").exists());
    incant_source::check_recovery(root.path()).unwrap();
}

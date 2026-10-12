#![cfg(feature = "typescript")]
use incant_cook::{TypeScriptCookError, compile_project_sources, prepare_script_scene};
use incant_doc::{Entity, Project, Scene, ScriptSource, new_id};
use serde_json::json;

#[test]
fn interrupted_source_transaction_refuses_before_opening_the_source_graph() {
    let root = tempfile::tempdir().unwrap();
    let mut project = Project::empty("Recovery refusal");
    project.settings.physics_mode = incant_runtime::PHYSICS_VARIANT.mode();
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

#[test]
fn canonical_project_root_with_spaces_and_unicode_cooks_local_imports_into_real_native_motion() {
    let parent = tempfile::tempdir().unwrap();
    let root = parent.path().join("compiler entry Ω case");
    std::fs::create_dir(&root).unwrap();
    let root = root.canonicalize().unwrap();
    #[cfg(windows)]
    {
        // This regression must exercise Rust's namespaced Windows spelling,
        // including the compiler's independently canonicalized engine cwd.
        let std::path::Component::Prefix(prefix) = root.components().next().unwrap() else {
            panic!("expected Windows canonical root prefix");
        };
        assert!(prefix.kind().is_verbatim());
    }
    let mut project = Project::empty("Canonical compiler startup");
    project.settings.physics_mode = incant_runtime::PHYSICS_VARIANT.mode();
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Mover");
    let source = new_id();
    entity
        .components
        .insert("Script".into(), json!({"source":source,"props":{}}));
    let entity_id = entity.id.clone();
    scene.entities.insert(entity_id.clone(), entity);
    let scene_id = scene.id.clone();
    project.scenes.insert(scene_id.clone(), scene);
    project.scripts.insert(
        source.clone(),
        ScriptSource {
            id: source,
            name: "Mover".into(),
            path: "main.ts".into(),
        },
    );
    let main = r#"
import type { NativeBehavior } from '@incant/runtime/native';
import { speed } from './shared';
export default { update({instances,chunks,dt}) {
  for (const chunk of chunks) for (let row=0;row<chunk.ids.length;row++) {
    if (chunk.ids[row]===instances[0].entity) chunk.transforms[row*10]+=speed*dt;
  }
  return undefined;
} } satisfies NativeBehavior;
"#;
    let shared = "export const speed: number = 3;";
    std::fs::write(root.join("main.ts"), main).unwrap();
    std::fs::write(root.join("shared.ts"), shared).unwrap();
    let before = project.canonical_text().unwrap();
    let prepared = prepare_script_scene(&project, &scene_id).unwrap();
    let compiled = compile_project_sources(prepared, &root).unwrap();
    assert_eq!(
        compiled.compiler_manifest["format"],
        "incant-native-typescript-project"
    );
    let (scene, script) = compiled.program.into_parts();
    let mut session = incant_script::NativeSession::new(scene, script, Default::default()).unwrap();
    for _ in 0..30 {
        session.tick(&[]).unwrap();
    }
    let actual = session
        .world()
        .unwrap()
        .inspect(incant_cook::stable_id(&entity_id).unwrap())
        .unwrap()
        .transform
        .translation[0];
    assert!((actual - 1.5).abs() < 1e-12);
    assert_eq!(project.canonical_text().unwrap(), before);
    assert_eq!(std::fs::read_to_string(root.join("main.ts")).unwrap(), main);
    assert_eq!(
        std::fs::read_to_string(root.join("shared.ts")).unwrap(),
        shared
    );
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 2);
}

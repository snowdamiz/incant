use incant_doc::{Entity, Project, Scene, ScriptSource, new_id};
use incant_game_runtime::OfflineGameSession;
use incant_play::{PreparedPlay, StartOptions};
use incant_runtime::StableId;
use incant_script::{CompiledScript, ScriptError};
use serde_json::json;
use std::cell::Cell;

fn fixture() -> (Project, String, String) {
    let mut project = Project::empty("Native startup");
    project.settings.physics_mode = incant_runtime::PHYSICS_VARIANT.mode();
    let mut scene = Scene::new("Main");
    let entity = Entity::new("Mover");
    let entity_id = entity.id.clone();
    scene.entities.insert(entity.id.clone(), entity);
    let scene_id = scene.id.clone();
    project.scenes.insert(scene.id.clone(), scene);
    (project, scene_id, entity_id)
}
fn fallback() -> Result<CompiledScript, ScriptError> {
    CompiledScript::compile(
        "exports.default={update({chunks,dt}){for(const c of chunks)for(let i=0;i<c.ids.length;i++)c.transforms[i*c.transformStride]+=6*dt;}}",
    )
}
fn play(prepared: PreparedPlay, entity: &str) {
    assert!(prepared.render.is_none());
    let mut game = OfflineGameSession::new(
        prepared.cooked,
        prepared.code,
        prepared.actions,
        prepared.clips,
        prepared.sample_rate,
    )
    .unwrap();
    for _ in 0..20 {
        let mut pcm = vec![0.; game.next_output_frames().unwrap() * 2];
        game.tick(&[], &mut pcm).unwrap();
    }
    let id = StableId(entity.parse::<ulid::Ulid>().unwrap().to_bytes());
    let value = game.world().unwrap().inspect(id).unwrap();
    assert!((value.transform.translation[0] - 2.).abs() < 1e-10);
}

#[test]
fn prepared_inputs_transfer_to_a_worker_and_outlive_the_authoring_document() {
    fn assert_send<T: Send>() {}
    assert_send::<PreparedPlay>();
    let root = tempfile::tempdir().unwrap();
    let (mut project, scene, entity) = fixture();
    let prepared = PreparedPlay::load(
        &project,
        root.path(),
        &StartOptions::default(),
        48000,
        fallback,
    )
    .unwrap();
    assert_eq!(prepared.scene, scene);
    assert!(!prepared.authored_scripts);
    project.scenes.clear();
    drop(project);
    root.close().unwrap();
    std::thread::spawn(move || play(prepared, &entity))
        .join()
        .unwrap();
}

#[test]
fn authored_startup_uses_shared_cook_and_never_the_engine_fallback() {
    let root = tempfile::tempdir().unwrap();
    let (mut project, scene, entity) = fixture();
    let script = new_id();
    project.scripts.insert(
        script.clone(),
        ScriptSource {
            id: script.clone(),
            name: "Move".into(),
            path: "move.ts".into(),
        },
    );
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&entity)
        .unwrap()
        .components
        .insert("Script".into(), json!({"source":script,"props":{}}));
    // Disposable unit fixture; the public source-transaction probe covers authoring.
    std::fs::write(root.path().join("move.ts"),
        "import type { NativeBehavior } from '@incant/runtime/native';\nexport default {update({chunks,dt}){for(const c of chunks)for(let i=0;i<c.ids.length;i++)c.transforms[i*c.transformStride]+=6*dt;}} satisfies NativeBehavior;")
        .unwrap();
    let before = project.clone();
    let prepared = PreparedPlay::load(
        &project,
        root.path(),
        &StartOptions::default(),
        48000,
        || panic!("attached scripts must not use the engine fallback"),
    )
    .unwrap();
    assert!(prepared.authored_scripts);
    assert_eq!(project, before);
    drop(project);
    root.close().unwrap();
    play(prepared, &entity);
}

#[test]
fn invalid_startup_selection_refuses_before_compiling_a_fallback() {
    let root = tempfile::tempdir().unwrap();
    let (mut project, scene, _) = fixture();
    let called = Cell::new(false);
    let code = || {
        called.set(true);
        fallback()
    };
    assert!(PreparedPlay::load(&project, root.path(), &StartOptions::default(), 0, code).is_err());
    assert!(!called.get());
    let other = Scene::new("Other");
    project.scenes.insert(other.id.clone(), other);
    assert!(
        PreparedPlay::load(&project, root.path(), &StartOptions::default(), 48000, code).is_err()
    );
    assert!(!called.get());
    let selected = StartOptions {
        scene: Some(scene),
        ..Default::default()
    };
    assert!(PreparedPlay::load(&project, root.path(), &selected, 48000, code).is_ok());
    assert!(called.get());
}

#[test]
fn expected_behavior_digest_is_enforced_before_returning_owned_inputs() {
    let root = tempfile::tempdir().unwrap();
    let (project, _, _) = fixture();
    let options = StartOptions {
        script_sha256: Some("0".repeat(64)),
        ..Default::default()
    };
    let error = PreparedPlay::load(&project, root.path(), &options, 48000, fallback)
        .err()
        .expect("wrong digest must fail");
    assert!(
        error
            .to_string()
            .contains("does not match requested source hash")
    );
}

#[cfg(not(feature = "wasmtime-host"))]
#[test]
fn selected_wasm_requires_explicit_development_host_instead_of_fallback() {
    let root = tempfile::tempdir().unwrap();
    let (mut project, scene, _) = fixture();
    let module = new_id();
    project.assets.insert(
        module.clone(),
        incant_doc::Asset {
            id: module.clone(),
            name: "Module".into(),
            path: "guest.wasm".into(),
            kind: "wasm".into(),
            sha256: "ab".repeat(32),
            import_settings: None,
        },
    );
    project.scenes.get_mut(&scene).unwrap().program =
        Some(incant_doc::SceneProgram::Wasm { module });
    let error = PreparedPlay::load(
        &project,
        root.path(),
        &StartOptions::default(),
        48000,
        || panic!("selected Wasm must not run fallback"),
    )
    .err()
    .expect("host is disabled");
    assert!(error.to_string().contains("wasmtime-host"));
}

#[test]
fn foreign_physics_mode_refuses_before_assets_or_fallback_are_touched() {
    let (mut project, _, _) = fixture();
    let requested = match incant_runtime::PHYSICS_VARIANT.mode() {
        incant_doc::PhysicsMode::Fast => incant_doc::PhysicsMode::Deterministic,
        incant_doc::PhysicsMode::Deterministic => incant_doc::PhysicsMode::Fast,
    };
    project.settings.physics_mode = requested;
    let before = project.canonical_text().unwrap();
    let root = tempfile::tempdir().unwrap();
    let error = PreparedPlay::load(
        &project,
        root.path(),
        &StartOptions::default(),
        48000,
        || panic!("must reject kernel mismatch before source compilation"),
    )
    .err()
    .unwrap();
    let incant_play::StartupError::Native(error) = error else {
        panic!("typed kernel error required")
    };
    assert!(matches!(error.downcast_ref::<incant_runtime::SceneError>(),
        Some(incant_runtime::SceneError::PhysicsMode { requested: mode, compiled })
        if *mode == requested && *compiled == incant_runtime::PHYSICS_VARIANT));
    assert_eq!(project.canonical_text().unwrap(), before);
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
}

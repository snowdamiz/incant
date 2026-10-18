#![cfg(feature = "wasmtime-host")]
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Entity, Project, Scene, SceneProgram};
use incant_import::{ImportRequest, ImportSnapshot};
use incant_play::{PreparedPlay, StartOptions};
use incant_script::{CheckpointError, NativeSession};
#[test]
fn authored_module_runs_from_captured_cache_and_checkpoints_refuse_explicitly() {
    let root = tempfile::tempdir().unwrap();
    let bytes = wat::parse_str(
        r#"(module
      (import "incant_v1" "read_transforms" (func $read (param i64 i32 i32)))
      (import "incant_v1" "write_transforms" (func $write (param i64 i32 i32)))
      (import "incant_log_v1" "write" (func $log (param i64 i32 i32 i32)))
      (memory (export "memory") 1) (data (i32.const 1024) "authored")
      (func (export "incant_abi") (result i32) i32.const 1)
      (func (export "update") (param $epoch i64) (param f64)
        local.get $epoch i32.const 0 i32.const 0 call $read
        i32.const 0 i32.const 0 f64.load f64.const 1 f64.add f64.store
        local.get $epoch i32.const 0 i32.const 0 call $write
        local.get $epoch i32.const 1 i32.const 1024 i32.const 8 call $log))"#,
    )
    .unwrap();
    std::fs::write(root.path().join("guest.wasm"), bytes).unwrap();
    let mut project = Project::empty("Wasm Play");
    project.settings.physics_mode = incant_runtime::PHYSICS_VARIANT.mode();
    let mut bus = CommandBus::new(project).unwrap();
    let asset = ImportSnapshot::capture(&bus)
        .prepare(
            root.path(),
            None,
            &[ImportRequest {
                source: "guest.wasm".into(),
                texture_usage: None,
            }],
        )
        .unwrap()
        .commit(&mut bus, Actor::import("wasm"), "Import")
        .unwrap()
        .imports
        .remove(0)
        .asset;
    let mut scene = Scene::new("Main");
    let entity = Entity::new("Mover");
    let id = entity.id.parse::<ulid::Ulid>().unwrap();
    scene.entities.insert(entity.id.clone(), entity);
    scene.program = Some(SceneProgram::Wasm {
        module: asset.id,
        instances: Default::default(),
    });
    bus.execute(
        vec![Command::CreateScene { scene }],
        Actor::user("test"),
        "Select module",
        None,
    )
    .unwrap();
    std::fs::remove_file(root.path().join("guest.wasm")).unwrap();
    let prepared = PreparedPlay::load(
        bus.project(),
        root.path(),
        &StartOptions::default(),
        48000,
        || panic!("must not fall back to TypeScript"),
    )
    .unwrap();
    root.close().unwrap();
    let mut session = NativeSession::new(prepared.cooked, prepared.code, prepared.actions).unwrap();
    for _ in 0..3 {
        session.tick(&[]).unwrap();
    }
    assert_eq!(
        session
            .world()
            .unwrap()
            .inspect(incant_runtime::StableId(id.to_bytes()))
            .unwrap()
            .transform
            .translation[0],
        3.
    );
    assert_eq!(session.logs().unwrap().entries().len(), 3);
    assert_eq!(session.logs().unwrap().entries()[2].message, "authored");
    assert!(matches!(
        session.checkpoint_bytes(),
        Err(CheckpointError::Unsupported("development module state ABI"))
    ));
}

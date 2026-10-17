#![cfg(feature = "wasmtime-host")]
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_script::{NativeSession, SessionProgram};
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::persistent::Declaration;

fn module(body: &str, start: bool, declaration: Option<Declaration>) -> Vec<u8> {
    let start = if start {
        "(func $start (drop (call $write (i64.const 0) (i32.const 0) (i32.const 0) (i32.const 8)))) (start $start)"
    } else {
        ""
    };
    let mut bytes = wat::parse_str(format!(
        r#"(module
        (import "incant_state_v1" "read" (func $read (param i64 i32 i32 i32) (result i32)))
        (import "incant_state_v1" "write" (func $write (param i64 i32 i32 i32) (result i32)))
        (memory (export "memory") 1)
        (func (export "incant_abi") (result i32) (i32.const 1))
        {start}
        (func (export "update") (param $e i64) (param f64) {body}))"#
    ))
    .unwrap();
    if let Some(declaration) = declaration {
        let mut section = vec![15];
        section.extend(b"incant_state_v1");
        section.extend(declaration.encode());
        bytes.extend([0, section.len() as u8]);
        bytes.extend(section);
    }
    bytes
}
fn declared(body: &str) -> Vec<u8> {
    module(body, false, Some(Declaration::new(16, [1; 32]).unwrap()))
}
const COUNT: &str = r#"
    (drop (call $read (local.get $e) (i32.const 0) (i32.const 0) (i32.const 8)))
    (i64.store (i32.const 0) (i64.add (i64.load (i32.const 0)) (i64.const 1)))
    (drop (call $write (local.get $e) (i32.const 0) (i32.const 0) (i32.const 8)))"#;
fn scene() -> CookedScene {
    CookedScene::new(StableId([0; 16]), 64, vec![]).unwrap()
}
fn count(host: &DevModuleHost, world: &NativeWorld) -> u64 {
    u64::from_le_bytes(
        host.persistent_checkpoint(world).unwrap().bytes()[..8]
            .try_into()
            .unwrap(),
    )
}

#[test]
fn declaration_required_and_duplicate_or_malformed_declarations_refuse_before_startup() {
    let missing = module(COUNT, false, None);
    for bytes in [&missing, &declared(COUNT)] {
        assert_eq!(DevModuleHost::new(bytes).is_ok(), bytes != &missing);
        assert_eq!(incant_wasm_cook::prepare(bytes).is_ok(), bytes != &missing);
    }
    let original = declared(COUNT);
    let custom = &original[missing.len()..];
    let mut duplicate = original.clone();
    duplicate.extend(custom);
    let mut malformed = original.clone();
    *malformed.last_mut().unwrap() = 1;
    for bytes in [duplicate, malformed] {
        assert!(DevModuleHost::new(&bytes).is_err());
        assert!(incant_wasm_cook::prepare(&bytes).is_err());
    }
}

#[test]
fn fresh_start_has_no_state_authority_and_failed_reload_preserves_old_committed_bytes() {
    let mut world = NativeWorld::new(scene());
    let mut host = DevModuleHost::new(&declared(COUNT)).unwrap();
    world.step().unwrap();
    host.tick(&mut world, 1. / 64.).unwrap();
    let before = host.persistent_checkpoint(&world).unwrap();
    let startup = module(COUNT, true, Some(Declaration::new(16, [1; 32]).unwrap()));
    assert!(DevModuleHost::new(&startup).is_err());
    assert!(host.reload(&startup).is_err());
    assert_eq!(host.persistent_checkpoint(&world).unwrap(), before);
    let mismatch = module(COUNT, false, Some(Declaration::new(16, [2; 32]).unwrap()));
    assert!(host.reload(&mismatch).is_err());
    assert_eq!(host.persistent_checkpoint(&world).unwrap(), before);
    host.reload(&declared(COUNT)).unwrap();
    world.step().unwrap();
    host.tick(&mut world, 1. / 64.).unwrap();
    assert_eq!(count(&host, &world), 2);
}

#[test]
fn actual_wasmtime_ordinary_checkpoint_restores_fresh_vm_at_exact_native_clock() {
    let code = declared(COUNT);
    let initial = scene();
    let mut live = NativeSession::new(
        initial.clone(),
        SessionProgram::DevelopmentWasm(code.clone()),
        Default::default(),
    )
    .unwrap();
    let zero = live.checkpoint_bytes().unwrap();
    assert_eq!(u32::from_le_bytes(zero[8..12].try_into().unwrap()), 7);
    for _ in 0..3 {
        live.tick(&[]).unwrap();
    }
    let saved = live.checkpoint_bytes().unwrap();
    let mut restored = NativeSession::from_checkpoint_bytes(
        &saved,
        &initial,
        SessionProgram::DevelopmentWasm(code.clone()),
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        live.world().unwrap().tick(),
        restored.world().unwrap().tick()
    );
    for _ in 0..4 {
        live.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
    }
    assert_eq!(
        live.checkpoint_bytes().unwrap(),
        restored.checkpoint_bytes().unwrap()
    );
    let before = live.checkpoint_bytes().unwrap();
    let mut corrupt = saved.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    assert!(live.restore_checkpoint_bytes(&corrupt).is_err());
    assert_eq!(live.checkpoint_bytes().unwrap(), before);
    live.restore_checkpoint_bytes(&saved).unwrap();
    live.tick(&[]).unwrap();
    assert_eq!(live.world().unwrap().tick(), 4);
    assert!(
        NativeSession::from_checkpoint_bytes(
            &saved,
            &initial,
            SessionProgram::DevelopmentWasm(declared("")),
            Default::default()
        )
        .is_err()
    );
    let mut fresh = NativeSession::from_checkpoint_bytes(
        &zero,
        &initial,
        SessionProgram::DevelopmentWasm(code),
        Default::default(),
    )
    .unwrap();
    fresh.tick(&[]).unwrap();
    assert_eq!(fresh.world().unwrap().tick(), 1);
}

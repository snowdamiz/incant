#![cfg(feature = "wasmtime-host")]
use incant_runtime::{
    CookedEntity, CookedScene, FrameCommands, NativeWorld, StableId, StructuralCommand,
};
use incant_types::{Transform, Velocity};
use incant_wasm_dev::{DevModuleHost, ModuleError};

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn scene() -> CookedScene {
    CookedScene::new(
        id(99),
        64,
        (1..=4)
            .map(|n| CookedEntity {
                id: id(n),
                parent: (n == 2).then_some(id(1)),
                transform: Transform::default(),
                velocity: (n % 2 == 1).then_some(Velocity {
                    linear: [1., 0., 0.],
                }),
            })
            .collect(),
    )
    .unwrap()
}
fn wasm(body: &str, extra: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
      (import "incant_v1" "chunk_count" (func $count (param i64) (result i32)))
      (import "incant_v1" "chunk_len" (func $len (param i64 i32) (result i32)))
      (import "incant_v1" "chunk_flags" (func $flags (param i64 i32) (result i32)))
      (import "incant_v1" "read_ids" (func $ids (param i64 i32 i32)))
      (import "incant_v1" "read_transforms" (func $read (param i64 i32 i32)))
      (import "incant_v1" "write_transforms" (func $write (param i64 i32 i32)))
      (import "incant_v1" "read_velocities" (func $vr (param i64 i32 i32)))
      (import "incant_v1" "write_velocities" (func $vw (param i64 i32 i32)))
      (import "incant_v1" "spawn" (func $spawn (param i64 i32 i32 i32 i32)))
      (import "incant_v1" "despawn" (func $despawn (param i64 i32)))
      (import "incant_v1" "set_parent" (func $parent (param i64 i32 i32)))
      (import "incant_v1" "set_velocity" (func $velocity (param i64 i32 i32)))
      (func (export "incant_abi") (result i32) (i32.const 1))
      (memory (export "memory") 4)
      {extra}
      (func (export "update") (param $e i64) (param $dt f64)
        (local $c i32) (local $i i32) (local $p i32) (local $n i32)
        {body}))"#
    ))
    .unwrap()
}
// Shared behavior: translate x by dt; encode stable ID in z; add 2 to velocity.x.
const BULK: &str = r#"
(block $done (loop $chunks
  (br_if $done (i32.ge_u (local.get $c) (call $count (local.get $e))))
  (call $read (local.get $e) (local.get $c) (i32.const 0))
  (call $ids (local.get $e) (local.get $c) (i32.const 32768))
  (local.set $n (call $len (local.get $e) (local.get $c)))
  (local.set $i (i32.const 0))
  (block $rows_done (loop $rows
    (br_if $rows_done (i32.ge_u (local.get $i) (local.get $n)))
    (local.set $p (i32.mul (local.get $i) (i32.const 80)))
    (f64.store (local.get $p) (f64.add (f64.load (local.get $p)) (local.get $dt)))
    (f64.store offset=16 (local.get $p)
      (f64.convert_i32_u (i32.load8_u (i32.add (i32.const 32783) (i32.mul (local.get $i) (i32.const 16))))))
    (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $rows)))
  (call $write (local.get $e) (local.get $c) (i32.const 0))
  (if (call $flags (local.get $e) (local.get $c)) (then
    (call $vr (local.get $e) (local.get $c) (i32.const 65536))
    (local.set $i (i32.const 0))
    (block $v_done (loop $v
      (br_if $v_done (i32.ge_u (local.get $i) (local.get $n)))
      (local.set $p (i32.add (i32.const 65536) (i32.mul (local.get $i) (i32.const 24))))
      (f64.store (local.get $p) (f64.add (f64.load (local.get $p)) (f64.const 2)))
      (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $v)))
    (call $vw (local.get $e) (local.get $c) (i32.const 65536))))
  (local.set $c (i32.add (local.get $c) (i32.const 1))) (br $chunks)))
"#;
fn fail_unchanged(bytes: &[u8], scene: CookedScene) -> ModuleError {
    let mut world = NativeWorld::new(scene);
    let before = world.snapshot();
    let mut host = DevModuleHost::new(bytes).unwrap();
    let error = host.tick(&mut world, 1. / 64.).unwrap_err();
    assert!(host.is_failed());
    assert_eq!(world.snapshot(), before);
    assert!(matches!(
        host.tick(&mut world, 1. / 64.),
        Err(ModuleError::Failed)
    ));
    error
}
#[test]
fn real_bulk_execution_matches_quickjs_and_native_columns_across_archetypes() {
    let scene = scene();
    let mut wasm_world = NativeWorld::new(scene.clone());
    let mut js_world = NativeWorld::new(scene.clone());
    let mut native_world = NativeWorld::new(scene);
    let mut host = DevModuleHost::new(&wasm(BULK, "")).unwrap();
    let code=incant_script::CompiledScript::compile("exports.default={update({chunks,dt}){for(const c of chunks) for(let i=0;i<c.ids.length;i++){c.transforms[i*10]+=dt;c.transforms[i*10+2]=Number(c.ids[i].at(-1));if(c.velocities)c.velocities[i*3]+=2;}}}").unwrap();
    let mut js = incant_script::ScriptHost::new(code).unwrap();
    for tick in 0..8 {
        for world in [&mut wasm_world, &mut js_world, &mut native_world] {
            world.step().unwrap();
        }
        host.tick(&mut wasm_world, 1. / 64.).unwrap();
        js.tick(&mut js_world, 1. / 64.).unwrap();
        native_world.with_numeric_columns(|columns| {
            columns.for_each_chunk(|c| {
                for (i, id) in c.ids.iter().enumerate() {
                    c.transforms[i * 10] += 1. / 64.;
                    c.transforms[i * 10 + 2] = f64::from(id.0[15]);
                }
                if let Some(v) = c.velocities {
                    for row in v.as_chunks_mut::<3>().0 {
                        row[0] += 2.;
                    }
                }
            })
        });
        assert_eq!(wasm_world.snapshot(), js_world.snapshot(), "tick {tick}");
        assert_eq!(wasm_world.snapshot(), native_world.snapshot());
    }
}
#[test]
fn trap_after_writes_discards_the_complete_numeric_prefix() {
    fail_unchanged(&wasm(&format!("{BULK} unreachable"), ""), scene());
}
#[test]
fn numeric_domain_and_hierarchy_failures_restore_every_column() {
    for (offset, value) in [(0, "nan"), (24, "2"), (56, "0"), (56, "1e308")] {
        let edit = format!(
            r#"(local.set $c (i32.const 0))
          (block $done (loop $l (br_if $done (i32.ge_u (local.get $c) (call $count (local.get $e))))
            (call $read (local.get $e) (local.get $c) (i32.const 0))
            (local.set $i (i32.const 0))
            (block $rd (loop $r (br_if $rd (i32.ge_u (local.get $i) (call $len (local.get $e) (local.get $c))))
              (f64.store (i32.add (i32.const {offset}) (i32.mul (local.get $i) (i32.const 80))) (f64.const {value}))
              (local.set $i (i32.add (local.get $i) (i32.const 1))) (br $r)))
            (call $write (local.get $e) (local.get $c) (i32.const 0))
            (local.set $c (i32.add (local.get $c) (i32.const 1))) (br $l)))"#
        );
        fail_unchanged(&wasm(&format!("{BULK} {edit}"), ""), scene());
    }
}
fn bytes(data: &[u8]) -> String {
    data.iter().map(|b| format!("\\{b:02x}")).collect()
}
fn ids_data() -> String {
    format!(
        r#"(data (i32.const 8192) "{}{}{}")"#,
        bytes(&id(9).0),
        bytes(&id(1).0),
        bytes(&id(2).0)
    )
}
#[test]
fn late_structural_rejection_restores_numeric_writes_and_discards_valid_spawn() {
    // Queue a valid spawn, then a cycle among existing parent and child.
    let body = format!(
        r#"{BULK}
      (call $read (local.get $e) (i32.const 0) (i32.const 0))
      (call $spawn (local.get $e) (i32.const 8192) (i32.const -1) (i32.const 0) (i32.const -1))
      (call $parent (local.get $e) (i32.const 8208) (i32.const 8224))"#
    );
    fail_unchanged(&wasm(&body, &ids_data()), scene());
}
#[test]
fn commands_reallocate_tables_and_next_call_reacquires_current_rows() {
    let mut world = NativeWorld::new(scene());
    let body = format!(
        r#"{BULK}
      (if (i64.eq (local.get $e) (i64.const 1)) (then
        (call $read (local.get $e) (i32.const 0) (i32.const 0))
        (call $spawn (local.get $e) (i32.const 8192) (i32.const 8208) (i32.const 0) (i32.const -1))
        (call $velocity (local.get $e) (i32.const 8224) (i32.const 65536))))"#
    );
    let mut host = DevModuleHost::new(&wasm(&body, &ids_data())).unwrap();
    host.tick(&mut world, 1. / 64.).unwrap();
    assert!(world.inspect(id(9)).is_some());
    assert!(world.inspect(id(2)).unwrap().velocity.is_some());
    let first = world.inspect(id(9)).unwrap().transform.translation[0];
    host.tick(&mut world, 1. / 64.).unwrap();
    assert_eq!(
        world.inspect(id(9)).unwrap().transform.translation[0],
        first + 1. / 64.
    );
    assert_eq!(world.inspect(id(9)).unwrap().transform.translation[2], 9.);
    // External structural growth between ticks cannot leave retained pointers.
    let mut commands = FrameCommands::default();
    for n in 10..99 {
        commands
            .push(StructuralCommand::Spawn(CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            }))
            .unwrap();
    }
    world.apply(&mut commands).unwrap();
    host.tick(&mut world, 1. / 64.).unwrap();
    assert_eq!(world.inspect(id(98)).unwrap().transform.translation[2], 98.);
}
#[test]
fn invalid_offsets_chunks_and_expired_epochs_cannot_access_native_storage() {
    for body in [
        "(call $read (local.get $e) (i32.const 0) (i32.const -1))",
        "(call $read (local.get $e) (i32.const 0) (i32.const 262143))",
        "(call $read (local.get $e) (i32.const 99) (i32.const 0))",
        "(call $read (i64.const 0) (i32.const 0) (i32.const 0))",
    ] {
        fail_unchanged(&wasm(&format!("{BULK} {body}"), ""), scene());
    }
    let mut world = NativeWorld::new(scene());
    let mut host=DevModuleHost::new(&wasm("(if (i64.ne (global.get $old) (i64.const 0)) (then (drop (call $count (global.get $old))))) (global.set $old (local.get $e))", "(global $old (mut i64) (i64.const 0))")).unwrap();
    host.tick(&mut world, 1. / 64.).unwrap();
    let before = world.snapshot();
    assert!(host.tick(&mut world, 1. / 64.).is_err());
    assert_eq!(world.snapshot(), before);
}
#[test]
fn fuel_memory_and_host_call_limits_trap_instead_of_hanging_or_publishing() {
    for (body, reason) in [
        ("(loop $forever (br $forever))", "fuel"),
        ("(drop (memory.grow (i32.const 1024)))", "memory"),
        (
            "(loop $calls (drop (call $count (local.get $e))) (br $calls))",
            "host call quota",
        ),
    ] {
        let error = fail_unchanged(&wasm(&format!("{BULK} {body}"), ""), scene());
        assert!(format!("{error:?}").contains(reason), "{error:?}");
    }
    assert!(
        DevModuleHost::new(&wasm("", "(func $start (loop $l (br $l))) (start $start)")).is_err()
    );
}
#[test]
fn imports_are_inactive_during_start_and_unknown_capabilities_fail() {
    assert!(
        DevModuleHost::new(&wasm(
            "",
            "(func $start (drop (call $count (i64.const 0)))) (start $start)"
        ))
        .is_err()
    );
    for import in [
        "(import \"wasi_snapshot_preview1\" \"fd_write\" (func))",
        "(import \"incant_v1\" \"network\" (func))",
        "(import \"incant_v1\" \"chunk_count\" (func (result i32)))",
    ] {
        let data=wat::parse_str(format!("(module {import} (memory (export \"memory\") 1) (func (export \"incant_abi\") (result i32) (i32.const 1)) (func (export \"update\") (param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&data).is_err());
    }
}
#[test]
fn unsupported_versions_proposals_and_oversized_modules_fail_before_execution() {
    assert!(DevModuleHost::new(&[0u8; 1024 * 1024 + 1]).is_err());
    assert!(DevModuleHost::new(b"\x7fELFserialized machine code").is_err());
    for extra in [
        "(memory (export \"memory\") i64 1)",
        "(memory (export \"memory\") 1 1 shared)",
    ] {
        let data=wat::parse_str(format!("(module {extra} (func (export \"incant_abi\") (result i32) (i32.const 1)) (func (export \"update\") (param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&data).is_err());
    }
    let simd = wasm("(drop (v128.const i32x4 1 2 3 4))", "");
    assert!(DevModuleHost::new(&simd).is_err());
    let too_many = wasm("", &"(func)".repeat(1025));
    assert!(matches!(
        DevModuleHost::new(&too_many),
        Err(ModuleError::Limit(_))
    ));
    for version in [
        "(result i32) (i32.const 2)",
        "(param i32) (result i32) (i32.const 1)",
    ] {
        let data=wat::parse_str(format!("(module (memory (export \"memory\") 1) (func (export \"incant_abi\") {version}) (func (export \"update\") (param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&data).is_err());
    }
}
#[test]
fn reload_is_atomic_for_the_current_instance_and_never_replaces_world_state() {
    let mut world = NativeWorld::new(scene());
    let mut host = DevModuleHost::new(&wasm(BULK, "")).unwrap();
    host.tick(&mut world, 1. / 64.).unwrap();
    let before = world.snapshot();
    assert!(host.reload(b"bad module").is_err());
    assert!(!host.is_failed());
    assert_eq!(world.snapshot(), before);
    host.tick(&mut world, 1. / 64.).unwrap();
    let before = world.snapshot();
    host.reload(&wasm("", "")).unwrap();
    assert_eq!(world.snapshot(), before);
    host.tick(&mut world, 1. / 64.).unwrap();
    assert_eq!(world.snapshot(), before);
    let mut other = NativeWorld::new(scene());
    assert!(matches!(
        host.tick(&mut other, 1. / 64.),
        Err(ModuleError::Abi(_))
    ));
    assert!(!host.is_failed());
    assert!(host.tick(&mut world, f64::NAN).is_err());
    assert!(!host.is_failed());
}

#[test]
fn transfer_and_command_quotas_bound_native_host_work() {
    let many = CookedScene::new(
        id(9000),
        64,
        (1..=1024)
            .map(|n| CookedEntity {
                id: id(n),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            })
            .collect(),
    )
    .unwrap();
    let error = fail_unchanged(
        &wasm(
            "(loop $l (call $read (local.get $e) (i32.const 0) (i32.const 0)) (br $l))",
            "",
        ),
        many,
    );
    assert!(format!("{error:?}").contains("transfer quota"), "{error:?}");
    let error = fail_unchanged(
        &wasm(
            "(loop $l (call $despawn (local.get $e) (i32.const 8208)) (br $l))",
            &ids_data(),
        ),
        scene(),
    );
    assert!(
        format!("{error:?}").contains("structural command quota"),
        "{error:?}"
    );
}
#[test]
fn missing_and_invalid_velocity_are_not_silently_accepted() {
    let plain = CookedScene::new(
        id(99),
        64,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        }],
    )
    .unwrap();
    fail_unchanged(
        &wasm("(call $vr (local.get $e) (i32.const 0) (i32.const 0))", ""),
        plain,
    );
    let body = r#"(block $done (loop $l
      (br_if $done (i32.ge_u (local.get $c) (call $count (local.get $e))))
      (if (call $flags (local.get $e) (local.get $c)) (then
        (call $vr (local.get $e) (local.get $c) (i32.const 0))
        (f64.store (i32.const 0) (f64.const inf))
        (call $vw (local.get $e) (local.get $c) (i32.const 0))))
      (local.set $c (i32.add (local.get $c) (i32.const 1))) (br $l)))"#;
    fail_unchanged(&wasm(body, ""), scene());
}
#[test]
fn initial_memory_table_and_stack_exhaustion_are_bounded() {
    for extra in [
        "(memory (export \"memory\") 513)",
        "(table 4097 funcref) (memory (export \"memory\") 1)",
    ] {
        let bytes=wat::parse_str(format!("(module {extra} (func (export \"incant_abi\") (result i32) (i32.const 1)) (func (export \"update\") (param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&bytes).is_err());
    }
    fail_unchanged(
        &wasm("(call $recurse)", "(func $recurse (call $recurse))"),
        scene(),
    );
}

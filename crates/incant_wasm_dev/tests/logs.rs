#![cfg(feature = "static-conformance")]
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_types::Transform;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{LogLevel, StaticModule, embedded};

fn bytes(name: &str) -> Vec<u8> {
    wat::parse_file(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../incant_wasm_runtime/fixtures")
            .join(format!("{name}.wat")),
    )
    .unwrap()
}
fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            StableId(99u128.to_be_bytes()),
            64,
            vec![CookedEntity {
                id: StableId(1u128.to_be_bytes()),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            }],
        )
        .unwrap(),
    )
}
fn pair() -> (StaticModule, DevModuleHost) {
    (
        StaticModule::new(embedded::linked("logs").unwrap()).unwrap(),
        DevModuleHost::new(&bytes("logs")).unwrap(),
    )
}
#[test]
fn actual_imports_copy_ordered_utf8_levels_and_match_quickjs_accepted_clock() {
    let (mut native, mut dev) = pair();
    let (mut a, mut b, mut c) = (world(), world(), world());
    let mut js = incant_script::ScriptHost::new(incant_script::CompiledScript::compile(
        "exports.default={update({clock,chunks,log}){chunks[0].transforms[0]+=1;log(clock.tick===1?'hello':'Xello');log('雪🙂\\n\\t','debug');log('');log('careful','warn');log('problem','error');}};",
    ).unwrap()).unwrap();
    // A module's successful-update clock is independent of the owner's world clock.
    for w in [&mut a, &mut b, &mut c] {
        w.step().unwrap();
    }
    for tick in 1..=2 {
        native.tick(&mut a, 1. / 64.).unwrap();
        dev.tick(&mut b, 1. / 64.).unwrap();
        js.tick(&mut c, 1. / 64.).unwrap();
        assert_eq!(a.snapshot(), b.snapshot());
        assert_eq!(a.snapshot(), c.snapshot());
        let logs = native.logs().unwrap();
        assert_eq!(logs.entries(), dev.logs().unwrap().entries());
        assert_eq!(logs.entries(), js.logs().unwrap().entries());
        assert_eq!(logs.dropped(), 0);
        assert_eq!(logs.entries().len(), tick * 5);
        let rows = &logs.entries()[(tick - 1) * 5..];
        assert!(rows.iter().all(|r| r.tick == tick as u64));
        assert_eq!(
            rows.iter().map(|r| r.level).collect::<Vec<_>>(),
            [
                LogLevel::Info,
                LogLevel::Debug,
                LogLevel::Info,
                LogLevel::Warn,
                LogLevel::Error
            ]
        );
        assert_eq!(rows[1].message, "雪🙂\n\t");
        assert_eq!(rows[2].message, "");
    }
    assert_eq!(native.logs().unwrap().entries()[0].message, "hello");
    assert_eq!(a.tick(), 1);
}
#[test]
fn history_retains_the_same_prefix_and_drops_without_rejecting_updates() {
    for (mode, count, retained, message) in [
        (2, 64, 1024, "bounded".to_owned()),
        (3, 16, 256, "x".repeat(1024)),
    ] {
        let (mut native, mut dev) = pair();
        let (mut a, mut b) = (world(), world());
        for _ in 0..20 {
            native.tick(&mut a, mode as f64 / 64.).unwrap();
            dev.tick(&mut b, mode as f64 / 64.).unwrap();
        }
        let logs = native.logs().unwrap();
        assert_eq!(logs.entries(), dev.logs().unwrap().entries());
        assert_eq!(logs.entries().len(), retained);
        assert_eq!(logs.dropped(), 20 * count - retained as u64);
        assert_eq!(logs.dropped(), dev.logs().unwrap().dropped());
        for (i, row) in logs.entries().iter().enumerate() {
            assert_eq!(row.tick, i as u64 / count + 1);
            assert_eq!(row.message, message);
        }
        let before = logs.dropped();
        native.tick(&mut a, 22. / 64.).unwrap();
        dev.tick(&mut b, 22. / 64.).unwrap();
        // Even empty records cannot skip an already-dropped prefix.
        assert_eq!(native.logs().unwrap().dropped(), before + 64);
        assert_eq!(a.snapshot(), b.snapshot());
        assert_eq!(
            a.inspect(StableId(1u128.to_be_bytes()))
                .unwrap()
                .transform
                .translation[0],
            21.
        );
    }
}
#[test]
fn exact_update_limits_accept_16k_and_64_records_including_empty_messages() {
    let (mut native, mut dev) = pair();
    let (mut a, mut b) = (world(), world());
    native.tick(&mut a, 22. / 64.).unwrap();
    dev.tick(&mut b, 22. / 64.).unwrap();
    let logs = native.logs().unwrap();
    assert_eq!(logs.entries(), dev.logs().unwrap().entries());
    assert_eq!(logs.entries().len(), 64);
    assert_eq!(
        logs.entries()
            .iter()
            .map(|r| r.message.len())
            .sum::<usize>(),
        16384
    );
}
#[test]
fn invalid_ingress_trap_and_rejected_native_writes_never_publish_any_prefix() {
    for mode in 4..=21 {
        let (mut native, mut dev) = pair();
        let (mut a, mut b) = (world(), world());
        native.tick(&mut a, 1. / 64.).unwrap();
        dev.tick(&mut b, 1. / 64.).unwrap();
        let before = a.snapshot();
        assert!(
            native.tick(&mut a, mode as f64 / 64.).is_err(),
            "static mode {mode}"
        );
        assert!(
            dev.tick(&mut b, mode as f64 / 64.).is_err(),
            "dev mode {mode}"
        );
        assert_eq!(a.snapshot(), before, "static mode {mode}");
        assert_eq!(b.snapshot(), before, "dev mode {mode}");
        assert!(native.logs().is_err() && dev.logs().is_err());
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.tick(&mut a, 1. / 64.).is_err());
        assert!(dev.tick(&mut b, 1. / 64.).is_err());
        dev.reload(&bytes("logs")).unwrap();
        assert_eq!(dev.logs().unwrap().entries().len(), 5, "mode {mode}");
        dev.tick(&mut b, 1. / 64.).unwrap();
        assert_eq!(dev.logs().unwrap().entries().len(), 10);
        assert_eq!(dev.logs().unwrap().entries()[5].tick, 2);
    }
}
#[test]
fn failed_reload_and_start_import_cannot_replace_or_publish_committed_history() {
    let (_, mut dev) = pair();
    let mut w = world();
    dev.tick(&mut w, 1. / 64.).unwrap();
    assert!(dev.reload(&bytes("log-start")).is_err());
    assert!(!dev.is_failed());
    assert_eq!(dev.logs().unwrap().entries().len(), 5);
    dev.tick(&mut w, 1. / 64.).unwrap();
    assert_eq!(dev.logs().unwrap().entries()[5].tick, 2);
    assert!(StaticModule::new(embedded::linked("log-start").unwrap()).is_err());
}

#[test]
fn static_native_session_observer_serializes_the_existing_report_shape_and_hides_failure() {
    let scene = CookedScene::new(
        StableId(99u128.to_be_bytes()),
        64,
        vec![CookedEntity {
            id: StableId(1u128.to_be_bytes()),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        }],
    )
    .unwrap();
    let mut session = incant_script::NativeSession::new(
        scene,
        embedded::linked("log-session").unwrap(),
        Default::default(),
    )
    .unwrap();
    for _ in 0..2 {
        session.tick(&[]).unwrap();
    }
    // This is the same public session observer serialized by native reports.
    assert_eq!(
        serde_json::to_value(session.logs().unwrap()).unwrap(),
        serde_json::json!({
            "entries": [
                {"tick":1,"level":"info","message":"Ready 雪🙂"},
                {"tick":2,"level":"info","message":"Ready 雪🙂"}
            ], "dropped":0
        })
    );
    assert!(session.tick(&[]).is_err());
    assert!(session.logs().is_err());
    assert!(session.world().is_err());
}

#[test]
fn logging_namespace_versions_and_signature_mismatches_are_refused_before_execution() {
    for import in [
        r#"(import "incant_log_v2" "write" (func (param i64 i32 i32 i32)))"#,
        r#"(import "incant_log_v1" "write" (func (param i64 i32 i32)))"#,
        r#"(import "incant_log_v1" "write" (func (param i64 i32 i32 i32) (result i32)))"#,
    ] {
        let bytes = wat::parse_str(format!(
            "(module {import} (memory (export \"memory\") 1) (func (export \"incant_abi\") (result i32) (i32.const 1)) (func (export \"update\") (param i64 f64)))"
        )).unwrap();
        assert!(DevModuleHost::new(&bytes).is_err(), "{import}");
        assert!(incant_wasm_cook::prepare(&bytes).is_err(), "{import}");
    }
}

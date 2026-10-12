#![cfg(feature = "static-conformance")]
#[path = "../../incant_wasm_runtime/tests/support/root_motion.rs"]
mod fixture;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
fn guest(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
#[test]
fn every_root_pose_ik_field_and_exact_clock_words_match_native_oracle_across_both_actual_hosts() {
    let name = "rust-root-motion";
    let mut static_host = StaticModule::new(embedded::linked(name).unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest(name)).unwrap();
    let mut a = NativeWorld::new(fixture::scene());
    let mut b = NativeWorld::new(fixture::scene());
    let mut oracle = NativeWorld::new(fixture::scene());
    let mut compound = false;
    let mut reverse = false;
    let mut outgoing = false;
    for turn in 0..192 {
        a.step().unwrap();
        b.step().unwrap();
        oracle.step().unwrap();
        let mut rows = Vec::new();
        oracle.snapshot_root_motion(&mut rows).unwrap();
        reverse |= rows.iter().any(|r| r.current.cycle < 0);
        outgoing |= rows.iter().any(|r| r.outgoing.is_some());
        let mut poses = Vec::new();
        oracle.snapshot_animations(&mut poses).unwrap();
        compound |= poses
            .iter()
            .any(|r| r.pose.is_some_and(|p| p.samples().len() == 16));
        fixture::oracle(&mut oracle, turn);
        static_host
            .tick(&mut a, 1. / 60.)
            .unwrap_or_else(|e| panic!("static turn{turn}: {e}"));
        dev.tick(&mut b, 1. / 60.)
            .unwrap_or_else(|e| panic!("dev turn{turn}: {e}"));
        assert_eq!(
            a.save_bytes().unwrap(),
            oracle.save_bytes().unwrap(),
            "static turn{turn}"
        );
        assert_eq!(
            b.save_bytes().unwrap(),
            oracle.save_bytes().unwrap(),
            "dev turn{turn}"
        );
        assert_eq!(
            static_host.logs().unwrap().entries(),
            dev.logs().unwrap().entries()
        );
        if turn == 47 {
            dev.reload(&guest(name)).unwrap();
        }
    }
    assert!(compound && reverse && outgoing);
}
fn world() -> NativeWorld {
    let mut w = NativeWorld::new(fixture::scene());
    for _ in 0..8 {
        w.step().unwrap();
    }
    w
}
#[test]
fn actual_caught_sdk_failures_and_late_host_failures_rollback_all_staged_effects() {
    for mode in [
        "capacity", "legacy", "v2", "v3", "v4", "v5", "ik-v1", "ik-v2", "owner", "quota", "late",
        "callback", "trap",
    ] {
        let name = format!("rust-root-motion-{mode}");
        let mut static_host = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        let mut a = world();
        let mut b = world();
        let before = a.save_bytes().unwrap();
        assert!(static_host.tick(&mut a, 1. / 60.).is_err(), "static{mode}");
        assert!(dev.tick(&mut b, 1. / 60.).is_err(), "dev{mode}");
        assert_eq!(a.save_bytes().unwrap(), before);
        assert_eq!(b.save_bytes().unwrap(), before);
        assert!(static_host.is_failed() && dev.is_failed());
        assert!(static_host.logs().is_err() && dev.logs().is_err());
        assert!(static_host.tick(&mut a, 1. / 60.).is_err() && dev.tick(&mut b, 1. / 60.).is_err());
    }
}
#[test]
fn identical_successful_prefix_commits_root_controls_structural_numeric_ik_and_logs() {
    let name = "rust-root-motion-prefix";
    let mut static_host = StaticModule::new(embedded::linked(name).unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest(name)).unwrap();
    let mut a = world();
    let mut b = world();
    let before = a.save_bytes().unwrap();
    static_host.tick(&mut a, 1. / 60.).unwrap();
    dev.tick(&mut b, 1. / 60.).unwrap();
    assert_ne!(a.save_bytes().unwrap(), before);
    assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
    assert!(a.entity_identity(fixture::id(77)).is_some());
    let mut rows = Vec::new();
    a.snapshot_root_motion(&mut rows).unwrap();
    assert!(!rows[0].applying);
    assert_eq!(a.ik_state(fixture::id(1)).unwrap()[0].target, [9., 1., 0.]);
    assert_eq!(static_host.logs().unwrap().entries().len(), 1);
    assert_eq!(
        static_host.logs().unwrap().entries(),
        dev.logs().unwrap().entries()
    );
}
#[test]
fn raw_snapshot_and_control_bounds_epochs_shared_quotas_and_legacy_readers_refuse_in_both_hosts() {
    let bytes = wat::parse_str(include_str!(
        "../../incant_wasm_runtime/fixtures/root-motion.wat"
    ))
    .unwrap();
    for mode in 0..28 {
        let mut static_host = StaticModule::new(embedded::linked("root-motion").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes).unwrap();
        let mut a = world();
        let mut b = world();
        let before = a.save_bytes().unwrap();
        let dt = f64::from(mode + 1) / 64.;
        let x = static_host.tick(&mut a, dt);
        let y = dev.tick(&mut b, dt);
        if mode == 0 {
            x.unwrap();
            y.unwrap();
            assert_ne!(a.save_bytes().unwrap(), before);
            assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
        } else {
            let x = x.unwrap_err().to_string();
            let y = y.unwrap_err().to_string();
            let expected = match mode {
                7 => Some("host call quota"),
                8 => Some("transfer quota"),
                9 => Some("command quota"),
                10..=23 => Some("cannot represent root stripping"),
                24 => Some("root-motion command length"),
                25 | 26 => Some("root-motion control values"),
                _ => None,
            };
            if let Some(text) = expected {
                assert!(x.contains(text), "static{mode}: {x}");
                assert!(y.contains(text), "dev{mode}: {y}");
            }
            assert_eq!(a.save_bytes().unwrap(), before, "static{mode}");
            assert_eq!(b.save_bytes().unwrap(), before, "dev{mode}");
        }
    }
}
#[test]
fn unknown_root_versions_signatures_and_imports_remain_closed() {
    for import in [
        r#"(import "incant_root_motion_v2" "snapshot_len" (func(param i64)(result i32)))"#,
        r#"(import "incant_root_motion_v1" "read_snapshot" (func(param i64 i32)(result i32)))"#,
        r#"(import "incant_root_motion_v1" "control" (func(param i64 i32 i32)))"#,
        r#"(import "incant_root_motion_v1" "reset" (func(param i64)(result i32)))"#,
    ] {
        let bytes=wat::parse_str(format!("(module {import} (memory(export \"memory\")1) (func(export \"incant_abi\")(result i32)i32.const 1)(func(export \"update\")(param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&bytes).is_err());
        assert!(incant_wasm_cook::prepare(&bytes).is_err());
    }
}

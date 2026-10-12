#![cfg(feature = "static-conformance")]
#[path = "../../incant_wasm_runtime/tests/support/animation_poses.rs"]
mod fixture;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
const DT: f64 = 1.0 / 60.0;
fn guest(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
#[test]
fn actual_sdk_records_every_tree_and_slerp_contribution_against_direct_native_oracle() {
    let name = "rust-animation-poses";
    let mut native = StaticModule::new(embedded::linked(name).unwrap()).unwrap();
    let mut dev = DevModuleHost::new(&guest(name)).unwrap();
    let mut a = NativeWorld::new(fixture::scene());
    let mut b = NativeWorld::new(fixture::scene());
    let mut oracle = NativeWorld::new(fixture::scene());
    for turn in 0..20 {
        a.step().unwrap();
        b.step().unwrap();
        oracle.step().unwrap();
        fixture::oracle(&mut oracle, turn);
        native.tick(&mut a, DT).unwrap();
        dev.tick(&mut b, DT).unwrap();
        assert_eq!(
            a.save_bytes().unwrap(),
            oracle.save_bytes().unwrap(),
            "static turn{turn}"
        );
        assert_eq!(
            b.save_bytes().unwrap(),
            oracle.save_bytes().unwrap(),
            "development turn{turn}"
        );
        assert_eq!(
            native.logs().unwrap().entries(),
            dev.logs().unwrap().entries()
        );
        assert_eq!(native.logs().unwrap().entries().len(), turn as usize + 1);
        if turn == 2 {
            assert!(a.animation_graph_state(fixture::id(1)).unwrap().paused);
        }
        if turn == 7 {
            dev.reload(&guest(name)).unwrap();
        }
        if turn == 12 {
            assert!(a.animation_state(fixture::id(1)).unwrap().completed);
        }
    }
}
#[test]
fn actual_caught_errors_and_old_snapshot_refusals_discard_the_entire_staged_prefix() {
    for mode in [
        "capacity",
        "legacy",
        "v2",
        "v3",
        "number",
        "parameter",
        "late",
        "callback",
        "trap",
    ] {
        let name = format!("rust-animation-poses-{mode}");
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        let mut a = NativeWorld::new(fixture::scene());
        let mut b = NativeWorld::new(fixture::scene());
        let before = a.save_bytes().unwrap();
        assert!(native.tick(&mut a, DT).is_err(), "static {mode}");
        assert!(dev.tick(&mut b, DT).is_err(), "development {mode}");
        assert_eq!(a.save_bytes().unwrap(), before);
        assert_eq!(b.save_bytes().unwrap(), before);
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.logs().is_err() && dev.logs().is_err());
        assert!(native.tick(&mut a, DT).is_err() && dev.tick(&mut b, DT).is_err());
    }
}
#[test]
fn weighted_snapshot_unknown_versions_wrong_signatures_and_control_import_are_refused() {
    for import in [
        r#"(import "incant_animation_v6" "snapshot_len" (func(param i64)(result i32)))"#,
        r#"(import "incant_animation_v4" "read_snapshot" (func(param i64 i32)(result i32)))"#,
        r#"(import "incant_animation_v4" "control" (func(param i64 i32 i32)))"#,
    ] {
        let bytes=wat::parse_str(format!("(module {import} (memory(export \"memory\")1) (func(export \"incant_abi\")(result i32)i32.const 1)(func(export \"update\")(param i64 f64)))")).unwrap();
        assert!(DevModuleHost::new(&bytes).is_err());
        assert!(incant_wasm_cook::prepare(&bytes).is_err());
    }
}

#[test]
fn actual_raw_imports_check_capacity_ranges_epoch_and_quota_before_world_acceptance() {
    let bytes = wat::parse_str(include_str!(
        "../../incant_wasm_runtime/fixtures/animation-poses.wat"
    ))
    .unwrap();
    for mode in [1, 2, 3, 4, 5, 6, 7, 8, 10, 12] {
        let mut native = StaticModule::new(embedded::linked("animation-poses").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes).unwrap();
        let mut a = NativeWorld::new(fixture::scene());
        let mut b = NativeWorld::new(fixture::scene());
        let before = a.save_bytes().unwrap();
        let dt = f64::from(mode) / 64.0;
        let native_result = native.tick(&mut a, dt);
        let dev_result = dev.tick(&mut b, dt);
        if mode == 1 || mode == 12 {
            native_result.unwrap();
            dev_result.unwrap();
            assert_eq!(a.save_bytes().unwrap(), b.save_bytes().unwrap());
            assert_ne!(a.save_bytes().unwrap(), before);
            assert_eq!(
                native.logs().unwrap().entries(),
                dev.logs().unwrap().entries()
            );
            assert_eq!(native.logs().unwrap().entries().len(), 1);
        } else {
            assert!(native_result.is_err(), "static mode{mode}");
            assert!(dev_result.is_err(), "development mode{mode}");
            if mode == 8 {
                assert!(
                    native_result
                        .unwrap_err()
                        .to_string()
                        .contains("host call quota")
                );
                assert!(
                    dev_result
                        .unwrap_err()
                        .to_string()
                        .contains("host call quota")
                );
            }
            assert_eq!(a.save_bytes().unwrap(), before);
            assert_eq!(b.save_bytes().unwrap(), before);
            assert!(native.logs().is_err() && dev.logs().is_err());
        }
    }
}

#![cfg(feature = "wasmtime-host")]
use incant_input::haptics::HapticCapabilities;
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_wasm_dev::DevModuleHost;
fn world() -> NativeWorld {
    NativeWorld::new(CookedScene::new(StableId([1; 16]), 64, vec![]).unwrap())
}
fn wasm(namespace: &str, parameter: &str, result: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
      (import "{namespace}" "snapshot_len" (func $read (param {parameter}) (result {result})))
      (memory (export "memory") 1)
      (func (export "incant_abi") (result i32) (i32.const 1))
      (func (export "update") (param i64 f64)))"#
    ))
    .unwrap()
}
#[test]
fn versioned_imports_reject_unknown_namespaces_and_wrong_signatures_before_startup() {
    assert!(DevModuleHost::new(&wasm("incant_haptics_v1", "i64", "i32")).is_ok());
    for (namespace, parameter, result) in [
        ("incant_haptics_v2", "i64", "i32"),
        ("incant_haptics_v1", "i32", "i32"),
        ("incant_haptics_v1", "i64", "i64"),
    ] {
        let bytes = wasm(namespace, parameter, result);
        assert!(incant_wasm_cook::prepare(&bytes).is_err());
        assert!(DevModuleHost::new(&bytes).is_err());
    }
}
#[test]
fn startup_has_no_haptic_tick_authority_and_failed_reload_preserves_current_host() {
    let bad = wat::parse_str(include_str!(
        "../../incant_wasm_runtime/fixtures/haptics-start.wat"
    ))
    .unwrap();
    assert!(DevModuleHost::new(&bad).is_err());
    let mut host = DevModuleHost::new(&wasm("incant_haptics_v1", "i64", "i32")).unwrap();
    host.set_haptic_capabilities(HapticCapabilities::default().with_gamepad(0, 7).unwrap())
        .unwrap();
    host.tick(&mut world(), 0.25).unwrap();
    assert!(host.reload(&bad).is_err());
    assert!(!host.is_failed());
    assert!(host.take_haptics().unwrap().is_empty());
}

#![cfg(feature = "wasmtime-host")]
use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_wasm_dev::DevModuleHost;

fn module(body: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
      (import "incant_session_v1" "request_restart" (func $restart (param i64) (result i32)))
      (memory (export "memory") 1)
      (func (export "incant_abi") (result i32) (i32.const 1))
      (func (export "update") (param $e i64) (param f64) {body}))"#
    ))
    .unwrap()
}
fn world() -> NativeWorld {
    NativeWorld::new(CookedScene::new(StableId([0; 16]), 64, vec![]).unwrap())
}
#[test]
fn actual_wasmtime_import_requires_opt_in_and_never_publishes_after_a_trap() {
    let bytes = module("(drop (call $restart (local.get $e)))");
    let mut host = DevModuleHost::new(&bytes).unwrap();
    assert!(host.tick(&mut world(), 0.25).is_err());
    assert!(host.take_restart_request().is_err());
    let mut host = DevModuleHost::new(&bytes).unwrap();
    host.enable_restart_requests().unwrap();
    let mut world = world();
    host.tick(&mut world, 0.25).unwrap();
    assert!(host.take_restart_request().unwrap());
    assert!(!host.take_restart_request().unwrap());
    host.reload(&module("(drop (call $restart (local.get $e))) unreachable"))
        .unwrap();
    assert!(host.tick(&mut world, 0.25).is_err());
    assert!(host.take_restart_request().is_err());
}
#[test]
fn successful_reload_keeps_trusted_authority_without_replaying_a_request() {
    let bytes = module("(drop (call $restart (local.get $e)))");
    let mut host = DevModuleHost::new(&bytes).unwrap();
    host.enable_restart_requests().unwrap();
    let mut world = world();
    host.tick(&mut world, 0.25).unwrap();
    host.reload(&bytes).unwrap();
    assert!(!host.take_restart_request().unwrap());
    host.tick(&mut world, 0.25).unwrap();
    assert!(host.take_restart_request().unwrap());
}
#[test]
fn unknown_session_namespace_or_signature_is_rejected_at_instantiation() {
    let bytes = module("(drop (call $restart (local.get $e)))");
    assert!(incant_wasm_cook::prepare(&bytes).is_ok());
    for (namespace, parameter, result) in [
        ("incant_session_v2", "i64", "i32"),
        ("incant_session_v1", "i32", "i32"),
        ("incant_session_v1", "i64", "i64"),
    ] {
        let bad = wat::parse_str(format!(
            r#"(module
      (import "{namespace}" "request_restart" (func (param {parameter}) (result {result})))
      (memory (export "memory") 1)
      (func (export "incant_abi") (result i32) (i32.const 1))
      (func (export "update") (param i64 f64)))"#
        ))
        .unwrap();
        assert!(incant_wasm_cook::prepare(&bad).is_err());
        assert!(DevModuleHost::new(&bad).is_err());
    }
}

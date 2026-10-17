#![cfg(feature = "wasmtime-host")]
use incant_wasm_dev::DevModuleHost;
#[test]
fn versioned_timer_imports_validate_signatures_and_refuse_startup_authority() {
    for (namespace, parameter, result, accepted) in [
        ("incant_timers_v1", "i64", "i32", true),
        ("incant_timers_v2", "i64", "i32", false),
        ("incant_timers_v1", "i32", "i32", false),
        ("incant_timers_v1", "i64", "i64", false),
    ] {
        let wasm = wat::parse_str(format!(
            r#"(module
              (import "{namespace}" "snapshot_len" (func (param {parameter}) (result {result})))
              (memory (export "memory") 1)
              (func (export "incant_abi") (result i32) (i32.const 1))
              (func (export "update") (param i64 f64)))"#
        ))
        .unwrap();
        assert_eq!(incant_wasm_cook::prepare(&wasm).is_ok(), accepted);
        assert_eq!(DevModuleHost::new(&wasm).is_ok(), accepted);
    }
    let startup = wat::parse_str(include_str!(
        "../../incant_wasm_runtime/fixtures/timers-start.wat"
    ))
    .unwrap();
    assert!(DevModuleHost::new(&startup).is_err());
}

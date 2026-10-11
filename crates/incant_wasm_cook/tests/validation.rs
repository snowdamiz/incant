fn module(body: &str, extra: &str) -> Vec<u8> {
    wat::parse_str(format!(r#"(module (memory (export "memory") 1) (func (export "incant_abi") (result i32) i32.const 1) {extra} (func (export "update") (param i64 f64) {body}))"#)).unwrap()
}
#[test]
fn static_growth_is_explicitly_unsupported() {
    for (body, extra) in [
        ("i32.const 1 memory.grow drop", ""),
        (
            "ref.null func i32.const 1 table.grow 0 drop",
            "(table 1 funcref)",
        ),
    ] {
        let error = incant_wasm_cook::prepare(&module(body, extra)).unwrap_err();
        assert!(error.to_string().contains("refuses memory/table growth"));
    }
}
#[test]
fn guest_cannot_access_private_fuel_or_smuggle_an_import() {
    for wasm in [
        module("global.get 0 drop", ""),
        module(
            "",
            "(global (export \"incant_private_fuel\") (mut i64) (i64.const 9))",
        ),
        wat::parse_str("(module (import \"wasi_snapshot_preview1\" \"path_open\" (func)))")
            .unwrap(),
    ] {
        assert!(incant_wasm_cook::prepare(&wasm).is_err());
    }
}
#[test]
fn input_stack_local_and_memory_bounds_are_checked_before_translation() {
    for wasm in [
        wat::parse_str("(module (memory 513))").unwrap(),
        module(
            &format!("{}{}", "i32.const 0 ".repeat(129), "drop ".repeat(129)),
            "",
        ),
        module(&format!("(local {})", "f64 ".repeat(129)), ""),
    ] {
        assert!(incant_wasm_cook::prepare(&wasm).is_err());
    }
    assert!(incant_wasm_cook::prepare(b"\x7fELFfake native cache").is_err());
}
#[test]
fn emitted_module_is_valid_and_meter_is_not_guest_addressable() {
    let result = incant_wasm_cook::prepare(&module("(loop $l (br $l))", "")).unwrap();
    wasmparser::Validator::new().validate_all(&result).unwrap();
    assert!(result.len() > module("(loop $l (br $l))", "").len());
}
#[test]
fn input_capability_is_versioned_with_exact_signatures_before_translation() {
    let source = include_str!("../../incant_wasm_runtime/fixtures/input.wat");
    assert!(incant_wasm_cook::prepare(&wat::parse_str(source).unwrap()).is_ok());
    for invalid in [
        source.replace("incant_input_v1", "incant_input_v2"),
        source.replace("snapshot_len", "unknown_input"),
    ] {
        assert!(incant_wasm_cook::prepare(&wat::parse_str(&invalid).unwrap()).is_err());
    }
    let wrong=wat::parse_str("(module (import \"incant_input_v1\" \"snapshot_len\" (func (param i32) (result i32))) (memory (export \"memory\") 1) (func (export \"incant_abi\") (result i32) i32.const 1) (func (export \"update\") (param i64 f64)))").unwrap();
    assert!(incant_wasm_cook::prepare(&wrong).is_err());
}

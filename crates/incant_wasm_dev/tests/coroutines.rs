#![cfg(feature = "wasmtime-host")]
use incant_wasm_abi::coroutines::{Routine, encode_catalog};
use incant_wasm_dev::DevModuleHost;
fn append_catalog(bytes: &mut Vec<u8>, catalog: &[u8]) {
    let mut section = vec![b"incant_coroutines_v1".len() as u8];
    section.extend(b"incant_coroutines_v1");
    section.extend(catalog);
    bytes.push(0);
    let mut len = section.len();
    loop {
        bytes.push((len & 127) as u8 | if len > 127 { 128 } else { 0 });
        len >>= 7;
        if len == 0 {
            break;
        }
    }
    bytes.extend(section);
}
fn module(startup: bool) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
      (import "incant_coroutines_v1" "next" (func $next (param i64 i32 i32) (result i32)))
      (memory (export "memory") 1)
      (func $start {}) {}
      (func (export "incant_abi") (result i32) i32.const 1)
      (func (export "update") (param i64 f64)))"#,
        if startup {
            "(drop (call $next (i64.const 0) (i32.const 0) (i32.const 0)))"
        } else {
            ""
        },
        if startup { "(start $start)" } else { "" }
    ))
    .unwrap()
}
fn accepted(bytes: &[u8], expected: bool) {
    assert_eq!(incant_wasm_cook::prepare(bytes).is_ok(), expected, "cooker");
    assert_eq!(DevModuleHost::new(bytes).is_ok(), expected, "development");
}
#[test]
fn imports_require_a_single_valid_immutable_catalog() {
    let declaration = encode_catalog::<120>(&[Routine::new("walk", 8, [1; 32]).unwrap()]);
    let mut wasm = module(false);
    accepted(&wasm, false);
    append_catalog(&mut wasm, &declaration);
    accepted(&wasm, true);
    append_catalog(&mut wasm, &declaration);
    accepted(&wasm, false);
    for offset in [0, 8, 12, 80, 84, 88] {
        let mut bad = declaration;
        // Bad magic/version/count/name-length/frame-size/schema respectively.
        match offset {
            0 | 8 | 12 => bad[offset] = 255,
            80 | 84 => bad[offset..offset + 4].fill(255),
            _ => bad[88..].fill(0),
        }
        let mut wasm = module(false);
        append_catalog(&mut wasm, &bad);
        accepted(&wasm, false);
    }
    for len in [0, 8, 16, 119] {
        let mut wasm = module(false);
        append_catalog(&mut wasm, &declaration[..len]);
        accepted(&wasm, false);
    }
}
#[test]
fn declaration_never_grants_startup_access() {
    let mut wasm = module(true);
    append_catalog(
        &mut wasm,
        &encode_catalog::<120>(&[Routine::new("walk", 8, [1; 32]).unwrap()]),
    );
    assert!(incant_wasm_cook::prepare(&wasm).is_ok());
    assert!(DevModuleHost::new(&wasm).is_err());
}

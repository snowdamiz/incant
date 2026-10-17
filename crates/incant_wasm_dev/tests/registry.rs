#![cfg(feature = "wasmtime-host")]
use incant_wasm_abi::{IMPORTS, ValueType};
use incant_wasm_dev::DevModuleHost;
#[test]
fn every_registered_import_has_identical_dev_and_static_cook_signatures() {
    for import in IMPORTS {
        let ty = |v: &ValueType| match v {
            ValueType::I32 => "i32",
            ValueType::I64 => "i64",
        };
        let params = import.params.iter().map(ty).collect::<Vec<_>>().join(" ");
        let results = if import.results.is_empty() {
            String::new()
        } else {
            format!(
                "(result {})",
                import.results.iter().map(ty).collect::<Vec<_>>().join(" ")
            )
        };
        let source = format!(
            "(module (import {:?} {:?} (func (param {params}) {results})) (memory (export \"memory\") 1) (func (export \"incant_abi\") (result i32) i32.const 1) (func (export \"update\") (param i64 f64)))",
            import.namespace, import.name
        );
        let with_declaration = |mut bytes: Vec<u8>| {
            let declaration = match import.namespace {
                "incant_state_v1" => Some(
                    incant_wasm_abi::persistent::Declaration::new(16, [1; 32])
                        .unwrap()
                        .encode()
                        .to_vec(),
                ),
                "incant_coroutines_v1" => Some(
                    incant_wasm_abi::coroutines::encode_catalog::<120>(&[
                        incant_wasm_abi::coroutines::Routine::new("walk", 8, [1; 32]).unwrap(),
                    ])
                    .to_vec(),
                ),
                _ => None,
            };
            if let Some(declaration) = declaration {
                let mut section = vec![import.namespace.len() as u8];
                section.extend(import.namespace.as_bytes());
                section.extend(declaration);
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
            bytes
        };
        let bytes = with_declaration(wat::parse_str(&source).unwrap());
        assert!(DevModuleHost::new(&bytes).is_ok(), "{}", import.name);
        assert!(incant_wasm_cook::prepare(&bytes).is_ok(), "{}", import.name);
        for bad in [
            source.replacen(import.namespace, "incant_unknown_v9", 1),
            source.replacen("(param i64", "(param f32", 1),
        ] {
            let bytes = with_declaration(wat::parse_str(bad).unwrap());
            assert!(DevModuleHost::new(&bytes).is_err());
            assert!(incant_wasm_cook::prepare(&bytes).is_err());
        }
    }
}

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
        let bytes = wat::parse_str(&source).unwrap();
        assert!(DevModuleHost::new(&bytes).is_ok(), "{}", import.name);
        assert!(incant_wasm_cook::prepare(&bytes).is_ok(), "{}", import.name);
        for bad in [
            source.replacen(import.namespace, "incant_unknown_v9", 1),
            source.replacen("(param i64", "(param f32", 1),
        ] {
            let bytes = wat::parse_str(bad).unwrap();
            assert!(DevModuleHost::new(&bytes).is_err());
            assert!(incant_wasm_cook::prepare(&bytes).is_err());
        }
    }
}

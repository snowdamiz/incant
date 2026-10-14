use super::*;
use crate::{CompiledScript, ScriptInstance};
use incant_types::StableId;

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}

#[test]
fn empty_props_emit_no_unused_value_import_or_owned_constructors() {
    let bindings = ScriptBindings::new(
        [3; 32],
        vec![id(2)],
        vec![ScriptInstance {
            entity: id(1),
            source: id(2),
            props: BTreeMap::new(),
        }],
    )
    .unwrap();
    let output = rust(&bindings);
    assert!(!output.contains("EmbeddedScriptValue as V"));
    assert!(output.contains("props: &[]"));
    assert_eq!(output.matches("ScriptBindings::from_embedded(").count(), 1);
    assert!(!output.contains("BTreeMap::from"));
    assert!(!output.contains("vec!"));
}

#[test]
fn generated_ten_thousand_rows_are_data_with_one_shared_materialization() {
    let mut rows: Vec<_> = (0..10_000)
        .map(|n| ScriptInstance {
            entity: id(n),
            source: id(50_000),
            props: BTreeMap::from([("ordinal".into(), ScriptValue::Number(n as f64))]),
        })
        .collect();
    // The compiled roundtrip fixture covers every type, escapes and bitwise -0.
    rows[0].props.insert(
        "mixed".into(),
        ScriptValue::Array(vec![
            ScriptValue::Null,
            ScriptValue::Bool(true),
            ScriptValue::Number(-0.),
            ScriptValue::String("quote\" slash\\ nul\0 newline\n Unicode🦀".into()),
            ScriptValue::Object(BTreeMap::from([(
                "__proto__".into(),
                ScriptValue::Array(vec![]),
            )])),
        ]),
    );
    let bindings = ScriptBindings::new([3; 32], vec![id(50_000)], rows).unwrap();
    let expression = rust(&bindings);
    assert_eq!(expression.matches("I {entity:").count(), 10_000);
    assert_eq!(
        expression.matches("ScriptBindings::from_embedded(").count(),
        1
    );
    assert!(!expression.contains("BTreeMap::from"));
    assert!(!expression.contains("vec!"));
    assert!(!expression.contains(".into()"));
    let code = CompiledScript::compile_project(
        "constant-bindings.cjs",
        "exports.default={sources:[]};",
        bindings.clone(),
    )
    .unwrap();
    assert_eq!(
        code.metadata().bindings_sha256,
        Some(bindings.fingerprint())
    );
    let generated = code.embedded_rust();
    assert_eq!(
        generated.matches("ScriptBindings::from_embedded(").count(),
        1
    );

    // A trusted CPU qualification may retain these exact compiler outputs for
    // a separate rustc consumer. The ordinary test never requires file output.
    if let Some(directory) = std::env::var_os("INCANT_BINDINGS_GENERATED_OUTPUT") {
        let directory = std::path::PathBuf::from(directory);
        std::fs::create_dir(&directory).unwrap();
        std::fs::write(directory.join("behavior.rs"), generated).unwrap();
        std::fs::write(directory.join("bindings.rs"), format!(
            "pub fn load_bindings() -> Result<incant_script::ScriptBindings, incant_script::ScriptError> {{ Ok({expression}) }}\n"
        )).unwrap();
        std::fs::write(
            directory.join("expected.json"),
            serde_json::to_vec(&serde_json::json!({
                "fingerprint": bindings.fingerprint(),
                "instances": 10000,
                "bytecode_sha256": code.metadata().bytecode_sha256,
                "hardening_sha256": code.metadata().hardening_sha256,
            }))
            .unwrap(),
        )
        .unwrap();
    }
}

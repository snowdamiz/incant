//! Trusted build output only; project data is escaped as Rust literals.
use crate::{ScriptBindings, ScriptValue};
use std::collections::BTreeMap;

pub fn rust(bindings: &ScriptBindings) -> String {
    let sources = bindings
        .sources
        .iter()
        .map(|id| format!("incant_types::StableId({:?})", id.0))
        .collect::<Vec<_>>()
        .join(",");
    let instances = bindings.instances.iter().map(|instance| format!(
        "incant_script::ScriptInstance {{entity: incant_types::StableId({:?}), source: incant_types::StableId({:?}), props: {} }}",
        instance.entity.0, instance.source.0, object(&instance.props))).collect::<Vec<_>>().join(",");
    format!(
        "incant_script::ScriptBindings::new({:?}, vec![{sources}], vec![{instances}])?",
        bindings.scene_sha256
    )
}
fn object(values: &BTreeMap<String, ScriptValue>) -> String {
    format!(
        "std::collections::BTreeMap::from([{}])",
        values
            .iter()
            .map(|(key, value)| format!("({key:?}.into(), {})", value_rust(value)))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn value_rust(value: &ScriptValue) -> String {
    let body = match value {
        ScriptValue::Null => "Null".into(),
        ScriptValue::Bool(value) => format!("Bool({value})"),
        ScriptValue::Number(value) => format!("Number(f64::from_bits({}))", value.to_bits()),
        ScriptValue::String(value) => format!("String({value:?}.into())"),
        ScriptValue::Array(values) => format!(
            "Array(vec![{}])",
            values.iter().map(value_rust).collect::<Vec<_>>().join(",")
        ),
        ScriptValue::Object(values) => format!("Object({})", object(values)),
    };
    format!("incant_script::ScriptValue::{body}")
}

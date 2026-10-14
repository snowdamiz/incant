//! Trusted build output only; project data is escaped as Rust literals.
use crate::{ScriptBindings, ScriptValue};
use std::collections::BTreeMap;

pub fn rust(bindings: &ScriptBindings) -> String {
    let value_import = if bindings.instances.iter().any(|row| !row.props.is_empty()) {
        "use incant_script::EmbeddedScriptValue as V;"
    } else {
        ""
    };
    let sources = bindings
        .sources
        .iter()
        .map(|id| format!("incant_types::StableId({:?})", id.0))
        .collect::<Vec<_>>()
        .join(",");
    let instances = bindings.instances.iter().map(|instance| format!(
        "I {{entity: incant_types::StableId({:?}), source: incant_types::StableId({:?}), props: {} }}",
        instance.entity.0, instance.source.0, object(&instance.props))).collect::<Vec<_>>().join(",");
    format!(
        "{{ use incant_script::EmbeddedScriptInstance as I; {value_import} \
         const SOURCES: &[incant_types::StableId] = &[{sources}]; \
         const INSTANCES: &[I<'static>] = &[{instances}]; \
         incant_script::ScriptBindings::from_embedded({:?}, SOURCES, INSTANCES)? }}",
        bindings.scene_sha256
    )
}

#[cfg(test)]
mod tests;
fn object(values: &BTreeMap<String, ScriptValue>) -> String {
    format!(
        "&[{}]",
        values
            .iter()
            .map(|(key, value)| format!("({key:?}, {})", value_rust(value)))
            .collect::<Vec<_>>()
            .join(",")
    )
}
fn value_rust(value: &ScriptValue) -> String {
    let body = match value {
        ScriptValue::Null => "Null".into(),
        ScriptValue::Bool(value) => format!("Bool({value})"),
        ScriptValue::Number(value) => format!("NumberBits({})", value.to_bits()),
        ScriptValue::String(value) => format!("String({value:?})"),
        ScriptValue::Array(values) => format!(
            "Array(&[{}])",
            values.iter().map(value_rust).collect::<Vec<_>>().join(",")
        ),
        ScriptValue::Object(values) => format!("Object({})", object(values)),
    };
    format!("V::{body}")
}

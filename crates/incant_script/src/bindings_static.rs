//! Borrowed data emitted by the trusted build, materialized once at startup.
use crate::{
    MAX_SCRIPT_INSTANCES, MAX_SCRIPT_SOURCES, ScriptBindings, ScriptError, ScriptInstance,
    ScriptValue, bindings::PropsBudget,
};
use incant_types::StableId;
use std::collections::BTreeMap;

/// Constant initial property data, not a persisted format or executable cache.
/// Object entries must have distinct keys in strictly increasing order. Number
/// bits preserve every finite f64, including negative zero, without a decimal
/// conversion. All limits are checked before allocating the owned catalog.
#[derive(Clone, Copy, Debug)]
pub enum EmbeddedScriptValue<'a> {
    Null,
    Bool(bool),
    NumberBits(u64),
    String(&'a str),
    Array(&'a [EmbeddedScriptValue<'a>]),
    Object(&'a [(&'a str, EmbeddedScriptValue<'a>)]),
}

/// A borrowed attachment row emitted as constant data by the trusted cook.
#[derive(Clone, Copy, Debug)]
pub struct EmbeddedScriptInstance<'a> {
    pub entity: StableId,
    pub source: StableId,
    pub props: &'a [(&'a str, EmbeddedScriptValue<'a>)],
}

impl ScriptBindings {
    /// Materialize bounded trusted build data through one shared startup path.
    /// The usual canonical identity, scene and property validation remains in
    /// `new`. This function grants no authority to load executable bytecode.
    #[inline(never)]
    pub fn from_embedded(
        scene_sha256: [u8; 32],
        sources: &[StableId],
        instances: &[EmbeddedScriptInstance<'_>],
    ) -> Result<Self, ScriptError> {
        if sources.is_empty()
            || sources.len() > MAX_SCRIPT_SOURCES
            || instances.is_empty()
            || instances.len() > MAX_SCRIPT_INSTANCES
        {
            return Err(ScriptError::InputLimit);
        }
        let mut budget = PropsBudget::default();
        for instance in instances {
            check_object(instance.props, 0, &mut budget)?;
        }
        let instances = instances
            .iter()
            .map(|instance| ScriptInstance {
                entity: instance.entity,
                source: instance.source,
                props: owned_object(instance.props),
            })
            .collect();
        Self::new(scene_sha256, sources.to_vec(), instances)
    }
}

fn check_object(
    values: &[(&str, EmbeddedScriptValue<'_>)],
    depth: usize,
    budget: &mut PropsBudget,
) -> Result<(), ScriptError> {
    budget.add(depth, 8)?;
    let mut previous = None;
    for (key, value) in values {
        budget.text_size(key)?;
        if previous.is_some_and(|previous| previous >= *key) {
            return Err(ScriptError::ScriptBindings);
        }
        previous = Some(*key);
        check_value(value, depth + 1, budget)?;
    }
    Ok(())
}

fn check_value(
    value: &EmbeddedScriptValue<'_>,
    depth: usize,
    budget: &mut PropsBudget,
) -> Result<(), ScriptError> {
    budget.add(depth, 1)?;
    match value {
        EmbeddedScriptValue::Null | EmbeddedScriptValue::Bool(_) => Ok(()),
        EmbeddedScriptValue::NumberBits(bits) => {
            if f64::from_bits(*bits).is_finite() {
                Ok(())
            } else {
                Err(ScriptError::ScriptBindings)
            }
        }
        EmbeddedScriptValue::String(value) => budget.text_size(value),
        EmbeddedScriptValue::Array(values) => {
            for value in *values {
                check_value(value, depth + 1, budget)?;
            }
            Ok(())
        }
        EmbeddedScriptValue::Object(values) => check_object(values, depth, budget),
    }
}

fn owned_object(values: &[(&str, EmbeddedScriptValue<'_>)]) -> BTreeMap<String, ScriptValue> {
    values
        .iter()
        .map(|(key, value)| ((*key).to_owned(), owned_value(value)))
        .collect()
}

fn owned_value(value: &EmbeddedScriptValue<'_>) -> ScriptValue {
    match value {
        EmbeddedScriptValue::Null => ScriptValue::Null,
        EmbeddedScriptValue::Bool(value) => ScriptValue::Bool(*value),
        EmbeddedScriptValue::NumberBits(bits) => ScriptValue::Number(f64::from_bits(*bits)),
        EmbeddedScriptValue::String(value) => ScriptValue::String((*value).to_owned()),
        EmbeddedScriptValue::Array(values) => {
            ScriptValue::Array(values.iter().map(owned_value).collect())
        }
        EmbeddedScriptValue::Object(values) => ScriptValue::Object(owned_object(values)),
    }
}

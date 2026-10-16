//! Serializable cooperative continuations. Runtime boundaries enforce finite
//! numbers, bounded depth/bytes, identifiers and positive tick delays.
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(untagged)]
pub enum CoroutineValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<CoroutineValue>),
    Object(BTreeMap<String, CoroutineValue>),
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CoroutineResult {
    Done,
    Wait {
        #[serde(rename = "delayTicks")]
        delay_ticks: u32,
        state: CoroutineValue,
    },
}

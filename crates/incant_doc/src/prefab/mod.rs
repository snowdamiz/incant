//! Authoring-only links. Cooked scenes consume the resolved ordinary entities.
mod budget;
mod references;
mod validate;
use crate::{Id, Provenance};
pub use budget::{MAX_PREFAB_BYTES, MAX_PREFAB_NODES, MAX_PREFABS, PayloadBudget, check_budget};
pub use references::remap_components;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
pub use validate::{instance_map, validate, validate_prototype};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Prefab {
    pub id: Id,
    pub name: String,
    pub root: Id,
    #[schemars(length(min = 1, max = 1024))]
    pub nodes: BTreeMap<Id, PrefabNode>,
    pub provenance: Option<Provenance>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrefabNode {
    pub name: String,
    pub parent: Option<Id>,
    #[schemars(length(max = 64))]
    pub components: BTreeMap<String, Value>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PrefabLink {
    pub source: Id,
    pub root: Id,
    pub node: Id,
    /// Whole-component overrides. A listed but absent component is removed.
    #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
    #[schemars(length(max = 64))]
    pub overrides: BTreeSet<String>,
}
#[derive(Debug, thiserror::Error)]
pub enum PrefabError {
    #[error("prefab limit: {0}")]
    Limit(&'static str),
    #[error("invalid prefab: {0}")]
    Invalid(String),
}
pub(crate) fn invalid(message: impl Into<String>) -> PrefabError {
    PrefabError::Invalid(message.into())
}

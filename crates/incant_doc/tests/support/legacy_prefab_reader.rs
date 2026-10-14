// Exact Project/Scene/Entity field definitions from main268 fdd480355ba70b28bbebe35f58b8af188f0abb82.
// Other referenced record types are unchanged by the prefab feature.
#![allow(dead_code)]
use incant_doc::{Asset, Id, ProjectSettings, Provenance, SceneProgram, ScriptSource, StringTable};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Project {
    pub id: Id,
    pub schema_version: u32,
    pub name: String,
    pub scenes: BTreeMap<Id, Scene>,
    pub assets: BTreeMap<Id, Asset>,
    pub scripts: BTreeMap<Id, ScriptSource>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub string_tables: BTreeMap<Id, StringTable>,
    pub settings: ProjectSettings,
    pub memory: BTreeMap<String, String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub id: Id,
    pub name: String,
    pub entities: BTreeMap<Id, Entity>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub program: Option<SceneProgram>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Entity {
    pub id: Id,
    pub name: String,
    pub parent: Option<Id>,
    pub components: BTreeMap<String, Value>,
    pub provenance: Option<Provenance>,
}

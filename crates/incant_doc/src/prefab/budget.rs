use super::{Prefab, PrefabError};
use crate::Project;
use serde_json::Value;
use std::collections::BTreeMap;

pub const MAX_PREFABS: usize = 256;
pub const MAX_PREFAB_NODES: usize = 1_024;
pub const MAX_PREFAB_BYTES: usize = 32 * 1024 * 1024;
/// Conservative logical payload charge, after JSON parsing. Not an allocator/RSS bound.
pub struct PayloadBudget {
    bytes: usize,
    values: usize,
}
impl Default for PayloadBudget {
    fn default() -> Self {
        Self {
            bytes: MAX_PREFAB_BYTES,
            values: 1_000_000,
        }
    }
}
impl PayloadBudget {
    pub fn charge(&mut self, bytes: usize) -> Result<(), PrefabError> {
        self.bytes = self
            .bytes
            .checked_sub(bytes)
            .ok_or(PrefabError::Limit("32 MiB payload"))?;
        Ok(())
    }
    pub fn text(&mut self, text: &str) -> Result<(), PrefabError> {
        self.charge(
            text.len()
                .checked_mul(6)
                .and_then(|n| n.checked_add(2))
                .ok_or(PrefabError::Limit("string bytes"))?,
        )
    }
    fn value(&mut self, value: &Value, depth: usize) -> Result<(), PrefabError> {
        if depth > 64 {
            return Err(PrefabError::Limit("JSON depth 64"));
        }
        self.values = self
            .values
            .checked_sub(1)
            .ok_or(PrefabError::Limit("one million values"))?;
        self.charge(32)?;
        match value {
            Value::String(s) => self.text(s)?,
            Value::Array(a) => {
                for v in a {
                    self.value(v, depth + 1)?;
                }
            }
            Value::Object(o) => {
                for (k, v) in o {
                    self.text(k)?;
                    self.value(v, depth + 1)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
    pub fn component(&mut self, kind: &str, value: &Value) -> Result<(), PrefabError> {
        self.text(kind)?;
        self.value(value, 0)
    }
    pub fn components(&mut self, values: &BTreeMap<String, Value>) -> Result<(), PrefabError> {
        if values.len() > 64 {
            return Err(PrefabError::Limit("64 component slots"));
        }
        for (kind, value) in values {
            self.text(kind)?;
            self.value(value, 0)?;
        }
        Ok(())
    }
    pub fn prototype(&mut self, prefab: &Prefab) -> Result<(), PrefabError> {
        if prefab.nodes.is_empty() || prefab.nodes.len() > MAX_PREFAB_NODES {
            return Err(PrefabError::Limit("prototype requires 1..1024 nodes"));
        }
        self.text(&prefab.id)?;
        self.text(&prefab.name)?;
        self.text(&prefab.root)?;
        if let Some(p) = &prefab.provenance {
            self.text(&p.actor)?;
            self.text(&p.transaction)?;
            if let Some(s) = &p.model {
                self.text(s)?;
            }
            if let Some(s) = &p.conversation_id {
                self.text(s)?;
            }
        }
        for (id, node) in &prefab.nodes {
            self.text(id)?;
            self.text(&node.name)?;
            if let Some(p) = &node.parent {
                self.text(p)?;
            }
            self.components(&node.components)?;
        }
        Ok(())
    }
}
pub fn check_budget(project: &Project) -> Result<(), PrefabError> {
    if project.prefabs.len() > MAX_PREFABS {
        return Err(PrefabError::Limit("256 prototypes"));
    }
    let mut budget = PayloadBudget::default();
    let mut nodes = 0usize;
    for p in project.prefabs.values() {
        nodes = nodes
            .checked_add(p.nodes.len())
            .ok_or(PrefabError::Limit("prototype nodes"))?;
        if nodes > 16_384 {
            return Err(PrefabError::Limit("16384 total prototype nodes"));
        }
        budget.prototype(p)?;
    }
    let mut linked = 0usize;
    for e in project.scenes.values().flat_map(|s| s.entities.values()) {
        let Some(link) = &e.prefab else {
            continue;
        };
        linked += 1;
        if linked > 100_000 {
            return Err(PrefabError::Limit("100000 linked entities"));
        }
        budget.text(&e.id)?;
        budget.text(&e.name)?;
        budget.text(&link.source)?;
        budget.text(&link.root)?;
        budget.text(&link.node)?;
        if link.overrides.len() > 64 {
            return Err(PrefabError::Limit("64 override slots"));
        }
        for kind in &link.overrides {
            budget.text(kind)?;
        }
        budget.components(&e.components)?;
    }
    Ok(())
}

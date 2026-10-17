//! Data-only cooperative tasks. A candidate owns typed state, never JS roots,
//! continuations or native loans. Publication shares the script transaction.
mod api;
use crate::{
    ScriptError, ScriptValue,
    persistence::value::Cost,
    schedule::{Key, Scope},
};
pub(crate) use api::Tick;
use incant_runtime::StableId;
use std::{collections::BTreeMap, sync::Arc};

pub(crate) const MAX_TASKS: usize = 128;
pub(crate) const MAX_CALLS: usize = 256;
pub(crate) const FRAME_BYTES: usize = 16 * 1024;
pub(crate) const FRAME_NODES: usize = 1024;
pub(crate) const STORE_BYTES: usize = 256 * 1024;
pub(crate) const STORE_NODES: usize = 16_384;
pub(crate) const ENCODED_BYTES: usize = 4 + STORE_BYTES + MAX_TASKS * 180;

#[derive(Clone)]
pub(crate) struct Task {
    pub routine: String,
    pub entity: Option<StableId>,
    pub due: u64,
    pub value: ScriptValue,
    pub cost: Cost,
}
#[derive(Clone, Default)]
pub(crate) struct Store {
    pub entries: BTreeMap<Key, Arc<Task>>,
    pub cost: Cost,
}
impl Store {
    pub fn insert(&mut self, key: Key, task: Arc<Task>) -> Result<(), ScriptError> {
        if !self.entries.contains_key(&key) && self.entries.len() >= MAX_TASKS {
            return Err(ScriptError::InputLimit);
        }
        let old = self.entries.get(&key).map_or(Cost::default(), |v| v.cost);
        let cost = Cost {
            bytes: self.cost.bytes - old.bytes + task.cost.bytes,
            nodes: self.cost.nodes - old.nodes + task.cost.nodes,
        };
        if cost.bytes > STORE_BYTES || cost.nodes > STORE_NODES {
            return Err(ScriptError::InputLimit);
        }
        self.entries.insert(key, task);
        self.cost = cost;
        Ok(())
    }
    pub fn remove(&mut self, key: &Key) {
        if let Some(task) = self.entries.remove(key) {
            self.cost.bytes -= task.cost.bytes;
            self.cost.nodes -= task.cost.nodes;
        }
    }
    pub fn retain(&mut self, mut keep: impl FnMut(&Key, &Task) -> bool) {
        self.entries.retain(|key, task| {
            if keep(key, task) {
                true
            } else {
                self.cost.bytes -= task.cost.bytes;
                self.cost.nodes -= task.cost.nodes;
                false
            }
        });
    }
}
pub(crate) fn valid_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_.:-".contains(&b))
}
pub(crate) fn due(tick: u64, delay: u32) -> Result<u64, ScriptError> {
    tick.checked_add(u64::from(delay))
        .filter(|&next| delay > 0 && next <= crate::schedule::MAX_TICK)
        .ok_or(ScriptError::TimerRange)
}
/// Source/routine identities are copied once from the captured JS catalog.
#[derive(Default)]
pub(crate) struct Catalog(pub Vec<Arc<[String]>>, pub bool);
impl Catalog {
    pub fn enabled(&self) -> bool {
        self.1
    }
    pub fn known(
        &self,
        bindings: Option<&crate::ScriptBindings>,
        scope: Scope,
        name: &str,
    ) -> bool {
        let index = match (bindings, scope) {
            (None, Scope::Global) => Some(0),
            (Some(b), Scope::Source(id)) => b.sources().binary_search(&id).ok(),
            _ => None,
        };
        index
            .and_then(|i| self.0.get(i))
            .is_some_and(|names| names.iter().any(|s| s == name))
    }
}

#[cfg(all(test, feature = "compiler"))]
mod tests;

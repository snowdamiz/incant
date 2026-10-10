//! Parent-first ordering shared by checked loads and structural sync points.
use crate::{SceneError, StableId};
use glam::DMat4;
use std::collections::{BTreeMap, VecDeque};

pub(crate) struct Node {
    pub id: StableId,
    pub parent: Option<usize>,
    pub world: DMat4,
}

pub(crate) fn prepare(
    parents: &BTreeMap<StableId, Option<StableId>>,
    local: impl Fn(StableId) -> DMat4,
) -> Result<Vec<Node>, SceneError> {
    let mut children = BTreeMap::<StableId, Vec<StableId>>::new();
    let mut pending = VecDeque::new();
    for (&id, &parent) in parents {
        if let Some(parent) = parent {
            if !parents.contains_key(&parent) {
                return Err(SceneError::Parent(id));
            }
            children.entry(parent).or_default().push(id);
        } else {
            pending.push_back((id, None));
        }
    }
    let mut nodes: Vec<Node> = Vec::with_capacity(parents.len());
    while let Some((id, parent)) = pending.pop_front() {
        let local = local(id);
        let world = parent.map_or(local, |parent: usize| nodes[parent].world * local);
        if !world.is_finite() {
            return Err(SceneError::Component(id));
        }
        let slot = nodes.len();
        nodes.push(Node { id, parent, world });
        if let Some(children) = children.remove(&id) {
            pending.extend(children.into_iter().map(|child| (child, Some(slot))));
        }
    }
    if nodes.len() != parents.len() {
        return Err(SceneError::Invalid("parent cycle"));
    }
    Ok(nodes)
}

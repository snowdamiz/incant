use crate::{Result, invalid};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelNode {
    pub name: Option<String>,
    pub children: Vec<usize>,
    /// Primitive indices in the containing imported or cooked model.
    pub meshes: Vec<usize>,
    /// Column-major local transform, as authored by glTF.
    pub transform: [[f32; 4]; 4],
}
/// Check child references, single-parent ownership and acyclicity. This does not
/// validate mesh references or transform values, which belong to the containing model.
pub fn validate_nodes(nodes: &[ModelNode]) -> Result<()> {
    if nodes
        .iter()
        .any(|node| node.children.iter().any(|&i| i >= nodes.len()))
    {
        return Err(invalid("invalid model metadata reference"));
    }
    let mut parents = vec![0; nodes.len()];
    for n in nodes {
        for &c in &n.children {
            parents[c] += 1;
            if parents[c] > 1 {
                return Err(invalid("node has multiple parents"));
            }
        }
    }
    let mut queue: Vec<_> = parents
        .iter()
        .enumerate()
        .filter_map(|(i, p)| (*p == 0).then_some(i))
        .collect();
    let mut visited = 0;
    while let Some(i) = queue.pop() {
        visited += 1;
        for &c in &nodes[i].children {
            parents[c] -= 1;
            if parents[c] == 0 {
                queue.push(c);
            }
        }
    }
    if visited != nodes.len() {
        return Err(invalid("glTF node cycle"));
    }
    Ok(())
}

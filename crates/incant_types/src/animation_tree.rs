//! Stable authoring references for bounded nested animation motions.
use crate::{AnimationParameter, Id, MAX_ANIMATION_BLEND_POINTS, MAX_ANIMATION_CLIPS};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_BLEND_TREE_NODES: usize = 64;
pub const MAX_BLEND_TREE_DEPTH: usize = 8;
pub const MAX_BLEND_TREE_POSES: usize = 8;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnimationTree {
    pub root: Id,
    #[schemars(length(min = 1, max = 64))]
    pub nodes: BTreeMap<Id, AnimationTreeNode>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnimationTreeNode {
    Clip {
        clip: Option<u16>,
    },
    Blend1d {
        parameter: Id,
        #[schemars(length(min = 2, max = 8))]
        points: Vec<AnimationTreePoint>,
    },
    Blend2d {
        parameters: [Id; 2],
        #[schemars(length(min = 2, max = 8))]
        x: Vec<f64>,
        #[schemars(length(min = 2, max = 8))]
        y: Vec<f64>,
        /// Row-major children: y * x.len() + x.
        #[schemars(length(min = 4, max = 64))]
        children: Vec<Id>,
    },
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnimationTreePoint {
    pub threshold: f64,
    pub node: Id,
}

impl AnimationTree {
    /// Cold deterministic postorder used by the cooker to resolve ULIDs once.
    /// This also validates cycles, connectivity, depth and active-pose capacity
    /// before an authoring transaction may commit.
    pub fn compile_order<'a>(
        &'a self,
        parameters: &BTreeMap<Id, AnimationParameter>,
    ) -> Result<Vec<&'a str>, &'static str> {
        if !(1..=MAX_BLEND_TREE_NODES).contains(&self.nodes.len())
            || !self.nodes.contains_key(&self.root)
            || self
                .nodes
                .keys()
                .any(|id| !crate::animation_graph::canonical_id(id))
        {
            return Err("animation tree capacity or canonical node ID");
        }
        let mut visiting = BTreeSet::new();
        let mut visited = BTreeMap::new();
        let mut order = Vec::with_capacity(self.nodes.len());
        self.visit(
            &self.root,
            parameters,
            &mut visiting,
            &mut visited,
            &mut order,
        )?;
        if order.len() != self.nodes.len() {
            return Err("animation tree unreachable node");
        }
        Ok(order)
    }

    fn visit<'a>(
        &'a self,
        id: &'a str,
        parameters: &BTreeMap<Id, AnimationParameter>,
        visiting: &mut BTreeSet<&'a str>,
        visited: &mut BTreeMap<&'a str, (usize, usize)>,
        order: &mut Vec<&'a str>,
    ) -> Result<(usize, usize), &'static str> {
        if let Some(result) = visited.get(id) {
            return Ok(*result);
        }
        if visiting.len() >= MAX_BLEND_TREE_DEPTH || !visiting.insert(id) {
            return Err("animation tree cycle or depth");
        }
        let node = self.nodes.get(id).ok_or("animation tree node reference")?;
        let mut depth = 1;
        let mut child = |id: &'a str| -> Result<usize, &'static str> {
            let (child_depth, poses) = self.visit(id, parameters, visiting, visited, order)?;
            depth = depth.max(child_depth + 1);
            Ok(poses)
        };
        let axis = |parameter: &str, values: &[f64]| -> Result<(), &'static str> {
            let p = parameters
                .get(parameter)
                .ok_or("animation tree parameter")?;
            if !(2..=MAX_ANIMATION_BLEND_POINTS).contains(&values.len())
                || values
                    .iter()
                    .any(|v| !v.is_finite() || *v < p.min || *v > p.max)
                || values.windows(2).any(|v| v[0] >= v[1])
            {
                return Err("animation tree axis");
            }
            Ok(())
        };
        let poses = match node {
            AnimationTreeNode::Clip { clip } => {
                if clip.is_some_and(|n| usize::from(n) >= MAX_ANIMATION_CLIPS) {
                    return Err("animation tree clip");
                }
                1
            }
            AnimationTreeNode::Blend1d { parameter, points } => {
                if !(2..=MAX_ANIMATION_BLEND_POINTS).contains(&points.len()) {
                    return Err("animation tree point count");
                }
                let mut values = [0.; MAX_ANIMATION_BLEND_POINTS];
                let mut counts = [0; MAX_ANIMATION_BLEND_POINTS];
                for (i, p) in points.iter().enumerate() {
                    values[i] = p.threshold;
                    counts[i] = child(&p.node)?;
                }
                axis(parameter, &values[..points.len()])?;
                counts[..points.len()]
                    .windows(2)
                    .map(|c| c[0] + c[1])
                    .max()
                    .unwrap_or(0)
            }
            AnimationTreeNode::Blend2d {
                parameters,
                x,
                y,
                children,
            } => {
                if parameters[0] == parameters[1] {
                    return Err("animation tree distinct axes");
                }
                axis(&parameters[0], x)?;
                axis(&parameters[1], y)?;
                if children.len() != x.len() * y.len() {
                    return Err("animation tree grid shape");
                }
                let mut counts = [0; MAX_ANIMATION_BLEND_POINTS * MAX_ANIMATION_BLEND_POINTS];
                for (i, id) in children.iter().enumerate() {
                    counts[i] = child(id)?;
                }
                let mut bound = 0;
                for row in 0..y.len() - 1 {
                    for col in 0..x.len() - 1 {
                        let i = row * x.len() + col;
                        bound = bound.max(
                            counts[i]
                                + counts[i + 1]
                                + counts[i + x.len()]
                                + counts[i + x.len() + 1],
                        );
                    }
                }
                bound
            }
        };
        if depth > MAX_BLEND_TREE_DEPTH || poses > MAX_BLEND_TREE_POSES {
            return Err("animation tree depth or active poses");
        }
        visiting.remove(id);
        visited.insert(id, (depth, poses));
        order.push(id);
        Ok((depth, poses))
    }
}

//! Compiled nested blend trees. These numeric tables are admitted once; sampling
//! uses bounded stack storage without authoring data, recursion or allocation.
pub(crate) mod binary;
mod evaluate;
mod validate;

use super::NativeAnimationParameter;
use crate::SceneError;
use std::sync::Arc;

pub use incant_types::{
    AnimationPoseSamples, MAX_ANIMATION_POSE_SAMPLES, MAX_BLEND_TREE_DEPTH, MAX_BLEND_TREE_NODES,
    MAX_BLEND_TREE_POSES, WeightedAnimationSample,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeBlendTreePoint {
    pub threshold: f64,
    pub child: u8,
}

/// Children precede parents in the compiled table. The last node is the root;
/// every node must be reachable from it. Shared children are allowed.
#[derive(Debug, Clone, PartialEq)]
pub enum NativeBlendTreeNode {
    Clip(Option<u16>),
    Blend1d {
        parameter: u8,
        points: Arc<[NativeBlendTreePoint]>,
    },
    /// Bilinear interpolation on a rectangular grid, clamped at its edges.
    /// Children use row-major order: y * x.len() + x.
    Blend2d {
        parameters: [u8; 2],
        x: Arc<[f64]>,
        y: Arc<[f64]>,
        children: Arc<[u8]>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct NativeAnimationBlendTree {
    nodes: Arc<[NativeBlendTreeNode]>,
    parameters: Arc<[NativeAnimationParameter]>,
    clip_durations: Arc<[f32]>,
}

impl NativeAnimationBlendTree {
    /// Cold boundary shared by compilation and future binary admission. No
    /// unchecked constructor is exposed. Immutable parameter/model tables may
    /// share their Arcs with the enclosing graph.
    pub fn new(
        nodes: Vec<NativeBlendTreeNode>,
        parameters: Arc<[NativeAnimationParameter]>,
        clip_durations: Arc<[f32]>,
    ) -> Result<Self, SceneError> {
        validate::validate(&nodes, &parameters, &clip_durations)?;
        Ok(Self {
            nodes: nodes.into(),
            parameters,
            clip_durations,
        })
    }

    pub fn matches_context(
        &self,
        parameters: &[NativeAnimationParameter],
        durations: &[f32],
    ) -> bool {
        self.parameters.as_ref() == parameters && self.clip_durations.as_ref() == durations
    }

    pub fn nodes(&self) -> &[NativeBlendTreeNode] {
        &self.nodes
    }
}

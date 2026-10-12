//! Compiled nested blend trees. These numeric tables are admitted once; sampling
//! uses bounded stack storage without authoring data, recursion or allocation.
mod evaluate;
mod validate;

use super::NativeAnimationParameter;
use crate::{AnimationSample, SceneError};
use std::sync::Arc;

pub const MAX_BLEND_TREE_NODES: usize = 64;
pub const MAX_BLEND_TREE_DEPTH: usize = 8;
pub const MAX_BLEND_TREE_POSES: usize = 8;
pub const MAX_ANIMATION_POSE_SAMPLES: usize = 2 * MAX_BLEND_TREE_POSES;

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

    pub fn nodes(&self) -> &[NativeBlendTreeNode] {
        &self.nodes
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WeightedAnimationSample {
    pub sample: AnimationSample,
    pub weight: f64,
}

/// Copyable normalized local-pose contributions. A state can contribute up to
/// eight samples and a fade can join two such states. This does not blend final
/// matrices; the retained model sampler blends TRS before building a hierarchy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationPoseSamples {
    samples: [WeightedAnimationSample; MAX_ANIMATION_POSE_SAMPLES],
    len: u8,
}

impl AnimationPoseSamples {
    const EMPTY: Self = Self {
        samples: [WeightedAnimationSample {
            sample: AnimationSample::REST,
            weight: 0.0,
        }; MAX_ANIMATION_POSE_SAMPLES],
        len: 0,
    };

    pub fn samples(&self) -> &[WeightedAnimationSample] {
        &self.samples[..usize::from(self.len)]
    }

    /// `weight` is the destination contribution, matching ordinary crossfades.
    /// Endpoints preserve the input exactly. Equal clip/time entries coalesce;
    /// different phases of the same clip remain separate.
    pub fn blend(&self, destination: &Self, weight: f64) -> Result<Self, SceneError> {
        if !weight.is_finite() || !(0.0..=1.0).contains(&weight) {
            return Err(SceneError::Invalid("animation pose blend weight"));
        }
        if weight == 0.0 {
            return Ok(*self);
        }
        if weight == 1.0 {
            return Ok(*destination);
        }
        let mut result = Self::EMPTY;
        for (pose, factor) in [(self, 1.0 - weight), (destination, weight)] {
            for entry in pose.samples() {
                result.push(entry.sample, entry.weight * factor)?;
            }
        }
        result.normalize();
        Ok(result)
    }

    fn push(&mut self, sample: AnimationSample, weight: f64) -> Result<(), SceneError> {
        if weight == 0.0 {
            return Ok(());
        }
        if let Some(entry) = self.samples[..usize::from(self.len)]
            .iter_mut()
            .find(|entry| entry.sample == sample)
        {
            entry.weight += weight;
            return Ok(());
        }
        let entry = self
            .samples
            .get_mut(usize::from(self.len))
            .ok_or(SceneError::Invalid("animation pose sample capacity"))?;
        *entry = WeightedAnimationSample { sample, weight };
        self.len += 1;
        Ok(())
    }

    fn normalize(&mut self) {
        let entries = &mut self.samples[..usize::from(self.len)];
        let sum: f64 = entries.iter().map(|entry| entry.weight).sum();
        for entry in entries {
            entry.weight /= sum;
        }
    }
}

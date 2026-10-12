//! Post-animation constraints. Targets and poles use cooked model coordinates,
//! before the entity/world transform. Ordered constraints solve sequentially.
use crate::{Id, StableId};
use serde::{Deserialize, Serialize};

pub const MAX_ANIMATION_IK: usize = 32;
pub const MAX_SCENE_ANIMATION_IK: usize = 4096;
pub const MAX_IK_MODEL_NODES: usize = 4096;
pub const MAX_IK_COORDINATE: f64 = 1_000_000_000.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnimationIk {
    #[schemars(length(min = 1, max = 32))]
    pub constraints: Vec<AnimationIkConstraint>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnimationIkConstraint {
    pub id: Id,
    /// Immediate root, middle and tip model-node indices; not entity IDs.
    pub chain: [u16; 3],
    pub target: [f64; 3],
    pub pole: [f64; 3],
    #[schemars(range(min = 0, max = 1))]
    pub weight: f64,
}
/// Immutable, reusable native/render pose descriptor. Copies contain numeric
/// values only; the surrounding Arc slice is replaced only by accepted controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationIkSample {
    pub id: StableId,
    pub chain: [u16; 3],
    pub target: [f64; 3],
    pub pole: [f64; 3],
    pub weight: f64,
}
impl AnimationIkSample {
    pub fn validate(&self) -> Result<(), &'static str> {
        validate(self.chain, self.target, self.pole, self.weight)
    }
}
fn validate(
    chain: [u16; 3],
    target: [f64; 3],
    pole: [f64; 3],
    weight: f64,
) -> Result<(), &'static str> {
    if chain.iter().any(|n| usize::from(*n) >= MAX_IK_MODEL_NODES)
        || chain[0] == chain[1]
        || chain[0] == chain[2]
        || chain[1] == chain[2]
        || target
            .iter()
            .chain(&pole)
            .any(|n| !n.is_finite() || n.abs() > MAX_IK_COORDINATE)
        || !weight.is_finite()
        || !(0.0..=1.0).contains(&weight)
    {
        return Err("invalid IK chain, target, pole or weight");
    }
    Ok(())
}
impl AnimationIk {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.constraints.is_empty() || self.constraints.len() > MAX_ANIMATION_IK {
            return Err("IK constraint count");
        }
        for (i, c) in self.constraints.iter().enumerate() {
            if !crate::animation_graph::canonical_id(&c.id)
                || self.constraints[..i].iter().any(|prior| prior.id == c.id)
            {
                return Err("IK constraint ID must be a unique canonical ULID");
            }
            validate(c.chain, c.target, c.pole, c.weight)?;
        }
        Ok(())
    }
}

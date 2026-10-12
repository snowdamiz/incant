//! Model-space, rotation-only two-bone constraints on the accepted local pose.
use super::*;
#[path = "ik_math.rs"]
mod math;

pub const MAX_TWO_BONE_IK: usize = 32;

/// An immediate model-node chain, not indices into a skin's palette. Targets
/// and poles are points in the same model space as `AnimationPlayer::globals`.
/// Weight blends each solved local rotation along its shortest quaternion arc;
/// it does not linearly interpolate the tip position. No orientation target,
/// stretching, joint limits, clock, retained target or implicit entity transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TwoBoneIk {
    pub root: usize,
    pub middle: usize,
    pub tip: usize,
    pub target: [f64; 3],
    pub pole: [f64; 3],
    pub weight: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum TwoBoneIkError {
    #[error("two-bone IK exceeds its constraint limit")]
    Limit,
    #[error("two-bone IK target/pole/weight is invalid")]
    Input,
    #[error(
        "two-bone IK requires distinct immediate parent-child nodes with editable root/middle TRS"
    )]
    Chain,
    #[error("two-bone IK requires nonsingular, nonreflected uniform orthogonal frames")]
    UnsupportedTransform,
    #[error("two-bone IK requires nonzero numerically representable bone lengths")]
    Degenerate,
    #[error("two-bone IK produced an unrepresentable pose")]
    Numeric,
}
type IkResult<T> = std::result::Result<T, TwoBoneIkError>;

impl AnimationPlayer {
    /// Apply at most 32 constraints in slice order to the already sampled pose.
    /// Later constraints see earlier staged results, so overlapping chains have
    /// explicit ordered semantics. The entire batch (locals, globals and skin
    /// palettes) publishes together or leaves the prior accepted pose untouched.
    /// No heap allocation, even on refusal; existing scratch buffers are reused.
    ///
    /// Only root/middle local rotations change. Their translations and scale
    /// lengths, the tip's local matrix and every other local matrix are retained.
    /// Effective parent/root/middle frames must be positive uniform similarities
    /// (orthogonality/uniformity tolerance 1e-6). Shear, nonuniform scale,
    /// reflection and fixed-matrix root/middle nodes refuse, including weight 0.
    ///
    /// Unreachable targets clamp to both minimum and maximum reach without bone
    /// stretching. A collinear pole uses the current bend, then a deterministic
    /// perpendicular axis. A target at the root uses the current tip direction,
    /// or the first bone if the tip is also coincident; equal bones fully fold.
    /// Subsequent ordinary sampling resets IK to that sample's authored pose.
    pub fn apply_two_bone_ik(&mut self, constraints: &[TwoBoneIk]) -> IkResult<()> {
        self.validate_ik_constraints(constraints)?;
        self.staged.local.copy_from_slice(&self.live.local);
        self.staged.global.copy_from_slice(&self.live.global);
        self.staged.joints.copy_from_slice(&self.live.joints);
        self.solve_staged_ik(constraints)?;
        std::mem::swap(&mut self.live, &mut self.staged);
        Ok(())
    }
    pub(super) fn validate_ik_constraints(&self, constraints: &[TwoBoneIk]) -> IkResult<()> {
        if constraints.len() > MAX_TWO_BONE_IK {
            return Err(TwoBoneIkError::Limit);
        }
        for c in constraints {
            if !c.weight.is_finite()
                || !(0. ..=1.).contains(&c.weight)
                || c.target.iter().chain(&c.pole).any(|n| !n.is_finite())
            {
                return Err(TwoBoneIkError::Input);
            }
            if [c.root, c.middle, c.tip]
                .iter()
                .any(|&i| i >= self.nodes.len())
                || self.parents[c.middle] != Some(c.root)
                || self.parents[c.tip] != Some(c.middle)
                || self.data.rest[c.root].is_none()
                || self.data.rest[c.middle].is_none()
            {
                return Err(TwoBoneIkError::Chain);
            }
        }
        Ok(())
    }
    pub(super) fn solve_staged_ik(&mut self, constraints: &[TwoBoneIk]) -> IkResult<()> {
        for c in constraints {
            let parent = self.parents[c.root].map_or(DMat4::IDENTITY, |p| self.staged.global[p]);
            let rotations = math::solve(
                c,
                parent,
                self.staged.local[c.root],
                self.staged.local[c.middle],
                self.staged.global[c.root],
                self.staged.global[c.middle],
                self.staged.global[c.tip].w_axis.truncate(),
            )?;
            if c.weight != 0. {
                // Rotate only the linear columns: translation remains exactly
                // unchanged and no TRS decomposition/reconstruction loses scale.
                for (i, delta) in [(c.root, rotations[0]), (c.middle, rotations[1])] {
                    let m = &mut self.staged.local[i];
                    for axis in [&mut m.x_axis, &mut m.y_axis, &mut m.z_axis] {
                        *axis = (delta * axis.truncate()).extend(axis.w);
                    }
                }
                self.staged
                    .rebuild(&self.parents, &self.order, &self.data, &self.offsets)
                    .map_err(|_| TwoBoneIkError::Numeric)?;
            }
        }
        Ok(())
    }
}

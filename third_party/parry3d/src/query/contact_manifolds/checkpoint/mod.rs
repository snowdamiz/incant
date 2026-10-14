//! Closed composite workspace support. HashMap physical buckets are not part of
//! the fast-mode record: capture canonicalizes keys. Deterministic IndexMap
//! retains its actual order. Both algorithms use only entry lookup and a pure
//! timestamp retain; exact manifold ordering comes from compound BVH traversal.
use super::*;
use incant_physics_snapshot as s;
use s::construction::{AllocationPermit, AllocationPlan, ConstructionError, ConstructionResult};
pub(super) mod map;
#[cfg(test)]
mod tests;

fn rows<'a>(
    w: &'a s::ValidatedPairState<'_>,
    pair: u32,
    expected: s::Identity,
) -> ConstructionResult<(&'a s::PairState, &'a [s::SubDetector])> {
    if w.record().identity != expected || expected.dimension as usize != crate::math::DIM {
        return Err(ConstructionError::Payload);
    }
    let p = w
        .record()
        .pairs
        .get(pair as usize)
        .ok_or(ConstructionError::Payload)?;
    let rows = w.detectors(pair).ok_or(ConstructionError::Payload)?;
    #[cfg(not(feature = "enhanced-determinism"))]
    if rows.windows(2).any(|w| w[0].key >= w[1].key) {
        return Err(ConstructionError::Payload);
    }
    Ok((p, rows))
}
impl ContactManifoldsWorkspace {
    /// Capture only the two pinned compound workspace families. Detector ids
    /// refer to the enclosing pair's ordinary manifold order, not solver clusters.
    pub fn checkpoint_capture(&self, budget: &mut s::Budget) -> s::Result<s::WorkspaceRecord> {
        // Downcast the concrete owner, not its user-implementable enum view.
        // A custom WorkspaceData may otherwise masquerade as a known family.
        if let Some(w) = self
            .0
            .downcast_ref::<CompositeShapeShapeContactManifoldsWorkspace>()
        {
            w.checkpoint_capture(budget)
        } else if let Some(w) = self
            .0
            .downcast_ref::<CompositeShapeCompositeShapeContactManifoldsWorkspace>()
        {
            w.checkpoint_capture(budget)
        } else {
            Err(s::Error::Workspace)
        }
    }
    /// Bound map tables, entries and the boxed dynamic workspace before any live
    /// construction. The immutable witness joins all keys and manifold ids.
    pub fn checkpoint_plan(
        w: &s::ValidatedPairState<'_>,
        pair: u32,
        expected: s::Identity,
    ) -> ConstructionResult<AllocationPlan> {
        let (p, rows) = rows(w, pair, expected)?;
        match p.workspace {
            s::WorkspaceKind::None => Ok(AllocationPlan::default()),
            s::WorkspaceKind::CompositeShape => {
                CompositeShapeShapeContactManifoldsWorkspace::checkpoint_plan(rows.len())
            }
            s::WorkspaceKind::CompositeComposite => {
                CompositeShapeCompositeShapeContactManifoldsWorkspace::checkpoint_plan(rows.len())
            }
        }
    }
    /// Reconstruct admitted logical map state through the same cumulative permit.
    /// This deliberately makes no claim to restore fast HashMap bucket layout.
    pub fn from_checkpoint(
        w: &s::ValidatedPairState<'_>,
        pair: u32,
        expected: s::Identity,
        permit: &mut AllocationPermit,
    ) -> ConstructionResult<Option<Self>> {
        let (p, rows) = rows(w, pair, expected)?;
        Ok(match p.workspace {
            s::WorkspaceKind::None => None,
            s::WorkspaceKind::CompositeShape => {
                let workspace = CompositeShapeShapeContactManifoldsWorkspace::checkpoint_construct(
                    p.timestamp,
                    rows,
                    permit,
                )?;
                Some(Self(permit.boxed(workspace)?))
            }
            s::WorkspaceKind::CompositeComposite => {
                let workspace =
                    CompositeShapeCompositeShapeContactManifoldsWorkspace::checkpoint_construct(
                        p.timestamp,
                        rows,
                        permit,
                    )?;
                Some(Self(permit.boxed(workspace)?))
            }
        })
    }
}

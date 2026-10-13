use super::*;
use crate::{
    Budget, Dimension, Error, GeometryKind, MAX_BYTES, MAX_ITEMS, Result,
    attachment::validate_pose, vector,
};
pub(super) const HEADER: usize = 56;
pub(super) fn length(p: usize, d: usize) -> Result<usize> {
    if p > MAX_ITEMS || d > MAX_ITEMS {
        return Err(Error::Limit);
    }
    p.checked_mul(104)
        .and_then(|v| d.checked_mul(16).and_then(|d| v.checked_add(d)))
        .and_then(|v| v.checked_add(HEADER))
        .filter(|v| *v <= MAX_BYTES)
        .ok_or(Error::Limit)
}
pub(super) fn context(
    identity: Identity,
    p: usize,
    d: usize,
    m: &ValidatedManifolds<'_>,
) -> Result<()> {
    if identity != m.record().identity {
        return Err(Error::Identity);
    }
    if p != m.record().pairs.len() {
        return Err(Error::Length);
    }
    length(p, d)?;
    Ok(())
}
fn metadata(p: &PairState, d: Dimension) -> Result<()> {
    if p.event_status & !3 != 0
        || !(p.solver_color <= 128 || p.solver_color == 255)
        || p.solver_color_bodies
            .iter()
            .any(|v| *v != u32::MAX && *v as usize >= MAX_ITEMS)
    {
        return Err(Error::Metadata);
    }
    if let Some(s) = p.recycle {
        validate_pose(s.relative_pose, d)?;
        for rotation in s.rotations {
            validate_pose(
                LocalPose {
                    translation: [0; 3],
                    rotation,
                },
                d,
            )?;
        }
        for b in [s.max_extent, s.max_drift] {
            let f = f32::from_bits(b);
            if !f.is_finite() || f < 0. {
                return Err(Error::Metadata);
            }
        }
    }
    Ok(())
}
pub fn validate_pair_state<'a>(
    r: PairStateTable,
    m: &'a ValidatedManifolds<'a>,
    budget: &mut Budget,
) -> Result<ValidatedPairState<'a>> {
    context(r.identity, r.pairs.len(), r.detectors.len(), m)?;
    budget.charge(
        m.record()
            .manifolds
            .len()
            .checked_add(
                r.detectors
                    .len()
                    .checked_mul(size_of::<[u32; 2]>())
                    .ok_or(Error::Limit)?,
            )
            .ok_or(Error::Limit)?,
    )?;
    let mut seen = vector::<u8>(m.record().manifolds.len())?;
    seen.resize(m.record().manifolds.len(), 0);
    let mut keys = vector::<[u32; 2]>(r.detectors.len())?;
    let mut end = 0usize;
    for (row, p) in r.pairs.iter().enumerate() {
        metadata(p, r.identity.dimension)?;
        let native = &m.record().pairs[row];
        if p.detectors.start as usize != end {
            return Err(Error::Ownership);
        }
        end = end
            .checked_add(p.detectors.count as usize)
            .ok_or(Error::Limit)?;
        if end > r.detectors.len() {
            return Err(Error::Ownership);
        }
        let shapes = m.pair_shapes(row).ok_or(Error::Ownership)?;
        let composite = shapes.map(|s| s.record().kind != GeometryKind::Primitive);
        match p.workspace {
            WorkspaceKind::None => {
                if p.timestamp
                    || p.detectors.count != 0
                    || (composite.iter().any(|v| *v) && native.plain.count != 0)
                    || (!composite.iter().any(|v| *v) && native.plain.count > 1)
                {
                    return Err(Error::Workspace);
                }
            }
            WorkspaceKind::CompositeShape if composite[0] == composite[1] => {
                return Err(Error::Workspace);
            }
            WorkspaceKind::CompositeComposite if composite != [true, true] => {
                return Err(Error::Workspace);
            }
            _ => {}
        }
        if p.workspace == WorkspaceKind::None {
            continue;
        }
        if p.detectors.count != native.plain.count {
            return Err(Error::Ownership);
        }
        keys.clear();
        for d in &r.detectors[p.detectors.start as usize..end] {
            if d.timestamp != p.timestamp || d.manifold >= native.plain.count {
                return Err(Error::Workspace);
            }
            let i = (native.plain.start + d.manifold) as usize;
            if seen[i] != 0 {
                return Err(Error::Ownership);
            }
            seen[i] = 1;
            let expected = match p.workspace {
                WorkspaceKind::CompositeShape => {
                    if d.key[1] != 0 {
                        return Err(Error::Workspace);
                    }
                    if composite[0] {
                        [d.key[0], 0]
                    } else {
                        [0, d.key[0]]
                    }
                }
                WorkspaceKind::CompositeComposite => d.key,
                WorkspaceKind::None => return Err(Error::Workspace),
            };
            if expected != m.record().manifolds[i].subshapes {
                return Err(Error::Workspace);
            }
            // Manifold admission already proved every child bound and exact pose.
            keys.push(d.key);
        }
        keys.sort_unstable();
        if keys.windows(2).any(|w| w[0] == w[1]) {
            return Err(Error::Ownership);
        }
    }
    if end != r.detectors.len() {
        return Err(Error::Ownership);
    }
    Ok(ValidatedPairState {
        record: r,
        manifolds: m,
    })
}

use super::*;
use crate::{BodyKind, Budget, Error, MAX_ITEMS, Result, vector};
pub(super) const HEADER: usize = 100;
pub(super) fn counts(r: &RigidOwnerRecord) -> [usize; 8] {
    [
        r.sleep_scan.len(),
        r.body_color_masks.len(),
        r.pair_hints.len(),
        r.buckets.len(),
        r.references.len(),
        r.force_pairs.len(),
        r.force_positions.len(),
        r.force_flagged.len(),
    ]
}
pub(super) fn length(n: [usize; 8]) -> Result<usize> {
    if n.iter().any(|n| *n > MAX_ITEMS) || n[3] != 130 {
        return Err(Error::Length);
    }
    n.into_iter()
        .zip([8, 16, 4, 8, 8, 4, 4, 4])
        .try_fold(HEADER, |len, (n, size)| {
            crate::arena_codec::frame_length(len, size, n)
        })
}
pub(super) fn context(id: Identity, c: RigidOwnerContext<'_>) -> Result<()> {
    let ic = c.islands.context();
    if id != c.islands.record().identity
        || id != c.pairs.record().identity
        || !std::ptr::eq(ic.manifolds, c.pairs.manifolds())
    {
        return Err(Error::Identity);
    }
    let cc = c.colliders.context();
    if id != c.colliders.record().identity
        || !std::ptr::eq(ic.bodies, cc.bodies)
        || !std::ptr::eq(ic.manifolds.geometries(), cc.geometries)
    {
        return Err(Error::Identity);
    }
    Ok(())
}
pub fn validate_rigid_owner<'a>(
    r: RigidOwnerRecord,
    c: RigidOwnerContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedRigidOwner<'a>> {
    context(r.identity, c)?;
    length(counts(&r))?;
    let ic = c.islands.context();
    let islands = c.islands.record();
    let m = ic.manifolds;
    let a = ic.bodies.attachments();
    let np = m.record().pairs.len();
    let nm = m.record().manifolds.len();
    if r.sleep_scan_touched != 0 || !r.force_flagged.is_empty() {
        return Err(Error::PendingMutation);
    }
    if r.sleep_scan_stamp != islands.sleep_scan_stamp
        || r.sleep_scan.len() < islands.persistent.len()
    {
        return Err(Error::SleepScan);
    }
    if r.pair_hints.len() != np
        || r.force_positions.len() != np
        || r.references.len() > nm
        || r.force_pairs.len() > np
    {
        return Err(Error::Length);
    }
    // Scratch, not record storage: account every allocation before reserving.
    budget.charge(
        r.body_color_masks
            .len()
            .checked_mul(16)
            .and_then(|n| n.checked_add(nm))
            .and_then(|n| n.checked_add(np))
            .ok_or(Error::Limit)?,
    )?;
    let mut masks = vector(r.body_color_masks.len())?;
    masks.resize(r.body_color_masks.len(), 0u128);
    let mut seen = vector(nm)?;
    seen.resize(nm, 0u8);
    let mut force_seen = vector(np)?;
    force_seen.resize(np, 0u8);
    let fresh = r.graph_valid
        && r.graph_epoch == islands.active_epoch
        && r.graph_multibody_epoch == c.multibody_epoch;
    let mut cursor = 0usize;
    for (bucket, span) in r.buckets.iter().enumerate() {
        if span.start as usize != cursor {
            return Err(Error::Solver);
        }
        cursor = cursor
            .checked_add(span.count as usize)
            .ok_or(Error::Limit)?;
        if cursor > r.references.len() || (bucket == 129 && span.count != 0) {
            return Err(Error::Solver);
        }
        for (local, reference) in r.references[span.start as usize..cursor].iter().enumerate() {
            let pair = m.edge_pair(reference.edge).ok_or(Error::Solver)?;
            let s = if pair.clusters.count != 0 {
                pair.clusters
            } else {
                pair.plain
            };
            if reference.manifold >= s.count {
                return Err(Error::Solver);
            }
            let index = (s.start + reference.manifold) as usize;
            let manifold = &m.record().manifolds[index];
            if seen[index] != 0
                || manifold.graph_position != (((bucket as u32) << 22) | local as u32)
                || manifold.solver_color != bucket as u32
            {
                return Err(Error::Solver);
            }
            seen[index] = 1;
        }
    }
    if cursor != r.references.len() {
        return Err(Error::Solver);
    }
    for (edge, hint) in r.pair_hints.iter().copied().enumerate() {
        if hint > u16::MAX as u32 {
            return Err(Error::Solver);
        }
        let row = m.edge_row(edge as u32).ok_or(Error::Solver)?;
        let pair = &m.record().pairs[row];
        let state = &c.pairs.record().pairs[row];
        let parents = pair.colliders.map(|h| {
            a.collider_parent(h)
                .expect("validated parent")
                .map(|p| p.body)
        });
        let bodies = parents.map(|h| h.and_then(|h| ic.bodies.body(h)));
        if state.solver_color < 128 {
            let expected = std::array::from_fn(|i| {
                parents[i]
                    .filter(|_| bodies[i].is_some_and(|b| b.kind != BodyKind::Fixed))
                    .map(|h| h.slot)
                    .unwrap_or(u32::MAX)
            });
            // Native stores a sole second endpoint in lane zero.
            let expected = if expected[0] == u32::MAX {
                [expected[1], u32::MAX]
            } else {
                expected
            };
            if state.solver_color_bodies != expected || expected == [u32::MAX; 2] {
                return Err(Error::Solver);
            }
            for slot in expected.into_iter().filter(|s| *s != u32::MAX) {
                let mask = masks.get_mut(slot as usize).ok_or(Error::Solver)?;
                let bit = 1u128 << state.solver_color;
                if *mask & bit != 0 {
                    return Err(Error::Solver);
                }
                *mask |= bit;
            }
        } else if state.solver_color_bodies != [u32::MAX; 2] {
            return Err(Error::Solver);
        }
        let awake = bodies
            .iter()
            .flatten()
            .any(|b| b.kind == BodyKind::Dynamic && !b.activation.sleeping);
        let s = if pair.clusters.count != 0 {
            pair.clusters
        } else {
            pair.plain
        };
        let selected = &m.record().manifolds[s.start as usize..(s.start + s.count) as usize];
        let count = selected
            .iter()
            .filter(|m| m.solver_flags & 1 != 0 && m.solver.count != 0)
            .count()
            .min(32767) as u32;
        if (hint & 32767) != count && !(hint & 32767 == 0 && !awake) {
            return Err(Error::Solver);
        }
        if hint & 32767 != 0 && (hint & 32768 != 0) != awake {
            return Err(Error::Solver);
        }
        let selectable = hint & 32768 != 0 && hint & 32767 != 0;
        let mut first = true;
        for (ordinal, manifold) in selected.iter().enumerate() {
            let index = s.start as usize + ordinal;
            if (seen[index] != 0) != (manifold.graph_position != u32::MAX) {
                return Err(Error::Solver);
            }
            if fresh {
                let ids = manifold.bodies.map(|h| {
                    h.and_then(|h| ic.bodies.body(h))
                        .filter(|b| Some(b.ids[0]) == islands.awake)
                        .map(|b| b.ids[1])
                        .unwrap_or(u32::MAX)
                });
                let dynamic = manifold.bodies.into_iter().flatten().any(|h| {
                    ic.bodies.body(h).is_some_and(|b| {
                        b.kind == BodyKind::Dynamic && Some(b.ids[0]) == islands.awake
                    })
                });
                let want = selectable
                    && manifold.solver_flags & 1 != 0
                    && manifold.solver.count != 0
                    && dynamic;
                if want != (seen[index] != 0) {
                    return Err(Error::Solver);
                }
                if want {
                    let color = if first && state.solver_color != 255 {
                        state.solver_color
                    } else {
                        128
                    };
                    first = false;
                    if manifold.solver_body_ids != ids || manifold.solver_color != color {
                        return Err(Error::Solver);
                    }
                }
            }
        }
        if r.force_valid {
            let enabled = pair.colliders.iter().any(|h| {
                f32::from_bits(
                    c.colliders
                        .effective_force_threshold(*h)
                        .expect("joined collider witness"),
                ) < f32::MAX
            });
            if (r.force_positions[edge] != u32::MAX) != (selectable && enabled) {
                return Err(Error::Solver);
            }
        }
    }
    // Pinned collider-node removal does not release old color bits. Those
    // reservations affect future coloring, so preserve the exact superset.
    if masks
        .iter()
        .zip(&r.body_color_masks)
        .any(|(owned, stored)| owned & stored != *owned)
    {
        return Err(Error::Solver);
    }
    for (position, edge) in r.force_pairs.iter().copied().enumerate() {
        let seen = force_seen.get_mut(edge as usize).ok_or(Error::Solver)?;
        if *seen != 0 || r.force_positions[edge as usize] != position as u32 {
            return Err(Error::Solver);
        }
        *seen = 1;
    }
    if r.force_positions
        .iter()
        .enumerate()
        .any(|(i, pos)| (*pos != u32::MAX) != (force_seen[i] != 0))
    {
        return Err(Error::Solver);
    }
    Ok(ValidatedRigidOwner {
        record: r,
        context: c,
    })
}

use super::*;
use crate::{
    Budget, ColliderKind, Error, MAX_ITEMS, Result, RowState, arena_codec::frame_length, vector,
};
pub(super) const HEADER: usize = 60;
pub(super) fn counts(r: &NarrowOwnerRecord) -> [usize; 3] {
    [r.indices.len(), r.nodes.len(), r.intersections.len()]
}
pub(super) fn length(n: [usize; 3]) -> Result<usize> {
    if n.into_iter().any(|v| v > MAX_ITEMS) {
        return Err(Error::Length);
    }
    n.into_iter()
        .try_fold(HEADER, |len, n| frame_length(len, 8, n))
}
pub(super) fn context(id: Identity, n: [usize; 3], c: NarrowOwnerContext<'_>) -> Result<()> {
    if id != c.rigid.record().identity
        || id != c.indices.record().identity
        || id != c.intersections.record().identity
        || !std::ptr::eq(
            c.indices.arena(),
            c.rigid
                .colliders()
                .context()
                .bodies
                .attachments()
                .collider_arena(),
        )
    {
        return Err(Error::Identity);
    }
    if n != [
        c.indices.record().rows.len(),
        c.intersections.record().nodes.len(),
        c.intersections.record().edges.len(),
    ] {
        return Err(Error::Length);
    }
    length(n)?;
    Ok(())
}
pub fn validate_narrow_owner<'a>(
    r: NarrowOwnerRecord,
    c: NarrowOwnerContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedNarrowOwner<'a>> {
    context(r.identity, counts(&r), c)?;
    let manifolds = c.rigid.pairs().manifolds();
    let contact = manifolds.graph().record();
    let intersection = c.intersections.record();
    // Check both directions without allocating or trusting packed arena indices.
    // Isolated nodes may remain after an edge is removed or changes graph kind.
    for (which, graph, nodes) in [
        (0, contact, manifolds.record().nodes.as_slice()),
        (1, intersection, r.nodes.as_slice()),
    ] {
        for (index, node) in graph.nodes.iter().enumerate() {
            let h = nodes[node.row as usize];
            c.rigid.colliders().collider(h).ok_or(Error::Generation)?;
            let row = c.indices.lookup(h).ok_or(Error::Ownership)? as usize;
            let indices = r.indices[row];
            if [indices.contact, indices.intersection][which] != index as u32 {
                return Err(Error::Ownership);
            }
        }
    }
    for (slot, co) in c.indices.record().rows.iter().enumerate() {
        let indices = r.indices[co.row as usize];
        for (index, graph, nodes) in [
            (
                indices.contact,
                contact,
                manifolds.record().nodes.as_slice(),
            ),
            (indices.intersection, intersection, r.nodes.as_slice()),
        ] {
            if index == u32::MAX {
                continue;
            }
            if c.indices.state(slot as u32) != Some(RowState::Current) {
                return Err(Error::Generation);
            }
            let node = graph.nodes.get(index as usize).ok_or(Error::Ownership)?;
            if nodes[node.row as usize]
                != (Handle {
                    slot: slot as u32,
                    generation: co.generation,
                })
            {
                return Err(Error::Ownership);
            }
        }
    }
    // The existing manifold witness already checks contact edge uniqueness and
    // endpoint ownership. Completed TYPE changes transfer edges between graphs.
    for pair in &manifolds.record().pairs {
        if pair
            .colliders
            .iter()
            .any(|h| c.rigid.colliders().collider(*h).unwrap().kind == ColliderKind::Sensor)
        {
            return Err(Error::Narrow);
        }
    }
    budget.charge(
        r.intersections
            .len()
            .checked_mul(size_of::<[u32; 2]>())
            .ok_or(Error::Limit)?,
    )?;
    let mut seen = vector(r.intersections.len())?;
    for e in &intersection.edges {
        if e.node[0] == e.node[1] {
            return Err(Error::Narrow);
        }
        let nodes = e
            .node
            .map(|n| r.nodes[intersection.nodes[n as usize].row as usize]);
        if !nodes
            .iter()
            .any(|h| c.rigid.colliders().collider(*h).unwrap().kind == ColliderKind::Sensor)
        {
            return Err(Error::Narrow);
        }
        seen.push([e.node[0].min(e.node[1]), e.node[0].max(e.node[1])]);
    }
    seen.sort_unstable();
    if seen.windows(2).any(|v| v[0] == v[1]) {
        return Err(Error::Narrow);
    }
    // No event-mask/intersecting equivalence: toggles can preserve emitted
    // history until a later stop/removal, affecting exactly which event fires.
    Ok(ValidatedNarrowOwner {
        record: r,
        context: c,
    })
}

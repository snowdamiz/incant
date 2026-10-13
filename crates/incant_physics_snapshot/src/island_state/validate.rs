use super::*;
use crate::{BodyKind, Budget, Error, MAX_ITEMS, Result, arena_codec::frame_length, vector};
pub(super) const HEADER: usize = 116;
pub(super) fn counts(r: &IslandTable) -> [usize; 8] {
    [
        r.active.len(),
        r.active_free.len(),
        r.active_bodies.len(),
        r.persistent.len(),
        r.persistent_free.len(),
        r.persistent_bodies.len(),
        r.contacts.len(),
        r.locations.len(),
    ]
}
pub(super) fn length(c: [usize; 8]) -> Result<usize> {
    if c.iter().any(|n| *n > MAX_ITEMS) {
        return Err(Error::Length);
    }
    let mut len = HEADER;
    for (n, size) in c.into_iter().zip([12, 4, 8, 32, 4, 8, 20, 8]) {
        len = frame_length(len, n, size)?;
    }
    Ok(len)
}
pub(super) fn context(id: Identity, c: IslandContext<'_>) -> Result<()> {
    if id != c.bodies.record().identity
        || id != c.manifolds.record().identity
        || !std::ptr::eq(c.bodies.attachments(), c.manifolds.attachments())
    {
        return Err(Error::Identity);
    }
    if c.domain != IslandDomain::default() {
        return Err(Error::UnsupportedIsland);
    }
    Ok(())
}
fn span(s: RecordSpan, cursor: &mut usize, total: usize) -> Result<()> {
    if s.start as usize != *cursor {
        return Err(Error::Island);
    }
    *cursor = cursor.checked_add(s.count as usize).ok_or(Error::Limit)?;
    if *cursor > total {
        return Err(Error::Island);
    }
    Ok(())
}
fn free_partition<T>(slots: &[Option<T>], live: u32, free: &[u32]) -> Result<()> {
    if live as usize > slots.len() || free.len() != slots.len() - live as usize {
        return Err(Error::Free);
    }
    let mut seen = vector::<u8>(slots.len())?;
    seen.resize(slots.len(), 0);
    for id in free {
        let i = *id as usize;
        if slots.get(i).is_none_or(Option::is_some) || seen[i] != 0 {
            return Err(Error::Free);
        }
        seen[i] = 1;
    }
    if slots
        .iter()
        .enumerate()
        .any(|(i, s)| s.is_none() != (seen[i] == 1))
    {
        return Err(Error::Free);
    }
    Ok(())
}
fn member(b: &crate::RigidBodyState) -> bool {
    b.enabled && b.kind != BodyKind::Fixed
}
pub fn validate_islands<'a>(
    r: IslandTable,
    c: IslandContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedIslands<'a>> {
    context(r.identity, c)?;
    length(counts(&r))?;
    if !r.bootstrapped || r.joint_links != 0 || r.joint_locations != 0 {
        return Err(Error::UnsupportedIsland);
    }
    if r.pending_removals != 0 {
        return Err(Error::PendingMutation);
    }
    let nb = c.bodies.record().bodies.len();
    let edges = c.manifolds.record().pairs.len();
    if r.active_bodies.len() > nb
        || r.persistent_bodies.len() > r.active_bodies.len()
        || r.contacts.len() > edges
        || r.locations.len() > edges
    {
        return Err(Error::Length);
    }
    budget.charge(
        nb.checked_mul(2)
            .and_then(|n| n.checked_add(r.active.len()))
            .and_then(|n| n.checked_add(r.persistent.len()))
            .ok_or(Error::Limit)?,
    )?;
    free_partition(&r.active, r.active_live, &r.active_free)?;
    free_partition(&r.persistent, r.persistent_live, &r.persistent_free)?;
    if r.awake
        .is_some_and(|i| r.active.get(i as usize).is_none_or(Option::is_none))
        || r.pending_split.is_some_and(|i| {
            r.persistent
                .get(i as usize)
                .and_then(Option::as_ref)
                .is_none_or(|p| p.sleeping)
        })
    {
        return Err(Error::Island);
    }
    let mut active_seen = vector::<u8>(nb)?;
    active_seen.resize(nb, 0);
    let mut persistent_seen = vector::<u8>(nb)?;
    persistent_seen.resize(nb, 0);
    let a = c.bodies.attachments();
    let mut cursor = 0;
    for (island, s) in r.active.iter().enumerate() {
        let Some(s) = *s else { continue };
        // Native sleep extraction can leave the designated awake container empty.
        if s.count == 0 && r.awake != Some(island as u32) {
            return Err(Error::Island);
        }
        span(s, &mut cursor, r.active_bodies.len())?;
        for (index, h) in r.active_bodies[s.start as usize..cursor].iter().enumerate() {
            let row = a.body_row(*h).ok_or(Error::Generation)? as usize;
            let b = &c.bodies.record().bodies[row];
            if active_seen[row] != 0
                || !b.enabled
                || b.ids[..2] != [island as u32, index as u32]
                || (r.awake != Some(island as u32)
                    && !b.activation.sleeping
                    && b.kind != BodyKind::Fixed)
            {
                return Err(Error::Island);
            }
            active_seen[row] = 1;
        }
    }
    if cursor != r.active_bodies.len() {
        return Err(Error::Island);
    }
    cursor = 0;
    let mut contacts = 0;
    let invalid = Handle {
        slot: u32::MAX,
        generation: u32::MAX,
    };
    for (island, p) in r.persistent.iter().enumerate() {
        let Some(p) = p else { continue };
        if p.bodies.count == 0 {
            return Err(Error::Island);
        }
        span(p.bodies, &mut cursor, r.persistent_bodies.len())?;
        for (index, h) in r.persistent_bodies[p.bodies.start as usize..cursor]
            .iter()
            .enumerate()
        {
            let row = a.body_row(*h).ok_or(Error::Generation)? as usize;
            let b = &c.bodies.record().bodies[row];
            if persistent_seen[row] != 0
                || !member(b)
                || b.ids[2..] != [island as u32, index as u32]
                || (p.sleeping && !b.activation.sleeping)
                || p.sleeping == (Some(b.ids[0]) == r.awake)
            {
                return Err(Error::Island);
            }
            persistent_seen[row] = 1;
        }
        // Contact arrays can be empty. Their flat storage still has one exact partition.
        if p.contacts.start as usize != contacts {
            return Err(Error::Island);
        }
        contacts = contacts
            .checked_add(p.contacts.count as usize)
            .ok_or(Error::Limit)?;
        if contacts > r.contacts.len() {
            return Err(Error::Island);
        }
        for (index, link) in r.contacts[p.contacts.start as usize..contacts]
            .iter()
            .enumerate()
        {
            let pair = c.manifolds.edge_pair(link.edge).ok_or(Error::Island)?;
            if c.manifolds.edge_touching(link.edge) != Some(true)
                || r.locations.get(link.edge as usize).copied().flatten()
                    != Some(IslandLocation {
                        island: island as u32,
                        index: index as u32,
                    })
            {
                return Err(Error::Island);
            }
            let mut owns = false;
            for (co, h) in pair.colliders.into_iter().zip(link.bodies) {
                let parent = a.collider_parent(co).ok_or(Error::Generation)?;
                if parent.map(|p| p.body).unwrap_or(invalid) != h {
                    return Err(Error::Island);
                }
                if let Some(parent) = parent {
                    let b = c.bodies.body(parent.body).ok_or(Error::Generation)?;
                    if member(b) {
                        if b.ids[2] != island as u32 {
                            return Err(Error::Island);
                        }
                        owns = true;
                    }
                }
            }
            if !owns {
                return Err(Error::Island);
            }
        }
    }
    if cursor != r.persistent_bodies.len() || contacts != r.contacts.len() {
        return Err(Error::Island);
    }
    for (row, b) in c.bodies.record().bodies.iter().enumerate() {
        if member(b) {
            if active_seen[row] != 1 || persistent_seen[row] != 1 {
                return Err(Error::Island);
            }
        } else {
            if persistent_seen[row] != 0 || b.ids[2..] != [u32::MAX; 2] {
                return Err(Error::Island);
            }
            if !b.enabled && active_seen[row] != 0 {
                return Err(Error::Island);
            }
            if active_seen[row] == 0 && b.ids[..2] != [u32::MAX; 2] {
                return Err(Error::Island);
            }
        }
    }
    for edge in 0..edges {
        let loc = r.locations.get(edge).copied().flatten();
        if let Some(loc) = loc {
            let p = r
                .persistent
                .get(loc.island as usize)
                .and_then(Option::as_ref)
                .ok_or(Error::Island)?;
            if loc.index >= p.contacts.count
                || r.contacts[(p.contacts.start + loc.index) as usize].edge != edge as u32
            {
                return Err(Error::Island);
            }
        }
        if c.manifolds.edge_touching(edge as u32) != Some(true) {
            continue;
        }
        let pair = c.manifolds.edge_pair(edge as u32).ok_or(Error::Island)?;
        for co in pair.colliders {
            if let Some(p) = a.collider_parent(co).ok_or(Error::Generation)? {
                let b = c.bodies.body(p.body).ok_or(Error::Generation)?;
                if member(b) && loc.is_none_or(|l| l.island != b.ids[2]) {
                    return Err(Error::Island);
                }
            }
        }
    }
    Ok(ValidatedIslands {
        record: r,
        context: c,
    })
}

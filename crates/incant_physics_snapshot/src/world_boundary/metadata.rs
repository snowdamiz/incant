use super::WorldBoundary;
use crate::{codec::Reader, *};

pub(super) struct Metadata {
    pub journals: PendingJournals,
    pub domain: IslandDomain,
    pub multibody_epoch: u32,
}
pub(super) fn decode(bytes: &[u8]) -> Result<Metadata> {
    if bytes.len() != 32 {
        return Err(Error::Length);
    }
    let mut r = Reader { bytes, position: 0 };
    let journals = PendingJournals {
        modified_bodies: r.word()?,
        modified_colliders: r.word()?,
        removed_colliders: r.word()?,
    };
    let domain = IslandDomain {
        impulse_joints: r.word()?,
        multibody_joints: r.word()?,
        soft_bodies: r.word()?,
    };
    let multibody_epoch = r.word()?;
    if r.word()? != 0 {
        return Err(Error::Header);
    }
    if journals != PendingJournals::default() {
        return Err(Error::PendingMutation);
    }
    if domain != IslandDomain::default() {
        return Err(Error::UnsupportedIsland);
    }
    Ok(Metadata {
        journals,
        domain,
        multibody_epoch,
    })
}
pub(super) fn encode(v: WorldBoundary<'_>, budget: &mut Budget) -> Result<Vec<u8>> {
    let rigid = v.broad().context().narrow.context().rigid;
    let j = rigid.collider_state().context().journals;
    let d = rigid.islands().context().domain;
    budget.charge(32)?;
    let mut b = vector(32)?;
    for word in [
        j.modified_bodies,
        j.modified_colliders,
        j.removed_colliders,
        d.impulse_joints,
        d.multibody_joints,
        d.soft_bodies,
        rigid.multibody_epoch(),
        0,
    ] {
        b.extend_from_slice(&word.to_le_bytes());
    }
    Ok(b)
}
pub(super) fn decode_pending(bytes: &[u8], budget: &mut Budget) -> Result<Vec<Handle>> {
    if bytes.len() < 4 {
        return Err(Error::Length);
    }
    let mut r = Reader { bytes, position: 0 };
    let n = r.word()? as usize;
    if n > MAX_ITEMS || bytes.len() != 4 + n * 8 {
        return Err(Error::Length);
    }
    budget.charge(n * size_of::<Handle>())?;
    let mut rows = vector(n)?;
    for _ in 0..n {
        rows.push(Handle {
            slot: r.word()?,
            generation: r.word()?,
        });
    }
    Ok(rows)
}
pub(super) fn encode_pending(v: &BroadOwnerRecord, budget: &mut Budget) -> Result<Vec<u8>> {
    let n = 4 + v.pending_aabbs.len() * 8;
    budget.charge(n)?;
    let mut b = vector(n)?;
    b.extend_from_slice(&(v.pending_aabbs.len() as u32).to_le_bytes());
    for h in &v.pending_aabbs {
        b.extend_from_slice(&h.slot.to_le_bytes());
        b.extend_from_slice(&h.generation.to_le_bytes());
    }
    Ok(b)
}

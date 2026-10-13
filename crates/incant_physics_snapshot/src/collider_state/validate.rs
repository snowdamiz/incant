use super::*;
use crate::{
    Dimension, Error, MAX_ITEMS, Result, arena_codec::frame_length, attachment::validate_pose,
};
pub(super) const HEADER: usize = 52;
pub(super) const STRIDE: usize = 168;
pub(super) fn context(id: Identity, count: usize, c: ColliderContext<'_>) -> Result<()> {
    if id != c.bodies.record().identity || c.geometries.iter().any(|g| g.record().identity != id) {
        return Err(Error::Identity);
    }
    if c.journals != PendingJournals::default() {
        return Err(Error::PendingMutation);
    }
    if count > MAX_ITEMS
        || count != c.bodies.attachments().record().colliders.len()
        || count != c.geometries.len()
    {
        return Err(Error::Length);
    }
    frame_length(HEADER, STRIDE, count)?;
    Ok(())
}
fn nonnegative(v: u32) -> Result<()> {
    let f = f32::from_bits(v);
    if f.is_finite() && f >= 0. {
        Ok(())
    } else {
        Err(Error::Collider)
    }
}
fn row(c: &ColliderState, d: Dimension) -> Result<()> {
    validate_pose(c.position, d)?;
    match c.mass {
        ColliderMass::Density(v) | ColliderMass::Mass(v) => nonnegative(v)?,
        ColliderMass::Properties(m) => {
            crate::body_state::local_mass(m, d).map_err(|_| Error::Collider)?
        }
    }
    for v in [
        c.material.friction,
        c.material.restitution,
        c.contact_skin,
        c.force_event_threshold,
    ] {
        nonnegative(v)?;
    }
    // The pinned native collision mask has intentional holes. Do not admit all u16 bits.
    if c.active_collision_types & !0xee2f != 0
        || c.active_hooks & !7 != 0
        || c.active_events & !3 != 0
        || c.changes & !0x3ff != 0
    {
        return Err(Error::Collider);
    }
    if c.deformable || c.changes & 512 != 0 {
        return Err(Error::UnsupportedCollider);
    }
    // Every supported native completed pipeline clears the collider change rows.
    // Pending user mutation cannot masquerade as an empty journal witness.
    if c.changes != 0 {
        return Err(Error::PendingMutation);
    }
    Ok(())
}
pub fn validate_collider_state<'a>(
    record: ColliderStateTable,
    c: ColliderContext<'a>,
) -> Result<ValidatedColliderState<'a>> {
    context(record.identity, record.colliders.len(), c)?;
    // Enabled is retained state, not a derived parent property. Native explicit
    // enable can outlive parent disabling; detachment can retain DisabledByParent.
    // Replacing either with a recomputed boolean would change continuation.
    for co in &record.colliders {
        row(co, record.identity.dimension)?;
    }
    Ok(ValidatedColliderState { record, context: c })
}

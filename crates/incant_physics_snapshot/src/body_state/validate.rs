use super::*;
use crate::{
    Dimension, Error, Identity, MAX_ITEMS, Result, arena_codec::frame_length,
    attachment::validate_pose,
};
pub(super) const HEADER: usize = 52;
pub(super) const STRIDE: usize = 464;
pub(super) fn context(identity: Identity, count: usize, c: BodyContext<'_>) -> Result<()> {
    if identity != c.attachments.record().identity {
        return Err(Error::Identity);
    }
    if c.journals != PendingJournals::default() {
        return Err(Error::PendingMutation);
    }
    if count > MAX_ITEMS || count != c.attachments.record().bodies.len() {
        return Err(Error::Length);
    }
    frame_length(HEADER, STRIDE, count)?;
    Ok(())
}
fn finite(v: u32) -> Result<()> {
    if f32::from_bits(v).is_finite() {
        Ok(())
    } else {
        Err(Error::Body)
    }
}
fn nonnegative(v: u32) -> Result<()> {
    finite(v)?;
    if f32::from_bits(v) < 0. {
        Err(Error::Body)
    } else {
        Ok(())
    }
}
fn vector(v: &[u32], active: usize) -> Result<()> {
    for &v in &v[..active] {
        finite(v)?;
    }
    if v[active..].iter().any(|v| *v != 0) {
        return Err(Error::Body);
    }
    Ok(())
}
fn local_mass(m: LocalMass, d: Dimension) -> Result<()> {
    let n = d as usize;
    let angular = if n == 2 { 1 } else { 3 };
    vector(&m.center, n)?;
    nonnegative(m.inverse_mass)?;
    vector(&m.inverse_principal_inertia, angular)?;
    for v in m.inverse_principal_inertia {
        nonnegative(v)?;
    }
    if n == 2 {
        if m.principal_frame != [0; 4] {
            return Err(Error::Body);
        }
    } else {
        validate_pose(
            LocalPose {
                translation: [0; 3],
                rotation: m.principal_frame,
            },
            d,
        )?;
    }
    Ok(())
}
fn row(b: &RigidBodyState, d: Dimension) -> Result<()> {
    let n = d as usize;
    let angular = if n == 2 { 1 } else { 3 };
    for p in [b.position, b.next_position, b.activation.previous_pose] {
        validate_pose(p, d)?;
    }
    for v in [b.velocity, b.ccd_velocity] {
        vector(&v.linear, n)?;
        vector(&v.angular, angular)?;
    }
    for v in [b.forces.force, b.forces.user_force] {
        vector(&v, n)?;
    }
    for v in [b.forces.torque, b.forces.user_torque] {
        vector(&v, angular)?;
    }
    finite(b.forces.gravity_scale)?;
    if n == 2 && b.forces.gyroscopic {
        return Err(Error::Body);
    }
    for v in b.damping {
        nonnegative(v)?;
    }
    let m = b.mass;
    vector(&m.world_center, n)?;
    vector(&m.effective_inverse_mass, n)?;
    for v in m.effective_inverse_mass {
        nonnegative(v)?;
    }
    vector(&m.effective_inverse_inertia, if n == 2 { 1 } else { 6 })?;
    // Native rotated inertia may have signed off-diagonal entries. Checking
    // these scalars is not a PSD or derived-world-mass consistency proof.
    for i in if n == 2 { &[0][..] } else { &[0, 3, 5][..] } {
        nonnegative(m.effective_inverse_inertia[*i])?;
    }
    local_mass(m.local, d)?;
    if m.locked_axes & !0x3f != 0 {
        return Err(Error::Body);
    }
    match m.additional {
        None => (),
        Some(AdditionalMass::Mass(v)) => nonnegative(v)?,
        Some(AdditionalMass::Properties(v)) => local_mass(v, d)?,
    }
    nonnegative(m.max_extent)?;
    let a = b.activation;
    finite(a.linear_threshold)?;
    finite(a.angular_threshold)?;
    nonnegative(a.time_until_sleep)?;
    nonnegative(a.time_since_can_sleep)?;
    nonnegative(b.ccd.thickness)?;
    nonnegative(b.ccd.soft_prediction)?;
    if b.ids
        .iter()
        .any(|v| *v != u32::MAX && *v as usize >= MAX_ITEMS)
        || b.changes & !0xff != 0
        || !(-128..=127).contains(&b.dominance)
    {
        return Err(Error::Body);
    }
    // These fields remain explicit so an unsupported soft owner cannot be
    // silently interpreted as a regular body. Both signed zeros are retained.
    if b.soft_body
        != (Handle {
            slot: u32::MAX,
            generation: u32::MAX,
        })
        || b.soft_cluster != u32::MAX
        || f32::from_bits(b.soft_motion_margin) != 0.
    {
        return Err(Error::UnsupportedBody);
    }
    Ok(())
}
pub fn validate_body_state(
    record: BodyStateTable,
    context_value: BodyContext<'_>,
) -> Result<ValidatedBodyState<'_>> {
    context(record.identity, record.bodies.len(), context_value)?;
    for b in &record.bodies {
        row(b, record.identity.dimension)?;
    }
    Ok(ValidatedBodyState {
        record,
        context: context_value,
    })
}

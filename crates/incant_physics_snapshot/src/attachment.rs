//! Rigid attachment structure only: no body state, shape or contact admission.
use crate::{
    Budget, Dimension, Error, Handle, Identity, MAX_ITEMS, Result, ValidatedArena, vector,
};

/// Exact native f32 bits. Spatial rotations are xyzw quaternions; planar
/// rotations are [real, imaginary, 0, 0], not quaternion conversions. Unused
/// planar translation Z and rotation lanes must be positive zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalPose {
    pub translation: [u32; 3],
    pub rotation: [u32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParentAttachment {
    pub body: Handle,
    pub local_pose: LocalPose,
}

/// Contiguous segment of the ordered `attached` table, indexed by body payload
/// row. Native detach uses swap_remove; that resulting order is preserved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttachmentSpan {
    pub start: u32,
    pub count: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AttachmentRecord {
    pub identity: Identity,
    pub bodies: Vec<AttachmentSpan>,
    /// One row per live collider payload, including standalone and disabled
    /// colliders. None means no parent; no local pose exists in that case.
    pub colliders: Vec<Option<ParentAttachment>>,
    pub attached: Vec<Handle>,
}

/// Borrows the exact two supplied layouts. This witnesses their relationship,
/// not their enclosing owner: equal unrelated arenas cannot authenticate a
/// saved world without the future complete envelope and payload validation.
pub struct ValidatedAttachments<'a> {
    record: AttachmentRecord,
    bodies: &'a ValidatedArena,
    colliders: &'a ValidatedArena,
}
impl ValidatedAttachments<'_> {
    pub fn collider_row(&self, collider: Handle) -> Option<u32> {
        self.colliders.lookup(collider)
    }
    pub fn record(&self) -> &AttachmentRecord {
        &self.record
    }
    pub fn body_colliders(&self, body: Handle) -> Option<&[Handle]> {
        let row = self.bodies.lookup(body)? as usize;
        let span = self.record.bodies[row];
        Some(&self.record.attached[span.start as usize..(span.start + span.count) as usize])
    }
    /// Outer None is an invalid handle; inner None is a live standalone collider.
    pub fn collider_parent(&self, collider: Handle) -> Option<Option<&ParentAttachment>> {
        let row = self.colliders.lookup(collider)? as usize;
        Some(self.record.colliders[row].as_ref())
    }
}

pub(crate) fn attachment_lengths(
    identity: Identity,
    counts: [usize; 3],
    bodies: &ValidatedArena,
    colliders: &ValidatedArena,
) -> Result<()> {
    if identity != bodies.record().identity || identity != colliders.record().identity {
        return Err(Error::Identity);
    }
    let [b, c, a] = counts;
    if b > MAX_ITEMS
        || c > MAX_ITEMS
        || a > c
        || b != bodies.record().live as usize
        || c != colliders.record().live as usize
    {
        return Err(Error::Length);
    }
    Ok(())
}

pub(crate) fn validate_pose(pose: LocalPose, dimension: Dimension) -> Result<()> {
    let planar = dimension == Dimension::Planar;
    if planar && (pose.translation[2] != 0 || pose.rotation[2..] != [0, 0]) {
        return Err(Error::Geometry);
    }
    // Local translations are not world positions. Native public reparenting and
    // set_position_wrt_parent impose no magnitude bound; retain every finite
    // f32 here. Composition/world-range and arithmetic overflow remain later
    // owner validation, not a new arbitrary local-offset restriction.
    if pose
        .translation
        .iter()
        .any(|v| !f32::from_bits(*v).is_finite())
    {
        return Err(Error::Geometry);
    }
    let n = if planar { 2 } else { 4 };
    let norm = pose.rotation[..n]
        .iter()
        .map(|v| f64::from(f32::from_bits(*v)).powi(2))
        .sum::<f64>();
    // Pinned glam/glamx rotation checks use squared-length tolerance 2e-4.
    // Our supported domain uses a strict f64 bound, not their rounded f32
    // boundary result. No stored bits change. Unchecked non-unit native values
    // and boundary rounding differences are deliberately not admitted.
    if !norm.is_finite() || (norm - 1.).abs() >= 2e-4 {
        return Err(Error::Geometry);
    }
    Ok(())
}

pub fn validate_attachments<'a>(
    record: AttachmentRecord,
    bodies: &'a ValidatedArena,
    colliders: &'a ValidatedArena,
    budget: &mut Budget,
) -> Result<ValidatedAttachments<'a>> {
    let c = record.colliders.len();
    attachment_lengths(
        record.identity,
        [record.bodies.len(), c, record.attached.len()],
        bodies,
        colliders,
    )?;
    let mut end = 0usize;
    for span in &record.bodies {
        if span.start as usize != end {
            return Err(Error::Ownership);
        }
        end = end.checked_add(span.count as usize).ok_or(Error::Limit)?;
        if end > record.attached.len() {
            return Err(Error::Ownership);
        }
    }
    if end != record.attached.len() {
        return Err(Error::Ownership);
    }
    for parent in record.colliders.iter().flatten() {
        if bodies.lookup(parent.body).is_none() {
            return Err(Error::Generation);
        }
        validate_pose(parent.local_pose, record.identity.dimension)?;
    }
    budget.charge(c)?;
    let mut seen = vector::<u8>(c)?;
    seen.resize(c, 0);
    for (body_row, span) in record.bodies.iter().enumerate() {
        for handle in &record.attached[span.start as usize..(span.start + span.count) as usize] {
            let row = colliders.lookup(*handle).ok_or(Error::Generation)? as usize;
            if seen[row] != 0
                || record.colliders[row]
                    .as_ref()
                    .and_then(|p| bodies.lookup(p.body))
                    != Some(body_row as u32)
            {
                return Err(Error::Ownership);
            }
            seen[row] = 1;
        }
    }
    for (parent, seen) in record.colliders.iter().zip(seen) {
        if parent.is_some() != (seen == 1) {
            return Err(Error::Ownership);
        }
    }
    Ok(ValidatedAttachments {
        record,
        bodies,
        colliders,
    })
}

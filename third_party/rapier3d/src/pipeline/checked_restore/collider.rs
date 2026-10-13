//! Private primitive shape and collider storage construction, not installation.
use super::{NumericFrame, allocation::*, convert::*};
use crate::data::Arena;
use crate::geometry::{Collider, ColliderSet, SharedShape};
use core::alloc::Layout;
use incant_physics_snapshot::{GeometryKind, Primitive, ValidatedGeometry};
use parry::shape::{Ball, Capsule, Cuboid};

// Conservatively charged logical Arc payload: two counters plus alignment
// padding before and after T. This is not allocator overhead or exact RSS.
fn shared_plan<T>() -> ConstructionResult<AllocationPlan> {
    let (layout, _) = Layout::array::<usize>(2)
        .map_err(|_| ConstructionError::Limit)?
        .extend(Layout::new::<T>())
        .map_err(|_| ConstructionError::Limit)?;
    AllocationPlan::array::<u8>(layout.pad_to_align().size())
}
pub(super) fn shape_plan(g: &ValidatedGeometry) -> ConstructionResult<AllocationPlan> {
    if g.record().kind != GeometryKind::Primitive {
        return Err(ConstructionError::Payload);
    }
    match g.record().parts[0].shape {
        Primitive::Box { .. } => shared_plan::<Cuboid>(),
        Primitive::Ball { .. } => shared_plan::<Ball>(),
        Primitive::CapsuleY { .. } => shared_plan::<Capsule>(),
    }
}
pub(super) fn shape(
    g: &ValidatedGeometry,
    p: &mut AllocationPermit,
) -> ConstructionResult<SharedShape> {
    p.charge(shape_plan(g)?)?;
    Ok(match g.record().parts[0].shape {
        Primitive::Box { half_extents } => SharedShape::new(Cuboid::new(v(half_extents))),
        Primitive::Ball { radius } => SharedShape::new(Ball::new(f32::from_bits(radius))),
        Primitive::CapsuleY {
            half_height,
            radius,
        } => SharedShape::new(Capsule::new_y(
            f32::from_bits(half_height),
            f32::from_bits(radius),
        )),
    })
}
pub(super) fn plan(frame: &NumericFrame<'_>) -> ConstructionResult<AllocationPlan> {
    let c = frame.frame.pipeline().context().ccd.context().colliders;
    let mut p = Arena::<Collider>::checkpoint_plan(frame.frame.collider_arena())?;
    for (c, g) in c.record().colliders.iter().zip(c.context().geometries) {
        p.append(Collider::checkpoint_payload_plan(c)?)?;
        p.append(shape_plan(g)?)?;
    }
    Ok(p)
}
pub(super) fn construct(
    frame: &NumericFrame<'_>,
    p: &mut AllocationPermit,
) -> ConstructionResult<ColliderSet> {
    let state = frame.frame.pipeline().context().ccd.context().colliders;
    let a = frame.frame.attachments().record();
    let colliders = Arena::from_checkpoint(frame.frame.collider_arena(), p, |row, p| {
        let row = row as usize;
        let c = state
            .record()
            .colliders
            .get(row)
            .ok_or(ConstructionError::Payload)?;
        let g = state
            .context()
            .geometries
            .get(row)
            .ok_or(ConstructionError::Payload)?;
        let parent = *a.colliders.get(row).ok_or(ConstructionError::Payload)?;
        Collider::from_checkpoint_fields(c, shape(g, p)?, parent, p)
    })?;
    Ok(ColliderSet {
        colliders,
        modified_colliders: Default::default(),
        removed_colliders: Default::default(),
    })
}

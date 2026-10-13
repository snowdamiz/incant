//! Default-off compound capture and exact private-field construction. The
//! enclosing owner authenticates the linked build and completed capture phase.
use super::*;
use incant_physics_snapshot as s;
use s::construction::{AllocationPermit, AllocationPlan, ConstructionError, ConstructionResult};
mod mass;
mod primitive;
#[cfg(test)]
mod tests;

fn kind() -> s::GeometryKind {
    #[cfg(feature = "dim2")]
    {
        s::GeometryKind::PlanarTiles
    }
    #[cfg(feature = "dim3")]
    {
        s::GeometryKind::SpatialCompound
    }
}
fn check(w: &s::ValidatedCompound<'_>, expected: s::Identity) -> ConstructionResult<()> {
    let g = w.geometry().record();
    if w.identity() != expected
        || expected.dimension as usize != crate::math::DIM
        || g.kind != kind()
        || g.parts.len() != w.child_aabbs().len()
    {
        return Err(ConstructionError::Payload);
    }
    // Check native primitive/pose arithmetic without allocating a temporary
    // shape or rebuilding the saved BVH. Every bound is compared bitwise.
    let mut merged = Aabb::new_invalid();
    for (part, saved) in g.parts.iter().zip(w.child_aabbs()) {
        let a = primitive::Primitive::new(part.shape)
            .shape()
            .compute_aabb(&primitive::pose(part));
        if primitive::bounds(a) != *saved {
            return Err(ConstructionError::Payload);
        }
        merged.merge(&a);
    }
    if primitive::bounds(merged) != w.root_aabb() {
        return Err(ConstructionError::Payload);
    }
    Ok(())
}
impl Compound {
    /// Observe supported ordered primitive children and their actual acceleration
    /// storage. Nondefault flags, pseudo normals and nested/custom shapes refuse.
    /// This inert record does not itself authenticate the enclosing owner.
    pub fn checkpoint_capture(
        &self,
        identity: s::Identity,
        budget: &mut s::Budget,
    ) -> s::Result<(s::GeometryRecord, s::CompoundRecord)> {
        if identity.dimension as usize != crate::math::DIM
            || !self.flags.is_empty()
            || self.pseudo_normals.is_some()
            || self.shapes.is_empty()
            || self.shapes.len() != self.aabbs.len()
        {
            return Err(s::Error::Geometry);
        }
        #[cfg(feature = "dim2")]
        let limit = s::MAX_ITEMS;
        #[cfg(feature = "dim3")]
        let limit = 64;
        if self.shapes.len() > limit {
            return Err(s::Error::Limit);
        }
        let mut parts = budget.vector(self.shapes.len())?;
        for (pose, shape) in &self.shapes {
            parts.push(primitive::part(*pose, shape)?);
        }
        let mut child_aabbs = budget.vector(self.aabbs.len())?;
        child_aabbs.extend(self.aabbs.iter().copied().map(primitive::bounds));
        Ok((
            s::GeometryRecord {
                identity,
                kind: kind(),
                parts,
            },
            s::CompoundRecord {
                identity,
                child_aabbs,
                root_aabb: primitive::bounds(self.aabb),
                tree: self.bvh.checkpoint_capture(identity, budget)?,
                flags: self.flags.bits() as u32,
                pseudo_normals: false,
            },
        ))
    }
    /// Precharge children, their Arc payloads, cached AABBs and the exact BVH.
    /// A caller wrapping the returned Compound in an Arc must charge that outer
    /// allocation separately. This plan does not create any live shape.
    pub fn checkpoint_plan(
        w: &s::ValidatedCompound<'_>,
        expected: s::Identity,
    ) -> ConstructionResult<AllocationPlan> {
        check(w, expected)?;
        let mut plan =
            AllocationPlan::array::<(Pose, SharedShape)>(w.geometry().record().parts.len())?;
        plan.append(AllocationPlan::array::<Aabb>(w.child_aabbs().len())?)?;
        plan.append(Bvh::checkpoint_plan(w.tree(), expected)?)?;
        for part in &w.geometry().record().parts {
            plan.append(primitive::Primitive::new(part.shape).plan()?)?;
        }
        Ok(plan)
    }
    /// Construct exact saved storage through one non-Clone cumulative permit.
    /// No Compound::new, BVH rebuild, native clone or raw bytes-to-live path.
    pub fn from_checkpoint(
        w: &s::ValidatedCompound<'_>,
        expected: s::Identity,
        permit: &mut AllocationPermit,
    ) -> ConstructionResult<Self> {
        check(w, expected)?;
        let mut shapes = permit.vector(w.geometry().record().parts.len())?;
        for p in &w.geometry().record().parts {
            shapes.push((
                primitive::pose(p),
                primitive::Primitive::new(p.shape).shared(permit)?,
            ));
        }
        let mut aabbs = permit.vector(w.child_aabbs().len())?;
        aabbs.extend(w.child_aabbs().iter().copied().map(primitive::aabb));
        Ok(Self {
            shapes,
            bvh: Bvh::from_checkpoint(w.tree(), expected, permit)?,
            aabbs,
            aabb: primitive::aabb(w.root_aabb()),
            flags: CompoundFlags::empty(),
            pseudo_normals: None,
        })
    }
}

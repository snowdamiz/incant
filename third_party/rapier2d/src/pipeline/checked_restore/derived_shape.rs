//! Allocation-free shape arithmetic for the closed primitive/compound witness.
use super::{AdmissionError as E, Result, convert::v};
use crate::{dynamics::MassProperties, math::Pose};
use incant_physics_snapshot::{GeometryKind, Primitive, ValidatedCompound, ValidatedGeometry};
use parry::{
    bounding_volume::{Aabb, BoundingSphere},
    shape::{Ball, Capsule, Compound, Cuboid, Shape},
};

pub(super) enum DerivedShape<'a> {
    Box(Cuboid),
    Ball(Ball),
    Capsule(Capsule),
    Compound(ValidatedCompound<'a>),
}
impl<'a> DerivedShape<'a> {
    pub(super) fn new(g: &'a ValidatedGeometry) -> Result<Self> {
        if g.record().kind != GeometryKind::Primitive {
            return Ok(Self::Compound(g.compound().ok_or(E::UnsupportedGeometry)?));
        }
        Ok(match g.record().parts[0].shape {
            Primitive::Box { half_extents } => Self::Box(Cuboid::new(v(half_extents))),
            Primitive::Ball { radius } => Self::Ball(Ball::new(f32::from_bits(radius))),
            Primitive::CapsuleY {
                half_height,
                radius,
            } => Self::Capsule(Capsule::new_y(
                f32::from_bits(half_height),
                f32::from_bits(radius),
            )),
        })
    }
    pub(super) fn validate(&self) -> Result<()> {
        if let Self::Compound(c) = self {
            Compound::checkpoint_plan(c, c.identity()).map_err(|_| E::UnsupportedGeometry)?;
        }
        Ok(())
    }
    pub(super) fn compute_aabb(&self, pose: &Pose) -> Aabb {
        match self {
            Self::Box(s) => s.compute_aabb(pose),
            Self::Ball(s) => s.compute_aabb(pose),
            Self::Capsule(s) => s.compute_aabb(pose),
            Self::Compound(c) => {
                let b = c.root_aabb();
                Aabb::new(v(b.mins), v(b.maxs)).transform_by(pose)
            }
        }
    }
    pub(super) fn compute_local_bounding_sphere(&self) -> BoundingSphere {
        match self {
            Self::Box(s) => s.compute_local_bounding_sphere(),
            Self::Ball(s) => s.compute_local_bounding_sphere(),
            Self::Capsule(s) => s.compute_local_bounding_sphere(),
            Self::Compound(c) => {
                let b = c.root_aabb();
                Aabb::new(v(b.mins), v(b.maxs)).bounding_sphere()
            }
        }
    }
    pub(super) fn mass_properties(&self, density: f32) -> MassProperties {
        match self {
            Self::Box(s) => s.mass_properties(density),
            Self::Ball(s) => s.mass_properties(density),
            Self::Capsule(s) => s.mass_properties(density),
            Self::Compound(c) => Compound::checkpoint_mass_properties(c, density),
        }
    }
}

use super::{AdmissionError as E, Result, convert::*};
use crate::dynamics::{
    LockedAxes, MassProperties, RigidBodyForces, RigidBodyMassProps, RigidBodyVelocity,
};
use incant_physics_snapshot::{
    AdditionalMass, BodyKind, CcdCache, ColliderEnabled, ColliderMass, GeometryKind, Primitive,
    WorldBoundary,
};
use parry::bounding_volume::BoundingVolume;
use parry::shape::{Ball, Capsule, Cuboid, Shape};

enum PrimitiveShape {
    Box(Cuboid),
    Ball(Ball),
    Capsule(Capsule),
}
impl PrimitiveShape {
    fn new(p: Primitive) -> Self {
        match p {
            Primitive::Box { half_extents } => Self::Box(Cuboid::new(v(half_extents))),
            Primitive::Ball { radius } => Self::Ball(Ball::new(f32::from_bits(radius))),
            Primitive::CapsuleY {
                half_height,
                radius,
            } => Self::Capsule(Capsule::new_y(
                f32::from_bits(half_height),
                f32::from_bits(radius),
            )),
        }
    }
    fn shape(&self) -> &dyn Shape {
        match self {
            Self::Box(v) => v,
            Self::Ball(v) => v,
            Self::Capsule(v) => v,
        }
    }
}
fn collider_mass(m: ColliderMass, s: &dyn Shape) -> MassProperties {
    // Exact ColliderMassProps::mass_properties order, including zero sentinels.
    match m {
        ColliderMass::Density(bits) => {
            let density = f32::from_bits(bits);
            if density != 0. {
                s.mass_properties(density)
            } else {
                MassProperties::default()
            }
        }
        ColliderMass::Mass(bits) => {
            let m = f32::from_bits(bits);
            if m != 0. {
                let mut p = s.mass_properties(1.);
                p.set_mass(m, true);
                p
            } else {
                MassProperties::default()
            }
        }
        ColliderMass::Properties(m) => mass(m),
    }
}
pub(super) fn admit(frame: WorldBoundary<'_>) -> Result<()> {
    let colliders = frame.pipeline().context().ccd.context().colliders;
    let c = colliders.context();
    let a = frame.attachments();
    for (row, (co, g)) in colliders
        .record()
        .colliders
        .iter()
        .zip(c.geometries)
        .enumerate()
    {
        if g.record().kind != GeometryKind::Primitive {
            return Err(E::UnsupportedGeometry);
        }
        if co.active_hooks != 0 {
            return Err(E::UnsupportedHooks);
        }
        let shape = PrimitiveShape::new(g.record().parts[0].shape);
        let bounds = shape.shape().compute_aabb(&pose(co.position));
        if !bounds.mins.is_finite() || !bounds.maxs.is_finite() {
            return Err(E::Arithmetic);
        }
        if let Some(parent) = a.record().colliders[row] {
            let body = c.bodies.body(parent.body).ok_or(E::Identity)?;
            if pose_bits(pose(body.position) * pose(parent.local_pose)) != co.position {
                return Err(E::ColliderPose);
            }
        }
    }
    for (body, span) in c.bodies.record().bodies.iter().zip(&a.record().bodies) {
        let handles = &a.record().attached[span.start as usize..(span.start + span.count) as usize];
        let mut local = MassProperties::default();
        for h in handles {
            let row = a.collider_row(*h).ok_or(E::Identity)? as usize;
            let co = &colliders.record().colliders[row];
            if co.enabled != ColliderEnabled::Enabled {
                continue;
            }
            let parent = a.record().colliders[row].ok_or(E::Identity)?;
            let shape = PrimitiveShape::new(c.geometries[row].record().parts[0].shape);
            local += collider_mass(co.mass, shape.shape()).transform_by(&pose(parent.local_pose));
        }
        // Exact recompute_mass_properties_from_colliders branches and ordering.
        match body.mass.additional {
            None => local += MassProperties::default(),
            Some(AdditionalMass::Properties(m)) => local += mass(m),
            Some(AdditionalMass::Mass(m)) => {
                let m = f32::from_bits(m);
                let previous = local.mass();
                if previous > 0. {
                    local.set_mass(previous + m, true);
                } else {
                    let mut unit = MassProperties::default();
                    for h in handles {
                        let row = a.collider_row(*h).ok_or(E::Identity)? as usize;
                        if colliders.record().colliders[row].enabled != ColliderEnabled::Enabled {
                            continue;
                        }
                        let parent = a.record().colliders[row].ok_or(E::Identity)?;
                        let shape = PrimitiveShape::new(c.geometries[row].record().parts[0].shape);
                        unit += shape
                            .shape()
                            .mass_properties(1.)
                            .transform_by(&pose(parent.local_pose));
                    }
                    if unit.mass() > 0. {
                        unit.set_mass(m, true);
                        local += unit;
                    } else {
                        local.set_mass(m, true);
                    }
                }
            }
        }
        if mass_bits(local) != body.mass.local {
            return Err(E::LocalMass);
        }
        let mut extent = 0.0f32;
        for h in handles {
            let row = a.collider_row(*h).ok_or(E::Identity)? as usize;
            if colliders.record().colliders[row].enabled != ColliderEnabled::Enabled {
                continue;
            }
            let parent = a.record().colliders[row].ok_or(E::Identity)?;
            let shape = PrimitiveShape::new(c.geometries[row].record().parts[0].shape);
            let sphere = shape
                .shape()
                .compute_local_bounding_sphere()
                .transform_by(&pose(parent.local_pose));
            let next = (sphere.center - local.local_com).length() + sphere.radius;
            if !next.is_finite() {
                return Err(E::Arithmetic);
            }
            extent = extent.max(next);
        }
        if extent.to_bits() != body.mass.max_extent {
            return Err(E::Extent);
        }
        let mut derived = RigidBodyMassProps::from(local);
        derived.flags = LockedAxes::from_bits(body.mass.locked_axes as u8).ok_or(E::WorldMass)?;
        derived.update_world_mass_properties(kind(body.kind), &pose(body.position));
        #[cfg(feature = "dim2")]
        let inertia = [derived.effective_world_inv_inertia.to_bits(), 0, 0, 0, 0, 0];
        #[cfg(feature = "dim3")]
        let inertia = {
            let i = derived.effective_world_inv_inertia;
            [i.m11, i.m12, i.m13, i.m22, i.m23, i.m33].map(f32::to_bits)
        };
        if bits(derived.world_com) != body.mass.world_center
            || bits(derived.effective_inv_mass) != body.mass.effective_inverse_mass
            || inertia != body.mass.effective_inverse_inertia
        {
            return Err(E::WorldMass);
        }
        if body.kind == BodyKind::Dynamic && body.enabled {
            let dt = f32::from_bits(frame.pipeline().context().integration.record().dt);
            // Match solve::update_body_energy_and_forces: the saved effective
            // force already contains gravity. The next solve overwrites it from
            // user force and effective (axis-locked) mass; never add gravity twice.
            let mut forces = RigidBodyForces {
                force: v(body.forces.force),
                torque: angular(body.forces.torque),
                gravity_scale: f32::from_bits(body.forces.gravity_scale),
                user_force: v(body.forces.user_force),
                user_torque: angular(body.forces.user_torque),
                #[cfg(feature = "dim3")]
                gyroscopic_forces_enabled: body.forces.gyroscopic,
            };
            forces.compute_effective_force_and_torque(
                v(frame.pipeline().record().gravity),
                derived.effective_mass(),
            );
            let velocity = RigidBodyVelocity {
                linvel: v(body.velocity.linear),
                angvel: angular(body.velocity.angular),
            };
            let next = forces.integrate(dt, &velocity, &derived);
            if !forces.force.is_finite()
                || !forces.torque.is_finite()
                || !next.linvel.is_finite()
                || !next.angvel.is_finite()
            {
                return Err(E::Arithmetic);
            }
            // This isolated force integration does not admit gyroscopic updates,
            // constraint Jacobians or CCD-generated shorter step intervals.
        }
    }
    if let CcdCache::Targets {
        prediction,
        targets,
    } = &frame.pipeline().context().ccd.record().cache
    {
        for t in targets {
            let row = a.collider_row(t.collider).ok_or(E::Identity)? as usize;
            let co = &colliders.record().colliders[row];
            let shape = PrimitiveShape::new(c.geometries[row].record().parts[0].shape);
            // CCD fixed target cache uses its historical prediction directly,
            // without collision skin or the broad-phase half-prediction rule.
            let bounds = shape
                .shape()
                .compute_aabb(&pose(co.position))
                .loosened(f32::from_bits(*prediction));
            if !bounds.mins.is_finite() || !bounds.maxs.is_finite() {
                return Err(E::Arithmetic);
            }
            if bits(bounds.mins) != t.mins || bits(bounds.maxs) != t.maxs {
                return Err(E::CcdBounds);
            }
        }
    }
    Ok(())
}

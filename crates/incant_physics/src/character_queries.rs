//! Numerical conditioning for controller sweeps only. Solver contacts and public
//! raycasts keep their original dispatcher and are never altered here.
use rapier3d::{
    parry::{query::*, shape::Shape},
    prelude::{Pose, Real, Vector},
};

pub(crate) struct CharacterQueries<'a>(pub &'a dyn QueryDispatcher);

fn unit_normal(normal: Vector, witness: Vector, shape: &dyn Shape) -> Vector {
    let Some(normal) = normal.try_normalize() else {
        return normal;
    };
    exact_face_normal(normal, witness, shape).unwrap_or(normal)
}

fn exact_face_normal(normal: Vector, witness: Vector, shape: &dyn Shape) -> Option<Vector> {
    // A box face has an exact geometric normal. GJK can return a slightly short
    // or tilted vector even with its witness strictly inside that planar face;
    // Rapier's slope decomposition may then discard the forward displacement.
    // Recover that face normal, preserving curved and edge/corner contacts.
    if let Some(cuboid) = shape.as_cuboid() {
        let margins = cuboid.half_extents - witness.abs();
        // A 0.1 mm witness tolerance also covers GJK's early-termination error.
        let epsilon = 1.0e-4;
        for (i, axis) in [Vector::X, Vector::Y, Vector::Z].into_iter().enumerate() {
            if margins[i].abs() <= epsilon
                && margins[(i + 1) % 3] > epsilon
                && margins[(i + 2) % 3] > epsilon
                && normal.dot(axis).abs() > 0.999
            {
                return Some(axis * witness[i].signum());
            }
        }
    }
    if let Some(compound) = shape.as_compound() {
        for (pose, child) in compound.shapes() {
            if let Some(face) = exact_face_normal(
                pose.rotation.inverse() * normal,
                pose.inverse_transform_point(witness),
                child.as_ref(),
            ) {
                return Some(pose.rotation * face);
            }
        }
    }
    None
}

impl QueryDispatcher for CharacterQueries<'_> {
    fn intersection_test(
        &self,
        pos: &Pose,
        a: &dyn Shape,
        b: &dyn Shape,
    ) -> Result<ShapeIntersection, Unsupported> {
        self.0.intersection_test(pos, a, b)
    }
    fn distance(
        &self,
        pos: &Pose,
        a: &dyn Shape,
        b: &dyn Shape,
    ) -> Result<ShapeDistance, Unsupported> {
        self.0.distance(pos, a, b)
    }
    fn contact(
        &self,
        pos: &Pose,
        a: &dyn Shape,
        b: &dyn Shape,
        prediction: Real,
    ) -> Result<Option<Contact>, Unsupported> {
        self.0.contact(pos, a, b, prediction)
    }
    fn closest_points(
        &self,
        pos: &Pose,
        a: &dyn Shape,
        b: &dyn Shape,
        distance: Real,
    ) -> Result<ClosestPoints, Unsupported> {
        self.0.closest_points(pos, a, b, distance)
    }
    fn cast_shapes(
        &self,
        pos: &Pose,
        velocity: Vector,
        a: &dyn Shape,
        b: &dyn Shape,
        options: ShapeCastOptions,
    ) -> Result<Option<ShapeCastHit>, Unsupported> {
        Ok(self
            .0
            .cast_shapes(pos, velocity, a, b, options)?
            .map(|mut hit| {
                hit.normal1 = unit_normal(hit.normal1, hit.witness1, a);
                hit.normal2 = unit_normal(hit.normal2, hit.witness2, b);
                hit
            }))
    }
    fn cast_shapes_nonlinear(
        &self,
        motion1: &NonlinearRigidMotion,
        a: &dyn Shape,
        motion2: &NonlinearRigidMotion,
        b: &dyn Shape,
        start: Real,
        end: Real,
        stop: bool,
    ) -> Result<Option<ShapeCastHit>, Unsupported> {
        self.0
            .cast_shapes_nonlinear(motion1, a, motion2, b, start, end, stop)
    }
}

use incant_doc::{ColliderShape, PrimitiveColliderShape};
use rapier3d::prelude::*;

fn primitive(shape: &PrimitiveColliderShape) -> SharedShape {
    match *shape {
        PrimitiveColliderShape::Box {
            half_extents: [x, y, z],
        } => SharedShape::cuboid(x as f32, y as f32, z as f32),
        PrimitiveColliderShape::Sphere { radius } => SharedShape::ball(radius as f32),
        PrimitiveColliderShape::Capsule {
            half_height,
            radius,
        } => SharedShape::capsule_y(half_height as f32, radius as f32),
    }
}
pub(crate) fn build(shape: &ColliderShape) -> SharedShape {
    match shape {
        ColliderShape::Box {
            half_extents: [x, y, z],
        } => SharedShape::cuboid(*x as f32, *y as f32, *z as f32),
        ColliderShape::Sphere { radius } => SharedShape::ball(*radius as f32),
        ColliderShape::Capsule {
            half_height,
            radius,
        } => SharedShape::capsule_y(*half_height as f32, *radius as f32),
        ColliderShape::Compound { parts } => {
            // Presentation order is not a physics input. Stable part order also
            // keeps floating-point mass accumulation reproducible after reorder.
            let mut parts: Vec<_> = parts.iter().collect();
            parts.sort_by(|a, b| a.id.cmp(&b.id));
            SharedShape::compound(
                parts
                    .into_iter()
                    .map(|part| {
                        (
                            Pose::from_parts(
                                super::vector(part.translation),
                                Rotation::from_array(part.rotation.map(|v| v as f32)).normalize(),
                            ),
                            primitive(&part.shape),
                        )
                    })
                    .collect(),
            )
        }
    }
}

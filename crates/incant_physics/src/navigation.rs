use incant_doc::{ColliderShape, PrimitiveColliderShape};
use incant_nav::{NavigationError, NavigationGeometry};
use rapier3d::prelude::*;

/// Static collision geometry for baking. Curves use 24x12 facets expanded to
/// enclose the analytic shape; they may remove walkable space, never shrink an
/// obstacle merely because its curved surface was tessellated.
pub fn navigation_geometry(shape: &ColliderShape) -> Result<NavigationGeometry, NavigationError> {
    if let ColliderShape::Compound { parts } = shape {
        if !(1..=64).contains(&parts.len()) {
            return Err(NavigationError::Invalid(
                "compound collider requires 1..64 parts".into(),
            ));
        }
        let mut parts: Vec<_> = parts.iter().collect();
        parts.sort_by(|a, b| a.id.cmp(&b.id));
        let mut out = NavigationGeometry {
            vertices: vec![],
            triangles: vec![],
        };
        for part in parts {
            let primitive = match part.shape {
                PrimitiveColliderShape::Box { half_extents } => ColliderShape::Box { half_extents },
                PrimitiveColliderShape::Sphere { radius } => ColliderShape::Sphere { radius },
                PrimitiveColliderShape::Capsule {
                    half_height,
                    radius,
                } => ColliderShape::Capsule {
                    half_height,
                    radius,
                },
            };
            let geometry = navigation_geometry(&primitive)?;
            let pose = Pose::from_parts(
                super::vector(part.translation),
                Rotation::from_array(part.rotation.map(|v| v as f32)).normalize(),
            );
            let base = out.vertices.len() as u32;
            out.vertices.extend(
                geometry
                    .vertices
                    .into_iter()
                    .map(|v| pose.transform_point(Vector::from_array(v)).to_array()),
            );
            out.triangles.extend(
                geometry
                    .triangles
                    .into_iter()
                    .map(|face| face.map(|i| base + i)),
            );
        }
        out.validate()?;
        return Ok(out);
    }
    let curved = |radius: f64, half_height: f64| {
        let (unit, faces) = Ball::new(1.).to_trimesh(24, 12);
        let support = faces
            .iter()
            .map(|face| {
                let [a, b, c] = face.map(|i| unit[i as usize]);
                (b - a).cross(c - a).normalize().dot(a).abs()
            })
            .fold(1_f32, f32::min);
        let radius = radius as f32 / support * (1. + 4. * f32::EPSILON);
        if half_height == 0. {
            Ball::new(radius).to_trimesh(24, 12)
        } else {
            Capsule::new_y(half_height as f32, radius).to_trimesh(24, 12)
        }
    };
    let (vertices, triangles) = match shape {
        ColliderShape::Box { half_extents } => {
            Cuboid::new(Vector::from_array(half_extents.map(|x| x as f32))).to_trimesh()
        }
        ColliderShape::Sphere { radius } => curved(*radius, 0.),
        ColliderShape::Capsule {
            half_height,
            radius,
        } => curved(*radius, *half_height),
        ColliderShape::Compound { .. } => unreachable!("compound handled above"),
    };
    let out = NavigationGeometry {
        vertices: vertices.into_iter().map(|v| v.to_array()).collect(),
        triangles,
    };
    out.validate()?;
    Ok(out)
}

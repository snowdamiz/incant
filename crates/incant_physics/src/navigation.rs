use incant_doc::ColliderShape;
use incant_nav::{NavigationError, NavigationGeometry};
use rapier3d::prelude::*;

/// Static collision geometry for baking. Curves use 24x12 facets expanded to
/// enclose the analytic shape; they may remove walkable space, never shrink an
/// obstacle merely because its curved surface was tessellated.
pub fn navigation_geometry(shape: &ColliderShape) -> Result<NavigationGeometry, NavigationError> {
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
    };
    let out = NavigationGeometry {
        vertices: vertices.into_iter().map(|v| v.to_array()).collect(),
        triangles,
    };
    out.validate()?;
    Ok(out)
}

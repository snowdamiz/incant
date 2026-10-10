use incant_doc::ColliderShape;
use incant_physics::navigation_geometry;
use rapier3d::prelude::Vector;
#[test]
fn tessellated_shapes_enclose_analytic_curves_with_bounded_extra_clearance() {
    for half_height in [0., 1., 4.] {
        let shape = if half_height == 0. {
            ColliderShape::Sphere { radius: 2. }
        } else {
            ColliderShape::Capsule {
                radius: 2.,
                half_height,
            }
        };
        let mesh = navigation_geometry(&shape).unwrap();
        for face in mesh.triangles {
            let [a, b, c] = face.map(|i| Vector::from_array(mesh.vertices[i as usize]));
            let n = (b - a).cross(c - a).normalize();
            let plane = n.dot(a);
            let support = 2. + half_height as f32 * n.y.abs();
            assert!(plane >= support - 1e-5, "{plane} < {support}, normal={n:?}");
            assert!(
                plane - support < 0.06,
                "tessellation expanded shape too far"
            );
        }
    }
}

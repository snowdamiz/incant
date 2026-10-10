use glam::{Vec3, Vec3A};
use rerecast::{Aabb3d, AreaType, HeightfieldBuilder};

#[test]
fn consumed_triangles_never_rasterize_outside_their_footprint() {
    // Three real capsule facets that exhausted the row-clipping input before
    // the rounded final row. Upstream 0.4.0 reused stale vertex counts and
    // invented spans over a metre east of these facets, removing open floor.
    for triangle in [
        [
            [-3.142_266_5, 1.192_092_9e-8, 1.],
            [-3.157_863_6, -0.118_470_12, 1.],
            [-3.157_863_6, 1.192_092_9e-8, 0.881_529_57],
        ],
        [
            [-3.157_863_6, 1.518_470_2, 1.],
            [-3.142_266_5, 1.4, 1.],
            [-3.157_863_6, 1.4, 0.881_529_57],
        ],
        [
            [-3.142_266_5, 1.4, 1.],
            [-3.142_266_5, 1.192_092_9e-8, 1.],
            [-3.157_863_6, 1.192_092_9e-8, 0.881_529_57],
        ],
    ] {
        let points = triangle.map(Vec3A::from_array);
        let min = points[0].min(points[1]).min(points[2]);
        let max = points[0].max(points[1]).max(points[2]);
        let mut field = HeightfieldBuilder {
            aabb: Aabb3d {
                min: Vec3::new(-5.600_000_4, -1., -2.6),
                max: Vec3::new(-0.800_000_13, 2.5, 2.2),
            },
            cell_size: 0.1,
            cell_height: 0.05,
        }
        .build()
        .unwrap();
        field
            .rasterize_triangle(points, AreaType::DEFAULT_WALKABLE, 5)
            .unwrap();
        let mut emitted = 0;
        for z in 0..field.height {
            for x in 0..field.width {
                if field.spans[x as usize + z as usize * field.width as usize].is_none() {
                    continue;
                }
                emitted += 1;
                let px = field.aabb.min.x + x as f32 * field.cell_size;
                let pz = field.aabb.min.z + z as f32 * field.cell_size;
                assert!(
                    px <= max.x + 1e-5
                        && px + field.cell_size >= min.x - 1e-5
                        && pz <= max.z + 1e-5
                        && pz + field.cell_size >= min.z - 1e-5,
                    "phantom span at {px},{pz} outside {min:?}..{max:?}"
                );
            }
        }
        assert!(emitted > 0, "the valid facet must still rasterize");
    }
}

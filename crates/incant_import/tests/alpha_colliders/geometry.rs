use super::support::{Fixture, assert_exact_cover, entities};
use incant_doc::{Collider2D, ColliderShape2D, SpriteRegion, Transform};
use incant_physics_2d_runtime::{BodyState, PhysicsWorld2D, PreparedBody, RayQuery};
use incant_types::StableId;
use serde_json::json;

#[test]
fn exact_cover_preserves_holes_islands_threshold_edges_and_physics_queries() {
    let alpha = [
        255, 255, 255, 0, 255, 255, 0, 255, 0, 0, 255, 255, 255, 127, 128,
    ];
    let f = Fixture::new(5, &alpha, false);
    let prepared = f.prepare(128).unwrap();
    assert_exact_cover(&prepared, &alpha, 5, 128);
    let again = f.prepare(128).unwrap();
    let regions = |p: &incant_import::PreparedAlphaColliders| {
        p.derivation()
            .rectangles
            .iter()
            .map(|r| r.pixels)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        regions(&prepared),
        regions(&again),
        "cover order is deterministic"
    );
    let mut world = PhysicsWorld2D::default();
    let changes = entities(&prepared)
        .into_iter()
        .enumerate()
        .map(|(i, entity)| {
            let transform: Transform =
                serde_json::from_value(entity.components["Transform"].clone()).unwrap();
            let collider: Collider2D =
                serde_json::from_value(entity.components["Collider2D"].clone()).unwrap();
            assert!(entity.parent.is_none());
            assert!(!entity.components.contains_key("RigidBody2D"));
            (
                StableId([(i + 1) as u8; 16]),
                Some(
                    PreparedBody::new(
                        None,
                        collider,
                        BodyState {
                            translation: [transform.translation[0], transform.translation[1]],
                            ..Default::default()
                        },
                    )
                    .unwrap(),
                ),
            )
        })
        .collect();
    world.apply(changes).unwrap();
    for (i, &a) in alpha.iter().enumerate() {
        let hit = world
            .raycast(&RayQuery {
                origin: [(i % 5) as f64 + 0.5, 3. - (i / 5) as f64 - 0.5],
                direction: [1., 0.],
                max_distance: 0.,
                include_sensors: false,
                exclude_entity: None,
                memberships: u32::MAX,
                filter: u32::MAX,
            })
            .unwrap();
        assert_eq!(
            hit.is_some(),
            a >= 128,
            "actual fixed-box query at texel {i}"
        );
    }
}

#[test]
fn opaque_mask_is_one_box_and_selected_atlas_frame_excludes_padding_and_other_frames() {
    let solid = Fixture::new(4, &[255; 12], false);
    let prepared = solid.prepare(255).unwrap();
    assert_eq!(prepared.derivation().rectangles.len(), 1);
    assert_eq!(prepared.derivation().rectangles[0].pixels, [0, 0, 4, 3]);
    let alpha = [0, 255, 0, 255, 0, 0];
    let atlas = Fixture::new(3, &alpha, true);
    let prepared = atlas.prepare(255).unwrap();
    assert_eq!(&prepared.derivation().region[2..], &[3, 2]);
    assert_ne!(prepared.derivation().region[0..2], [0, 0]);
    assert_exact_cover(&prepared, &alpha, 3, 255);
}

#[test]
fn selected_crop_pivot_units_flips_and_planar_rotation_match_sprite_geometry() {
    // Only [2,1] in selected [1,1,4,2] is opaque; other opaque pixels are outside.
    let mut alpha = [0u8; 24];
    alpha[0] = 255;
    alpha[8] = 255;
    alpha[23] = 255;
    for flip_x in [false, true] {
        for flip_y in [false, true] {
            let mut f = Fixture::new(6, &alpha, false);
            let mut sprite = f.sprite();
            sprite.region = SpriteRegion::Pixels([1, 1, 4, 2]);
            sprite.size = [8., 6.];
            sprite.pivot = [0.25, 0.75];
            sprite.flip_x = flip_x;
            sprite.flip_y = flip_y;
            f.component("SpriteRenderer", json!(sprite));
            let q = std::f64::consts::FRAC_1_SQRT_2;
            f.component(
                "Transform",
                json!(Transform {
                    translation: [10., 20., 3.],
                    rotation: [0., 0., q, q],
                    scale: [1.; 3]
                }),
            );
            let prepared = f.prepare(1).unwrap();
            assert_eq!(prepared.derivation().rectangles.len(), 1);
            assert_eq!(prepared.derivation().rectangles[0].pixels, [1, 0, 1, 1]);
            let entity = entities(&prepared)[0];
            let t: Transform =
                serde_json::from_value(entity.components["Transform"].clone()).unwrap();
            let c: Collider2D =
                serde_json::from_value(entity.components["Collider2D"].clone()).unwrap();
            // Pixel center at (3,4.5) metres before pivot. Flips mirror about bounds.
            let local_x = if flip_x { 3. } else { 1. };
            let local_y = if flip_y { -3. } else { 0. };
            assert!((t.translation[0] - (10. - local_y)).abs() < 1e-12);
            assert!((t.translation[1] - (20. + local_x)).abs() < 1e-12);
            assert_eq!(t.translation[2], 3.);
            assert_eq!(t.rotation, [0., 0., q, q]);
            assert_eq!(
                c.shape,
                ColliderShape2D::Box {
                    half_extents: [1., 1.5]
                }
            );
        }
    }
}

#[test]
fn threshold_is_inclusive_on_base_alpha_bytes_not_filtered_mips() {
    let alpha = [0, 1, 127, 128, 254, 255];
    let f = Fixture::new(3, &alpha, false);
    for threshold in [1, 128, 255] {
        let prepared = f.prepare(threshold).unwrap();
        assert_exact_cover(&prepared, &alpha, 3, threshold);
    }
}

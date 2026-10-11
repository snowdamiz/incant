use incant_runtime::*;
use incant_types::{BodyMotion, Collider, Collider2D, RigidBody, Transform, Velocity};
pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn scene() -> CookedScene {
    let mut tiles = CookedSpatial::empty(id(1));
    tiles.tilemap = Some(Box::new(NativeTilemap {
        tileset: id(90),
        size: [3, 1],
        cell_size: [1., 1.],
        layers: vec![NativeTilemapLayer {
            id: id(80),
            cells: vec![Some(id(81)), None, Some(id(81))],
            layer: 0,
            order: 0,
            solid: true,
        }],
    }));
    CookedScene::new(
        id(99),
        64,
        [
            (1, [0., 0., 7.]),
            (2, [0.5, 2., 4.]),
            (3, [0.5, 2., 9.]),
            (4, [5., 0., 0.]),
        ]
        .into_iter()
        .map(|(n, translation)| CookedEntity {
            id: id(n),
            parent: None,
            transform: Transform {
                translation,
                ..Default::default()
            },
            velocity: (n == 2 || n == 4).then_some(Velocity { linear: [0.; 3] }),
        })
        .collect(),
    )
    .unwrap()
    .with_spatial(
        vec![VisualAssetRef {
            id: id(90),
            kind: VisualAssetKind::Texture,
            fingerprint: [3; 32],
        }],
        vec![tiles],
    )
    .unwrap()
    .with_physics(vec![CookedPhysics {
        entity: id(4),
        body: Some(RigidBody {
            motion: BodyMotion::Kinematic,
            ..Default::default()
        }),
        collider: Collider::default(),
        angular_velocity: None,
    }])
    .unwrap()
    .with_physics_2d(
        [2, 3]
            .into_iter()
            .map(|n| CookedPhysics2D {
                entity: id(n),
                body: (n == 2).then_some(RigidBody {
                    motion: BodyMotion::Kinematic,
                    ..Default::default()
                }),
                collider: Collider2D {
                    sensor: n == 3,
                    ..Default::default()
                },
                angular_velocity: None,
            })
            .collect(),
    )
    .unwrap()
}
pub fn ray() -> RayQuery2D {
    RayQuery2D {
        origin: [0.5, 3.],
        direction: [0., -2.],
        max_distance: 10.,
        include_sensors: false,
        exclude_entity: Some(id(2)),
        memberships: u32::MAX,
        filter: u32::MAX,
    }
}

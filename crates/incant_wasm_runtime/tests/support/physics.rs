#![allow(dead_code)]
use incant_runtime::*;
use incant_types::{
    BodyMotion, Collider, Collider2D, ColliderShape, RigidBody, Transform, Velocity,
};
pub const DT: f64 = 1. / 64.;
pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn scene(slope: bool) -> CookedScene {
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
    let specs = [
        (10, [0., -0.5, 0.], [20., 0.5, 20.], false),
        (11, [0., 0.6, 0.], [0.5; 3], false),
        (12, [2.5, 1., 0.], [0.5, 2., 4.], false),
        (13, [1., 1., 0.], [1.5, 2., 4.], true),
    ];
    let mut entities: Vec<_> = [
        (1, [0., 0., 7.]),
        (2, [0.5, 2., 4.]),
        (3, [0.5, 2., 9.]),
        (20, [0.; 3]),
    ]
    .into_iter()
    .map(|(n, t)| CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform {
            translation: t,
            ..Default::default()
        },
        velocity: (n == 2).then_some(Velocity { linear: [0.; 3] }),
    })
    .collect();
    entities.extend(specs.iter().map(|&(n, t, _, _)| CookedEntity {
        id: id(n),
        parent: None,
        transform: Transform {
            translation: t,
            rotation: if slope && n == 10 {
                [0., 0., (0.2f64).sin(), (0.2f64).cos()]
            } else {
                [0., 0., 0., 1.]
            },
            ..Default::default()
        },
        velocity: (n == 11).then_some(Velocity { linear: [0.; 3] }),
    }));
    CookedScene::new(id(99), 64, entities)
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
        .with_physics(
            specs
                .iter()
                .map(|&(n, _, half_extents, sensor)| CookedPhysics {
                    entity: id(n),
                    body: (n == 11).then_some(RigidBody {
                        motion: BodyMotion::Kinematic,
                        ..Default::default()
                    }),
                    collider: Collider {
                        shape: ColliderShape::Box { half_extents },
                        sensor,
                        ..Default::default()
                    },
                    angular_velocity: None,
                })
                .collect(),
        )
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
pub fn ray3() -> RayQuery<StableId> {
    RayQuery {
        scene_id: id(99),
        origin: [-8., 1., 0.],
        direction: [2., 0., 0.],
        max_distance: 20.,
        include_sensors: false,
        exclude_entity: Some(id(11)),
        memberships: u32::MAX,
        filter: u32::MAX,
    }
}
pub fn ray2() -> RayQuery2D {
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
pub fn character() -> CharacterQuery<StableId> {
    CharacterQuery {
        scene_id: id(99),
        entity_id: id(11),
        translation: [5., -1., 0.],
        options: CharacterOptions::default(),
    }
}
pub fn rays() -> Vec<incant_wasm_abi::PhysicsQuery> {
    use incant_wasm_abi::PhysicsQuery;
    let base = PhysicsQuery {
        kind: 0,
        flags: 2,
        entity: id(11).0,
        a: [-8., 1., 0.],
        b: [2., 0., 0.],
        c: 20.,
        d: 0.,
        e: 0.,
        memberships: u32::MAX,
        filter: u32::MAX,
        reserved: 0,
    };
    let planar = PhysicsQuery {
        kind: 1,
        entity: id(2).0,
        a: [0.5, 3., 0.],
        b: [0., -2., 0.],
        c: 10.,
        ..base
    };
    let character = PhysicsQuery {
        kind: 2,
        flags: 3,
        entity: id(11).0,
        a: [5., -1., 0.],
        b: [
            0.01,
            std::f64::consts::FRAC_PI_4,
            std::f64::consts::FRAC_PI_4,
        ],
        c: 0.2,
        d: 0.,
        e: 0.,
        memberships: 0,
        filter: 0,
        reserved: 0,
    };
    vec![
        base,
        PhysicsQuery { flags: 3, ..base },
        PhysicsQuery { filter: 0, ..base },
        planar,
        PhysicsQuery { flags: 3, ..planar },
        PhysicsQuery {
            a: [1.5, 3., 0.],
            ..planar
        },
        character,
    ]
}

use incant_physics_runtime::{BodyState, PhysicsRuntime, PhysicsSpec, PreparedPhysics};
use incant_types::{Collider, ColliderShape, RigidBody, StableId};
use std::collections::BTreeMap;
fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn specs<I: Ord>(scene: I, floor: I, ball: I) -> PreparedPhysics<I> {
    let state = |y| BodyState {
        translation: [0., y, 0.],
        rotation: [0., 0., 0., 1.],
        velocity: [0.; 3],
        angular_velocity: [0.; 3],
        sleeping: false,
    };
    PreparedPhysics::new(BTreeMap::from([(
        scene,
        BTreeMap::from([
            (
                floor,
                PhysicsSpec {
                    body: None,
                    collider: Collider {
                        shape: ColliderShape::Box {
                            half_extents: [5., 0.5, 5.],
                        },
                        ..Default::default()
                    },
                    state: state(-0.5),
                },
            ),
            (
                ball,
                PhysicsSpec {
                    body: Some(RigidBody::default()),
                    collider: Collider::default(),
                    state: state(2.),
                },
            ),
        ]),
    )]))
    .unwrap()
}
#[test]
fn binary_ids_and_text_ids_have_identical_motion_and_keep_unchanged_bodies_asleep() {
    let mut native = PhysicsRuntime::default();
    native.sync(specs(id(0), id(1), id(2)));
    let mut text = PhysicsRuntime::default();
    text.sync(specs(
        "scene".to_owned(),
        "floor".to_owned(),
        "object".to_owned(),
    ));
    for _ in 0..600 {
        let motion = native.state(&id(0), &id(2)).unwrap().clone();
        native.sync_motion(&id(0), &id(2), &motion).unwrap();
        text.sync_motion(&"scene".to_owned(), &"object".to_owned(), &motion)
            .unwrap();
        native.step(1. / 60.).unwrap();
        text.step(1. / 60.).unwrap();
        assert_eq!(
            native.state_iter().map(|(_, s)| s).collect::<Vec<_>>(),
            text.state_iter().map(|(_, s)| s).collect::<Vec<_>>()
        );
    }
    assert!(native.state(&id(0), &id(2)).unwrap().sleeping);
    assert_eq!(
        native.state(&id(0), &id(1)).unwrap().translation,
        [0., -0.5, 0.]
    );
    let before = native.states();
    let mut invalid = native.state(&id(0), &id(2)).unwrap().clone();
    invalid.velocity[0] = f64::NAN;
    assert!(native.sync_motion(&id(0), &id(2), &invalid).is_err());
    assert_eq!(native.states(), before);
    assert!(native.state(&id(0), &id(2)).unwrap().sleeping);
}

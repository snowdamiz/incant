use super::*;
use incant_types::{Collider2D, RigidBody};

fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn body(shape: ColliderShape2D) -> PhysicsWorld2D {
    let config = PreparedBody::new(
        Some(RigidBody::default()),
        Collider2D {
            shape,
            ..Default::default()
        },
        BodyState::default(),
    )
    .unwrap();
    let mut runtime = PhysicsWorld2D::default();
    runtime.apply(vec![(id(10), Some(config))]).unwrap();
    runtime.step(1. / 60.).unwrap();
    runtime
}

#[test]
fn checkpoint_outer_joins_accept_committed_planar_primitives() {
    for shape in [
        ColliderShape2D::Box {
            half_extents: [0.5, 0.6],
        },
        ColliderShape2D::Circle { radius: 0.4 },
        ColliderShape2D::Capsule {
            half_height: 0.6,
            radius: 0.2,
        },
        ColliderShape2D::Capsule {
            half_height: 0.,
            radius: 0.2,
        },
    ] {
        let mut runtime = body(shape);
        for _ in 0..4 {
            assert_eq!(validate(&runtime), Ok(()));
            runtime.step(1. / 60.).unwrap();
        }
    }
}

#[test]
fn checkpoint_outer_joins_preserve_removed_planar_entity_exit() {
    let config = || {
        PreparedBody::new(
            None,
            Collider2D {
                sensor: true,
                ..Default::default()
            },
            BodyState::default(),
        )
        .unwrap()
    };
    let mut runtime = PhysicsWorld2D::default();
    runtime
        .apply(vec![(id(10), Some(config())), (id(11), Some(config()))])
        .unwrap();
    runtime.step(1. / 60.).unwrap();
    assert_eq!(runtime.events().len(), 1);
    assert!(runtime.events()[0].entered);
    assert_eq!(validate(&runtime), Ok(()));
    runtime.apply(vec![(id(11), None)]).unwrap();
    runtime.step(1. / 60.).unwrap();
    assert_eq!(runtime.events().len(), 1);
    assert!(!runtime.events()[0].entered);
    assert_eq!(validate(&runtime), Ok(()));
}

#[test]
fn checkpoint_outer_joins_refuse_planar_identity_config_and_output_corruption() {
    let make = || body(ColliderShape2D::Circle { radius: 0.4 });
    let mut runtime = make();
    runtime.ids.values_mut().for_each(|v| *v = id(99));
    assert_eq!(validate(&runtime), Err("collider reverse identity"));
    let mut runtime = make();
    runtime
        .entries
        .get_mut(&id(10))
        .unwrap()
        .config
        .collider
        .friction = 0.9;
    assert_eq!(validate(&runtime), Err("collider declaration"));
    let mut runtime = make();
    runtime
        .entries
        .get_mut(&id(10))
        .unwrap()
        .config
        .state
        .translation[0] = 3.;
    assert_eq!(validate(&runtime), Err("committed body observation"));
    let mut runtime = make();
    runtime.overlaps.insert((id(10), id(20)));
    assert_eq!(validate(&runtime), Err("extra committed overlap"));
    let mut runtime = make();
    runtime.events.push(TriggerEvent {
        first: id(10),
        second: id(20),
        entered: true,
    });
    assert_eq!(
        validate(&runtime),
        Err("event transition disagrees with committed overlaps")
    );
    let mut runtime = make();
    runtime.query_dirty = true;
    assert_eq!(validate(&runtime), Err("unsettled query frame"));
    let mut runtime = make();
    runtime.failed = true;
    assert_eq!(validate(&runtime), Err("failed runtime"));
}

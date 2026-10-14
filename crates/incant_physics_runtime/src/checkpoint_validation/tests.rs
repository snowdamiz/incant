use super::*;

fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn state() -> BodyState {
    BodyState {
        translation: [0.; 3],
        rotation: [0., 0., 0., 1.],
        velocity: [0.; 3],
        angular_velocity: [0.; 3],
        sleeping: false,
    }
}
fn body(shape: ColliderShape) -> PhysicsRuntime<StableId> {
    let spec = PhysicsSpec {
        body: Some(DocBody::default()),
        collider: DocCollider {
            shape,
            ..Default::default()
        },
        state: state(),
    };
    let mut runtime = PhysicsRuntime::default();
    runtime
        .sync(
            PreparedPhysics::new(BTreeMap::from([(id(1), BTreeMap::from([(id(10), spec)]))]))
                .unwrap(),
        )
        .unwrap();
    runtime.step(1. / 60.).unwrap();
    runtime
}

#[test]
fn checkpoint_outer_joins_accept_committed_primitive_worlds() {
    for shape in [
        ColliderShape::Box {
            half_extents: [0.5, 0.6, 0.7],
        },
        ColliderShape::Sphere { radius: 0.4 },
        ColliderShape::Capsule {
            half_height: 0.6,
            radius: 0.2,
        },
        ColliderShape::Capsule {
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
fn checkpoint_outer_joins_preserve_removed_entity_and_scene_exit_events() {
    let spec = PhysicsSpec {
        body: None,
        collider: DocCollider {
            sensor: true,
            ..Default::default()
        },
        state: state(),
    };
    for remove_scene in [false, true] {
        let mut runtime = PhysicsRuntime::default();
        runtime
            .sync(
                PreparedPhysics::new(BTreeMap::from([(
                    id(1),
                    BTreeMap::from([(id(10), spec.clone()), (id(11), spec.clone())]),
                )]))
                .unwrap(),
            )
            .unwrap();
        runtime.step(1. / 60.).unwrap();
        assert_eq!(runtime.events().len(), 1);
        assert!(runtime.events()[0].entered);
        assert_eq!(validate(&runtime), Ok(()));
        if remove_scene {
            runtime
                .sync(PreparedPhysics::new(BTreeMap::new()).unwrap())
                .unwrap();
        } else {
            runtime.remove(&id(1), &id(11)).unwrap();
        }
        runtime.step(1. / 60.).unwrap();
        assert_eq!(runtime.events().len(), 1);
        assert!(!runtime.events()[0].entered);
        assert_eq!(validate(&runtime), Ok(()));
    }
}

#[test]
fn checkpoint_outer_joins_refuse_mismatched_identity_configuration_and_outputs() {
    let make = || body(ColliderShape::Sphere { radius: 0.4 });
    let mut runtime = make();
    runtime
        .scenes
        .get_mut(&id(1))
        .unwrap()
        .ids
        .values_mut()
        .for_each(|v| *v = id(99));
    assert_eq!(validate(&runtime), Err("collider reverse identity"));
    let mut runtime = make();
    runtime
        .scenes
        .get_mut(&id(1))
        .unwrap()
        .entries
        .get_mut(&id(10))
        .unwrap()
        .config
        .collider
        .friction = 0.9;
    assert_eq!(validate(&runtime), Err("collider declaration"));
    let mut runtime = make();
    runtime
        .scenes
        .get_mut(&id(1))
        .unwrap()
        .entries
        .get_mut(&id(10))
        .unwrap()
        .config
        .state
        .translation[0] = 3.;
    assert_eq!(validate(&runtime), Err("committed body observation"));
    let mut runtime = make();
    runtime.overlaps.insert((id(1), id(10), id(20)));
    assert_eq!(validate(&runtime), Err("extra committed overlap"));
    let mut runtime = make();
    runtime.events.push(TriggerEvent {
        scene_id: id(1),
        first: id(10),
        second: id(20),
        entered: true,
    });
    assert_eq!(
        validate(&runtime),
        Err("event transition disagrees with committed overlaps")
    );
    let mut runtime = make();
    runtime.scenes.get_mut(&id(1)).unwrap().query_dirty = true;
    assert_eq!(
        validate(&runtime),
        Err("unsettled query or character frame")
    );
    let mut runtime = make();
    runtime.failed = true;
    assert_eq!(validate(&runtime), Err("failed runtime"));
}

fn compound() -> ColliderShape {
    ColliderShape::Compound {
        parts: [1, 2]
            .map(|n| incant_types::ColliderPart {
                id: format!("{n:026}"),
                translation: [if n == 1 { -0.3 } else { 0.3 }, 0., 0.],
                rotation: [0., 0., (0.2_f64).sin(), (0.2_f64).cos()],
                shape: PrimitiveColliderShape::Box {
                    half_extents: [0.2, 0.3, 0.4],
                },
            })
            .into(),
    }
}
#[test]
fn compound_join_matches_ordered_normalized_native_parts_without_rebuilding() {
    let mut runtime = body(compound());
    for _ in 0..8 {
        assert_eq!(validate(&runtime), Ok(()));
        runtime.step(1. / 64.).unwrap();
    }
    let scene = runtime.scenes.get_mut(&id(1)).unwrap();
    let ColliderShape::Compound { parts } = &mut scene
        .entries
        .get_mut(&id(10))
        .unwrap()
        .config
        .collider
        .shape
    else {
        panic!()
    };
    parts.swap(0, 1);
    assert_eq!(validate(&runtime), Err("compound part order or limit"));
    let scene = runtime.scenes.get_mut(&id(1)).unwrap();
    let ColliderShape::Compound { parts } = &mut scene
        .entries
        .get_mut(&id(10))
        .unwrap()
        .config
        .collider
        .shape
    else {
        panic!()
    };
    parts.swap(0, 1);
    parts[1].translation[0] += 0.1;
    assert_eq!(validate(&runtime), Err("collider geometry"));
}
#[test]
fn compound_join_refuses_primitive_dimension_and_parent_mismatches() {
    let mut runtime = body(compound());
    let scene = runtime.scenes.get_mut(&id(1)).unwrap();
    let ColliderShape::Compound { parts } = &mut scene
        .entries
        .get_mut(&id(10))
        .unwrap()
        .config
        .collider
        .shape
    else {
        panic!()
    };
    parts[0].shape = PrimitiveColliderShape::Sphere { radius: 0.2 };
    assert_eq!(validate(&runtime), Err("collider geometry"));
    let mut runtime = body(compound());
    let config = &mut runtime
        .scenes
        .get_mut(&id(1))
        .unwrap()
        .entries
        .get_mut(&id(10))
        .unwrap()
        .config;
    config.body = None;
    config.state.velocity = [0.; 3];
    config.state.angular_velocity = [0.; 3];
    assert_eq!(validate(&runtime), Err("body declaration presence"));
}

use super::*;
use incant_runtime::{CookedPhysics, CookedPhysics2D};
use incant_types::{
    AngularVelocity, AngularVelocity2D, Collider, Collider2D, ColliderShape, ColliderShape2D,
    RigidBody, Transform,
};

fn physical() -> NativeSession {
    let entities = [1, 2, 3, 11, 12, 13]
        .into_iter()
        .map(|n| CookedEntity {
            id: id(n),
            parent: None,
            transform: Transform {
                translation: [
                    0.,
                    if n == 2 || n == 12 {
                        1.
                    } else if n == 3 || n == 13 {
                        0.6
                    } else {
                        -0.5
                    },
                    0.,
                ],
                ..Default::default()
            },
            velocity: None,
        })
        .collect();
    let initial = CookedScene::new(id(99), 64, entities)
        .unwrap()
        .with_physics(
            (1..=3)
                .map(|n| CookedPhysics {
                    entity: id(n),
                    body: (n == 2).then(RigidBody::default),
                    collider: Collider {
                        shape: ColliderShape::Box {
                            half_extents: if n == 2 { [0.4; 3] } else { [4., 0.5, 4.] },
                        },
                        sensor: n == 3,
                        ..Default::default()
                    },
                    angular_velocity: (n == 2).then_some(AngularVelocity {
                        angular: [0., 0., 0.02],
                    }),
                })
                .collect(),
        )
        .unwrap()
        .with_physics_2d(
            (11..=13)
                .map(|n| CookedPhysics2D {
                    entity: id(n),
                    body: (n == 12).then(RigidBody::default),
                    collider: Collider2D {
                        shape: ColliderShape2D::Box {
                            half_extents: if n == 12 { [0.4; 2] } else { [4., 0.5] },
                        },
                        sensor: n == 13,
                        ..Default::default()
                    },
                    angular_velocity: (n == 12).then_some(AngularVelocity2D { angular: 0.02 }),
                })
                .collect(),
        )
        .unwrap();
    let a = ulid::Ulid::from_bytes(id(2).0).to_string();
    let b = ulid::Ulid::from_bytes(id(12).0).to_string();
    let code = program(&format!(
        r#"
        let old=t.state.get()||{{count:0,priorY:0}};
        let next={{count:old.count+1,priorY:old.priorY}};
        if(t.clock.tick===1)t.timers.set('beat',3,3);
        if(t.clock.tick===64) {{
            for(const c of t.chunks) for(let r=0;r<c.ids.length;r++) {{
                if(c.ids[r]==={a:?}||c.ids[r]==={b:?}) {{
                    if(c.ids[r]==={a:?})next.priorY=c.transforms[r*10+1];
                    c.transforms[r*10+1]+=0.75;
                    c.velocities[r*3]=0.2;
                }}
            }}
        }}
        t.state.set(next);
    "#
    ));
    NativeSession::new(initial, code, Default::default()).unwrap()
}
fn observations(a: &NativeSession, b: &NativeSession) {
    let a = a.world().unwrap();
    let b = b.world().unwrap();
    assert_eq!(a.tick(), b.tick());
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(a.physics_events(), b.physics_events());
    assert_eq!(a.physics_2d_events(), b.physics_2d_events());
    for n in [1, 2, 3, 11, 12, 13] {
        assert_eq!(a.physics_state(id(n)), b.physics_state(id(n)));
        assert_eq!(a.physics_2d_state(id(n)), b.physics_2d_state(id(n)));
        assert_eq!(
            a.physics_configuration(id(n)),
            b.physics_configuration(id(n))
        );
        assert_eq!(
            a.physics_2d_configuration(id(n)),
            b.physics_2d_configuration(id(n))
        );
    }
}

#[test]
fn mixed_physics_and_pending_script_inputs_continue_after_fresh_vm_restore() {
    let mut live = physical();
    for _ in 0..64 {
        live.tick(&[]).unwrap();
    }
    let before = live.world.snapshot();
    let ecs_before = live.world.physics_state(id(2)).unwrap();
    let checkpoint = capture(&mut live).unwrap();
    assert_eq!(live.world.snapshot(), before);
    assert_eq!(live.world.physics_state(id(2)), Some(ecs_before));
    // The script's accepted post-physics ECS input must not be flushed at capture.
    // physics_state intentionally exposes pending ECS values, not raw solver
    // fields. The guest records the freshly published pre-write value instead.
    let stored = host(&live).unwrap().persistent.borrow();
    let crate::ScriptValue::Object(value) = &stored.entries.values().next().unwrap().value else {
        panic!("fixture object")
    };
    let crate::ScriptValue::Number(prior_y) = value["priorY"] else {
        panic!("fixture scalar")
    };
    assert_eq!(
        live.world.inspect(id(2)).unwrap().transform.translation[1].to_bits(),
        (prior_y + 0.75).to_bits()
    );
    drop(stored);
    assert!(matches!(
        live.checkpoint_bytes(),
        Err(CheckpointError::World(SaveError::PhysicsUnsupported))
    ));
    let mut restored = reconstruct(&checkpoint).unwrap();
    assert_ne!(live.world.incarnation(), restored.world.incarnation());
    observations(&live, &restored);
    for _ in 0..96 {
        live.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
        observations(&live, &restored);
        assert_eq!(
            host(&live).unwrap().schedule.tick,
            host(&restored).unwrap().schedule.tick
        );
        assert_eq!(
            host(&live)
                .unwrap()
                .persistent
                .borrow()
                .entries
                .values()
                .next()
                .unwrap()
                .value,
            host(&restored)
                .unwrap()
                .persistent
                .borrow()
                .entries
                .values()
                .next()
                .unwrap()
                .value
        );
    }
    restore(&mut live, &checkpoint).unwrap();
    let mut oracle = reconstruct(&checkpoint).unwrap();
    for _ in 0..16 {
        live.tick(&[]).unwrap();
        oracle.tick(&[]).unwrap();
        observations(&live, &oracle);
    }
}

#[test]
fn short_physics_budgets_leave_source_world_vm_and_input_unchanged() {
    let mut live = physical();
    for _ in 0..64 {
        live.tick(&[]).unwrap();
    }
    let before = live.world.snapshot();
    let incarnation = live.world.incarnation();
    assert!(
        live.checkpoint(
            &mut CheckpointPayloadBudget::new(0).unwrap(),
            &mut planar(),
            &mut spatial()
        )
        .is_err()
    );
    assert_eq!(live.world.snapshot(), before);
    let checkpoint = capture(&mut live).unwrap();
    for kind in 0..4 {
        let mut p = if kind == 0 {
            CheckpointPayloadBudget::new(0).unwrap()
        } else {
            payload()
        };
        let mut planar = PlanarCheckpointWorkBudget::new(if kind == 1 { 0 } else { 100_000_000 });
        let mut spatial = SpatialCheckpointWorkBudget::new(if kind == 2 { 0 } else { 100_000_000 });
        let mut a = if kind == 3 {
            CheckpointAllocationBudget::new(0, 0)
        } else {
            allocations()
        };
        assert!(
            live.restore_checkpoint(&checkpoint, &mut p, &mut planar, &mut spatial, &mut a)
                .is_err()
        );
        assert_eq!(live.world.snapshot(), before);
        assert_eq!(live.world.incarnation(), incarnation);
        assert_eq!(host(&live).unwrap().schedule.tick, 64);
    }
    let mut oracle = reconstruct(&checkpoint).unwrap();
    live.tick(&[]).unwrap();
    oracle.tick(&[]).unwrap();
    observations(&live, &oracle);
}

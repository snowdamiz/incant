use super::*;
use incant_types::{ColliderPart, PrimitiveColliderShape};

fn compound_spec(dynamic: bool, sensor: bool, y: f64) -> PhysicsSpec {
    let mut result = spec(dynamic, sensor, y);
    result.collider.shape = ColliderShape::Compound {
        parts: [1, 2]
            .map(|n| ColliderPart {
                id: format!("{n:026}"),
                translation: [if n == 1 { -0.3 } else { 0.3 }, 0., 0.],
                rotation: [0., 0., 0., 1.],
                shape: PrimitiveColliderShape::Box {
                    half_extents: if dynamic { [0.35; 3] } else { [2., 0.5, 2.] },
                },
            })
            .into(),
    };
    result
}
#[test]
fn compound_configuration_copy_charges_all_part_and_identity_payloads() {
    let source = compound_spec(true, false, 1.);
    let required = 2 * (size_of::<ColliderPart>() + 26);
    assert!(copy_config(&source, &mut Budget::new(required - 1).unwrap()).is_err());
    let mut exact = Budget::new(required).unwrap();
    let copied = copy_config(&source, &mut exact).unwrap();
    assert_eq!(exact.remaining(), 0);
    assert_eq!(source.body, copied.body);
    assert_eq!(source.collider, copied.collider);
    assert_eq!(source.state, copied.state);
    let ColliderShape::Compound { parts: a } = &source.collider.shape else {
        panic!()
    };
    let ColliderShape::Compound { parts: b } = &copied.collider.shape else {
        panic!()
    };
    assert_ne!(a.as_ptr(), b.as_ptr());
    assert_ne!(a[0].id.as_ptr(), b[0].id.as_ptr());
    assert!(copy_config(&source, &mut exact).is_err());
    assert!(charge_geometry(std::iter::once(&source), &mut WorkBudget::new(63)).is_err());
    let mut work = WorkBudget::new(64);
    charge_geometry(std::iter::once(&source), &mut work).unwrap();
    assert_eq!(work.remaining(), 0);
}
#[test]
fn compound_native_checkpoint_retains_warm_contacts_sleep_wake_and_events() {
    // Both compound/primitive and compound/compound warm contacts are retained.
    let mut source = PhysicsRuntime::default();
    source.sync(
        PreparedPhysics::new(BTreeMap::from([
            (
                id(1),
                BTreeMap::from([
                    (id(10), spec(false, false, -0.5)),
                    (id(11), compound_spec(true, false, 1.)),
                    (id(12), compound_spec(false, true, 0.6)),
                ]),
            ),
            (
                id(2),
                BTreeMap::from([
                    (id(10), compound_spec(false, false, -0.5)),
                    (id(11), compound_spec(true, false, 1.)),
                ]),
            ),
        ]))
        .unwrap(),
    );
    for _ in 0..512 {
        source.step(1. / 64.).unwrap();
    }
    assert!(source.state(&id(1), &id(11)).unwrap().sleeping);
    assert!(source.raycast(&ray(id(1))).unwrap().is_some());
    let checkpoint = capture(&mut source);
    let mut restored = checkpoint
        .reconstruct(&mut budget(), &mut work(), &mut allocations())
        .unwrap();
    assert_observations(&source, &restored);
    assert_eq!(
        frames(&capture(&mut source)),
        frames(&capture(&mut restored))
    );
    for tick in 0..96 {
        for world in [&mut source, &mut restored] {
            if tick == 8 {
                world.remove(&id(1), &id(12));
            }
            if tick == 16 {
                world.install(
                    id(1),
                    id(13),
                    PreparedBody::new(compound_spec(false, true, 0.6)).unwrap(),
                );
            }
            if tick == 40 {
                let mut state = world.state(&id(2), &id(11)).unwrap().clone();
                state.translation[1] += 1.;
                state.velocity[0] = 0.2;
                world.sync_motion(&id(2), &id(11), &state).unwrap();
            }
            world.step(1. / 64.).unwrap();
        }
        assert_observations(&source, &restored);
        assert_eq!(
            frames(&capture(&mut source)),
            frames(&capture(&mut restored)),
            "tick {tick}"
        );
    }
    let before = frames(&capture(&mut restored));
    let mut corrupt = capture(&mut source);
    let ColliderShape::Compound { parts } = &mut corrupt.scenes[1].entries[1].config.collider.shape
    else {
        panic!()
    };
    parts[0].translation[0] += 0.1;
    assert!(matches!(
        restored.restore_checkpoint(&corrupt, &mut budget(), &mut work(), &mut allocations()),
        Err(CheckpointError::Owner("collider geometry"))
    ));
    assert_eq!(frames(&capture(&mut restored)), before);
}

use incant_runtime::{
    CookedEntity, CookedScene, FrameCommands, NativeWorld, RuntimeError, SceneError, StableId,
    StructuralCommand,
};
use incant_types::{Transform, Velocity};

fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn entity(n: u8, parent: Option<u8>, scale: f64, position: f64, velocity: f64) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: parent.map(id),
        transform: Transform {
            translation: [position, 0., 0.],
            scale: [scale, 1., 1.],
            ..Default::default()
        },
        velocity: Some(Velocity {
            linear: [velocity, 0., 0.],
        }),
    }
}

#[test]
fn numeric_overflow_cannot_partially_advance_components_or_clock() {
    let mut world = NativeWorld::new(
        CookedScene::new(
            id(99),
            1,
            vec![
                entity(1, None, 1., 0., 4.),
                entity(2, None, 1., 1e308, 1e308),
            ],
        )
        .unwrap(),
    );
    let before = world.snapshot();
    let saved = world.save_bytes().unwrap();
    for _ in 0..2 {
        assert_eq!(
            world.step(),
            Err(RuntimeError::Scene(SceneError::Component(id(2))))
        );
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.save_bytes().unwrap(), saved);
        assert_eq!(world.tick(), 0);
    }
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetVelocity {
            entity: id(2),
            velocity: None,
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    world.step().unwrap();
    assert_eq!(world.tick(), 1);
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 4.);
    assert_eq!(
        world.inspect(id(2)).unwrap().transform.translation[0],
        1e308
    );
}

#[test]
fn numeric_hierarchy_overflow_is_checked_before_any_native_integration() {
    let mut world = NativeWorld::new(
        CookedScene::new(
            id(99),
            1,
            vec![
                entity(1, None, 1e200, 0., 3.),
                entity(2, Some(1), 1., 0., 1e200),
                entity(3, None, 1., 0., 2.),
            ],
        )
        .unwrap(),
    );
    let before = world.snapshot();
    assert_eq!(
        world.step(),
        Err(RuntimeError::Scene(SceneError::Component(id(2))))
    );
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.tick(), 0);
    // A later successful pass replaces every candidate scratch entry. No
    // partial parent movement from the rejected step can leak into the child.
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::SetVelocity {
            entity: id(2),
            velocity: Some(Velocity {
                linear: [1., 0., 0.],
            }),
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
    world.step().unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 3.);
    assert_eq!(world.inspect(id(2)).unwrap().world_transform[3][0], 1e200);
    assert_eq!(world.inspect(id(3)).unwrap().transform.translation[0], 2.);
}

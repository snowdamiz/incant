use incant_runtime::{
    CookedEntity, CookedScene, FrameCommands, MAX_FRAME_COMMANDS, NativeWorld, RuntimeError,
    SceneError, StableId, StructuralCommand as Command,
};
use incant_types::{Transform, Velocity};

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn entity(n: u128, parent: Option<u128>, moving: bool) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: parent.map(id),
        transform: Transform {
            translation: [1., 0., 0.],
            ..Default::default()
        },
        velocity: moving.then_some(Velocity {
            linear: [2., 0., 0.],
        }),
    }
}
fn world(entities: Vec<CookedEntity>) -> NativeWorld {
    NativeWorld::new(CookedScene::new(id(u128::MAX), 2, entities).unwrap())
}
fn queue(commands: impl IntoIterator<Item = Command>) -> FrameCommands {
    let mut result = FrameCommands::default();
    for command in commands {
        result.push(command).unwrap();
    }
    result
}
fn velocity(n: u128, value: Option<f64>) -> Command {
    Command::SetVelocity {
        entity: id(n),
        velocity: value.map(|x| Velocity {
            linear: [x, 0., 0.],
        }),
    }
}

#[test]
fn queue_from_component_view_applies_only_after_borrow_and_changes_live_queries() {
    let mut world = world(vec![entity(1, None, true), entity(2, None, false)]);
    let mut commands = FrameCommands::default();
    world.for_each_moving_mut(|id, transform, _| {
        transform.translation[0] = 8.;
        commands.push(Command::Despawn(id)).unwrap();
    });
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 8.);
    commands
        .push(Command::Spawn(entity(3, Some(4), true)))
        .unwrap();
    commands
        .push(Command::Spawn(entity(4, None, false)))
        .unwrap();
    commands.push(velocity(2, Some(6.))).unwrap();
    world.apply(&mut commands).unwrap();
    assert!(commands.is_empty());
    assert_eq!(world.tick(), 0);
    assert_eq!(world.entity_count(), 3);
    assert!(world.inspect(id(1)).is_none());
    assert_eq!(world.inspect(id(3)).unwrap().parent, Some(id(4)));
    assert_eq!(world.inspect(id(3)).unwrap().world_transform[3][0], 2.);
    world.step().unwrap();
    assert_eq!(world.inspect(id(2)).unwrap().transform.translation[0], 4.);
    assert_eq!(world.inspect(id(3)).unwrap().world_transform[3][0], 3.);
    let mut moving = Vec::new();
    world.for_each_moving_mut(|id, _, _| moving.push(id));
    moving.sort();
    assert_eq!(moving, [id(2), id(3)]);
}

#[test]
fn invalid_batch_preserves_every_entity_and_retains_queue_for_diagnostics() {
    let mut world = world(vec![entity(1, None, true), entity(2, Some(1), false)]);
    world.step().unwrap();
    let before = world.snapshot();
    let mut commands = queue([
        velocity(1, Some(99.)),
        Command::Spawn(entity(3, None, false)),
        Command::SetParent {
            entity: id(1),
            parent: Some(id(2)),
        },
    ]);
    assert!(matches!(
        world.apply(&mut commands),
        Err(RuntimeError::Scene(SceneError::Invalid("parent cycle")))
    ));
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.tick(), 1);
    assert_eq!(commands.len(), 3);
    commands.clear();
    commands.push(velocity(1, Some(4.))).unwrap();
    world.apply(&mut commands).unwrap();
    world.step().unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 4.);
}

#[test]
fn despawn_requires_explicit_child_removal_or_reparent_and_keeps_local_pose() {
    let mut world = world(vec![entity(1, None, false), entity(2, Some(1), false)]);
    let before = world.snapshot();
    let mut invalid = queue([Command::Despawn(id(1))]);
    assert_eq!(
        world.apply(&mut invalid),
        Err(RuntimeError::Scene(SceneError::Parent(id(2))))
    );
    assert_eq!(world.snapshot(), before);
    let mut commands = queue([
        Command::Despawn(id(1)),
        Command::SetParent {
            entity: id(2),
            parent: None,
        },
    ]);
    world.apply(&mut commands).unwrap();
    let child = world.inspect(id(2)).unwrap();
    assert_eq!(child.parent, None);
    assert_eq!(child.transform.translation[0], 1.);
    assert_eq!(child.world_transform[3][0], 1.);
    commands.push(Command::Despawn(id(2))).unwrap();
    world.apply(&mut commands).unwrap();
    world.step().unwrap();
    assert_eq!(world.entity_count(), 0);
    assert!(world.snapshot().is_empty());
}

#[test]
fn velocity_components_can_be_added_removed_and_readded_after_schedule_initialization() {
    let mut world = world(vec![entity(1, None, false)]);
    world.step().unwrap();
    let mut commands = queue([velocity(1, Some(4.))]);
    world.apply(&mut commands).unwrap();
    world.step().unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 3.);
    commands.push(velocity(1, None)).unwrap();
    world.apply(&mut commands).unwrap();
    world.step().unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().velocity, None);
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 3.);
    commands.push(velocity(1, Some(8.))).unwrap();
    commands.push(velocity(1, Some(2.))).unwrap();
    world.apply(&mut commands).unwrap();
    world.step().unwrap();
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 4.);
}

#[test]
fn missing_duplicate_and_recycled_in_batch_ids_fail_without_partial_effects() {
    let mut world = world(vec![entity(1, None, false)]);
    let before = world.snapshot();
    let cases = vec![
        vec![
            Command::Spawn(entity(2, None, true)),
            Command::Despawn(id(404)),
        ],
        vec![Command::Spawn(entity(1, None, true))],
        vec![Command::Spawn(entity(u128::MAX, None, false))],
        vec![
            Command::Spawn(entity(2, None, false)),
            Command::Spawn(entity(2, None, true)),
        ],
        vec![
            Command::Despawn(id(1)),
            Command::Spawn(entity(1, None, false)),
        ],
        vec![velocity(1, Some(4.)), velocity(404, None)],
        vec![Command::SetParent {
            entity: id(404),
            parent: None,
        }],
        vec![Command::SetParent {
            entity: id(1),
            parent: Some(id(404)),
        }],
    ];
    for case in cases {
        let count = case.len();
        let mut commands = queue(case);
        assert!(world.apply(&mut commands).is_err());
        assert_eq!(commands.len(), count);
        assert_eq!(world.snapshot(), before);
        assert_eq!(world.tick(), 0);
    }
}

#[test]
fn invalid_component_data_and_composed_overflow_are_rejected_atomically() {
    let mut big = entity(1, None, false);
    big.transform.scale = [1e308; 3];
    let mut other = entity(2, None, false);
    other.transform.scale = [1e308; 3];
    let mut world = world(vec![big, other]);
    let before = world.snapshot();
    let mut invalid = entity(3, None, true);
    invalid.transform.rotation = [0.; 4];
    for case in [
        vec![Command::Spawn(invalid)],
        vec![velocity(1, Some(f64::NAN))],
        vec![
            Command::Spawn(entity(3, None, false)),
            velocity(2, Some(f64::INFINITY)),
        ],
        vec![Command::SetParent {
            entity: id(2),
            parent: Some(id(1)),
        }],
    ] {
        assert!(world.apply(&mut queue(case)).is_err());
        assert_eq!(world.snapshot(), before);
    }
}

#[test]
fn cancelled_spawns_and_whole_hierarchy_deletion_have_no_dangling_handles() {
    let mut world = world(vec![entity(1, None, true), entity(2, Some(1), true)]);
    let mut commands = queue([
        Command::Spawn(entity(3, None, false)),
        velocity(3, Some(5.)),
        Command::Despawn(id(3)),
        Command::Despawn(id(1)),
        Command::Despawn(id(2)),
    ]);
    world.apply(&mut commands).unwrap();
    world.step().unwrap();
    assert!(world.snapshot().is_empty());
    commands
        .push(Command::Spawn(entity(4, None, true)))
        .unwrap();
    world.apply(&mut commands).unwrap();
    world.step().unwrap();
    assert_eq!(world.inspect(id(4)).unwrap().transform.translation[0], 2.);
}

#[test]
fn ten_thousand_reverse_order_spawns_resolve_forward_parents_without_recursion() {
    let mut a = world(vec![]);
    let mut b = world(vec![]);
    let mut commands = queue(
        (1..=10_000)
            .rev()
            .map(|n| Command::Spawn(entity(n, (n > 1).then_some(n - 1), false))),
    );
    let mut repeated = queue(commands.iter().cloned());
    a.apply(&mut commands).unwrap();
    b.apply(&mut repeated).unwrap();
    a.step().unwrap();
    b.step().unwrap();
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(
        a.inspect(id(10_000)).unwrap().world_transform[3][0],
        10_000.
    );
    for n in 1..=10_000 {
        commands.push(Command::Despawn(id(n))).unwrap();
    }
    a.apply(&mut commands).unwrap();
    a.step().unwrap();
    assert!(a.snapshot().is_empty());
}

#[test]
fn command_capacity_is_bounded_and_clearing_reuses_the_queue() {
    let mut commands = FrameCommands::default();
    for _ in 0..MAX_FRAME_COMMANDS {
        commands.push(velocity(1, None)).unwrap();
    }
    assert_eq!(
        commands.push(velocity(1, None)),
        Err(RuntimeError::BufferFull(MAX_FRAME_COMMANDS))
    );
    assert_eq!(commands.len(), MAX_FRAME_COMMANDS);
    commands.clear();
    commands
        .push(Command::Spawn(entity(1, None, false)))
        .unwrap();
    let mut world = world(vec![]);
    world.apply(&mut commands).unwrap();
    assert!(commands.is_empty());
    assert_eq!(world.entity_count(), 1);
}

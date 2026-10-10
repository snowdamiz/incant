use incant_runtime::{
    CookedEntity, CookedScene, FrameCommands, NativeWorld, StableId, StructuralCommand as Command,
    TRANSFORM_STRIDE, VELOCITY_STRIDE,
};
use incant_types::{Transform, Velocity};
use std::collections::BTreeMap;

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn entity(n: u128, parent: Option<u128>, moving: bool) -> CookedEntity {
    CookedEntity {
        id: id(n),
        parent: parent.map(id),
        transform: Transform::default(),
        velocity: moving.then_some(Velocity { linear: [0.; 3] }),
    }
}
fn world(entities: Vec<CookedEntity>) -> NativeWorld {
    NativeWorld::new(CookedScene::new(id(u128::MAX), 2, entities).unwrap())
}
fn apply(world: &mut NativeWorld, commands: impl IntoIterator<Item = Command>) {
    let mut buffer = FrameCommands::default();
    for command in commands {
        buffer.push(command).unwrap();
    }
    world.apply(&mut buffer).unwrap();
}

#[test]
fn numeric_rows_cover_static_and_moving_entities_and_feed_the_native_schedule() {
    let mut world = world(vec![entity(1, None, false), entity(2, Some(1), true)]);
    let mut visited = Vec::new();
    world.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| {
            assert_eq!(chunk.transforms.len(), chunk.ids.len() * TRANSFORM_STRIDE);
            for (row, &id) in chunk.ids.iter().enumerate() {
                visited.push(id);
                let values = &mut chunk.transforms[row * TRANSFORM_STRIDE..][..TRANSFORM_STRIDE];
                values.copy_from_slice(&[3., 4., 5., 0., 0., 1., 0., 2., 3., 4.]);
            }
            if let Some(velocities) = chunk.velocities {
                assert_eq!(velocities.len(), chunk.ids.len() * VELOCITY_STRIDE);
                for row in velocities.as_chunks_mut::<VELOCITY_STRIDE>().0 {
                    row.copy_from_slice(&[2., 4., 6.]);
                }
            } else {
                assert_eq!(chunk.ids, [id(1)]);
            }
        });
    });
    visited.sort();
    assert_eq!(visited, [id(1), id(2)]);
    assert_eq!(world.tick(), 0);
    let child = world.inspect(id(2)).unwrap();
    assert_eq!(child.transform.translation, [3., 4., 5.]);
    assert_eq!(child.transform.rotation, [0., 0., 1., 0.]);
    assert_eq!(child.transform.scale, [2., 3., 4.]);
    assert_eq!(child.world_transform[3], [-3., -8., 25., 1.]);
    world.step().unwrap();
    let child = world.inspect(id(2)).unwrap();
    assert_eq!(child.transform.translation, [4., 6., 8.]);
    assert_eq!(child.world_transform[3], [-5., -14., 37., 1.]);
    assert_eq!(
        world.inspect(id(1)).unwrap().transform.translation,
        [3., 4., 5.]
    );
}

#[test]
fn structural_growth_swap_removal_and_component_moves_get_fresh_aligned_views() {
    let mut world = world(vec![entity(1, None, true)]);
    world.with_numeric_columns(|columns| columns.for_each_chunk(|chunk| chunk.transforms[0] = 7.));
    // Force table growth, both component-set moves and swap removal between loans.
    apply(
        &mut world,
        (2..=4096).map(|n| Command::Spawn(entity(n, None, n % 2 == 0))),
    );
    apply(
        &mut world,
        [
            Command::Despawn(id(1)),
            Command::Despawn(id(2048)),
            Command::SetVelocity {
                entity: id(2),
                velocity: None,
            },
            Command::SetVelocity {
                entity: id(3),
                velocity: Some(Velocity { linear: [0.; 3] }),
            },
        ],
    );
    let mut seen = BTreeMap::new();
    world.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| {
            for (row, &id) in chunk.ids.iter().enumerate() {
                let value = u128::from_be_bytes(id.0) as f64;
                chunk.transforms[row * TRANSFORM_STRIDE] = value;
                assert!(seen.insert(id, chunk.velocities.is_some()).is_none());
            }
            if let Some(velocities) = chunk.velocities {
                for row in velocities.as_chunks_mut::<VELOCITY_STRIDE>().0 {
                    row[0] = 2.;
                }
            }
        });
    });
    assert_eq!(seen.len(), 4094);
    assert!(!seen[&id(2)]);
    assert!(seen[&id(3)]);
    world.step().unwrap();
    for (&id, &moving) in &seen {
        assert_eq!(
            world.inspect(id).unwrap().transform.translation[0],
            u128::from_be_bytes(id.0) as f64 + f64::from(moving)
        );
    }
    apply(&mut world, seen.keys().copied().map(Command::Despawn));
    world.with_numeric_columns(|columns| columns.for_each_chunk(|_| panic!("empty tables")));
    apply(&mut world, [Command::Spawn(entity(1, None, false))]);
    world.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| {
            assert_eq!(chunk.ids, [id(1)]);
            assert!(chunk.velocities.is_none());
            assert_eq!(chunk.transforms.len(), TRANSFORM_STRIDE);
            chunk.transforms[0] = 9.;
        })
    });
    assert_eq!(world.inspect(id(1)).unwrap().world_transform[3][0], 9.);
}

#[test]
fn callback_error_stops_visits_and_publishes_partial_writes_without_advancing_time() {
    let mut world = world(vec![entity(1, None, false), entity(2, Some(1), true)]);
    let mut visited = Vec::new();
    let result = world.with_numeric_columns(|columns| {
        columns.try_for_each_chunk(|chunk| {
            visited.extend_from_slice(chunk.ids);
            chunk.transforms[0] = 8.;
            Err("script stopped")
        })
    });
    assert_eq!(result, Err("script stopped"));
    assert_eq!(visited.len(), 1);
    assert_eq!(
        world.inspect(visited[0]).unwrap().transform.translation[0],
        8.
    );
    assert_eq!(world.inspect(id(2)).unwrap().world_transform[3][0], 8.);
    assert_eq!(world.tick(), 0);
}

#[test]
fn outer_error_can_restore_bounded_undo_within_the_same_exclusive_scope() {
    let mut world = world(vec![entity(1, None, false), entity(2, Some(1), true)]);
    let before = world.snapshot();
    let result = world.with_numeric_columns(|columns| {
        // This copying is explicitly the caller's transaction policy, not the
        // native loan's cost. A foreign adapter must first detach foreign views.
        let mut undo = Vec::new();
        columns.for_each_chunk(|chunk| {
            undo.push((
                chunk.ids.to_vec(),
                chunk.transforms.to_vec(),
                chunk.velocities.as_deref().map(<[f64]>::to_vec),
            ));
            chunk.transforms[0] = 20.;
            if let Some(velocities) = chunk.velocities {
                velocities[0] = 100.;
            }
        });
        let mut undo = undo.into_iter();
        columns.for_each_chunk(|chunk| {
            let (ids, transforms, velocities) = undo.next().unwrap();
            assert_eq!(chunk.ids, ids);
            chunk.transforms.copy_from_slice(&transforms);
            if let (Some(destination), Some(values)) = (chunk.velocities, velocities) {
                destination.copy_from_slice(&values);
            }
        });
        assert!(undo.next().is_none());
        Err::<(), _>("adapter rejected values")
    });
    assert_eq!(result, Err("adapter rejected values"));
    assert_eq!(world.snapshot(), before);
}

#[test]
fn early_return_and_unwinding_leave_hierarchy_consistent_and_loan_reusable() {
    let mut world = world(vec![entity(1, None, false), entity(2, Some(1), true)]);
    let returned = world.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| chunk.transforms[0] = 3.);
        42
    });
    assert_eq!(returned, 42);
    assert_eq!(world.inspect(id(2)).unwrap().world_transform[3][0], 6.);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        world.with_numeric_columns(|columns| {
            columns.for_each_chunk(|chunk| {
                chunk.transforms[0] = 7.;
                panic!("native callback panic");
            })
        });
    }));
    assert!(panic.is_err());
    assert_eq!(world.inspect(id(2)).unwrap().world_transform[3][0], 10.);
    apply(
        &mut world,
        [Command::SetParent {
            entity: id(2),
            parent: None,
        }],
    );
    world.with_numeric_columns(|columns| columns.for_each_chunk(|chunk| chunk.transforms[0] = 5.));
    assert_eq!(world.inspect(id(2)).unwrap().world_transform[3][0], 5.);
    world.step().unwrap();
}

#[test]
fn empty_world_and_unused_scope_are_valid() {
    let mut world = world(vec![]);
    assert_eq!(world.with_numeric_columns(|_| 12), 12);
    world.with_numeric_columns(|columns| columns.for_each_chunk(|_| panic!("empty world")));
    assert!(world.snapshot().is_empty());
    // Motion has never been registered in this world, not merely removed from
    // an existing table. Transform-only access still obtains the native column.
    apply(&mut world, [Command::Spawn(entity(1, None, false))]);
    world.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| {
            assert_eq!(chunk.ids, [id(1)]);
            assert!(chunk.velocities.is_none());
            chunk.transforms[0] = 12.;
        });
    });
    assert_eq!(world.inspect(id(1)).unwrap().world_transform[3][0], 12.);
    world.step().unwrap();
}

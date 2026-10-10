use super::*;
use crate::world::Global;
use crate::{CookedEntity, CookedScene};
use bevy_ecs::prelude::{Changed, Component, Query, ResMut, Resource, Schedule};
use std::collections::BTreeMap;

#[derive(Component)]
struct Extra;
#[derive(Component)]
#[component(storage = "SparseSet")]
struct Sparse;

fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            StableId([0; 16]),
            60,
            (1..=6)
                .map(|n| CookedEntity {
                    id: StableId([n; 16]),
                    parent: None,
                    transform: Transform::default(),
                    velocity: (n % 2 == 0).then_some(Velocity { linear: [0.; 3] }),
                })
                .collect(),
        )
        .unwrap(),
    )
}

#[test]
fn slices_are_actual_component_allocations_and_shared_tables_are_visited_once() {
    let mut native = world();
    // Both a new table and sparse-set archetypes sharing existing tables.
    for n in [3, 4] {
        native
            .world
            .entity_mut(native.entities[&StableId([n; 16])])
            .insert(Extra);
    }
    for n in [1, 2, 3] {
        native
            .world
            .entity_mut(native.entities[&StableId([n; 16])])
            .insert(Sparse);
    }
    let mut addresses = BTreeMap::new();
    for (&id, &entity) in &native.entities {
        let local = native.world.get::<Local>(entity).unwrap();
        let motion = native.world.get::<Motion>(entity);
        addresses.insert(
            id,
            (
                local.0.translation.as_ptr() as usize,
                motion.map(|m| m.0.linear.as_ptr() as usize),
            ),
        );
    }
    let mut visited = BTreeMap::new();
    let mut chunks = 0;
    native.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| {
            chunks += 1;
            for (row, &id) in chunk.ids.iter().enumerate() {
                let transform = &mut chunk.transforms[row * TRANSFORM_STRIDE..][..TRANSFORM_STRIDE];
                // An implementation staging copies and committing afterward fails
                // these address checks even if its final numeric values are equal.
                assert_eq!(transform.as_ptr() as usize, addresses[&id].0);
                assert_eq!(
                    chunk
                        .velocities
                        .as_deref()
                        .map(|v| v[row * VELOCITY_STRIDE..].as_ptr() as usize),
                    addresses[&id].1
                );
                transform[0] = f64::from(id.0[0]);
                assert!(visited.insert(id, ()).is_none());
            }
        })
    });
    assert_eq!(chunks, 4);
    assert_eq!(visited.len(), 6);
    for id in visited.keys() {
        assert_eq!(
            native.inspect(*id).unwrap().transform.translation[0],
            f64::from(id.0[0])
        );
    }
}

#[derive(Resource, Default)]
struct Changes {
    local: Vec<StableId>,
    motion: Vec<StableId>,
    global: Vec<StableId>,
}

fn observe(
    locals: Query<&Identity, Changed<Local>>,
    motions: Query<&Identity, Changed<Motion>>,
    globals: Query<&Identity, Changed<Global>>,
    mut changes: ResMut<Changes>,
) {
    changes.local = locals.iter().map(|id| id.0).collect();
    changes.motion = motions.iter().map(|id| id.0).collect();
    changes.global = globals.iter().map(|id| id.0).collect();
    changes.local.sort();
    changes.motion.sort();
    changes.global.sort();
}

#[test]
fn bevy_observers_see_touched_columns_and_derived_values_after_scope_exit() {
    let mut native = world();
    native.world.insert_resource(Changes::default());
    let mut observe_schedule = Schedule::default();
    observe_schedule.add_systems(observe);
    observe_schedule.run(&mut native.world);
    observe_schedule.run(&mut native.world);
    assert!(native.world.resource::<Changes>().local.is_empty());
    native.with_numeric_columns(|_| ());
    observe_schedule.run(&mut native.world);
    assert!(native.world.resource::<Changes>().global.is_empty());

    // Even unchanged or restored values are conservatively published, like a
    // mutable Bevy component access. Repeated visits still publish on scope exit.
    native.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| chunk.transforms[0] = 4.);
        columns.for_each_chunk(|chunk| chunk.transforms[0] = 0.);
    });
    observe_schedule.run(&mut native.world);
    let changes = native.world.resource::<Changes>();
    assert_eq!(
        changes.local,
        (1..=6).map(|n| StableId([n; 16])).collect::<Vec<_>>()
    );
    assert_eq!(
        changes.motion,
        [StableId([2; 16]), StableId([4; 16]), StableId([6; 16])]
    );
    assert_eq!(changes.global, changes.local);
    observe_schedule.run(&mut native.world);
    assert!(native.world.resource::<Changes>().local.is_empty());

    let mut visited = Vec::new();
    let result = native.with_numeric_columns(|columns| {
        columns.try_for_each_chunk(|chunk| {
            visited.extend_from_slice(chunk.ids);
            chunk.transforms[0] = 9.;
            Err(())
        })
    });
    assert_eq!(result, Err(()));
    visited.sort();
    observe_schedule.run(&mut native.world);
    let changes = native.world.resource::<Changes>();
    assert_eq!(changes.local, visited);
    assert_eq!(
        changes.motion,
        visited
            .iter()
            .copied()
            .filter(|id| id.0[0] % 2 == 0)
            .collect::<Vec<_>>()
    );
    assert_eq!(changes.global.len(), 6);
}

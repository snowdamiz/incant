use super::*;
use crate::{CookedEntity, CookedScene};
use std::panic::{AssertUnwindSafe, catch_unwind};

fn id(n: u8) -> StableId {
    StableId([n; 16])
}

fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            id(0),
            60,
            (1..=3)
                .map(|n| CookedEntity {
                    id: id(n),
                    parent: (n > 1).then(|| id(n - 1)),
                    transform: Transform {
                        translation: [f64::from(n), -0., f64::MIN_POSITIVE],
                        rotation: [0., 0., 0.6, 0.8],
                        scale: [-1., 2., 0.5],
                    },
                    velocity: (n == 2).then_some(Velocity { linear: [0.; 3] }),
                })
                .collect(),
        )
        .unwrap(),
    )
}

fn write(columns: &mut NumericColumns<'_>, entity: StableId, field: usize, value: f64) {
    columns.for_each_chunk(|chunk| {
        if let Some(row) = chunk.ids.iter().position(|&current| current == entity) {
            chunk.transforms[row * TRANSFORM_STRIDE + field] = value;
        }
    });
}

fn global_bits(world: &NativeWorld) -> Vec<Vec<u64>> {
    world
        .snapshot()
        .iter()
        .map(|entity| {
            entity
                .world_transform
                .iter()
                .flatten()
                .map(|value| value.to_bits())
                .collect()
        })
        .collect()
}

#[test]
fn validated_cleanup_matches_unchecked_publication_bit_for_bit() {
    let mut checked = world();
    let mut reference = world();
    let before = global_bits(&checked);
    for (native, validate) in [(&mut checked, true), (&mut reference, false)] {
        native.with_numeric_columns(|columns| {
            write(columns, id(1), 0, 7.);
            write(columns, id(2), 1, -0.);
            write(columns, id(3), 2, -f64::MIN_POSITIVE);
            if validate {
                columns.validate().unwrap();
                columns.validate().unwrap();
                // Only scratch is prepared: derived components stay unpublished
                // until the exclusive numeric scope ends.
                assert_eq!(global_bits(columns.owner), before);
            }
        });
    }
    assert_ne!(global_bits(&checked), before);
    assert_eq!(global_bits(&checked), global_bits(&reference));
    assert_eq!(checked.snapshot(), reference.snapshot());
    assert_eq!(
        checked.save_bytes().unwrap(),
        reference.save_bytes().unwrap()
    );
}

#[test]
fn later_mutable_visits_replace_prepared_values_even_on_error_or_unwind() {
    for unwind in [false, true] {
        let mut checked = world();
        let mut reference = world();
        reference.with_numeric_columns(|columns| write(columns, id(1), 0, 23.));
        let result = catch_unwind(AssertUnwindSafe(|| {
            checked.with_numeric_columns(|columns| {
                write(columns, id(1), 0, 7.);
                columns.validate().unwrap();
                let visit: Result<(), ()> = columns.try_for_each_chunk(|chunk| {
                    if let Some(row) = chunk.ids.iter().position(|&entity| entity == id(1)) {
                        chunk.transforms[row * TRANSFORM_STRIDE] = 23.;
                        if unwind {
                            panic!("after a new mutable loan");
                        }
                        return Err(());
                    }
                    Ok(())
                });
                assert_eq!(visit, Err(()));
            });
        }));
        assert_eq!(result.is_err(), unwind);
        assert_eq!(global_bits(&checked), global_bits(&reference));
        assert_eq!(checked.snapshot(), reference.snapshot());
    }
}

#[test]
fn successful_validation_still_publishes_during_unwind() {
    let mut checked = world();
    let mut reference = world();
    reference.with_numeric_columns(|columns| write(columns, id(1), 0, 17.));
    assert!(
        catch_unwind(AssertUnwindSafe(|| {
            checked.with_numeric_columns(|columns| {
                write(columns, id(1), 0, 17.);
                columns.validate().unwrap();
                panic!("after successful validation");
            });
        }))
        .is_err()
    );
    assert_eq!(global_bits(&checked), global_bits(&reference));
}

#[test]
fn failed_partial_validation_then_restore_never_publishes_stale_scratch() {
    for revalidate in [false, true] {
        let mut checked = world();
        let before = checked.snapshot();
        let bits = global_bits(&checked);
        checked.with_numeric_columns(|columns| {
            write(columns, id(1), 0, 7.);
            columns.validate().unwrap();
            write(columns, id(1), 0, 19.);
            write(columns, id(3), 0, f64::NAN);
            assert!(matches!(
                columns.validate(),
                Err(crate::RuntimeError::Scene(crate::SceneError::Component(entity))) if entity == id(3)
            ));
            // Model a trusted foreign adapter restoring its complete undo set
            // after a late numeric refusal, through fresh safe borrows.
            write(columns, id(1), 0, 1.);
            write(columns, id(3), 0, 3.);
            if revalidate {
                columns.validate().unwrap();
            }
        });
        assert_eq!(checked.snapshot(), before);
        assert_eq!(global_bits(&checked), bits);
    }
}

#[test]
fn validation_after_new_writes_publishes_the_new_complete_hierarchy() {
    let mut checked = world();
    let mut reference = world();
    for (native, validate) in [(&mut checked, true), (&mut reference, false)] {
        native.with_numeric_columns(|columns| {
            write(columns, id(1), 0, 5.);
            if validate {
                columns.validate().unwrap();
            }
            write(columns, id(2), 0, 11.);
            if validate {
                columns.validate().unwrap();
            }
        });
    }
    assert_eq!(global_bits(&checked), global_bits(&reference));
}

// Topology order deliberately disagrees with Transform-only/Velocity table
// order. A table-order validation would report child 1 before parent 3.
fn reordered_world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            id(0),
            60,
            (1..=3)
                .map(|n| CookedEntity {
                    id: id(n),
                    parent: match n {
                        1 => Some(id(3)),
                        2 => Some(id(1)),
                        _ => None,
                    },
                    transform: Transform {
                        translation: [f64::from(n), -0., f64::MIN_POSITIVE],
                        rotation: [0., 0., 0.6, 0.8],
                        scale: [-1., 2., 0.5],
                    },
                    velocity: (n != 1).then_some(Velocity { linear: [0.; 3] }),
                })
                .collect(),
        )
        .unwrap(),
    )
}

#[test]
fn invalid_numeric_rows_keep_parent_first_error_order_and_restoration() {
    let mut native = reordered_world();
    let before = native.snapshot();
    let bits = global_bits(&native);
    native.with_numeric_columns(|columns| {
        let mut storage_order = Vec::new();
        columns.for_each_chunk(|chunk| storage_order.extend_from_slice(chunk.ids));
        assert!(
            storage_order.iter().position(|&n| n == id(1)).unwrap()
                < storage_order.iter().position(|&n| n == id(3)).unwrap()
        );
        write(columns, id(1), 0, f64::NAN);
        write(columns, id(3), 3, 2.); // Invalid parent quaternion.
        assert_eq!(
            columns.validate(),
            Err(crate::SceneError::Component(id(3)).into())
        );
        assert_eq!(global_bits(columns.owner), bits);
        write(columns, id(3), 3, 0.);
        assert_eq!(
            columns.validate(),
            Err(crate::SceneError::Component(id(1)).into())
        );
        write(columns, id(1), 0, 1.);
        columns.validate().unwrap();
    });
    assert_eq!(native.snapshot(), before);
    assert_eq!(global_bits(&native), bits);
}

#[test]
fn ordered_validation_matches_scalar_publication_after_structural_changes() {
    use crate::{FrameCommands, StructuralCommand};

    let mut checked = reordered_world();
    let mut reference = reordered_world();
    let original = checked.entity_identity(id(2)).unwrap();
    for round in 0..4 {
        for (native, validate) in [(&mut checked, true), (&mut reference, false)] {
            let mut commands = FrameCommands::default();
            match round {
                1 => {
                    commands
                        .push(StructuralCommand::SetParent {
                            entity: id(2),
                            parent: Some(id(3)),
                        })
                        .unwrap();
                    commands
                        .push(StructuralCommand::SetVelocity {
                            entity: id(1),
                            velocity: Some(Velocity { linear: [0.; 3] }),
                        })
                        .unwrap();
                }
                2 => commands.push(StructuralCommand::Despawn(id(2))).unwrap(),
                3 => commands
                    .push(StructuralCommand::Spawn(CookedEntity {
                        id: id(2),
                        parent: Some(id(1)),
                        transform: Transform::default(),
                        velocity: None,
                    }))
                    .unwrap(),
                _ => (),
            }
            native.apply(&mut commands).unwrap();
            native.with_numeric_columns(|columns| {
                columns.for_each_chunk(|chunk| {
                    for (row, entity) in chunk.ids.iter().enumerate() {
                        let transform =
                            &mut chunk.transforms[row * TRANSFORM_STRIDE..][..TRANSFORM_STRIDE];
                        transform[0] = f64::from(entity.0[0]) + f64::from(round) * 0.125;
                        transform[1] = -0.;
                        transform[2] = -f64::MIN_POSITIVE;
                    }
                });
                if validate {
                    columns.validate().unwrap();
                }
            });
        }
        // The reference still uses the original per-entity get_mut refresh,
        // independently of the ordered preparation and publication path.
        assert_eq!(global_bits(&checked), global_bits(&reference));
        assert_eq!(checked.snapshot(), reference.snapshot());
        assert_eq!(
            checked.save_bytes().unwrap(),
            reference.save_bytes().unwrap()
        );
    }
    assert!(!checked.matches_entity_identity(id(2), original));
    assert_ne!(checked.entity_identity(id(2)).unwrap(), original);
}

#[test]
fn missing_query_component_cannot_shift_hierarchy_correspondence() {
    use crate::world::Global;

    let mut native = reordered_world();
    let child = native.entities[&id(1)];
    let descendant = native.entities[&id(2)];
    let before = native.world.get::<Global>(descendant).unwrap().0;
    // Private corruption controls: normal structural commands always preserve
    // these mandatory components. QueryMany must not silently skip this row.
    let local = native.world.entity_mut(child).take::<Local>().unwrap();
    assert!(catch_unwind(AssertUnwindSafe(|| native.prepare_numeric_globals())).is_err());
    assert_eq!(native.world.get::<Global>(descendant).unwrap().0, before);
    native.world.entity_mut(child).insert(local);
    native.world.get_mut::<Local>(child).unwrap().0.translation[0] = 29.;
    native.prepare_numeric_globals().unwrap();
    native.world.entity_mut(child).remove::<Global>();
    assert!(catch_unwind(AssertUnwindSafe(|| native.publish_numeric_globals())).is_err());
    // In particular, the child's scratch must not be assigned to its descendant.
    assert_eq!(native.world.get::<Global>(descendant).unwrap().0, before);
}

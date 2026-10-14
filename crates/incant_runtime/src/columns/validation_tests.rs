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

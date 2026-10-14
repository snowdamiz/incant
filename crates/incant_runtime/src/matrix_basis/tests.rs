use super::*;
use crate::{CookedEntity, CookedScene, NativeWorld, SceneError, StableId, TRANSFORM_STRIDE};

fn id(n: u8) -> StableId {
    StableId([n; 16])
}
fn bits(matrix: DMat4) -> [u64; 16] {
    matrix.to_cols_array().map(f64::to_bits)
}
fn global_bits(world: &NativeWorld) -> Vec<Vec<u64>> {
    world
        .snapshot()
        .iter()
        .map(|row| {
            row.world_transform
                .iter()
                .flatten()
                .map(|v| v.to_bits())
                .collect()
        })
        .collect()
}

#[test]
fn basis_reuse_matches_original_matrix_bits_for_every_current_translation() {
    let mut cache = MatrixBasisCache::default();
    let bases = [
        ([0., 0., 0., 1.], [1., 1., 1.]),
        ([-0., 0., -0., 1.], [1., 1., 1.]),
        ([0., -0., 0., 1.], [-1., 2., 0.5]),
        ([0.1, -0.2, 0.3, 0.86_f64.sqrt()], [1e-9, -1e200, 3.]),
        ([0., 0., 0., 1.00002], [-3., 0.5, 4.]),
    ];
    // Consecutive reuse, transitions, then revisiting evicted keys. No arithmetic
    // oracle is reimplemented: the existing uncached function supplies all bits.
    for index in [0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 0, 3, 1] {
        for translation in [
            [0., -0., f64::MIN_POSITIVE],
            [-0., 0., -f64::MIN_POSITIVE],
            [f64::MAX, -f64::MAX, 1e-300],
            [7., -11., 13.],
        ] {
            let value = Transform {
                translation,
                rotation: bases[index].0,
                scale: bases[index].1,
            };
            crate::scene::validate_components(id(1), &value, None).unwrap();
            assert_eq!(
                bits(cache.matrix(&value)),
                bits(crate::scene::matrix(&value))
            );
        }
    }
}

fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            id(0),
            60,
            (1..=12)
                .map(|n| CookedEntity {
                    id: id(n),
                    parent: (n % 4 != 1).then(|| id(n - 1)),
                    transform: Transform {
                        translation: [f64::from(n), -0., f64::MIN_POSITIVE],
                        rotation: if n <= 4 {
                            [-0., 0., -0., 1.]
                        } else {
                            [0., 0., 0.6, 0.8]
                        },
                        scale: [-1., 2., 0.5],
                    },
                    velocity: None,
                })
                .collect(),
        )
        .unwrap(),
    )
}

#[test]
fn numeric_cache_preserves_parent_root_publication_and_save_bits() {
    let mut actual = world();
    let mut reference = world();
    for tick in 0..64 {
        for (world, validated) in [(&mut actual, true), (&mut reference, false)] {
            world.with_numeric_columns(|columns| {
                columns.for_each_chunk(|chunk| {
                    for (entity, transform) in chunk
                        .ids
                        .iter()
                        .zip(chunk.transforms.as_chunks_mut::<TRANSFORM_STRIDE>().0)
                    {
                        transform[0] = f64::from(entity.0[0]) + f64::from(tick) / 64.;
                        transform[1] = if tick % 2 == 0 { -0. } else { 0. };
                        // Change basis within and between passes, including a
                        // permitted non-unit quaternion that must normalize.
                        transform[6] = if entity.0[0] <= 4 && tick % 3 == 0 {
                            1.00002
                        } else if entity.0[0] <= 4 {
                            1.
                        } else {
                            0.8
                        };
                    }
                });
                if validated {
                    columns.validate().unwrap();
                }
            });
        }
        assert_eq!(global_bits(&actual), global_bits(&reference));
        assert_eq!(
            actual.save_bytes().unwrap(),
            reference.save_bytes().unwrap()
        );
    }
}

#[test]
fn cached_basis_never_skips_component_errors_or_changes_parent_first_refusal() {
    // A hit-eligible basis does not authorize bad transform values. Preserve
    // the original hierarchy order when several subsequent rows are invalid.
    for field in [0, 3, 7] {
        let mut actual = world();
        let before = actual.save_bytes().unwrap();
        actual.with_numeric_columns(|columns| {
            let mut originals = Vec::new();
            columns.for_each_chunk(|chunk| {
                originals.push(chunk.transforms.to_vec());
                for (entity, transform) in chunk.ids.iter().zip(chunk.transforms.as_chunks_mut::<TRANSFORM_STRIDE>().0) {
                    if entity.0[0] == 2 || entity.0[0] == 3 { transform[field] = f64::NAN; }
                }
            });
            assert!(matches!(columns.validate(), Err(crate::RuntimeError::Scene(SceneError::Component(entity))) if entity == id(2)));
            let mut next = originals.into_iter();
            columns.for_each_chunk(|chunk| chunk.transforms.copy_from_slice(&next.next().unwrap()));
            columns.validate().unwrap();
        });
        assert_eq!(actual.save_bytes().unwrap(), before);
    }
}

#[test]
fn composed_overflow_still_precedes_later_component_error() {
    let mut actual = NativeWorld::new(
        CookedScene::new(
            id(0),
            60,
            (1..=3)
                .map(|n| CookedEntity {
                    id: id(n),
                    parent: (n > 1).then(|| id(n - 1)),
                    transform: Transform::default(),
                    velocity: None,
                })
                .collect(),
        )
        .unwrap(),
    );
    actual.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| {
            for (entity, transform) in chunk.ids.iter().zip(chunk.transforms.as_chunks_mut::<TRANSFORM_STRIDE>().0) {
                transform[7..].fill(1e200);
                if entity.0[0] == 3 { transform[0] = f64::NAN; }
            }
        });
        assert!(matches!(columns.validate(), Err(crate::RuntimeError::Scene(SceneError::Component(entity))) if entity == id(2)));
        columns.for_each_chunk(|chunk| {
            for transform in chunk.transforms.as_chunks_mut::<TRANSFORM_STRIDE>().0 {
                transform[..3].fill(0.); transform[7..].fill(1.);
            }
        });
        columns.validate().unwrap();
    });
}

#[test]
fn cache_eligible_rows_still_enforce_velocity_ui_and_physics_domains() {
    use incant_types::{Collider, ColliderShape, UiKind, UiNode, UiStyle, Velocity};
    for case in 0..3 {
        let mut scene = CookedScene::new(
            id(0),
            60,
            (1..=2)
                .map(|n| CookedEntity {
                    id: id(n),
                    parent: None,
                    transform: Transform::default(),
                    velocity: (case == 0 && n == 2).then_some(Velocity { linear: [0.; 3] }),
                })
                .collect(),
        )
        .unwrap();
        if case == 1 {
            scene = scene
                .with_ui(vec![crate::CookedUi {
                    entity: id(2),
                    node: std::sync::Arc::new(UiNode {
                        kind: UiKind::Root,
                        style: UiStyle::default(),
                        order: 0,
                        disabled: false,
                    }),
                }])
                .unwrap();
        } else if case == 2 {
            scene = scene
                .with_physics(vec![crate::CookedPhysics {
                    entity: id(2),
                    body: None,
                    collider: Collider {
                        shape: ColliderShape::Box {
                            half_extents: [0.5; 3],
                        },
                        ..Default::default()
                    },
                    angular_velocity: None,
                }])
                .unwrap();
        }
        let mut actual = NativeWorld::new(scene);
        let before = global_bits(&actual);
        actual.with_numeric_columns(|columns| {
            columns.for_each_chunk(|mut chunk| {
                for (row, entity) in chunk.ids.iter().enumerate() {
                    if *entity == id(2) {
                        if case == 0 {
                            chunk.velocities.as_deref_mut().unwrap()
                                [row * crate::VELOCITY_STRIDE] = f64::NAN;
                        } else {
                            chunk.transforms[row * TRANSFORM_STRIDE] =
                                if case == 1 { 1. } else { 2_000_000. };
                        }
                    }
                }
            });
            let error = columns.validate().unwrap_err();
            if case == 1 {
                assert_eq!(
                    error,
                    crate::RuntimeError::Scene(SceneError::Invalid(
                        "UI numeric transform or velocity"
                    ))
                );
            } else {
                assert_eq!(
                    error,
                    crate::RuntimeError::Scene(SceneError::Component(id(2)))
                );
            }
            columns.for_each_chunk(|chunk| {
                for transform in chunk.transforms.as_chunks_mut::<TRANSFORM_STRIDE>().0 {
                    transform[..3].fill(0.);
                }
                if let Some(velocity) = chunk.velocities {
                    velocity.fill(0.);
                }
            });
            columns.validate().unwrap();
        });
        assert_eq!(global_bits(&actual), before);
    }
}

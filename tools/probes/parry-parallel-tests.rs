// Shared public-API regression cases, compiled against both pinned dimensions.
// Full Debug state equality covers node order, flags, parents and reverse maps;
// this is trusted live state, not an imported physics snapshot format.
use parry::bounding_volume::Aabb;
use parry::math::Vector;
use parry::partitioning::{Bvh, BvhBuildStrategy, BvhLeafUpdateStatus, BvhWorkspace};

fn bounds(id: u32, shift: f32) -> Aabb {
    let mut center = Vector::splat(0.0);
    for axis in 0..parry::math::DIM {
        center[axis] = ((id * (13 + axis as u32 * 8)) % 97) as f32 + shift;
    }
    Aabb::from_half_extents(center, Vector::splat(0.5))
}

fn exact(left: &Bvh, right: &Bvh) {
    assert!(
        format!("{left:?}") == format!("{right:?}"),
        "complete BVH states differ"
    );
}

// The contract deliberately updates *all existing* leaves before any insertion.
fn serial_batch(tree: &mut Bvh, updates: &[(Aabb, u32, f32)]) -> Vec<BvhLeafUpdateStatus> {
    let statuses: Vec<_> = updates
        .iter()
        .map(|(aabb, id, margin)| {
            tree.update_partially_if_present(*aabb, *id, *margin)
                .unwrap_or(BvhLeafUpdateStatus::Inserted)
        })
        .collect();
    for ((aabb, id, margin), status) in updates.iter().zip(&statuses) {
        if *status == BvhLeafUpdateStatus::Inserted {
            assert_eq!(
                tree.insert_or_update_partially(*aabb, *id, *margin),
                *status
            );
        }
    }
    statuses
}

#[test]
fn duplicate_updates_refuse_before_tree_or_status_changes() {
    for ids in [[0, 0, 1], [9, 9, 0], [0, 9, 0], [9, 0, 9]] {
        let mut tree =
            Bvh::from_leaves(BvhBuildStrategy::Binned, &[bounds(0, 0.0), bounds(1, 0.0)]);
        let before = tree.clone();
        let mut statuses = vec![BvhLeafUpdateStatus::Inserted];
        let updates: Vec<_> = ids
            .into_iter()
            .map(|id| (bounds(id, 4.0), id, 0.2))
            .collect();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            tree.insert_or_update_batch_partially_parallel(&updates, &mut statuses);
        }));
        assert!(result.is_err(), "duplicate {ids:?} was accepted");
        exact(&tree, &before);
        assert_eq!(statuses, [BvhLeafUpdateStatus::Inserted]);
        tree.assert_well_formed();
    }
}

#[test]
fn sibling_leaf_updates_and_empty_batches_match_serial() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    pool.install(|| {
        let mut serial =
            Bvh::from_leaves(BvhBuildStrategy::Binned, &[bounds(0, 0.0), bounds(1, 0.0)]);
        let mut parallel = serial.clone();
        let mut statuses = vec![BvhLeafUpdateStatus::Inserted];
        parallel.insert_or_update_batch_partially_parallel(&[], &mut statuses);
        assert!(statuses.is_empty());
        exact(&serial, &parallel);
        for step in 0..64 {
            // The two updates occupy different fields of the same wide node.
            let updates = [
                (bounds(1, step as f32), 1, 0.25),
                (bounds(0, step as f32), 0, 0.0),
            ];
            let expected = serial_batch(&mut serial, &updates);
            parallel.insert_or_update_batch_partially_parallel(&updates, &mut statuses);
            assert_eq!(statuses, expected);
            exact(&serial, &parallel);
        }
    });
}

#[test]
fn mixed_batch_preserves_two_phase_order_statuses_and_reverse_indices() {
    for workers in [1, 2, 4] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .unwrap();
        pool.install(|| {
            let initial: Vec<_> = (0..256).map(|id| bounds(id, 0.0)).collect();
            for order in 0..3 {
                let mut serial = Bvh::from_leaves(BvhBuildStrategy::Binned, &initial);
                let mut parallel = serial.clone();
                let mut updates: Vec<_> = (0..320)
                    .map(|id| {
                        let shift = if id % 3 == 0 { 0.0 } else { 2.0 };
                        (bounds(id, shift), id, if id % 2 == 0 { 0.25 } else { 0.0 })
                    })
                    .collect();
                if order == 1 {
                    updates.reverse();
                }
                if order == 2 {
                    updates.rotate_left(139);
                }
                let expected = serial_batch(&mut serial, &updates);
                let mut statuses = Vec::new();
                parallel.insert_or_update_batch_partially_parallel(&updates, &mut statuses);
                assert_eq!(statuses, expected);
                assert!(statuses.contains(&BvhLeafUpdateStatus::Unchanged));
                assert!(statuses.contains(&BvhLeafUpdateStatus::UpdatedInPlace));
                assert!(statuses.contains(&BvhLeafUpdateStatus::Inserted));
                exact(&serial, &parallel);
                serial.refit(&mut BvhWorkspace::default());
                parallel.refit_parallel(&mut BvhWorkspace::default());
                exact(&serial, &parallel);
                parallel.assert_well_formed();
                for id in (0..320).step_by(5) {
                    serial.remove(id);
                    parallel.remove(id);
                }
                exact(&serial, &parallel);
                parallel.assert_well_formed();
            }
        });
    }
}

#[test]
fn refit_matches_serial_across_thresholds_sparse_slots_and_repeated_flags() {
    for workers in [1, 2, 4] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(workers)
            .build()
            .unwrap();
        pool.install(|| {
            for count in [0, 1, 2, 3, 2048, 2049, 4097] {
                let initial: Vec<_> = (0..count).map(|id| bounds(id, 0.0)).collect();
                let mut serial = Bvh::from_leaves(BvhBuildStrategy::Binned, &initial);
                if count == 4097 {
                    // Leave orphans, recycle some slots, and use sparse leaf IDs.
                    for id in (1..count).step_by(7) {
                        serial.remove(id);
                    }
                    for id in (1..count).step_by(19) {
                        serial.insert(bounds(id, 1.0), count * 2 + id);
                    }
                }
                let mut parallel = serial.clone();
                let mut sw = BvhWorkspace::default();
                let mut pw = BvhWorkspace::default();
                for pass in 0..4 {
                    for id in 0..count {
                        let aabb = bounds(id, pass as f32 * 0.4);
                        let margin = if id % 2 == 0 { 0.5 } else { 0.0 };
                        assert_eq!(
                            serial.update_partially_if_present(aabb, id, margin),
                            parallel.update_partially_if_present(aabb, id, margin)
                        );
                    }
                    if pass % 2 == 0 {
                        serial.refit_without_resolve(&mut sw);
                        parallel.refit_without_resolve_parallel(&mut pw);
                    } else {
                        serial.refit(&mut sw);
                        parallel.refit_parallel(&mut pw);
                    }
                    exact(&serial, &parallel);
                    // The upstream full validator assumes pending flags have
                    // been resolved; the flag-preserving pass deliberately has
                    // not done that. Compare full state there, then validate
                    // both trees after the next resolving pass.
                    if pass % 2 != 0 {
                        serial.assert_well_formed();
                        parallel.assert_well_formed();
                    }
                }
                // Removal uses both parent and leaf reverse mappings.
                for id in (0..count).step_by(11) {
                    serial.remove(id);
                    parallel.remove(id);
                }
                exact(&serial, &parallel);
                parallel.assert_well_formed();
            }
        });
    }
}

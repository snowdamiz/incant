use super::*;
use crate::motion::owner::{Extent, History};
use glam::Mat4;
use std::sync::Arc;
fn frame(history: &mut History) -> MotionFrame {
    let camera = crate::camera::CameraView::preview();
    let pending = history
        .prepare(
            Extent::new([64, 32], [64, 32], [0., 0., 64., 32.], camera),
            Mat4::IDENTITY,
        )
        .unwrap();
    history.accept(pending);
    history.frame().unwrap()
}
#[test]
fn mirrored_twins_share_ids_distinct_allocations_and_bindings_do_not() {
    let a: Arc<[u32]> = Arc::from([1, 2, 3]);
    let twin = Arc::clone(&a);
    let b: Arc<[u32]> = Arc::from([1, 2, 3]);
    let key = |p: &Arc<[u32]>| Key([0, p.as_ptr() as usize, p.len(), 0]);
    let animated = Key([1, 0, 3, 0]);
    let other_primitive = Key([1, 0, 3, 1]);
    let mut groups = [
        Group {
            key: key(&a),
            batch: 0,
            count: 3,
        },
        Group {
            key: animated,
            batch: 1,
            count: 1,
        },
        Group {
            key: key(&b),
            batch: 2,
            count: 3,
        },
        Group {
            key: key(&twin),
            batch: 3,
            count: 3,
        },
        Group {
            key: animated,
            batch: 4,
            count: 1,
        },
        Group {
            key: other_primitive,
            batch: 5,
            count: 1,
        },
    ];
    let (first, n) = assign(&mut groups, 6).unwrap();
    assert_eq!(n, 8);
    assert_eq!(
        first.iter().map(|b| b.first).collect::<Vec<_>>(),
        [1, 4, 5, 1, 4, 8]
    );
    groups.reverse();
    let (again, m) = assign(&mut groups, 6).unwrap();
    assert_eq!(m, n);
    assert_eq!(
        first.iter().map(|b| b.first).collect::<Vec<_>>(),
        again.iter().map(|b| b.first).collect::<Vec<_>>()
    );
}
#[test]
fn exclusion_capacity_and_grouping_refusal_are_bounded() {
    let mut groups = [Group {
        key: Key([0, 1, 1, 0]),
        batch: 1,
        count: MAX,
    }];
    let (bases, n) = assign(&mut groups, 3).unwrap();
    assert_eq!(n, MAX as u32);
    assert_eq!(bases[0].first, 0);
    assert_eq!(bases[1].first, 1);
    assert_eq!(bases[2].first, 0);
    groups[0].count = MAX + 1;
    assert!(matches!(assign(&mut groups, 3), Err(MotionError::Capacity)));
    let mut broken = [
        Group {
            key: Key([0, 1, 1, 0]),
            batch: 0,
            count: 1,
        },
        Group {
            key: Key([0, 1, 1, 0]),
            batch: 1,
            count: 2,
        },
    ];
    assert!(matches!(assign(&mut broken, 2), Err(MotionError::Topology)));
    let allocations = crate::retained_alloc::measured(|| {
        let mut full = plan::MAX_MOTION_INSTANCE_BYTES;
        let mut empty = 0;
        assert!(charge(&mut full, 1, 1).is_err());
        assert!(charge(&mut empty, usize::MAX, MAX).is_err());
    });
    assert_eq!(allocations, 0);
}
#[test]
fn candidate_changes_are_private_retry_deduplicates_and_classes_never_alias_identity() {
    let mut state = State::new(100_000);
    let mut history = History::default();
    let accepted = frame(&mut history);
    state.begin();
    assert_eq!(state.mark(0, true), 0);
    for id in 1..=31 {
        state.mark(id, true);
        state.mark(id, true);
    }
    // Merely preparing or abandoning a candidate does not publish anything.
    assert_eq!(state.view(accepted.clone()).changed_count(), 0);
    state.begin();
    state.mark(32, true);
    state.mark(1, false);
    state.mark(100_000, true);
    assert_eq!(state.view(accepted.clone()).changed_count(), 0);
    state.accept();
    let v = state.view(frame(&mut history));
    assert_eq!(v.changed_count(), 2);
    assert_eq!(v.changed(32), Some(true));
    assert_eq!(v.changed(1), Some(false));
    assert_eq!(v.changed(100_000), Some(true));
    assert_eq!(v.changed(0), None);
    assert_eq!(v.changed(100_001), None);
    assert_ne!(
        v.changed_classes() & (1 << 1),
        0,
        "owner32 collides with1 only in class mask"
    );
    let allocations = crate::retained_alloc::measured(|| {
        for _ in 0..64 {
            state.begin();
            for id in 1..=31 {
                state.mark(id, true);
            }
            state.accept();
        }
    });
    assert_eq!(allocations, 0);
    let v = state.view(frame(&mut history));
    assert_eq!(v.changed_count(), 31);
    assert_eq!(v.changed_classes(), 0x7fff_ffff);
}
#[test]
fn accepted_epoch_and_owner_distinguish_cut_retry_and_replacement() {
    let mut history = History::default();
    let mut state = State::new(1);
    state.begin();
    state.mark(1, true);
    state.accept();
    let before = frame(&mut history);
    history.invalidate();
    state.begin();
    state.mark(1, true);
    let camera = crate::camera::CameraView::preview();
    assert!(
        history
            .prepare(
                Extent::new([64, 32], [64, 32], [0., 0., 64., 32.], camera),
                Mat4::NAN
            )
            .is_err()
    );
    let unchanged = state.view(history.frame().unwrap());
    assert_eq!(unchanged.frame().sequence(), before.sequence());
    assert_eq!(unchanged.frame().epoch(), before.epoch());
    assert!(unchanged.frame().same_owner(&before));
    let after = frame(&mut history);
    state.accept();
    assert!(after.same_owner(&before));
    assert_eq!(after.epoch(), before.epoch() + 1);
    assert!(!after.has_previous());
    let replacement = frame(&mut History::default());
    assert!(!replacement.same_owner(&before));
    assert!(!replacement.same_owner(&after));
}
#[test]
fn matrix_changes_are_exact_not_numeric_equality_or_lossy_hashes() {
    let a = DMat4::IDENTITY;
    let mut b = a;
    b.x_axis.y = -0.;
    assert_eq!(a, b);
    assert!(!same_matrix(a, b));
    b = a;
    b.w_axis.x = f64::from_bits(1);
    assert!(!same_matrix(a, b));
    assert!(same_matrix(a, a));
}

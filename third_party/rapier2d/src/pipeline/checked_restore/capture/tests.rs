use super::super::WorkBudget;
use super::*;
use crate::{dynamics::RigidBodyBuilder, geometry::ColliderBuilder, math::Vector};
use alloc::vec::Vec;
use incant_physics_snapshot::{Budget, MAX_ALLOCATION, construction::AllocationBudget};
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn work() -> WorkBudget {
    WorkBudget::new(u64::MAX)
}
fn allocation() -> AllocationBudget {
    AllocationBudget::new(MAX_ALLOCATION, 1_000_000)
}
fn frame(w: &mut CheckpointOwner) -> Result<NativeFrame, CaptureError> {
    w.checkpoint(&mut budget(), &mut work())
}
fn owner() -> CheckpointOwner {
    let mut o = CheckpointOwner::new().unwrap();
    let w = o.access_mut().unwrap();
    #[cfg(all(feature = "parallel", not(feature = "unsync-callbacks")))]
    w.configure_thread_pool(2).unwrap();
    #[cfg(feature = "dim2")]
    let floor = ColliderBuilder::cuboid(8., 0.25);
    #[cfg(feature = "dim3")]
    let floor = ColliderBuilder::cuboid(8., 0.25, 8.);
    w.colliders
        .insert(floor.translation(-Vector::Y * 0.25).build());
    for i in 0..4 {
        let b = RigidBodyBuilder::dynamic()
            .translation(Vector::Y * (1. + i as f32 * 1.1))
            .can_sleep(false)
            .ccd_enabled(i == 3)
            .build();
        let c = if i % 2 == 0 {
            ColliderBuilder::ball(0.45)
        } else {
            ColliderBuilder::capsule_y(0.15, 0.3)
        };
        w.insert(b, c.friction(0.6).restitution(0.1));
    }
    o
}
#[test]
fn direct_checkpoint_construct_recapture_and_full_ordered_continuation() {
    let mut source = owner();
    for _ in 0..64 {
        source.step().unwrap();
    }
    let saved = frame(&mut source).unwrap();
    let mut restored = saved
        .reconstruct(&mut budget(), &mut work(), &mut allocation())
        .unwrap();
    assert_eq!(saved.encoded(), frame(&mut restored).unwrap().encoded());
    #[cfg(all(feature = "parallel", not(feature = "unsync-callbacks")))]
    assert!(std::sync::Arc::ptr_eq(
        &source.get().unwrap().thread_pool().unwrap(),
        &restored.get().unwrap().thread_pool().unwrap()
    ));
    for tick in 0..96 {
        source.step().unwrap();
        restored.step().unwrap();
        let a = frame(&mut source).unwrap_or_else(|e| panic!("source tick {tick}: {e:?}"));
        let b = frame(&mut restored).unwrap_or_else(|e| panic!("restored tick {tick}: {e:?}"));
        assert_eq!(
            a.encoded(),
            b.encoded(),
            "complete ordered frame tick {tick}"
        );
        let a = source.get().unwrap();
        let b = restored.get().unwrap();
        assert_eq!(
            a.bodies
                .iter()
                .map(|(h, _)| h.into_raw_parts())
                .collect::<Vec<_>>(),
            b.bodies
                .iter()
                .map(|(h, _)| h.into_raw_parts())
                .collect::<Vec<_>>()
        );
    }
}
#[test]
fn direct_checkpoint_budget_and_corruption_refusals_leave_source_unchanged() {
    let mut o = owner();
    o.step().unwrap();
    let before = frame(&mut o).unwrap();
    assert!(matches!(
        o.checkpoint(&mut Budget::new(0).unwrap(), &mut work()),
        Err(CaptureError::Record(incant_physics_snapshot::Error::Limit))
    ));
    assert!(
        before
            .reconstruct(&mut budget(), &mut work(), &mut AllocationBudget::new(0, 0))
            .is_err()
    );
    assert_eq!(before.encoded(), frame(&mut o).unwrap().encoded());
    let mut bad = frame(&mut o).unwrap();
    bad.bytes[0] ^= 1;
    assert!(
        bad.reconstruct(&mut budget(), &mut work(), &mut allocation())
            .is_err()
    );
    let mut bad = frame(&mut o).unwrap();
    bad.identity.build[0] ^= 1;
    assert!(matches!(
        bad.reconstruct(&mut budget(), &mut work(), &mut allocation()),
        Err(CaptureError::Build)
    ));
    assert_eq!(before.encoded(), frame(&mut o).unwrap().encoded());
}
#[test]
fn direct_checkpoint_empty_world_and_access_invalidation_remain_explicit() {
    let mut o = CheckpointOwner::new().unwrap();
    o.step().unwrap();
    let saved = frame(&mut o).unwrap();
    let mut restored = saved
        .reconstruct(&mut budget(), &mut work(), &mut allocation())
        .unwrap();
    assert_eq!(saved.encoded(), frame(&mut restored).unwrap().encoded());
    let _ = restored.access_mut().unwrap();
    assert!(matches!(
        frame(&mut restored),
        Err(CaptureError::NotCompleted)
    ));
    restored.step().unwrap();
    assert!(frame(&mut restored).is_ok());
}

#[test]
fn direct_checkpoint_mutations_preserve_native_generations_and_exact_refusal_ticks() {
    use crate::dynamics::{RigidBodyHandle, RigidBodyType};
    fn change(o: &mut CheckpointOwner, tick: usize, ids: &mut [RigidBodyHandle; 3]) {
        let w = o.access_mut().unwrap();
        match tick {
            3 => w.bodies[ids[0]].sleep(),
            7 => w.bodies[ids[0]].wake_up(true),
            9 => w.bodies[ids[1]].set_translation(Vector::X * 4. + Vector::Y, true),
            13 => {
                w.remove_body(ids[1]).unwrap();
            }
            15 => {
                ids[1] = w
                    .insert(
                        RigidBodyBuilder::dynamic().translation(Vector::X + Vector::Y * 0.5),
                        ColliderBuilder::ball(0.5),
                    )
                    .0;
            }
            21 => w.bodies[ids[1]].set_enabled(false),
            24 => w.bodies[ids[1]].set_enabled(true),
            29 => w.bodies[ids[2]].set_body_type(RigidBodyType::Fixed, true),
            32 => w.bodies[ids[2]].set_body_type(RigidBodyType::Dynamic, true),
            _ => {}
        }
    }
    let mut source = CheckpointOwner::new().unwrap();
    let w = source.access_mut().unwrap();
    w.insert_collider(
        ColliderBuilder::ball(100.).translation(-Vector::Y * 100.5),
        None,
    );
    let mut ids = [0., 1., -1.].map(|x| {
        w.insert(
            RigidBodyBuilder::dynamic().translation(Vector::X * x),
            ColliderBuilder::ball(0.5),
        )
        .0
    });
    for _ in 0..64 {
        source.step().unwrap();
    }
    let saved = frame(&mut source).unwrap();
    let mut restored = saved
        .reconstruct(&mut budget(), &mut work(), &mut allocation())
        .unwrap();
    let mut other_ids = ids;
    let mut refused = std::collections::BTreeSet::new();
    let mut mass_refused = std::collections::BTreeSet::new();
    for tick in 0..40 {
        change(&mut source, tick, &mut ids);
        change(&mut restored, tick, &mut other_ids);
        assert!(matches!(
            frame(&mut source),
            Err(CaptureError::NotCompleted)
        ));
        assert!(matches!(
            frame(&mut restored),
            Err(CaptureError::NotCompleted)
        ));
        source.step().unwrap();
        restored.step().unwrap();
        match (frame(&mut source), frame(&mut restored)) {
            (Ok(a), Ok(b)) => assert_eq!(a.encoded(), b.encoded(), "full frame tick {tick}"),
            (Err(CaptureError::PendingMutation), Err(CaptureError::PendingMutation)) => {
                refused.insert(tick);
            }
            (
                Err(CaptureError::Numeric(super::super::AdmissionError::LocalMass)),
                Err(CaptureError::Numeric(super::super::AdmissionError::LocalMass)),
            ) => {
                mass_refused.insert(tick);
            }
            (a, b) => panic!(
                "unexpected capture outcomes at {tick}: {:?} / {:?}",
                a.err(),
                b.err()
            ),
        }
        assert_eq!(ids, other_ids);
        // Compare even refused boundaries, independently of the admitted frame.
        assert_eq!(
            serde_json::to_vec(source.get().unwrap()).unwrap(),
            serde_json::to_vec(restored.get().unwrap()).unwrap()
        );
        assert_eq!(
            source
                .access_mut()
                .unwrap()
                .physics_pipeline
                .checkpoint_observation(),
            restored
                .access_mut()
                .unwrap()
                .physics_pipeline
                .checkpoint_observation()
        );
    }
    assert_eq!(refused, [7].into_iter().collect());
    assert_eq!(mass_refused, [21, 22, 23].into_iter().collect());
    std::eprintln!(
        "DIRECT_CAPTURE_CHURN dimension={} paired_steps=40 journal_refused={refused:?} mass_refused={mass_refused:?}",
        crate::math::DIM
    );
}

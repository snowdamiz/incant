use super::*;
use crate::dynamics::{FixedJointBuilder, RigidBodyBuilder};
use crate::geometry::{ColliderBuilder, NarrowPhase};
fn work() -> WorkBudget {
    WorkBudget::new(1_000_000)
}
#[test]
fn capture_owner_actual_positive_step_and_read_borrow_preserve_authority() {
    let mut owner = CheckpointOwner::new().unwrap();
    assert!(matches!(
        owner.completed_step(&mut work()),
        Err(CaptureError::NotCompleted)
    ));
    let (body, collider) = owner
        .access_mut()
        .unwrap()
        .insert(RigidBodyBuilder::dynamic(), ColliderBuilder::ball(0.5));
    owner.step().unwrap();
    assert!(owner.get().unwrap().bodies.contains(body));
    let identity = owner.identity().unwrap();
    {
        let capture = owner.completed_step(&mut work()).unwrap();
        assert_eq!(capture.identity(), identity);
        assert!(capture.get().colliders.contains(collider));
    }
    assert!(owner.completed_step(&mut work()).is_ok());
    assert_eq!(identity, compiled_identity().unwrap());
    assert_ne!(identity.build, [0; 32]);
}
#[test]
fn capture_owner_mutable_borrow_and_direct_native_step_cannot_reissue_completion() {
    let mut owner = CheckpointOwner::new().unwrap();
    owner.step().unwrap();
    assert!(owner.completed_step(&mut work()).is_ok());
    let native = owner.access_mut().unwrap();
    native.step();
    assert!(matches!(
        owner.completed_step(&mut work()),
        Err(CaptureError::NotCompleted)
    ));
    owner.step().unwrap();
    assert!(owner.completed_step(&mut work()).is_ok());
    let _ = owner.access_mut().unwrap();
    assert!(matches!(
        owner.completed_step(&mut work()),
        Err(CaptureError::NotCompleted)
    ));
}
#[test]
fn capture_owner_zero_time_and_dispatcher_replacement_refuse_without_repair() {
    let mut owner = CheckpointOwner::new().unwrap();
    owner.access_mut().unwrap().integration_parameters.dt = 0.;
    owner.step().unwrap();
    assert!(matches!(
        owner.completed_step(&mut work()),
        Err(CaptureError::NotCompleted)
    ));
    owner.access_mut().unwrap().integration_parameters.dt = 1. / 60.;
    owner.step().unwrap();
    assert!(owner.completed_step(&mut work()).is_ok());
    owner.access_mut().unwrap().narrow_phase = NarrowPhase::new();
    owner.step().unwrap();
    assert!(matches!(
        owner.completed_step(&mut work()),
        Err(CaptureError::Dispatcher)
    ));
    assert!(matches!(
        owner.completed_step(&mut work()),
        Err(CaptureError::Dispatcher)
    ));
}
#[test]
fn capture_owner_unwind_poison_blocks_reads_mutations_steps_and_capture() {
    let mut owner = CheckpointOwner::new().unwrap();
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = owner.advance(|w| {
            w.gravity *= 2.;
            panic!("controlled post-mutation step unwind");
        });
    }));
    assert!(panic.is_err());
    assert!(matches!(owner.get(), Err(CaptureError::Poisoned)));
    assert!(matches!(owner.access_mut(), Err(CaptureError::Poisoned)));
    assert_eq!(owner.step(), Err(CaptureError::Poisoned));
    assert!(matches!(
        owner.completed_step(&mut work()),
        Err(CaptureError::Poisoned)
    ));
}
#[test]
fn capture_owner_joint_history_and_budget_failure_do_not_clear_native_state() {
    let mut owner = CheckpointOwner::new().unwrap();
    let w = owner.access_mut().unwrap();
    let a = w.bodies.insert(RigidBodyBuilder::dynamic().build());
    let b = w.bodies.insert(RigidBodyBuilder::dynamic().build());
    let joint = w
        .impulse_joints
        .insert(a, b, FixedJointBuilder::new(), false);
    assert!(w.impulse_joints.remove(joint, false).is_some());
    owner.step().unwrap();
    assert!(matches!(
        owner.completed_step(&mut work()),
        Err(CaptureError::Family(_))
    ));
    assert!(matches!(
        owner.completed_step(&mut work()),
        Err(CaptureError::Family(_))
    ));
    let mut fresh = CheckpointOwner::new().unwrap();
    fresh.step().unwrap();
    assert!(matches!(
        fresh.completed_step(&mut WorkBudget::new(0)),
        Err(CaptureError::Numeric(
            super::super::super::AdmissionError::Work
        ))
    ));
    assert!(fresh.completed_step(&mut work()).is_ok());
}

#[test]
fn capture_only_mode_refusal_does_not_disable_ordinary_simulation() {
    let mut owner = CheckpointOwner::for_simulation();
    // A test-only stand-in for a separately compiled unsupported capture mode;
    // no public setter changes the actual compiled admission result.
    owner.identity = Err(CaptureError::Build);
    owner
        .access_mut()
        .unwrap()
        .insert(RigidBodyBuilder::dynamic(), ColliderBuilder::ball(0.5));
    owner.step().unwrap();
    assert_eq!(owner.get().unwrap().bodies.len(), 1);
    assert_eq!(owner.identity(), Err(CaptureError::Build));
    assert!(matches!(
        owner.completed_step(&mut work()),
        Err(CaptureError::Build)
    ));
    owner.step().unwrap();
    assert!(owner.get().is_ok());
}

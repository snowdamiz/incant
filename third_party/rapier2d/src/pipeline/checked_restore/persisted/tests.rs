use super::super::capture::{FrameOrigin, NativeFrame};
use super::*;
use crate::{dynamics::RigidBodyBuilder, geometry::ColliderBuilder, math::Vector};
use incant_physics_snapshot::MAX_ALLOCATION;
fn payload() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn work() -> WorkBudget {
    WorkBudget::new(u64::MAX)
}
fn allocation() -> AllocationBudget {
    AllocationBudget::new(MAX_ALLOCATION, 1_000_000)
}
fn capture(owner: &mut CheckpointOwner) -> NativeFrame {
    owner.checkpoint(&mut payload(), &mut work()).unwrap()
}
fn boxes() -> CheckpointOwner {
    let mut o = CheckpointOwner::new().unwrap();
    let w = o.access_mut().unwrap();
    #[cfg(feature = "dim2")]
    let floor = ColliderBuilder::cuboid(8., 0.25);
    #[cfg(feature = "dim3")]
    let floor = ColliderBuilder::cuboid(8., 0.25, 8.);
    w.insert_collider(floor.translation(-Vector::Y * 0.25), None);
    for i in 0..2 {
        #[cfg(feature = "dim2")]
        let box_ = ColliderBuilder::cuboid(0.4, 0.4);
        #[cfg(feature = "dim3")]
        let box_ = ColliderBuilder::cuboid(0.4, 0.4, 0.4);
        // Default gyroscopic/CCD/material settings, matching ordinary runtimes.
        w.insert(
            RigidBodyBuilder::dynamic().translation(Vector::Y * (0.6 + i as f32)),
            box_,
        );
    }
    for _ in 0..67 {
        o.step().unwrap();
    }
    o
}
#[test]
fn persisted_native_default_boxes_warm_import_recapture_and_continuation() {
    let mut original = boxes();
    let frame = capture(&mut original);
    let bytes = frame.persisted_bytes(&mut payload(), &mut work()).unwrap();
    let mut imported =
        CheckpointOwner::from_persisted(&bytes, &mut payload(), &mut work(), &mut allocation())
            .unwrap();
    assert!(matches!(
        imported.completed_step(&mut work()),
        Err(CaptureError::NotCompleted)
    ));
    let fresh = capture(&mut imported);
    assert_eq!(fresh.origin(), FrameOrigin::ImportedValidated);
    assert_eq!(fresh.encoded(), bytes);
    assert_eq!(
        fresh.persisted_bytes(&mut payload(), &mut work()).unwrap(),
        bytes
    );
    let mut opaque_again = fresh
        .reconstruct(&mut payload(), &mut work(), &mut allocation())
        .unwrap();
    assert!(matches!(
        opaque_again.completed_step(&mut work()),
        Err(CaptureError::NotCompleted)
    ));
    assert_eq!(
        capture(&mut opaque_again).origin(),
        FrameOrigin::ImportedValidated
    );
    for tick in 0..32 {
        original.step().unwrap();
        imported.step().unwrap();
        let a = capture(&mut original);
        let b = capture(&mut imported);
        assert_eq!(a.encoded(), b.encoded(), "native continuation {tick}");
        assert_eq!(b.origin(), FrameOrigin::ImportedCompleted);
        assert!(imported.completed_step(&mut work()).is_ok());
    }
}
#[test]
fn persisted_native_empty_import_budget_header_and_identity_refusals() {
    let mut o = CheckpointOwner::new().unwrap();
    o.step().unwrap();
    let f = capture(&mut o);
    let bytes = f.persisted_bytes(&mut payload(), &mut work()).unwrap();
    assert!(
        CheckpointOwner::from_persisted(
            &bytes,
            &mut Budget::new(0).unwrap(),
            &mut work(),
            &mut allocation()
        )
        .is_err()
    );
    assert!(
        CheckpointOwner::from_persisted(
            &bytes,
            &mut payload(),
            &mut WorkBudget::new(0),
            &mut allocation()
        )
        .is_err()
    );
    assert!(
        CheckpointOwner::from_persisted(
            &bytes,
            &mut payload(),
            &mut work(),
            &mut AllocationBudget::new(0, 0)
        )
        .is_err()
    );
    let mut bad = bytes.clone();
    bad[0] ^= 1;
    assert!(
        CheckpointOwner::from_persisted(&bad, &mut payload(), &mut work(), &mut allocation())
            .is_err()
    );
    assert!(
        CheckpointOwner::from_persisted(
            &bytes[..bytes.len() - 1],
            &mut payload(),
            &mut work(),
            &mut allocation()
        )
        .is_err()
    );
    assert_eq!(capture(&mut o).encoded(), bytes);
    let mut loaded =
        CheckpointOwner::from_persisted(&bytes, &mut payload(), &mut work(), &mut allocation())
            .unwrap();
    assert_eq!(
        capture(&mut loaded)
            .persisted_bytes(&mut payload(), &mut work())
            .unwrap(),
        bytes
    );
    loaded.access_mut().unwrap();
    assert!(matches!(
        loaded.checkpoint(&mut payload(), &mut work()),
        Err(CaptureError::NotCompleted)
    ));
}
#[test]
fn persisted_native_ccd_refusal_and_imported_next_step_guard() {
    let mut o = CheckpointOwner::new().unwrap();
    o.access_mut().unwrap().insert(
        RigidBodyBuilder::dynamic().ccd_enabled(true),
        ColliderBuilder::ball(0.5),
    );
    o.step().unwrap();
    let f = capture(&mut o);
    assert_eq!(
        f.persisted_bytes(&mut payload(), &mut work()),
        Err(PersistedError::Domain(DomainError::Ccd))
    );
    // Input bytes can be copied from an opaque CCD capture, but imported admission
    // independently rejects them before construction.
    assert!(matches!(
        CheckpointOwner::from_persisted(
            f.encoded(),
            &mut payload(),
            &mut work(),
            &mut allocation()
        ),
        Err(PersistedError::Domain(DomainError::Ccd))
    ));
    let mut o = boxes();
    let f = capture(&mut o);
    let mut imported = CheckpointOwner::from_persisted(
        f.encoded(),
        &mut payload(),
        &mut work(),
        &mut allocation(),
    )
    .unwrap();
    let w = imported.access_mut().unwrap();
    w.bodies.iter_mut().next().unwrap().1.enable_ccd(true);
    let before = serde_json::to_vec(w).unwrap();
    assert_eq!(
        imported.step(),
        Err(CaptureError::ImportedDomain(DomainError::Ccd))
    );
    assert_eq!(serde_json::to_vec(imported.get().unwrap()).unwrap(), before);
}
mod adversarial;
#[test]
fn persisted_native_flat_compound_workspace_and_tile_order_continue() {
    use crate::math::Pose;
    use parry::shape::SharedShape;
    let mut o = CheckpointOwner::new().unwrap();
    let w = o.access_mut().unwrap();
    #[cfg(feature = "dim2")]
    let tile = SharedShape::cuboid(1., 0.25);
    #[cfg(feature = "dim3")]
    let tile = SharedShape::cuboid(1., 0.25, 1.);
    let parts = alloc::vec![
        (
            Pose::from_translation(Vector::X + Vector::Y * 0.25),
            tile.clone()
        ),
        (
            Pose::from_translation(Vector::X * 4. + Vector::Y * 0.25),
            tile
        )
    ];
    w.insert_collider(
        ColliderBuilder::new(SharedShape::compound(parts))
            .translation(-Vector::X - Vector::Y * 0.5),
        None,
    );
    w.insert(
        RigidBodyBuilder::dynamic().translation(Vector::Y * 0.6),
        ColliderBuilder::ball(0.4),
    );
    for _ in 0..67 {
        o.step().unwrap();
    }
    let f = capture(&mut o);
    let bytes = f.persisted_bytes(&mut payload(), &mut work()).unwrap();
    let mut loaded =
        CheckpointOwner::from_persisted(&bytes, &mut payload(), &mut work(), &mut allocation())
            .unwrap();
    assert_eq!(
        capture(&mut loaded)
            .persisted_bytes(&mut payload(), &mut work())
            .unwrap(),
        bytes
    );
    for tick in 0..32 {
        o.step().unwrap();
        loaded.step().unwrap();
        assert_eq!(
            capture(&mut o).encoded(),
            capture(&mut loaded).encoded(),
            "compound tick {tick}"
        );
    }
}

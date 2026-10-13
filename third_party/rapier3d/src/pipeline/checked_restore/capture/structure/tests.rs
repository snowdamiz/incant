use super::*;
use crate::{dynamics::RigidBodyBuilder, geometry::ColliderBuilder};
fn work() -> WorkBudget {
    WorkBudget::new(1_000_000)
}
fn two() -> (
    CheckpointOwner,
    RigidBodyHandle,
    RigidBodyHandle,
    ColliderHandle,
) {
    let mut o = CheckpointOwner::new().unwrap();
    let w = o.access_mut().unwrap();
    let (a, c) = w.insert(RigidBodyBuilder::dynamic(), ColliderBuilder::ball(0.5));
    let b = w.insert_body(RigidBodyBuilder::dynamic());
    o.step().unwrap();
    (o, a, b, c)
}
#[test]
fn persisted_structure_preflight_preserves_authority_and_rejects_duplicates_overlap_missing() {
    let (mut o, a, b, c) = two();
    let n = StructuralInsertions::default();
    assert_eq!(
        o.preflight_structure(
            &[StructuralRemoval::Body(a), StructuralRemoval::Body(a)],
            n,
            &mut work()
        )
        .map(|_| ()),
        Err(StructuralError::Order)
    );
    assert_eq!(
        o.preflight_structure(
            &[StructuralRemoval::Body(a), StructuralRemoval::Collider(c)],
            n,
            &mut work()
        )
        .map(|_| ()),
        Err(StructuralError::Overlap)
    );
    let missing = RigidBodyHandle::from_raw_parts(b.into_raw_parts().0, u32::MAX);
    assert_eq!(
        o.preflight_structure(&[StructuralRemoval::Body(missing)], n, &mut work())
            .map(|_| ()),
        Err(StructuralError::Missing)
    );
    assert_eq!(
        o.preflight_structure(&[], n, &mut WorkBudget::new(0))
            .map(|_| ()),
        Err(StructuralError::Work)
    );
    assert!(o.completed_step(&mut work()).is_ok());
    assert!(o.get().unwrap().bodies.contains(a));
    assert!(o.get().unwrap().colliders.contains(c));
    assert!(
        o.remove_body_checked(missing, &mut work())
            .unwrap()
            .is_none()
    );
    assert!(o.completed_step(&mut work()).is_ok());
}
#[test]
fn persisted_structure_cumulative_generation_refusal_precedes_every_removal() {
    let (mut o, a, b, _) = two();
    // Test-only hostile scalar injection into otherwise real native state. The
    // production file route never invokes serde-to-live.
    let w = o.access_mut().unwrap();
    let mut saved = serde_json::to_value(&w.bodies.bodies).unwrap();
    saved["generation"] = (u32::MAX - 3).into();
    w.bodies.bodies = serde_json::from_value(saved).unwrap();
    let one = [StructuralRemoval::Body(a)];
    assert!(
        o.preflight_structure(&one, StructuralInsertions::default(), &mut work())
            .is_ok()
    );
    let all = [StructuralRemoval::Body(a), StructuralRemoval::Body(b)];
    assert_eq!(
        o.preflight_structure(&all, StructuralInsertions::default(), &mut work())
            .map(|_| ()),
        Err(StructuralError::BodyGenerationOrCapacity)
    );
    assert!(o.get().unwrap().bodies.contains(a) && o.get().unwrap().bodies.contains(b));
    o.remove_body_checked(a, &mut work()).unwrap().unwrap();
    assert_eq!(
        o.remove_body_checked(b, &mut work()).map(|_| ()),
        Err(StructuralError::BodyGenerationOrCapacity)
    );
    assert!(o.get().unwrap().bodies.contains(b));
}
#[test]
fn persisted_structure_attached_collider_generation_and_island_sum_are_checked() {
    let (mut o, a, _, _) = two();
    let w = o.access_mut().unwrap();
    let mut saved = serde_json::to_value(&w.colliders.colliders).unwrap();
    saved["generation"] = (u32::MAX - 2).into();
    w.colliders.colliders = serde_json::from_value(saved).unwrap();
    assert_eq!(
        o.preflight_structure(
            &[StructuralRemoval::Body(a)],
            StructuralInsertions::default(),
            &mut work()
        )
        .map(|_| ()),
        Err(StructuralError::ColliderGenerationOrCapacity)
    );
    let (mut o, a, _, _) = two();
    let w = o.access_mut().unwrap();
    let id = w.bodies[a].ids.island_id as usize;
    w.islands.persistent.islands[id].constraint_remove_count = u32::MAX;
    assert_eq!(
        o.preflight_structure(
            &[StructuralRemoval::Body(a)],
            StructuralInsertions::default(),
            &mut work()
        )
        .map(|_| ()),
        Err(StructuralError::IslandCounter)
    );
    assert!(o.get().unwrap().bodies.contains(a));
}
#[test]
fn persisted_structure_permit_exact_order_reuse_and_peak_capacity() {
    let (mut o, a, _, c) = two();
    let plan = [StructuralRemoval::Body(a)];
    assert_eq!(
        o.preflight_structure(
            &plan,
            StructuralInsertions {
                bodies: MAX_ITEMS,
                colliders: 0
            },
            &mut work()
        )
        .map(|_| ()),
        Err(StructuralError::BodyGenerationOrCapacity)
    );
    let admitted = o
        .preflight_structure(
            &plan,
            StructuralInsertions {
                bodies: 1,
                colliders: 1,
            },
            &mut work(),
        )
        .unwrap();
    let mut permit = o.structural_permit(admitted).unwrap();
    assert_eq!(
        permit.insert_body(RigidBodyBuilder::dynamic()).map(|_| ()),
        Err(StructuralError::Plan)
    );
    assert_eq!(
        permit.remove_collider(c).map(|_| ()),
        Err(StructuralError::Plan)
    );
    permit.remove_body(a).unwrap();
    let next = permit.insert_body(RigidBodyBuilder::dynamic()).unwrap();
    assert_ne!(next, a);
    permit
        .insert_collider(ColliderBuilder::ball(0.5), Some(next))
        .unwrap();
    assert_eq!(
        permit.insert_body(RigidBodyBuilder::dynamic()).map(|_| ()),
        Err(StructuralError::Plan)
    );
    permit.finish().unwrap();
    assert!(!o.get().unwrap().bodies.contains(a));
    assert_eq!(o.get().unwrap().bodies.len(), 2);
    assert!(matches!(
        o.completed_step(&mut work()),
        Err(CaptureError::NotCompleted)
    ));
    o.step().unwrap();
    assert!(o.completed_step(&mut work()).is_ok());
}
#[test]
fn persisted_structure_plan_is_owner_bound_stale_on_edit_or_step_and_work_is_prepaid() {
    let (mut a, h, _, _) = two();
    let (mut b, _, _, _) = two();
    let removals = [StructuralRemoval::Body(h)];
    let plan = a
        .preflight_structure(&removals, StructuralInsertions::default(), &mut work())
        .unwrap();
    assert!(matches!(
        b.structural_permit(plan),
        Err(StructuralError::Plan)
    ));
    let plan = a
        .preflight_structure(&removals, StructuralInsertions::default(), &mut work())
        .unwrap();
    let _ = a.access_mut().unwrap();
    assert!(matches!(
        a.structural_permit(plan),
        Err(StructuralError::Plan)
    ));
    a.step().unwrap();
    let plan = a
        .preflight_structure(&removals, StructuralInsertions::default(), &mut work())
        .unwrap();
    a.step().unwrap();
    assert!(matches!(
        a.structural_permit(plan),
        Err(StructuralError::Plan)
    ));
    let mut budget = work();
    let plan = a
        .preflight_structure(&removals, StructuralInsertions::default(), &mut budget)
        .unwrap();
    budget.charge(budget.remaining()).unwrap();
    let mut permit = a.structural_permit(plan).unwrap();
    permit.remove_body(h).unwrap();
    permit.finish().unwrap();
    assert_eq!(budget.remaining(), 0);
    assert!(!a.get().unwrap().bodies.contains(h));
}
#[test]
fn persisted_structure_mutation_revision_and_zero_capacity_refuse_before_change() {
    let (mut o, a, _, _) = two();
    o.mutation_revision = u64::MAX;
    assert!(matches!(
        o.access_mut(),
        Err(CaptureError::MutationExhausted)
    ));
    assert_eq!(o.step(), Err(CaptureError::MutationExhausted));
    assert_eq!(
        o.preflight_structure(
            &[StructuralRemoval::Body(a)],
            StructuralInsertions::default(),
            &mut work()
        )
        .map(|_| ()),
        Err(StructuralError::Owner(CaptureError::MutationExhausted))
    );
    assert!(o.get().unwrap().bodies.contains(a));
    let mut o = CheckpointOwner::new().unwrap();
    let w = o.access_mut().unwrap();
    let mut empty = serde_json::to_value(&w.bodies.bodies).unwrap();
    empty["items"] = serde_json::json!([]);
    empty["free_list_head"] = serde_json::Value::Null;
    w.bodies.bodies = serde_json::from_value(empty).unwrap();
    assert_eq!(
        o.preflight_structure(
            &[],
            StructuralInsertions {
                bodies: 1,
                colliders: 0
            },
            &mut work()
        )
        .map(|_| ()),
        Err(StructuralError::BodyGenerationOrCapacity)
    );
    assert_eq!(o.get().unwrap().bodies.len(), 0);
}

#[test]
fn persisted_structure_sparse_island_capacity_is_precharged_before_iteration() {
    let (mut o, a, _, _) = two();
    let w = o.access_mut().unwrap();
    let id = w.bodies[a].ids.island_id as usize;
    let island = w.islands.persistent.islands[id].clone();
    w.islands.persistent.islands.insert(4095, island);
    w.islands.persistent.islands.remove(4095);
    let capacity = w.islands.persistent.islands.capacity();
    assert!(capacity >= 4096);
    assert!(w.islands.persistent.islands.len() < 8);
    let before = serde_json::to_vec(&w.islands).unwrap();
    let mut short = WorkBudget::new(capacity as u64);
    assert_eq!(
        island_headroom(w, 0, &mut short),
        Err(StructuralError::Work)
    );
    assert_eq!(short.remaining(), capacity as u64);
    let mut exact = WorkBudget::new(capacity as u64 + 1);
    island_headroom(w, 0, &mut exact).unwrap();
    assert_eq!(exact.remaining(), 0);
    assert_eq!(serde_json::to_vec(&w.islands).unwrap(), before);
}

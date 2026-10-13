// Included only within the prepared one-way capture module's dimensional cases.
use crate::pipeline::checked_restore::{WorkBudget, admit_numeric_frame, allocation::*, world};

// The test harness owns construction, default dispatcher, all mutation and
// actual positive steps. This token cannot be made by a caller boolean or frame
// phase label. No production constructor exists for world::CaptureContext.
struct CompletedCapture<'a> {
    world: &'a mut PhysicsWorld,
    bytes: Vec<u8>,
    context: world::CaptureContext,
}
impl<'a> CompletedCapture<'a> {
    fn step(w: &'a mut PhysicsWorld) -> Self {
        assert!(w.integration_parameters.dt > 0.);
        narrow_step(w);
        Self::completed(w)
    }
    // Only called after the test's own narrow_step, with exclusive ownership.
    fn completed(w: &'a mut PhysicsWorld) -> Self {
        assert_eq!(capture_gate(w), Ok(()));
        let inner = world_bytes(w);
        let bytes = world::AssemblyFrame::encode(
            &inner,
            w.impulse_joints.assembly_epoch,
            &mut AllocationBudget::new(MAX_BYTES + 16, 1),
        )
        .unwrap();
        let context = world::CaptureContext {
            #[cfg(all(feature = "parallel", not(feature = "unsync-callbacks")))]
            pool: w.thread_pool(),
        };
        Self {
            world: w,
            bytes,
            context,
        }
    }
    fn restore(&mut self) -> PhysicsWorld {
        let source_before = world_bytes(self.world);
        let assembly = world::AssemblyFrame::decode(&self.bytes).unwrap();
        assert_eq!(source_before, assembly.inner());
        assert_eq!(
            self.world.impulse_joints.assembly_epoch,
            assembly.impulse_epoch()
        );
        let mut result = None;
        world::with_admitted(&assembly,identity(),&mut Budget::new(MAX_ALLOCATION).unwrap(),&mut WorkBudget::new(u64::MAX),|n|{
            let plan=world::plan(n).unwrap();
            for (bytes,calls) in [(plan.bytes()-1,plan.allocations()),(plan.bytes(),plan.allocations()-1)] {
                assert!(matches!(world::construct(n,&self.context,&mut AllocationBudget::new(bytes,calls)),Err(ConstructionError::Limit)));
            }
            let (candidate,actual_bytes,actual_calls)=crate::pipeline::checked_restore::allocation_test_meter::measure(||
                world::construct(n,&self.context,&mut AllocationBudget::new(plan.bytes(),plan.allocations())).unwrap());
            assert!(actual_bytes<=plan.bytes(),"actual={actual_bytes} plan={}",plan.bytes());
            assert_eq!(actual_calls,plan.allocations());
            eprintln!("CHECKED_WORLD_ALLOC dimension={} charged_bytes={} actual_bytes={actual_bytes} calls={actual_calls}",crate::math::DIM,plan.bytes());
            // A deliberately truncated internal permit fails only after earlier
            // candidate owners exist; locals drop, and source is never modified.
            let first=crate::pipeline::checked_restore::body::plan(n.numeric()).unwrap();
            let mut short=AllocationBudget::new(first.bytes(),first.allocations()).admit(first).unwrap();
            assert!(matches!(world::construct_with_permit(n,&self.context,&mut short),Err(ConstructionError::Limit)));
            short.finish().unwrap();
            result=Some(candidate);
            Ok(())
        }).unwrap();
        assert_eq!(source_before, world_bytes(self.world));
        let mut restored = result.unwrap();
        assert_eq!(
            restored.impulse_joints.assembly_epoch,
            assembly.impulse_epoch()
        );
        assert_eq!(source_before, world_bytes(&mut restored));
        assert_eq!(world_cache(self.world), world_cache(&restored));
        assert_eq!(
            self.world
                .physics_pipeline
                .checkpoint_observation()
                .unwrap(),
            restored.physics_pipeline.checkpoint_observation().unwrap()
        );
        #[cfg(all(feature = "parallel", not(feature = "unsync-callbacks")))]
        match (self.world.thread_pool(), restored.thread_pool()) {
            (None, None) => {}
            (Some(a), Some(b)) => assert!(std::sync::Arc::ptr_eq(&a, &b)),
            _ => panic!("changed execution policy"),
        }
        restored
    }
}
#[derive(Debug, PartialEq, Eq)]
enum CaptureRefusal {
    Omitted(world::OmittedOwnerError),
    Observation(crate::pipeline::CheckpointPipelineError),
    Deferred,
    Quarantine,
    NonPositiveStep,
    PendingMutation,
}
fn capture_gate(w: &mut PhysicsWorld) -> core::result::Result<(), CaptureRefusal> {
    w.impulse_joints
        .checkpoint_empty_gate()
        .map_err(CaptureRefusal::Omitted)?;
    w.multibody_joints
        .checkpoint_empty_gate()
        .map_err(CaptureRefusal::Omitted)?;
    w.soft_bodies
        .checkpoint_empty_gate()
        .map_err(CaptureRefusal::Omitted)?;
    let o = w
        .physics_pipeline
        .checkpoint_observation()
        .map_err(CaptureRefusal::Observation)?;
    if o.deferred_bvh_receiver || o.deferred_bvh_inline {
        return Err(CaptureRefusal::Deferred);
    }
    if [
        o.quarantine_bodies,
        o.quarantine_colliders,
        o.quarantine_soft_bodies,
        o.quarantine_body_workspace,
        o.quarantine_collider_workspace,
    ] != [0; 5]
    {
        return Err(CaptureRefusal::Quarantine);
    }
    if !w.integration_parameters.dt.is_finite() || w.integration_parameters.dt <= 0. {
        return Err(CaptureRefusal::NonPositiveStep);
    }
    if !w.bodies.modified_bodies.is_empty()
        || !w.colliders.modified_colliders.is_empty()
        || !w.colliders.removed_colliders.is_empty()
        || w.colliders.iter().any(|(_, c)| !c.changes.is_empty())
    {
        return Err(CaptureRefusal::PendingMutation);
    }
    Ok(())
}
fn whole_equal(a: &mut PhysicsWorld, b: &mut PhysicsWorld) -> bool {
    assert_eq!(
        a.impulse_joints.assembly_epoch,
        b.impulse_joints.assembly_epoch
    );
    for x in [-3., -1., 0., 1., 3.] {
        let ray = Ray::new(world_vector(x, 6.), world_vector(0., -1.));
        let hit = |w: &PhysicsWorld| {
            w.cast_ray(&ray, 20., true, QueryFilter::default())
                .map(|(h, t)| (h.into_raw_parts(), t.to_bits()))
        };
        assert_eq!(hit(a), hit(b));
    }
    let gate_a = capture_gate(a);
    let gate_b = capture_gate(b);
    assert_eq!(gate_a, gate_b, "the exact admission outcome must match");
    let admitted = match gate_a {
        Ok(()) => {
            assert_eq!(
                world_bytes(a),
                world_bytes(b),
                "complete re-captured joined frame"
            );
            true
        }
        Err(CaptureRefusal::PendingMutation) => false,
        other => panic!("unexpected continuation capture refusal: {other:?}"),
    };
    assert_eq!(
        a.islands.checkpoint_sleep_scan(),
        b.islands.checkpoint_sleep_scan()
    );
    assert_eq!(world_cache(a), world_cache(b));
    assert_eq!(
        a.physics_pipeline.checkpoint_observation().unwrap(),
        b.physics_pipeline.checkpoint_observation().unwrap()
    );
    assert_eq!(
        a.physics_pipeline.counters.ccd.num_substeps,
        b.physics_pipeline.counters.ccd.num_substeps
    );
    // Native full-owner serialization is supplementary; it omits CCD and
    // pipeline state, which are compared explicitly above.
    assert_eq!(
        serde_json::to_vec(a).unwrap(),
        serde_json::to_vec(b).unwrap()
    );
    admitted
}

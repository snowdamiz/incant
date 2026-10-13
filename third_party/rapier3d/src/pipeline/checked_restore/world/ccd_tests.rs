#[test]
fn checked_world_retained_ccd_verdict_cache_and_nondefault_settings_continue() {
    let mut w = PhysicsWorld::default();
    w.gravity = world_vector(-0., 0.);
    w.integration_parameters.num_solver_iterations = 2;
    w.integration_parameters.warmstart_coefficient = 0.7;
    w.integration_parameters.contact_softness.damping_ratio = 7.;
    w.integration_parameters
        .static_contact_softness
        .natural_frequency = 45.;
    w.insert_collider(world_cuboid(0.25, 0.25), None);
    let (b, _) = w.insert(
        RigidBodyBuilder::dynamic()
            .translation(world_vector(-20., 10.))
            .linvel(world_vector(60., 0.)),
        world_cuboid(0.1, 0.1),
    );
    let mut restored = CompletedCapture::step(&mut w).restore();
    assert_eq!(
        w.physics_pipeline
            .checkpoint_observation()
            .unwrap()
            .post_solve_ccd_active,
        Some(true)
    );
    let mut omitted = CompletedCapture::completed(&mut w).restore();
    omitted.physics_pipeline = PhysicsPipeline::new();
    for world in [&mut w, &mut restored, &mut omitted] {
        world.remove_body(b).unwrap();
    }
    for _ in 0..4 {
        w.step();
        restored.step();
        omitted.step();
        assert!(whole_equal(&mut w, &mut restored));
        assert_eq!(
            serde_json::to_vec(&w).unwrap(),
            serde_json::to_vec(&omitted).unwrap()
        );
        assert_ne!(world_cache(&w), world_cache(&omitted));
        assert_eq!(w.physics_pipeline.counters.ccd.num_substeps, 1);
        assert_eq!(omitted.physics_pipeline.counters.ccd.num_substeps, 0);
    }
    let mut empty = CompletedCapture::completed(&mut w).restore();
    for tick in 0..8 {
        for world in [&mut w, &mut restored, &mut empty] {
            world.integration_parameters.max_ccd_substeps = if tick % 2 == 0 { 0 } else { 1 };
            world.step();
        }
        assert!(whole_equal(&mut w, &mut restored));
        assert!(whole_equal(&mut w, &mut empty));
    }
    eprintln!(
        "CHECKED_WORLD_CCD dimension={} paired_steps=20 exact_retained=true",
        crate::math::DIM
    );
}
#[test]
fn checked_world_ccd_three_states_empty_ordered_and_full_bvh() {
    let mut observed = std::collections::BTreeSet::new();
    for (fixed, enabled) in [(1, false), (0, true), (3, true), (513, true)] {
        let mut w = PhysicsWorld::default();
        w.gravity = world_vector(0., -0.);
        w.integration_parameters.max_ccd_substeps = usize::from(enabled);
        for i in 0..fixed {
            w.insert_collider(
                world_cuboid(0.1, 0.1).translation(world_vector(i as f32 * 3., -10.)),
                None,
            );
        }
        w.insert(
            RigidBodyBuilder::dynamic()
                .translation(world_vector(-20., 10.))
                .linvel(world_vector(60., 0.)),
            world_cuboid(0.1, 0.1),
        );
        let mut restored = CompletedCapture::step(&mut w).restore();
        let c = world_cache(&w);
        match &c.cache {
            CcdCache::Uninitialized => {
                assert!(!enabled);
                observed.insert(0);
            }
            CcdCache::Targets { targets, .. } => {
                assert!(enabled);
                assert_eq!(targets.len(), fixed);
                observed.insert(if targets.is_empty() { 1 } else { 2 });
            }
            CcdCache::FullBvh { .. } => {
                assert_eq!(fixed, 513);
                observed.insert(3);
            }
        }
        for _ in 0..3 {
            w.step();
            restored.step();
            assert!(whole_equal(&mut w, &mut restored));
        }
    }
    assert_eq!(observed, [0, 1, 2, 3].into_iter().collect());
}
#[test]
fn checked_world_verdict_none_false_true_and_capture_refusals() {
    for expected in [None, Some(false), Some(true)] {
        let mut w = PhysicsWorld::default();
        w.gravity = world_vector(0., 0.);
        w.insert_collider(world_cuboid(0.25, 0.25), None);
        if let Some(fast) = expected {
            w.insert(
                RigidBodyBuilder::dynamic()
                    .translation(world_vector(-20., 10.))
                    .linvel(world_vector(if fast { 60. } else { 0. }, 0.)),
                world_cuboid(0.1, 0.1),
            );
        }
        let mut restored = CompletedCapture::step(&mut w).restore();
        assert_eq!(
            w.physics_pipeline
                .checkpoint_observation()
                .unwrap()
                .post_solve_ccd_active,
            expected
        );
        for _ in 0..3 {
            w.step();
            restored.step();
            assert!(whole_equal(&mut w, &mut restored));
        }
    }
    let (mut w, owners) = broad_three();
    w.step();
    w.step();
    for (i, (b, _)) in owners.iter().enumerate() {
        w.bodies[*b].set_translation(world_vector(i as f32 * 3., 3.), true);
    }
    w.integration_parameters.dt = 0.;
    w.step();
    let before = w.physics_pipeline.checkpoint_observation().unwrap();
    assert!(before.deferred_bvh_receiver ^ before.deferred_bvh_inline);
    assert_eq!(capture_gate(&mut w), Err(CaptureRefusal::Deferred));
    assert_eq!(before, w.physics_pipeline.checkpoint_observation().unwrap());
    w.integration_parameters.dt = 1. / 60.;
    w.step();
    assert_eq!(capture_gate(&mut w), Ok(()));
    w.bodies[owners[0].0].set_linvel(world_vector(f32::NAN, 0.), true);
    w.step();
    let before = w.physics_pipeline.checkpoint_observation().unwrap();
    assert_eq!(before.quarantine_bodies, 1);
    assert_eq!(capture_gate(&mut w), Err(CaptureRefusal::Quarantine));
    assert_eq!(before, w.physics_pipeline.checkpoint_observation().unwrap());
    let mut empty = PhysicsWorld::default();
    empty.integration_parameters.dt = 0.;
    empty.step();
    assert_eq!(
        capture_gate(&mut empty),
        Err(CaptureRefusal::NonPositiveStep)
    );
}

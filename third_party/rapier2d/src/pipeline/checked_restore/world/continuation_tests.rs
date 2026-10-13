#[test]
fn checked_world_all_owners_decoded_events_generations_and_queries_continue() {
    let mut w = PhysicsWorld::default();
    #[cfg(all(feature = "parallel", not(feature = "unsync-callbacks")))]
    w.configure_thread_pool(2).unwrap();
    w.insert_collider(
        ColliderBuilder::ball(100.).translation(world_vector(0., -100.5)),
        None,
    );
    let sensor = w.insert_collider(
        ColliderBuilder::ball(1.25)
            .translation(world_vector(0., 0.5))
            .sensor(true),
        None,
    );
    let mut owners = [0., 1., -1.].map(|x| {
        w.insert(
            RigidBodyBuilder::dynamic().translation(world_vector(x, 0.)),
            ColliderBuilder::ball(0.5),
        )
        .0
    });
    for (_, c) in w.colliders.iter_mut() {
        c.set_active_events(ActiveEvents::COLLISION_EVENTS | ActiveEvents::CONTACT_FORCE_EVENTS);
        c.set_contact_force_event_threshold(0.);
    }
    for _ in 0..64 {
        narrow_step(&mut w);
    }
    let mut followers: Vec<(PhysicsWorld, [RigidBodyHandle; 3])> = vec![];
    let (mut steps, mut collisions, mut forces) = (0, 0, 0);
    let mut refused = std::collections::BTreeSet::new();
    for tick in 0..64 {
        narrow_change(&mut w, tick, &mut owners, sensor);
        let events = narrow_step(&mut w);
        collisions += events.iter().filter(|r| r[0] == 0).count();
        forces += events.iter().filter(|r| r[0] == 1).count();
        for (f, ids) in &mut followers {
            narrow_change(f, tick, ids, sensor);
            assert_eq!(
                narrow_step(f),
                events,
                "exact complete event records at {tick}"
            );
            if !whole_equal(&mut w, f) {
                refused.insert(tick);
                eprintln!(
                    "CHECKED_WORLD_REFUSAL dimension={} tick={tick} both=PendingMutation",
                    crate::math::DIM
                );
            }
            steps += 1;
            assert_eq!(owners, *ids, "subsequent handle generations/order");
        }
        if [0, 16, 32, 48].contains(&tick) {
            let restored = CompletedCapture::completed(&mut w).restore();
            followers.push((restored, owners));
        }
    }
    assert_eq!(steps, 156);
    assert_eq!(
        refused,
        [7].into_iter().collect(),
        "only the pinned native wake journal is unsupported"
    );
    eprintln!(
        "CHECKED_WORLD_REFUSED_TICKS dimension={} ticks={refused:?}",
        crate::math::DIM
    );
    assert!(collisions > 0 && forces > 0);
    eprintln!(
        "CHECKED_WORLD dimension={} frames=4 paired_steps={steps} collisions={collisions} forces={forces} trusted_owner_clones=0",
        crate::math::DIM
    );
}
#[test]
fn checked_world_fresh_workspace_allocation_diagnostic() {
    use crate::pipeline::checked_restore::allocation_test_meter::measure;
    macro_rules! m {
        ($label:literal,$e:expr) => {{
            let (v, b, c) = measure(|| $e);
            eprintln!("WORKSPACE {} bytes={b} calls={c}", $label);
            drop(v);
        }};
    }
    m!("pipeline", PhysicsPipeline::new());
    m!("impulse", ImpulseJointSet::new());
    m!("multibody", MultibodyJointSet::new());
    m!("soft", SoftBodySet::new());
    m!("staged", crate::dynamics::StagedIslandSolver::new());
}

#[cfg(feature = "dim3")]
#[test]
fn checked_world_warm_multipoint_friction_models_rebuild_solver_scratch() {
    use crate::dynamics::FrictionModel;
    for model in [FrictionModel::Simplified, FrictionModel::Coulomb] {
        let mut w = PhysicsWorld::default();
        w.integration_parameters.friction_model = model;
        w.insert_collider(
            world_cuboid(8., 0.5).translation(world_vector(0., -0.5)),
            None,
        );
        let (body, _) = w.insert(
            RigidBodyBuilder::dynamic()
                .translation(world_vector(0., 0.5))
                .can_sleep(false),
            world_cuboid(0.5, 0.5).friction(0.7),
        );
        for (_, c) in w.colliders.iter_mut() {
            c.set_active_events(ActiveEvents::CONTACT_FORCE_EVENTS);
            c.set_contact_force_event_threshold(0.);
        }
        for _ in 0..32 {
            narrow_step(&mut w);
        }
        let warm = w
            .narrow_phase
            .contact_graph()
            .raw_graph()
            .raw_edges()
            .iter()
            .any(|e| {
                let r = e.weight.rigid().unwrap();
                r.manifolds.iter().chain(&r.solver_clusters).any(|m| {
                    m.points.len() >= 2 && m.points.iter().any(|p| p.data.warmstart_impulse != 0.)
                })
            });
        assert!(warm, "actual nonzero multipoint warm start for {model:?}");
        let mut restored = CompletedCapture::completed(&mut w).restore();
        let mut forces = 0;
        for tick in 0..32 {
            for world in [&mut w, &mut restored] {
                if tick == 0 {
                    world.bodies[body].set_linvel(world_vector(0.12, 0.), true);
                    world.bodies[body].set_angvel(world_vector(0., 0.1), true);
                }
            }
            let a = narrow_step(&mut w);
            assert_eq!(a, narrow_step(&mut restored));
            forces += a.len();
            assert!(whole_equal(&mut w, &mut restored));
        }
        assert!(forces > 0);
        eprintln!(
            "CHECKED_WORLD_FRICTION dimension=3 model={model:?} paired_steps=32 warm=true events={forces}"
        );
    }
}

//! Trusted public-step observations, never deserialization into live native state.
macro_rules! native {
    ($module:ident, $rapier:ident, $vector:expr) => {
        mod $module {
            use $rapier::pipeline::CheckpointPipelineObservation;
            use $rapier::prelude::*;
            fn vector(x: f32, y: f32) -> $rapier::math::Vector {
                ($vector)(x, y)
            }
            fn observe(w: &mut PhysicsWorld) -> CheckpointPipelineObservation {
                let before = serde_json::to_vec(w).unwrap();
                let a = w.physics_pipeline.checkpoint_observation().unwrap();
                let b = w.physics_pipeline.checkpoint_observation().unwrap();
                assert_eq!(a, b, "observation must not consume tasks or reports");
                assert_eq!(before, serde_json::to_vec(w).unwrap());
                a
            }
            fn healthy(o: CheckpointPipelineObservation) {
                assert!(!o.deferred_bvh_receiver && !o.deferred_bvh_inline);
                assert_eq!(o.quarantine_bodies, 0);
                assert_eq!(o.quarantine_colliders, 0);
                assert_eq!(o.quarantine_soft_bodies, 0);
                assert_eq!(o.quarantine_body_workspace, 0);
                assert_eq!(o.quarantine_collider_workspace, 0);
            }
            fn world(speed: f32) -> (PhysicsWorld, RigidBodyHandle) {
                let mut w = PhysicsWorld::default();
                w.gravity = vector(0., 0.);
                w.insert_collider(ColliderBuilder::ball(0.25), None);
                let (body, _) = w.insert(
                    RigidBodyBuilder::dynamic()
                        .translation(vector(-20., 10.))
                        .linvel(vector(speed, 0.)),
                    ColliderBuilder::ball(0.1),
                );
                (w, body)
            }
            fn cache(w: &PhysicsWorld) -> Option<(u32, Option<Vec<(u32, u32)>>)> {
                w.ccd_solver.checkpoint_fixed_targets().map(|(p, list)| {
                    (p.to_bits(), list.map(|l| l.iter().map(|(h, _)| h.into_raw_parts()).collect()))
                })
            }
            #[test]
            fn unavailable_false_true_and_disabled_ccd_are_not_collapsed() {
                let (mut w, body) = world(0.);
                let o = observe(&mut w);
                healthy(o);
                assert_eq!(o.post_solve_ccd_active, None);
                w.step();
                let o = observe(&mut w);
                healthy(o);
                assert_eq!(o.post_solve_ccd_active, Some(false));
                w.bodies[body].set_linvel(vector(60., 0.), true);
                w.step();
                let o = observe(&mut w);
                healthy(o);
                assert_eq!(o.post_solve_ccd_active, Some(true));
                assert!(cache(&w).is_some());
                w.integration_parameters.max_ccd_substeps = 0;
                w.bodies[body].set_linvel(vector(0., 0.), true);
                w.step();
                assert_eq!(observe(&mut w).post_solve_ccd_active, Some(false));
            }
            #[test]
            fn retained_true_after_last_body_removal_changes_cache_and_counter() {
                let (mut retained, body) = world(60.);
                let (mut fresh, other_body) = world(60.);
                assert_eq!(body, other_body);
                retained.step();
                fresh.step();
                assert_eq!(observe(&mut retained).post_solve_ccd_active, Some(true));
                assert_eq!(cache(&retained), cache(&fresh));
                assert_eq!(serde_json::to_vec(&retained).unwrap(), serde_json::to_vec(&fresh).unwrap());
                // This is an experimental workspace replacement, not a restore API.
                fresh.physics_pipeline = PhysicsPipeline::new();
                assert_eq!(observe(&mut fresh).post_solve_ccd_active, None);
                retained.remove_body(body).unwrap();
                fresh.remove_body(body).unwrap();
                for step in 0..3 {
                    retained.step();
                    fresh.step();
                    let a = observe(&mut retained);
                    let b = observe(&mut fresh);
                    healthy(a);
                    healthy(b);
                    assert_eq!(a.post_solve_ccd_active, Some(true));
                    assert_eq!(b.post_solve_ccd_active, None);
                    assert_eq!(serde_json::to_vec(&retained).unwrap(), serde_json::to_vec(&fresh).unwrap());
                    assert!(cache(&retained).is_some());
                    assert!(cache(&fresh).is_none());
                    assert_eq!(retained.physics_pipeline.counters.ccd.num_substeps, 1);
                    assert_eq!(fresh.physics_pipeline.counters.ccd.num_substeps, 0);
                    eprintln!("PIPELINE_NATIVE dimension={} removed_last_body_step={step} retained=true fresh=none retained_cache=some fresh_cache=none retained_substeps=1 fresh_substeps=0 serialized_owners_equal=true", stringify!($module));
                }
                let mut handles = vec![];
                for w in [&mut retained, &mut fresh] {
                    handles.push(w.insert(RigidBodyBuilder::dynamic().translation(vector(10., 10.)).linvel(vector(60., 0.)), ColliderBuilder::ball(0.1)).0);
                    w.step();
                    assert_eq!(observe(w).post_solve_ccd_active, Some(true));
                }
                assert_eq!(handles[0], handles[1]);
                assert_eq!(cache(&retained), cache(&fresh));
                assert_eq!(serde_json::to_vec(&retained).unwrap(), serde_json::to_vec(&fresh).unwrap());
            }
            #[test]
            fn retained_false_after_removal_is_distinct_without_cache_difference() {
                let (mut retained, body) = world(0.);
                let (mut fresh, _) = world(0.);
                retained.step();
                fresh.step();
                fresh.physics_pipeline = PhysicsPipeline::new();
                retained.remove_body(body).unwrap();
                fresh.remove_body(body).unwrap();
                retained.step();
                fresh.step();
                assert_eq!(observe(&mut retained).post_solve_ccd_active, Some(false));
                assert_eq!(observe(&mut fresh).post_solve_ccd_active, None);
                assert_eq!(cache(&retained), None);
                assert_eq!(cache(&retained), cache(&fresh));
                assert_eq!(serde_json::to_vec(&retained).unwrap(), serde_json::to_vec(&fresh).unwrap());
            }
            #[test]
            fn zero_dt_retains_deferred_owner_until_an_actual_step_joins() {
                let mut w = PhysicsWorld::default();
                w.gravity = vector(0., 0.);
                let bodies: Vec<_> = (0..3).map(|i| w.insert(RigidBodyBuilder::dynamic().translation(vector(i as f32*0.2, 0.)), ColliderBuilder::ball(0.5).sensor(true)).0).collect();
                w.step();
                w.step();
                for (i, body) in bodies.iter().enumerate() {
                    w.bodies[*body].set_translation(vector(i as f32 * 3., 3.), true);
                }
                w.integration_parameters.dt = 0.;
                w.step();
                let o = observe(&mut w);
                assert!(o.deferred_bvh_receiver ^ o.deferred_bvh_inline);
                eprintln!("PIPELINE_NATIVE dimension={} zero_dt=true receiver={} inline={} observation_did_not_join=true", stringify!($module), o.deferred_bvh_receiver, o.deferred_bvh_inline);
                w.integration_parameters.dt = 1. / 60.;
                w.step();
                healthy(observe(&mut w));
            }
            #[test]
            fn quarantine_reports_survive_observation_and_clear_on_native_step() {
                let (mut w, body) = world(0.);
                w.step();
                w.bodies[body].set_linvel(vector(f32::NAN, 0.), true);
                w.step();
                let o = observe(&mut w);
                assert_eq!(o.quarantine_bodies, 1);
                assert_eq!(w.quarantine().bodies(), &[body]);
                assert_eq!(o.quarantine_body_workspace, 0);
                assert!(!w.bodies[body].is_enabled());
                w.step();
                healthy(observe(&mut w));
                assert!(!w.bodies[body].is_enabled());
            }
        }
    };
}
native!(planar, rapier2d, rapier2d::math::Vector::new);
native!(spatial, rapier3d, |x, y| rapier3d::math::Vector::new(
    x, y, 0.
));

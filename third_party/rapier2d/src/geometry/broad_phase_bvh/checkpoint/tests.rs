//! Actual joined bytes are captured by trusted_manifolds::checked_islands_capture.
//! The shared deterministic native fixture is independently stepped here. Only
//! broad-phase state is decoded/constructed; all other live owners remain trusted clones.
use super::*;
use crate::pipeline::checked_restore::{WorkBudget, admit_numeric_frame};
use crate::pipeline::{CollisionPipeline, PhysicsWorld};
use crate::prelude::*;
use incant_physics_snapshot::{
    Budget, Dimension, Identity, MAX_ALLOCATION, with_decoded_world_boundary,
};
use std::eprintln;
include!("../../../pipeline/checked_restore/island_fixture.rs");
#[cfg(feature = "dim2")]
fn vector(x: f32, y: f32) -> Vector {
    Vector::new(x, y)
}
#[cfg(feature = "dim3")]
fn vector(x: f32, y: f32) -> Vector {
    Vector::new(x, y, 0.)
}
manager_fixture!(vector);
fn identity() -> Identity {
    Identity {
        dimension: if crate::math::DIM == 2 {
            Dimension::Planar
        } else {
            Dimension::Spatial
        },
        build: [92; 32],
    }
}
fn bytes(tick: usize) -> &'static [u8] {
    match tick {
        0 => include_bytes!("../../../pipeline/checked_restore/island_fixtures/0.bin"),
        16 => include_bytes!("../../../pipeline/checked_restore/island_fixtures/16.bin"),
        32 => include_bytes!("../../../pipeline/checked_restore/island_fixtures/32.bin"),
        48 => include_bytes!("../../../pipeline/checked_restore/island_fixtures/48.bin"),
        _ => panic!("uncaptured tick"),
    }
}
fn fresh(w: &mut PhysicsWorld, broad_phase: BroadPhaseBvh) -> PhysicsWorld {
    PhysicsWorld {
        gravity: w.gravity,
        integration_parameters: w.integration_parameters,
        physics_pipeline: w.physics_pipeline.checkpoint_fresh_workspace().unwrap(),
        collision_pipeline: CollisionPipeline::new(),
        islands: w.islands.clone(),
        broad_phase,
        narrow_phase: w.narrow_phase.clone(),
        bodies: w.bodies.clone(),
        colliders: w.colliders.clone(),
        impulse_joints: w.impulse_joints.clone(),
        multibody_joints: w.multibody_joints.clone(),
        soft_bodies: w.soft_bodies.clone(),
        ccd_solver: w.ccd_solver.clone(),
    }
}
#[test]
fn checked_broad_decoded_owner_and_native_mutation_order_continue() {
    let mut followers: Vec<(PhysicsWorld, [RigidBodyHandle; 3])> = Vec::new();
    let mut steps = 0;
    let mut nonzero = false;
    manager_run(|tick, w, owners| {
        for (f, ids) in &mut followers {
            manager_change(f, tick, ids);
            f.step();
            steps += 1;
            assert_eq!(
                bincode::serialize(&w).unwrap(),
                bincode::serialize(&f).unwrap(),
                "tick={tick}"
            );
            assert_eq!(
                w.islands.checkpoint_sleep_scan().unwrap(),
                f.islands.checkpoint_sleep_scan().unwrap()
            );
            assert_eq!(
                w.physics_pipeline.checkpoint_observation().unwrap(),
                f.physics_pipeline.checkpoint_observation().unwrap()
            );
        }
        if [0, 16, 32, 48].contains(&tick) {
            healthy(w);
            with_decoded_world_boundary(
                bytes(tick),
                identity(),
                &mut Budget::new(MAX_ALLOCATION).unwrap(),
                |frame| {
                    let n = crate::pipeline::checked_restore::admit_numeric_frame(
                        frame,
                        identity(),
                        &mut crate::pipeline::checked_restore::WorkBudget::new(u64::MAX),
                    )
                    .unwrap();
                    let p = BroadPhaseBvh::checkpoint_plan(frame.broad(), n.identity()).unwrap();
                    assert!(p.bytes() > 0 && p.allocations() > 0);
                    assert!(
                        AllocationBudget::new(p.bytes() - 1, p.allocations())
                            .admit(p)
                            .is_err()
                    );
                    assert!(
                        AllocationBudget::new(p.bytes(), p.allocations() - 1)
                            .admit(p)
                            .is_err()
                    );
                    let mut permit = AllocationBudget::new(p.bytes(), p.allocations())
                        .admit(p)
                        .unwrap();
                    let restored =
                        BroadPhaseBvh::from_checkpoint(frame.broad(), n.identity(), &mut permit)
                            .unwrap();
                    permit.finish().unwrap();
                    assert_eq!(
                        bincode::serialize(&w.broad_phase).unwrap(),
                        bincode::serialize(&restored).unwrap()
                    );
                    nonzero |= !restored.pending_set_aabb.is_empty();
                    assert!(!restored.deferred_optimize_pending);
                    assert!(
                        restored.updated_colliders.is_empty()
                            && restored.stale_pairs.is_empty()
                            && restored.curr_updated_leaves.is_empty()
                            && restored.updated_mask.is_empty()
                    );
                    followers.push((fresh(w, restored), owners));
                    Ok(())
                },
            )
            .unwrap();
        }
    });
    assert_eq!(steps, 156);
    assert!(nonzero);
    eprintln!(
        "CHECKED_BROAD dimension={} decoded_frames=4 actual_paired_steps=156 exact_tree_pairs_adjacency_queue=true fresh_scratch=true other_owners=trusted_clones",
        crate::math::DIM
    );
}

fn healthy(w: &mut PhysicsWorld) {
    assert!(w.integration_parameters.dt > 0. && !w.broad_phase.deferred_optimize_pending);
    let p = w.physics_pipeline.checkpoint_observation().unwrap();
    assert!(!p.deferred_bvh_receiver && !p.deferred_bvh_inline);
    assert_eq!(
        [
            p.quarantine_bodies,
            p.quarantine_colliders,
            p.quarantine_soft_bodies,
            p.quarantine_body_workspace,
            p.quarantine_collider_workspace
        ],
        [0; 5]
    );
    // The trusted harness itself just completed a step; zero counters alone
    // are not the completed-phase authentication of a public saved frame.
}
#[test]
fn checked_broad_owner_identity_queue_generations_and_late_budget_refuse() {
    let raw = bytes(0);
    assert!(
        with_decoded_world_boundary(
            &raw[..raw.len() - 1],
            identity(),
            &mut Budget::new(MAX_ALLOCATION).unwrap(),
            |_| Ok(())
        )
        .is_err()
    );
    with_decoded_world_boundary(
        raw,
        identity(),
        &mut Budget::new(MAX_ALLOCATION).unwrap(),
        |frame| {
            let w = frame.broad();
            let mut wrong = identity();
            wrong.build[0] ^= 1;
            assert!(BroadPhaseBvh::checkpoint_plan(w, wrong).is_err());
            wrong = identity();
            wrong.dimension = if wrong.dimension == Dimension::Planar {
                Dimension::Spatial
            } else {
                Dimension::Planar
            };
            assert!(BroadPhaseBvh::checkpoint_plan(w, wrong).is_err());
            let plan = Bvh::checkpoint_pending_plan(w.context().tree, identity()).unwrap();
            let mut permit = AllocationBudget::new(plan.bytes(), plan.allocations())
                .admit(plan)
                .unwrap();
            assert!(matches!(
                BroadPhaseBvh::from_checkpoint(w, identity(), &mut permit),
                Err(ConstructionError::Limit)
            ));
            permit.finish().unwrap();
            let mut bad = w.record().clone();
            if bad.pending_aabbs.is_empty() {
                bad.pending_aabbs.push(bad.pairs[0].colliders[0]);
            } else {
                bad.pending_aabbs.clear();
            }
            assert!(
                incant_physics_snapshot::validate_broad_owner(
                    bad,
                    w.context(),
                    &mut Budget::new(MAX_ALLOCATION).unwrap()
                )
                .is_err()
            );
            let mut bad = w.record().clone();
            assert!(!bad.neighbors.is_empty());
            bad.neighbors[0].generation ^= 1;
            assert!(
                incant_physics_snapshot::validate_broad_owner(
                    bad,
                    w.context(),
                    &mut Budget::new(MAX_ALLOCATION).unwrap()
                )
                .is_err()
            );
            Ok(())
        },
    )
    .unwrap();
}
#[derive(Debug, PartialEq, Eq)]
struct EventRecord {
    started: bool,
    a: (u32, u32),
    b: (u32, u32),
    flags: u32,
}
#[derive(Default)]
struct Events(std::sync::Mutex<Vec<EventRecord>>);
impl EventHandler for Events {
    fn handle_collision_event(
        &self,
        _: &RigidBodySet,
        _: &ColliderSet,
        e: CollisionEvent,
        _: Option<&ContactPair>,
    ) {
        let (started, a, b, flags) = match e {
            CollisionEvent::Started(a, b, f) => (true, a, b, f),
            CollisionEvent::Stopped(a, b, f) => (false, a, b, f),
        };
        self.0.lock().unwrap().push(EventRecord {
            started,
            a: a.into_raw_parts(),
            b: b.into_raw_parts(),
            flags: flags.bits(),
        });
    }
    fn handle_contact_force_event(
        &self,
        _: Real,
        _: &RigidBodySet,
        _: &ColliderSet,
        _: &ContactPair,
        _: Real,
    ) {
    }
    fn handle_soft_body_tear_event(&self, _: &SoftBodySet, _: &SoftBodyTearEvent) {}
}
#[test]
fn checked_broad_native_policies_wrap_and_collision_events_continue() {
    let mut paired = 0;
    let mut events = 0;
    manager_run(|tick, w, owners| {
        if tick != 0 {
            return;
        }
        healthy(w);
        with_decoded_world_boundary(
            bytes(0),
            identity(),
            &mut Budget::new(MAX_ALLOCATION).unwrap(),
            |frame| {
                for adaptive in [false, true] {
                    for optimize in [false, true] {
                        let mut r = frame.broad().record().clone();
                        r.adaptive_margin = adaptive;
                        r.frame = u32::MAX;
                        r.optimization = if optimize {
                            BroadOptimization::Subtree
                        } else {
                            BroadOptimization::None
                        };
                        let v = incant_physics_snapshot::validate_broad_owner(
                            r,
                            frame.broad().context(),
                            &mut Budget::new(MAX_ALLOCATION).unwrap(),
                        )
                        .unwrap();
                        let joined =
                            incant_physics_snapshot::join_world_boundary(&v, frame.pipeline())
                                .unwrap();
                        let n =
                            admit_numeric_frame(joined, identity(), &mut WorkBudget::new(u64::MAX))
                                .unwrap();
                        let p = BroadPhaseBvh::checkpoint_plan(&v, n.identity()).unwrap();
                        let mut permit = AllocationBudget::new(p.bytes(), p.allocations())
                            .admit(p)
                            .unwrap();
                        let restored =
                            BroadPhaseBvh::from_checkpoint(&v, n.identity(), &mut permit).unwrap();
                        permit.finish().unwrap();
                        let original_broad = w.broad_phase.clone();
                        let mut original = fresh(w, original_broad);
                        original.broad_phase.adaptive_change_detection_margin = adaptive;
                        original.broad_phase.frame_index = u32::MAX;
                        original.broad_phase.optimization_strategy = if optimize {
                            BvhOptimizationStrategy::SubtreeOptimizer
                        } else {
                            BvhOptimizationStrategy::None
                        };
                        let mut follower = fresh(w, restored);
                        let (mut a, mut b) = (owners, owners);
                        assert_eq!(
                            bincode::serialize(&original).unwrap(),
                            bincode::serialize(&follower).unwrap()
                        );
                        // These are ordinary subsequent native mutations, applied equally.
                        for world in [&mut original, &mut follower] {
                            for (_, c) in world.colliders.iter_mut() {
                                c.set_active_events(ActiveEvents::COLLISION_EVENTS);
                            }
                        }
                        for t in 1..=16 {
                            manager_change(&mut original, t, &mut a);
                            manager_change(&mut follower, t, &mut b);
                            let ae = Events::default();
                            let be = Events::default();
                            original.step_with_events(&(), &ae);
                            follower.step_with_events(&(), &be);
                            paired += 1;
                            let av = ae.0.into_inner().unwrap();
                            let bv = be.0.into_inner().unwrap();
                            assert_eq!(av, bv);
                            events += av.len();
                            assert_eq!(
                                bincode::serialize(&original).unwrap(),
                                bincode::serialize(&follower).unwrap()
                            );
                            assert_eq!(
                                original.physics_pipeline.checkpoint_observation().unwrap(),
                                follower.physics_pipeline.checkpoint_observation().unwrap()
                            );
                        }
                    }
                }
                Ok(())
            },
        )
        .unwrap();
    });
    assert_eq!(paired, 64);
    assert!(events > 0);
    eprintln!(
        "CHECKED_BROAD_POLICY dimension={} actual_paired_steps={} collision_events={} strategies=2 adaptive=2 wrapped_frame=true",
        crate::math::DIM,
        paired,
        events
    );
}

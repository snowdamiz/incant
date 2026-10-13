//! Actual joined bytes are captured by trusted_manifolds::checked_islands_capture.
//! The shared deterministic native fixture is independently stepped here. Only
//! narrow-phase state is decoded/constructed; all other live owners remain trusted clones.
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
fn fresh(w: &mut PhysicsWorld, narrow_phase: NarrowPhase) -> PhysicsWorld {
    PhysicsWorld {
        gravity: w.gravity,
        integration_parameters: w.integration_parameters,
        physics_pipeline: w.physics_pipeline.checkpoint_fresh_workspace().unwrap(),
        collision_pipeline: CollisionPipeline::new(),
        islands: w.islands.clone(),
        broad_phase: w.broad_phase.clone(),
        narrow_phase,
        bodies: w.bodies.clone(),
        colliders: w.colliders.clone(),
        impulse_joints: w.impulse_joints.clone(),
        multibody_joints: w.multibody_joints.clone(),
        soft_bodies: w.soft_bodies.clone(),
        ccd_solver: w.ccd_solver.clone(),
    }
}

fn construct(frame: incant_physics_snapshot::WorldBoundary<'_>) -> NarrowPhase {
    let n = admit_numeric_frame(frame, identity(), &mut WorkBudget::new(u64::MAX)).unwrap();
    let plan = NarrowPhase::checkpoint_plan(&n).unwrap();
    assert!(
        AllocationBudget::new(plan.bytes() - 1, plan.allocations())
            .admit(plan)
            .is_err()
    );
    assert!(
        AllocationBudget::new(plan.bytes(), plan.allocations() - 1)
            .admit(plan)
            .is_err()
    );
    let mut p = AllocationBudget::new(plan.bytes(), plan.allocations())
        .admit(plan)
        .unwrap();
    let (out, actual_bytes, calls) =
        crate::pipeline::checked_restore::allocation_test_meter::measure(|| {
            NarrowPhase::from_checkpoint(&n, &mut p).unwrap()
        });
    assert!(actual_bytes <= plan.bytes());
    assert_eq!(calls, plan.allocations());
    eprintln!(
        "CHECKED_NARROW_ALLOC dimension={} charged_bytes={} actual_bytes={actual_bytes} calls={calls}",
        crate::math::DIM,
        plan.bytes()
    );
    p.finish().unwrap();
    out
}
fn healthy(w: &mut PhysicsWorld) {
    let p = w.physics_pipeline.checkpoint_observation().unwrap();
    assert!(w.integration_parameters.dt > 0. && !p.deferred_bvh_inline && !p.deferred_bvh_receiver);
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
}
#[test]
fn checked_narrow_decoded_warm_history_graphs_and_native_order_continue() {
    let mut followers: Vec<(PhysicsWorld, [RigidBodyHandle; 3])> = Vec::new();
    let mut steps = 0;
    let mut warm = false;
    let mut retired = false;
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
                w.physics_pipeline.checkpoint_observation().unwrap(),
                f.physics_pipeline.checkpoint_observation().unwrap()
            );
        }
        retired |= !w.narrow_phase.retired_pairs.is_empty();
        if [0, 16, 32, 48].contains(&tick) {
            healthy(w);
            with_decoded_world_boundary(
                bytes(tick),
                identity(),
                &mut Budget::new(MAX_ALLOCATION).unwrap(),
                |frame| {
                    warm |= frame
                        .manifolds()
                        .record()
                        .points
                        .iter()
                        .any(|p| f32::from_bits(p.data.warmstart_impulse) != 0.);
                    let out = construct(frame);
                    assert_eq!(
                        bincode::serialize(&w.narrow_phase).unwrap(),
                        bincode::serialize(&out).unwrap()
                    );
                    assert!(
                        out.retired_pairs.is_empty()
                            && out.update_candidates.is_empty()
                            && out.body_qualify_info.is_empty()
                            && out.awake_body_mask.is_empty()
                            && out.solver_graph_dirty.is_empty()
                            && out.solver_color_todo.is_empty()
                    );
                    followers.push((fresh(w, out), owners));
                    Ok(())
                },
            )
            .unwrap();
        }
    });
    assert!(warm && retired);
    assert_eq!(steps, 156);
    eprintln!(
        "CHECKED_NARROW dimension={} decoded_frames=4 actual_paired_steps=156 nonzero_warmstart=true populated_retired_pool=true other_owners=trusted_clones",
        crate::math::DIM
    );
}
#[test]
fn checked_narrow_late_permit_and_wrong_identity_refuse() {
    with_decoded_world_boundary(
        bytes(0),
        identity(),
        &mut Budget::new(MAX_ALLOCATION).unwrap(),
        |frame| {
            let mut wrong = identity();
            wrong.build[0] ^= 1;
            assert!(admit_numeric_frame(frame, wrong, &mut WorkBudget::new(u64::MAX)).is_err());
            let n = admit_numeric_frame(frame, identity(), &mut WorkBudget::new(u64::MAX)).unwrap();
            let plan = Graph::<ColliderHandle, ContactPair>::checkpoint_plan(frame.contact_graph())
                .unwrap();
            let mut p = AllocationBudget::new(plan.bytes(), plan.allocations())
                .admit(plan)
                .unwrap();
            assert!(matches!(
                NarrowPhase::from_checkpoint(&n, &mut p),
                Err(ConstructionError::Limit)
            ));
            p.finish().unwrap();
            Ok(())
        },
    )
    .unwrap();
}
include!("../../../pipeline/checked_restore/narrow_fixture.rs");
narrow_fixture!(vector, crate);
fn event_bytes(tick: usize) -> &'static [u8] {
    match tick {
        0 => include_bytes!("../../../pipeline/checked_restore/narrow_fixtures/0.bin"),
        16 => include_bytes!("../../../pipeline/checked_restore/narrow_fixtures/16.bin"),
        32 => include_bytes!("../../../pipeline/checked_restore/narrow_fixtures/32.bin"),
        48 => include_bytes!("../../../pipeline/checked_restore/narrow_fixtures/48.bin"),
        _ => panic!("uncaptured"),
    }
}
#[test]
fn checked_narrow_collision_sensor_force_history_continues_exactly() {
    let mut followers: Vec<(PhysicsWorld, [RigidBodyHandle; 3], ColliderHandle)> = vec![];
    let (mut collisions, mut forces, mut starts, mut ongoing, mut steps) = (0, 0, 0, 0, 0);
    let (mut event_bits, mut force_list, mut sensor_history, mut retired_capture, mut omitted_diff) =
        (false, false, false, false, false);
    narrow_run(|tick, w, owners, sensor, events| {
        collisions += events.iter().filter(|r| r[0] == 0).count();
        forces += events.iter().filter(|r| r[0] == 1).count();
        starts += events.iter().filter(|r| r[0] == 1 && r[1] == 1).count();
        ongoing += events.iter().filter(|r| r[0] == 1 && r[1] == 0).count();
        for (f, ids, s) in &mut followers {
            narrow_change(f, tick, ids, *s);
            assert_eq!(narrow_step(f), events, "complete event records tick={tick}");
            assert_eq!(
                bincode::serialize(&f).unwrap(),
                bincode::serialize(&w).unwrap(),
                "all serialized native owners tick={tick}"
            );
            assert_eq!(
                f.physics_pipeline.checkpoint_observation().unwrap(),
                w.physics_pipeline.checkpoint_observation().unwrap()
            );
            steps += 1;
        }
        if [0, 16, 32, 48].contains(&tick) {
            healthy(w);
            with_decoded_world_boundary(
                event_bytes(tick),
                identity(),
                &mut Budget::new(MAX_ALLOCATION).unwrap(),
                |frame| {
                    let out = construct(frame);
                    assert_eq!(
                        bincode::serialize(&out).unwrap(),
                        bincode::serialize(&w.narrow_phase).unwrap()
                    );
                    event_bits |= out
                        .contact_graph
                        .interactions()
                        .any(|p| !p.event_status.is_empty());
                    force_list |= !out.force_event_pairs.is_empty();
                    sensor_history |= out
                        .intersection_graph
                        .interactions()
                        .any(|p| p.start_event_emitted && p.intersecting);
                    retired_capture |= !w.narrow_phase.retired_pairs.is_empty();
                    if !omitted_diff {
                        let mut bad = out.clone();
                        for edge in 0..bad.contact_graph.graph.raw_edges().len() {
                            bad.contact_graph
                                .graph
                                .edge_weight_mut(crate::data::graph::EdgeIndex::new(
                                    edge.try_into().unwrap(),
                                ))
                                .unwrap()
                                .event_status
                                .remove(PairEventStatus::INITIAL_FORCE_THRESHOLD_EVENT_EMITTED);
                        }
                        let (a, b) = (NarrowEvents::default(), NarrowEvents::default());
                        let mut good = out.clone();
                        good.emit_contact_force_events(
                            w.integration_parameters.dt,
                            &w.bodies,
                            &w.colliders,
                            &a,
                        );
                        bad.emit_contact_force_events(
                            w.integration_parameters.dt,
                            &w.bodies,
                            &w.colliders,
                            &b,
                        );
                        let mut a = a.0.into_inner().unwrap();
                        let mut b = b.0.into_inner().unwrap();
                        a.sort();
                        b.sort();
                        omitted_diff = a != b;
                    }
                    followers.push((fresh(w, out), owners, sensor));
                    Ok(())
                },
            )
            .unwrap();
        }
    });
    assert!(collisions > 0 && forces > 0 && starts > 0 && ongoing > 0);
    assert!(event_bits && force_list && sensor_history && omitted_diff);
    assert_eq!(steps, 156);
    eprintln!(
        "CHECKED_NARROW_EVENTS dimension={} frames=4 paired_steps=156 collision_records={collisions} force_records={forces} force_started={starts} force_ongoing={ongoing} retired_at_capture={retired_capture} omitted_force_status=native_postsolve_events_differ sorted_multiset=true all_flags_handles_generations_scalar_vector_bits=true",
        crate::math::DIM
    );
}

#[cfg(feature = "dim3")]
#[test]
fn checked_narrow_previous_clusters_are_overwritten_before_reuse_in_native_compound_control() {
    let mut w = PhysicsWorld::default();
    w.integration_parameters.contact_clustering = true;
    w.integration_parameters.contact_recycling = false;
    w.insert_collider(
        ColliderBuilder::compound(
            [0.4, 1.6]
                .map(|x| {
                    let mut p = Pose::IDENTITY;
                    p.translation = vector(x, 0.5);
                    (p, SharedShape::cuboid(0.6, 0.5, 0.5))
                })
                .to_vec(),
        )
        .friction(0.9),
        None,
    );
    let (body, _) = w.insert(
        RigidBodyBuilder::dynamic()
            .translation(vector(1., 1.49))
            .linvel(vector(0.05, 0.)),
        ColliderBuilder::cuboid(0.95, 0.5, 0.5).friction(0.9),
    );
    for _ in 0..8 {
        w.step();
    }
    assert!(
        w.narrow_phase.contact_graph.interactions().any(|p| !p
            .rigid()
            .unwrap()
            .solver_clusters_prev
            .is_empty())
    );
    assert!(
        w.narrow_phase
            .contact_graph
            .interactions()
            .flat_map(|p| &p.rigid().unwrap().solver_clusters)
            .flat_map(|m| &m.points)
            .any(|p| p.data.warmstart_impulse != 0.)
    );
    let mut narrow = w.narrow_phase.clone();
    for edge in 0..narrow.contact_graph.graph.raw_edges().len() {
        let p = narrow
            .contact_graph
            .graph
            .edge_weight_mut(crate::data::graph::EdgeIndex::new(edge.try_into().unwrap()))
            .unwrap();
        p.rigid_mut().unwrap().solver_clusters_prev.clear();
    }
    let mut f = fresh(&mut w, narrow);
    for tick in 0..32 {
        if tick == 8 {
            w.bodies[body].set_translation(vector(3., 1.49), true);
            f.bodies[body].set_translation(vector(3., 1.49), true);
        }
        if tick == 16 {
            w.bodies[body].set_translation(vector(1., 1.49), true);
            f.bodies[body].set_translation(vector(1., 1.49), true);
        }
        assert_eq!(narrow_step(&mut w), narrow_step(&mut f));
        assert_eq!(
            bincode::serialize(&w).unwrap(),
            bincode::serialize(&f).unwrap(),
            "compound scratch control tick={tick}"
        );
    }
    eprintln!(
        "CHECKED_NARROW_CLUSTER_SCRATCH dimension=3 populated_previous=true nonzero_current_warmstart=true paired_steps=32 constructor_admission=false compound_owners=trusted_native_clones"
    );
}

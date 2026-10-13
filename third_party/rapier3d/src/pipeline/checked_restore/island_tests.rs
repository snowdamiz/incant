//! Actual joined bytes are captured by trusted_manifolds::checked_islands_capture.
//! The shared deterministic native fixture is independently stepped here. Only
//! islands are decoded/constructed; all other live owners remain trusted clones.
use super::*;
use crate::alloc_prelude::*;
use crate::pipeline::{CollisionPipeline, PhysicsWorld};
use crate::prelude::*;
use incant_physics_snapshot::{
    Budget, Dimension, Identity, MAX_ALLOCATION, with_decoded_world_boundary,
};
use std::eprintln;
include!("island_fixture.rs");
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
        0 => include_bytes!("island_fixtures/0.bin"),
        16 => include_bytes!("island_fixtures/16.bin"),
        32 => include_bytes!("island_fixtures/32.bin"),
        48 => include_bytes!("island_fixtures/48.bin"),
        _ => panic!("uncaptured tick"),
    }
}
fn fresh(w: &mut PhysicsWorld, islands: IslandManager) -> PhysicsWorld {
    PhysicsWorld {
        gravity: w.gravity,
        integration_parameters: w.integration_parameters,
        physics_pipeline: w.physics_pipeline.checkpoint_fresh_workspace().unwrap(),
        collision_pipeline: CollisionPipeline::new(),
        islands,
        broad_phase: w.broad_phase.clone(),
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
fn checked_islands_decoded_owner_and_native_mutation_order_continue() {
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
                    let p = plan(&n).unwrap();
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
                    let restored = construct(&n, &mut permit).unwrap();
                    permit.finish().unwrap();
                    assert_eq!(
                        bincode::serialize(&w.islands).unwrap(),
                        bincode::serialize(&restored).unwrap()
                    );
                    let scan = w.islands.checkpoint_sleep_scan().unwrap();
                    nonzero |= scan.slots.iter().any(|v| v.0 != 0);
                    assert_eq!(scan, restored.checkpoint_sleep_scan().unwrap());
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
        "CHECKED_ISLANDS dimension={} decoded_frames=4 actual_paired_steps=156 exact_slots_and_order=true retained_scan=true other_owners=trusted_clones",
        crate::math::DIM
    );
}
#[test]
fn checked_islands_wrong_identity_short_permit_and_truncation_refuse() {
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
            let n = crate::pipeline::checked_restore::admit_numeric_frame(
                frame,
                identity(),
                &mut crate::pipeline::checked_restore::WorkBudget::new(u64::MAX),
            )
            .unwrap();
            let owner = frame.broad().context().narrow.context().rigid;
            let mut wrong = identity();
            wrong.build[0] ^= 1;
            assert!(IslandManager::checkpoint_plan(owner, wrong).is_err());
            wrong = identity();
            wrong.dimension = if wrong.dimension == Dimension::Planar {
                Dimension::Spatial
            } else {
                Dimension::Planar
            };
            assert!(IslandManager::checkpoint_plan(owner, wrong).is_err());
            let mut p = AllocationBudget::new(0, 0)
                .admit(AllocationPlan::default())
                .unwrap();
            assert!(matches!(
                construct(&n, &mut p),
                Err(ConstructionError::Limit)
            ));
            // Retained sleep history is observable even though native serde omits it.
            let plan = plan(&n).unwrap();
            let mut p = AllocationBudget::new(plan.bytes(), plan.allocations())
                .admit(plan)
                .unwrap();
            let mut island = construct(&n, &mut p).unwrap();
            p.finish().unwrap();
            let scan = island.checkpoint_sleep_scan().unwrap();
            assert!(scan.slots.iter().any(|s| s.0 != 0));
            let mut omitted = scan.clone();
            omitted.slots.fill((0, false));
            island.restore_sleep_scan(omitted).unwrap();
            assert_ne!(scan, island.checkpoint_sleep_scan().unwrap());
            Ok(())
        },
    )
    .unwrap();
}

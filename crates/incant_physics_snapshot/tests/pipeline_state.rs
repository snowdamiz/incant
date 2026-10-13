//! Inert boundary joins and hostile frames; no input becomes a live solver.
use incant_physics_snapshot::*;
#[path = "support/body_capture.rs"]
mod body_capture;
#[path = "support/ccd_capture.rs"]
mod ccd_capture;
#[path = "support/collider_capture.rs"]
mod collider_capture;
#[path = "support/integration_fixture.rs"]
mod fixture;
#[path = "support/ordered_capture.rs"]
mod ordered_capture;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
macro_rules! native {
    ($module:ident, $rapier:ident, $dimension:expr, $vector:expr, $bits:expr) => {
        mod $module {
            use super::*;
            use $rapier::parry::bounding_volume::BoundingVolume;
            use $rapier::prelude::*;
            fn vector(x: f32, y: f32) -> $rapier::math::Vector {
                ($vector)(x, y)
            }
            fn bits(v: $rapier::math::Vector) -> [u32; 3] {
                ($bits)(v)
            }
            ccd_capture::ccd_observer!($dimension);
            fn with_pipeline(
                w: &mut PhysicsWorld,
                phase: PipelinePhase,
                f: impl FnOnce(PipelineContext<'_>, PipelineState),
            ) {
                let o = w.physics_pipeline.checkpoint_observation().unwrap();
                let record = PipelineState {
                    identity: fixture::identity($dimension),
                    gravity: bits(w.gravity),
                    post_solve_ccd_active: o.post_solve_ccd_active,
                    phase,
                    deferred_bvh_receiver: o.deferred_bvh_receiver,
                    deferred_bvh_inline: o.deferred_bvh_inline,
                    quarantine: [
                        o.quarantine_bodies as u64,
                        o.quarantine_colliders as u64,
                        o.quarantine_soft_bodies as u64,
                        o.quarantine_body_workspace as u64,
                        o.quarantine_collider_workspace as u64,
                    ],
                };
                observe(w, |c, r| {
                    let ccd = validate_ccd_state(r, c).unwrap();
                    f(
                        PipelineContext {
                            integration: c.integration,
                            ccd: &ccd,
                        },
                        record,
                    );
                });
            }
            fn check(c: PipelineContext<'_>, r: PipelineState) -> Vec<u8> {
                let v = validate_pipeline_state(r, c).unwrap();
                assert!(std::ptr::eq(v.context().ccd, c.ccd));
                assert!(std::ptr::eq(v.context().integration, c.integration));
                let b = encode_pipeline_state(&v, &mut budget()).unwrap();
                assert_eq!(b.len(), 112);
                let d = decode_pipeline_state(&b, c).unwrap();
                assert_eq!(d.record(), &r);
                assert_eq!(encode_pipeline_state(&d, &mut budget()).unwrap(), b);
                b
            }
            #[test]
            fn actual_positive_steps_preserve_all_verdicts_and_gravity_bits() {
                let mut w = PhysicsWorld::default();
                w.gravity = vector(-0., 0.);
                w.step();
                assert_eq!(roundtrip(&w).cache, CcdCache::Uninitialized);
                with_pipeline(&mut w, PipelinePhase::CompletedPositive, |c, r| {
                    assert_eq!(r.gravity[0], (-0f32).to_bits());
                    assert_eq!(r.post_solve_ccd_active, None);
                    check(c, r);
                });
                let (body, _) = w.insert(
                    RigidBodyBuilder::dynamic().translation(vector(0., 10.)),
                    ColliderBuilder::ball(0.1),
                );
                w.step();
                with_pipeline(&mut w, PipelinePhase::CompletedPositive, |c, r| {
                    assert_eq!(r.post_solve_ccd_active, Some(false));
                    check(c, r);
                });
                w.bodies[body].set_linvel(vector(60., 0.), true);
                w.step();
                with_pipeline(&mut w, PipelinePhase::CompletedPositive, |c, r| {
                    assert_eq!(r.post_solve_ccd_active, Some(true));
                    check(c, r);
                });
                w.remove_body(body).unwrap();
                w.step();
                with_pipeline(&mut w, PipelinePhase::CompletedPositive, |c, r| {
                    assert_eq!(r.post_solve_ccd_active, Some(true));
                    check(c, r);
                });
            }
            #[test]
            fn actual_quarantine_and_deferred_observations_are_refused() {
                let mut w = PhysicsWorld::default();
                w.gravity = vector(0., 0.);
                let bodies: Vec<_> = (0..3)
                    .map(|i| {
                        w.insert(
                            RigidBodyBuilder::dynamic().translation(vector(i as f32 * 0.2, 0.)),
                            ColliderBuilder::ball(0.5).sensor(true),
                        )
                        .0
                    })
                    .collect();
                w.step();
                w.step();
                for (i, b) in bodies.iter().enumerate() {
                    w.bodies[*b].set_translation(vector(i as f32 * 3., 3.), true);
                }
                w.integration_parameters.dt = 0.;
                w.step();
                with_pipeline(&mut w, PipelinePhase::CollisionOnly, |c, r| {
                    assert!(r.deferred_bvh_receiver ^ r.deferred_bvh_inline);
                    assert_eq!(
                        validate_pipeline_state(r, c).err(),
                        Some(Error::PipelinePhase)
                    );
                });
                // Changing a config cannot retrospectively complete a step. A forged
                // positive phase is still rejected by the actual deferred owner.
                w.integration_parameters.dt = 1. / 60.;
                with_pipeline(&mut w, PipelinePhase::CompletedPositive, |c, r| {
                    assert_eq!(
                        validate_pipeline_state(r, c).err(),
                        Some(Error::PipelineDeferred)
                    )
                });
                w.step();
                with_pipeline(&mut w, PipelinePhase::CompletedPositive, |c, r| {
                    check(c, r);
                });
                w.bodies[bodies[0]].set_linvel(vector(f32::NAN, 0.), true);
                w.step();
                let q = w.physics_pipeline.checkpoint_observation().unwrap();
                assert_eq!(q.quarantine_bodies, 1);
                let bv = ordered_capture::capture(&w.bodies);
                let cv = ordered_capture::capture(&w.colliders);
                let journals = PendingJournals {
                    modified_bodies: bv["modified_bodies"][0].as_array().unwrap().len() as u32,
                    modified_colliders: cv["modified_colliders"][0].as_array().unwrap().len()
                        as u32,
                    removed_colliders: cv["removed_colliders"].as_array().unwrap().len() as u32,
                };
                assert_ne!(journals, PendingJournals::default());
                // The actual quarantined world also has pending body changes, so
                // it cannot provide an admitted CCD context. Preserve that refusal.
                // Exercise scalar quarantine rejection using a separate healthy
                // context; this deliberately is not a joined native-owner claim.
                let mut healthy = PhysicsWorld::default();
                healthy.step();
                with_pipeline(
                    &mut healthy,
                    PipelinePhase::CompletedPositive,
                    |c, mut r| {
                        let colliders = c.ccd.context().colliders;
                        assert_eq!(
                            validate_collider_state(
                                colliders.record().clone(),
                                ColliderContext {
                                    journals,
                                    ..colliders.context()
                                }
                            )
                            .err(),
                            Some(Error::PendingMutation)
                        );
                        r.quarantine = [
                            q.quarantine_bodies as u64,
                            q.quarantine_colliders as u64,
                            q.quarantine_soft_bodies as u64,
                            q.quarantine_body_workspace as u64,
                            q.quarantine_collider_workspace as u64,
                        ];
                        assert_eq!(
                            validate_pipeline_state(r, c).err(),
                            Some(Error::PipelineQuarantine)
                        );
                    },
                );
            }
            #[test]
            fn hostile_fixed_frame_and_aggregate_budget_refuse_before_allocation() {
                let mut w = PhysicsWorld::default();
                w.step();
                with_pipeline(&mut w, PipelinePhase::CompletedPositive, |c, r| {
                    let bytes = check(c, r);
                    for n in 0..bytes.len() {
                        assert!(decode_pipeline_state(&bytes[..n], c).is_err());
                    }
                    let mut extra = bytes.clone();
                    extra.push(0);
                    assert!(matches!(
                        decode_pipeline_state(&extra, c),
                        Err(Error::Length)
                    ));
                    for offset in [0, 8, 10, 11, 12, 43] {
                        let mut b = bytes.clone();
                        b[offset] ^= 255;
                        assert!(decode_pipeline_state(&b, c).is_err());
                    }
                    for (offset, value) in [(56, 3u32), (56, u32::MAX), (60, 3), (64, 4), (68, 1)] {
                        let mut b = bytes.clone();
                        b[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
                        assert!(decode_pipeline_state(&b, c).is_err());
                    }
                    for (offset, value, error) in [
                        (60, 0, Error::PipelinePhase),
                        (60, 2, Error::PipelinePhase),
                        (64, 1, Error::PipelineDeferred),
                        (64, 2, Error::PipelineDeferred),
                        (64, 3, Error::PipelineDeferred),
                    ] {
                        let mut b = bytes.clone();
                        b[offset..offset + 4].copy_from_slice(&u32::to_le_bytes(value));
                        assert_eq!(decode_pipeline_state(&b, c).err(), Some(error));
                    }
                    for offset in [72, 80, 88, 96, 104] {
                        for value in [1u64, u64::MAX] {
                            let mut b = bytes.clone();
                            b[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
                            assert_eq!(
                                decode_pipeline_state(&b, c).err(),
                                Some(Error::PipelineQuarantine)
                            );
                        }
                    }
                    for axis in 0..3 {
                        for value in [
                            f32::NAN.to_bits(),
                            f32::INFINITY.to_bits(),
                            f32::NEG_INFINITY.to_bits(),
                        ] {
                            let mut bad = r;
                            bad.gravity[axis] = value;
                            assert_eq!(
                                validate_pipeline_state(bad, c).err(),
                                Some(Error::Pipeline)
                            );
                        }
                    }
                    for state in [None, Some(false), Some(true)] {
                        let mut same = r;
                        same.post_solve_ccd_active = state;
                        same.gravity[0] = (-0f32).to_bits();
                        same.gravity[1] = f32::MAX.to_bits();
                        assert_eq!(
                            &check(c, same)[44..52],
                            &[
                                (-0f32).to_bits().to_le_bytes(),
                                f32::MAX.to_bits().to_le_bytes()
                            ]
                            .concat()
                        );
                    }
                    if $dimension == Dimension::Planar {
                        let mut bad = r;
                        bad.gravity[2] = (-0f32).to_bits();
                        assert_eq!(validate_pipeline_state(bad, c).err(), Some(Error::Pipeline));
                    }
                    let v = validate_pipeline_state(r, c).unwrap();
                    let mut tiny = Budget::new(111).unwrap();
                    assert!(matches!(
                        encode_pipeline_state(&v, &mut tiny),
                        Err(Error::Limit)
                    ));
                    let mut combined = Budget::new(223).unwrap();
                    encode_pipeline_state(&v, &mut combined).unwrap();
                    assert!(matches!(
                        encode_pipeline_state(&v, &mut combined),
                        Err(Error::Limit)
                    ));
                });
            }
            #[test]
            fn exact_context_association_and_claimed_phase_are_distinct() {
                let mut w = PhysicsWorld::default();
                w.step();
                with_pipeline(&mut w, PipelinePhase::CompletedPositive, |c, r| {
                    let bytes = check(c, r);
                    // Same bytes in a separately validated config are not the same witness.
                    let independent = validate_integration_config(*c.integration.record()).unwrap();
                    let wrong = PipelineContext {
                        integration: &independent,
                        ..c
                    };
                    assert_eq!(
                        validate_pipeline_state(r, wrong).err(),
                        Some(Error::Identity)
                    );
                    assert_eq!(
                        decode_pipeline_state(&bytes, wrong).err(),
                        Some(Error::Identity)
                    );
                    let mut bad = r;
                    bad.identity.build[0] ^= 1;
                    assert_eq!(validate_pipeline_state(bad, c).err(), Some(Error::Identity));
                    bad = r;
                    bad.identity.dimension = if $dimension == Dimension::Planar {
                        Dimension::Spatial
                    } else {
                        Dimension::Planar
                    };
                    assert_eq!(validate_pipeline_state(bad, c).err(), Some(Error::Identity));
                    for phase in [PipelinePhase::Unverified, PipelinePhase::CollisionOnly] {
                        bad = r;
                        bad.phase = phase;
                        assert_eq!(
                            validate_pipeline_state(bad, c).err(),
                            Some(Error::PipelinePhase)
                        );
                    }
                    for count in 0..5 {
                        bad = r;
                        bad.quarantine[count] = 1;
                        assert_eq!(
                            validate_pipeline_state(bad, c).err(),
                            Some(Error::PipelineQuarantine)
                        );
                    }
                    // Exact association is not authentication: a wholly independent,
                    // internally consistent context with equal identity is still inert.
                    let other_ccd = validate_ccd_state(
                        c.ccd.record().clone(),
                        CcdContext {
                            integration: &independent,
                            ..c.ccd.context()
                        },
                    )
                    .unwrap();
                    let internally_matched = PipelineContext {
                        integration: &independent,
                        ccd: &other_ccd,
                    };
                    check(internally_matched, r);
                });
                for dt in [0f32, -0., -1.] {
                    w.integration_parameters.dt = dt;
                    w.step();
                    with_pipeline(&mut w, PipelinePhase::CompletedPositive, |c, r| {
                        assert_eq!(
                            validate_pipeline_state(r, c).err(),
                            Some(Error::PipelinePhase)
                        )
                    });
                }
            }
        }
    };
}
native!(
    planar,
    rapier2d,
    Dimension::Planar,
    rapier2d::math::Vector::new,
    |v: rapier2d::math::Vector| [v.x.to_bits(), v.y.to_bits(), 0]
);
native!(
    spatial,
    rapier3d,
    Dimension::Spatial,
    |x, y| rapier3d::math::Vector::new(x, y, 0.),
    |v: rapier3d::math::Vector| v.to_array().map(f32::to_bits)
);

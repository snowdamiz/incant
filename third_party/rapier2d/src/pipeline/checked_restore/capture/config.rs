//! Exact integration settings and persistent CCD observation, with no serde.
use super::super::convert::bits;
use crate::dynamics::{CCDSolver, IntegrationParameters, SoftBodiesSettings, SoftRecoverySettings};
use incant_physics_snapshot as s;

// Exhaustive destructuring makes additions to native settings a compile error.
// PartialEq would fold signed zero and is therefore insufficient for this gate.
fn recovery_bits(value: SoftRecoverySettings) -> [u64; 27] {
    let SoftRecoverySettings {
        authored_velocity_margin,
        edge_speculation,
        inverted_cell_detection,
        self_crossing_detection,
        detection_motion_gating,
        cross_body_detection,
        self_stand_down,
        cross_body_expel_gate,
        edge_stand_down,
        crossing_repulsion,
        crossing_repulsion_guide,
        crossing_repulsion_self_guide,
        recovery_pace,
        overlap_constraints,
        overlap_rigid,
        overlap_skip_self_tangled,
        overlap_edge_stand_down,
        overlap_constraint_pace,
        overlap_patch_constraints,
        overlap_skin_volume,
        overlap_kept_depth,
        overlap_self_regions,
        overlap_normal_push,
        overlap_multi_volume,
        overlap_split,
        overlap_patience,
        overlap_progress_margin,
    } = value;
    [
        authored_velocity_margin as u64,
        edge_speculation as u64,
        inverted_cell_detection as u64,
        self_crossing_detection as u64,
        detection_motion_gating as u64,
        cross_body_detection as u64,
        self_stand_down as u64,
        cross_body_expel_gate as u64,
        edge_stand_down as u64,
        crossing_repulsion as u64,
        crossing_repulsion_guide as u64,
        crossing_repulsion_self_guide as u64,
        recovery_pace.to_bits() as u64,
        overlap_constraints as u64,
        overlap_rigid as u64,
        overlap_skip_self_tangled as u64,
        overlap_edge_stand_down as u64,
        overlap_constraint_pace.to_bits() as u64,
        overlap_patch_constraints as u64,
        overlap_skin_volume as u64,
        overlap_kept_depth.to_bits() as u64,
        overlap_self_regions as u64,
        overlap_normal_push as u64,
        overlap_multi_volume as u64,
        overlap_split as u64,
        overlap_patience as u64,
        overlap_progress_margin.to_bits() as u64,
    ]
}
#[cfg(not(feature = "fem"))]
fn soft_defaults(value: &SoftBodiesSettings) -> bool {
    let SoftBodiesSettings {
        recovery,
        resweep_strain,
        max_extra_substeps,
        contact_stiffening,
    } = *value;
    let default = SoftBodiesSettings::default();
    recovery_bits(recovery) == recovery_bits(default.recovery)
        && resweep_strain.to_bits() == default.resweep_strain.to_bits()
        && max_extra_substeps == default.max_extra_substeps
        && contact_stiffening.to_bits() == default.contact_stiffening.to_bits()
}
#[cfg(feature = "fem")]
fn soft_defaults(_: &SoftBodiesSettings) -> bool {
    false
}

pub(super) fn config(
    p: &IntegrationParameters,
    identity: s::Identity,
) -> Result<s::IntegrationConfig, s::Error> {
    if identity.dimension as usize != crate::math::DIM {
        return Err(s::Error::Identity);
    }
    if !soft_defaults(&p.soft_bodies) {
        return Err(s::Error::UnsupportedIntegration);
    }
    let IntegrationParameters {
        dt,
        min_ccd_dt,
        contact_softness,
        static_contact_softness,
        warmstart_coefficient,
        length_unit,
        soft_bodies: _,
        normalized_allowed_linear_error,
        normalized_max_corrective_velocity,
        normalized_prediction_distance,
        normalized_max_linear_velocity,
        num_solver_iterations,
        num_internal_pgs_iterations,
        num_internal_stabilization_iterations,
        max_ccd_substeps,
        contact_clustering,
        contact_recycling,
        normalized_contact_recycle_distance,
        friction_in_bias_pass,
        warmstart_joints,
        #[cfg(feature = "dim3")]
        friction_model,
    } = *p;
    let softness = |v: crate::dynamics::SpringCoefficients<f32>| s::ContactSoftness {
        natural_frequency: v.natural_frequency.to_bits(),
        damping_ratio: v.damping_ratio.to_bits(),
    };
    let record = s::IntegrationConfig {
        identity,
        dt: dt.to_bits(),
        min_ccd_dt: min_ccd_dt.to_bits(),
        contact_softness: softness(contact_softness),
        static_contact_softness: softness(static_contact_softness),
        warmstart_coefficient: warmstart_coefficient.to_bits(),
        length_unit: length_unit.to_bits(),
        normalized_allowed_linear_error: normalized_allowed_linear_error.to_bits(),
        normalized_max_corrective_velocity: normalized_max_corrective_velocity.to_bits(),
        normalized_prediction_distance: normalized_prediction_distance.to_bits(),
        normalized_max_linear_velocity: normalized_max_linear_velocity.to_bits(),
        normalized_contact_recycle_distance: normalized_contact_recycle_distance.to_bits(),
        num_solver_iterations: num_solver_iterations as u64,
        num_internal_pgs_iterations: num_internal_pgs_iterations as u64,
        num_internal_stabilization_iterations: num_internal_stabilization_iterations as u64,
        max_ccd_substeps: max_ccd_substeps as u64,
        contact_clustering,
        contact_recycling,
        friction_in_bias_pass,
        warmstart_joints,
        #[cfg(feature = "dim2")]
        friction_model: s::IntegrationFriction::Planar,
        #[cfg(feature = "dim3")]
        friction_model: match friction_model {
            crate::dynamics::FrictionModel::Simplified => s::IntegrationFriction::Simplified,
            crate::dynamics::FrictionModel::Coulomb => s::IntegrationFriction::Coulomb,
        },
        soft_bodies: s::IntegrationSoftBodies::PinnedDefaults,
    };
    s::validate_integration_config(record)?;
    Ok(record)
}

pub(super) fn ccd(
    solver: &CCDSolver,
    identity: s::Identity,
    budget: &mut s::Budget,
) -> Result<s::CcdState, s::Error> {
    if identity.dimension as usize != crate::math::DIM {
        return Err(s::Error::Identity);
    }
    let cache = match solver.checkpoint_fixed_targets() {
        None => s::CcdCache::Uninitialized,
        Some((prediction, targets)) => {
            if !prediction.is_finite() || prediction < 0. {
                return Err(s::Error::Ccd);
            }
            match targets {
                None => s::CcdCache::FullBvh {
                    prediction: prediction.to_bits(),
                },
                Some(rows) => {
                    if rows.len() > s::MAX_CCD_FIXED_TARGETS {
                        return Err(s::Error::Limit);
                    }
                    let mut targets = budget.vector::<s::CcdTarget>(rows.len())?;
                    for (handle, aabb) in rows {
                        let (slot, generation) = handle.into_raw_parts();
                        targets.push(s::CcdTarget {
                            collider: s::Handle { slot, generation },
                            mins: bits(aabb.mins),
                            maxs: bits(aabb.maxs),
                        });
                    }
                    s::CcdCache::Targets {
                        prediction: prediction.to_bits(),
                        targets,
                    }
                }
            }
        }
    };
    // Reciprocal collider ownership and AABB coherence are checked by the
    // aggregate joined frame, before any live reconstruction is permitted.
    Ok(s::CcdState { identity, cache })
}

#[cfg(all(test, not(feature = "fem")))]
mod tests {
    use super::*;
    use crate::dynamics::RigidBodyBuilder;
    use crate::geometry::ColliderBuilder;
    use crate::math::Vector;
    use crate::pipeline::PhysicsWorld;
    fn id() -> s::Identity {
        s::Identity {
            dimension: if crate::math::DIM == 2 {
                s::Dimension::Planar
            } else {
                s::Dimension::Spatial
            },
            build: [43; 32],
        }
    }
    fn budget() -> s::Budget {
        s::Budget::new(s::MAX_ALLOCATION).unwrap()
    }
    #[test]
    fn capture_config_preserves_switches_sentinels_and_signed_zero() {
        let p = IntegrationParameters {
            dt: -0.0,
            min_ccd_dt: -0.0,
            warmstart_coefficient: 0.75,
            length_unit: 2.0,
            normalized_prediction_distance: -0.0,
            normalized_max_linear_velocity: f32::MAX,
            normalized_max_corrective_velocity: f32::MAX,
            num_solver_iterations: 0,
            num_internal_pgs_iterations: 0,
            num_internal_stabilization_iterations: 0,
            max_ccd_substeps: 0,
            contact_clustering: false,
            contact_recycling: false,
            friction_in_bias_pass: true,
            warmstart_joints: true,
            ..IntegrationParameters::default()
        };
        let record = config(&p, id()).unwrap();
        assert_eq!(record.dt, (-0.0f32).to_bits());
        assert_eq!(record.min_ccd_dt, (-0.0f32).to_bits());
        assert_eq!(record.normalized_prediction_distance, (-0.0f32).to_bits());
        assert_eq!(record.normalized_max_linear_velocity, f32::MAX.to_bits());
        assert_eq!(
            record.normalized_max_corrective_velocity,
            f32::MAX.to_bits()
        );
        assert_eq!(record.num_solver_iterations, 0);
        assert_eq!(record.max_ccd_substeps, 0);
        assert!(!record.contact_clustering && !record.contact_recycling);
        assert!(record.friction_in_bias_pass && record.warmstart_joints);
        let checked = s::validate_integration_config(record).unwrap();
        let bytes = s::encode_integration_config(&checked, &mut budget()).unwrap();
        assert_eq!(
            s::decode_integration_config(&bytes, id()).unwrap().record(),
            &record
        );
        #[cfg(feature = "dim3")]
        for model in [
            crate::dynamics::FrictionModel::Coulomb,
            crate::dynamics::FrictionModel::Simplified,
        ] {
            let mut varied = p;
            varied.friction_model = model;
            let captured = config(&varied, id()).unwrap();
            assert_eq!(
                captured.friction_model == s::IntegrationFriction::Coulomb,
                model == crate::dynamics::FrictionModel::Coulomb
            );
        }
    }
    #[test]
    fn capture_config_refuses_every_changed_soft_setting_and_invalid_rigid_input() {
        let changes: [fn(&mut SoftBodiesSettings); 30] = [
            |v| v.recovery.authored_velocity_margin = !v.recovery.authored_velocity_margin,
            |v| v.recovery.edge_speculation = !v.recovery.edge_speculation,
            |v| v.recovery.inverted_cell_detection = !v.recovery.inverted_cell_detection,
            |v| v.recovery.self_crossing_detection = !v.recovery.self_crossing_detection,
            |v| v.recovery.detection_motion_gating = !v.recovery.detection_motion_gating,
            |v| v.recovery.cross_body_detection = !v.recovery.cross_body_detection,
            |v| v.recovery.self_stand_down = !v.recovery.self_stand_down,
            |v| v.recovery.cross_body_expel_gate = !v.recovery.cross_body_expel_gate,
            |v| v.recovery.edge_stand_down = !v.recovery.edge_stand_down,
            |v| v.recovery.crossing_repulsion = !v.recovery.crossing_repulsion,
            |v| v.recovery.crossing_repulsion_guide = !v.recovery.crossing_repulsion_guide,
            |v| {
                v.recovery.crossing_repulsion_self_guide = !v.recovery.crossing_repulsion_self_guide
            },
            |v| v.recovery.recovery_pace = v.recovery.recovery_pace + 0.25,
            |v| v.recovery.overlap_constraints = !v.recovery.overlap_constraints,
            |v| v.recovery.overlap_rigid = !v.recovery.overlap_rigid,
            |v| v.recovery.overlap_skip_self_tangled = !v.recovery.overlap_skip_self_tangled,
            |v| v.recovery.overlap_edge_stand_down = !v.recovery.overlap_edge_stand_down,
            |v| v.recovery.overlap_constraint_pace = v.recovery.overlap_constraint_pace + 0.25,
            |v| v.recovery.overlap_patch_constraints = crate::dynamics::SoftPatchConstraints::Keep,
            |v| v.recovery.overlap_skin_volume = !v.recovery.overlap_skin_volume,
            |v| v.recovery.overlap_kept_depth = -0.0,
            |v| v.recovery.overlap_self_regions = !v.recovery.overlap_self_regions,
            |v| v.recovery.overlap_normal_push = !v.recovery.overlap_normal_push,
            |v| v.recovery.overlap_multi_volume = !v.recovery.overlap_multi_volume,
            |v| v.recovery.overlap_split = v.recovery.overlap_split + 1,
            |v| v.recovery.overlap_patience = v.recovery.overlap_patience + 1,
            |v| v.recovery.overlap_progress_margin = v.recovery.overlap_progress_margin + 0.25,
            |v| v.resweep_strain += 0.25,
            |v| v.max_extra_substeps += 1,
            |v| v.contact_stiffening += 0.25,
        ];
        for change in changes {
            let mut p = IntegrationParameters::default();
            change(&mut p.soft_bodies);
            assert_eq!(config(&p, id()), Err(s::Error::UnsupportedIntegration));
        }
        let mut p = IntegrationParameters::default();
        p.dt = f32::NAN;
        assert_eq!(config(&p, id()), Err(s::Error::Integration));
        p = IntegrationParameters::default();
        p.length_unit = 0.;
        assert_eq!(config(&p, id()), Err(s::Error::Integration));
        let mut other = id();
        other.dimension = if other.dimension == s::Dimension::Planar {
            s::Dimension::Spatial
        } else {
            s::Dimension::Planar
        };
        assert_eq!(
            config(&IntegrationParameters::default(), other),
            Err(s::Error::Identity)
        );
    }
    fn ccd_world(count: usize) -> PhysicsWorld {
        let mut w = PhysicsWorld::default();
        w.gravity = Vector::ZERO;
        w.integration_parameters.normalized_prediction_distance = -0.0;
        for i in 0..count {
            w.colliders.insert(
                ColliderBuilder::ball(0.25)
                    .translation(Vector::X * (i as f32 * 3.0))
                    .build(),
            );
        }
        w.insert(
            RigidBodyBuilder::dynamic()
                .translation(Vector::Y * 20.0)
                .linvel(Vector::X * 100.0),
            ColliderBuilder::ball(0.1),
        );
        w.step();
        w
    }
    #[test]
    fn capture_ccd_preserves_three_states_empty_cache_order_and_historical_prediction() {
        let none = ccd(&CCDSolver::new(), id(), &mut budget()).unwrap();
        assert_eq!(none.cache, s::CcdCache::Uninitialized);
        for count in [0, 2, s::MAX_CCD_FIXED_TARGETS + 1] {
            let mut w = ccd_world(count);
            // Capturing CCD must not substitute the now-current parameter.
            w.integration_parameters.normalized_prediction_distance = 0.25;
            let mut b = budget();
            let before = b.remaining();
            let captured = ccd(&w.ccd_solver, id(), &mut b).unwrap();
            let (native_prediction, native_rows) = w
                .ccd_solver
                .checkpoint_fixed_targets()
                .expect("actual CCD cache");
            assert_eq!(native_prediction.to_bits(), (-0.0f32).to_bits());
            match (captured.cache, native_rows) {
                (
                    s::CcdCache::Targets {
                        prediction,
                        targets,
                    },
                    Some(rows),
                ) => {
                    assert_eq!(prediction, native_prediction.to_bits());
                    assert_eq!(targets.len(), count);
                    assert_eq!(
                        before - b.remaining(),
                        count * core::mem::size_of::<s::CcdTarget>()
                    );
                    for (target, (handle, aabb)) in targets.iter().zip(rows) {
                        assert_eq!(
                            (target.collider.slot, target.collider.generation),
                            handle.into_raw_parts()
                        );
                        assert_eq!(target.mins, bits(aabb.mins));
                        assert_eq!(target.maxs, bits(aabb.maxs));
                    }
                }
                (s::CcdCache::FullBvh { prediction }, None) => {
                    assert!(count > s::MAX_CCD_FIXED_TARGETS);
                    assert_eq!(prediction, native_prediction.to_bits());
                    assert_eq!(before, b.remaining());
                }
                _ => panic!("CCD cache kind changed"),
            }
        }
    }
    #[test]
    fn capture_ccd_budget_refusal_leaves_native_cache_intact() {
        let w = ccd_world(2);
        let before = ccd(&w.ccd_solver, id(), &mut budget()).unwrap();
        let mut small = s::Budget::new(2 * core::mem::size_of::<s::CcdTarget>() - 1).unwrap();
        assert_eq!(ccd(&w.ccd_solver, id(), &mut small), Err(s::Error::Limit));
        assert_eq!(ccd(&w.ccd_solver, id(), &mut budget()).unwrap(), before);
    }
}

//! Exact scalar conversion after the joined numeric preflight.
use super::{NumericFrame, allocation::*};
use crate::dynamics::{IntegrationParameters, SoftBodiesSettings, SpringCoefficients};
use incant_physics_snapshot::{IntegrationFriction, IntegrationSoftBodies};
pub(super) fn construct(frame: &NumericFrame<'_>) -> ConstructionResult<IntegrationParameters> {
    let r = frame.boundary().pipeline().context().integration.record();
    if r.soft_bodies != IntegrationSoftBodies::PinnedDefaults {
        return Err(ConstructionError::Payload);
    }
    #[cfg(feature = "dim2")]
    if r.friction_model != IntegrationFriction::Planar {
        return Err(ConstructionError::Payload);
    }
    #[cfg(feature = "dim3")]
    let friction_model = match r.friction_model {
        IntegrationFriction::Simplified => crate::dynamics::FrictionModel::Simplified,
        IntegrationFriction::Coulomb => crate::dynamics::FrictionModel::Coulomb,
        IntegrationFriction::Planar => return Err(ConstructionError::Payload),
    };
    let softness = |x: incant_physics_snapshot::ContactSoftness| SpringCoefficients {
        natural_frequency: f32::from_bits(x.natural_frequency),
        damping_ratio: f32::from_bits(x.damping_ratio),
    };
    let count = |x: u64| usize::try_from(x).map_err(|_| ConstructionError::Payload);
    Ok(IntegrationParameters {
        dt: f32::from_bits(r.dt),
        min_ccd_dt: f32::from_bits(r.min_ccd_dt),
        contact_softness: softness(r.contact_softness),
        static_contact_softness: softness(r.static_contact_softness),
        warmstart_coefficient: f32::from_bits(r.warmstart_coefficient),
        length_unit: f32::from_bits(r.length_unit),
        soft_bodies: SoftBodiesSettings::default(),
        normalized_allowed_linear_error: f32::from_bits(r.normalized_allowed_linear_error),
        normalized_max_corrective_velocity: f32::from_bits(r.normalized_max_corrective_velocity),
        normalized_prediction_distance: f32::from_bits(r.normalized_prediction_distance),
        normalized_max_linear_velocity: f32::from_bits(r.normalized_max_linear_velocity),
        normalized_contact_recycle_distance: f32::from_bits(r.normalized_contact_recycle_distance),
        num_solver_iterations: count(r.num_solver_iterations)?,
        num_internal_pgs_iterations: count(r.num_internal_pgs_iterations)?,
        num_internal_stabilization_iterations: count(r.num_internal_stabilization_iterations)?,
        max_ccd_substeps: count(r.max_ccd_substeps)?,
        contact_clustering: r.contact_clustering,
        contact_recycling: r.contact_recycling,
        friction_in_bias_pass: r.friction_in_bias_pass,
        warmstart_joints: r.warmstart_joints,
        #[cfg(feature = "dim3")]
        friction_model,
    })
}

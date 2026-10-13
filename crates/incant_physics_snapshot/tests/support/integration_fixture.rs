use incant_physics_snapshot::*;
pub fn identity(dimension: Dimension) -> Identity {
    Identity {
        dimension,
        build: [91; 32],
    }
}
pub fn config(d: Dimension) -> IntegrationConfig {
    IntegrationConfig {
        identity: identity(d),
        dt: (1f32 / 60.).to_bits(),
        min_ccd_dt: (1f32 / 60. / 100.).to_bits(),
        contact_softness: ContactSoftness {
            natural_frequency: 30f32.to_bits(),
            damping_ratio: 10f32.to_bits(),
        },
        static_contact_softness: ContactSoftness {
            natural_frequency: 60f32.to_bits(),
            damping_ratio: 10f32.to_bits(),
        },
        warmstart_coefficient: 1f32.to_bits(),
        length_unit: 1f32.to_bits(),
        normalized_allowed_linear_error: 0.005f32.to_bits(),
        normalized_max_corrective_velocity: 3f32.to_bits(),
        normalized_prediction_distance: 0.02f32.to_bits(),
        normalized_max_linear_velocity: 400f32.to_bits(),
        normalized_contact_recycle_distance: 0.05f32.to_bits(),
        num_solver_iterations: 4,
        num_internal_pgs_iterations: 1,
        num_internal_stabilization_iterations: 1,
        max_ccd_substeps: 1,
        contact_clustering: true,
        contact_recycling: true,
        friction_in_bias_pass: false,
        warmstart_joints: false,
        friction_model: if d == Dimension::Planar {
            IntegrationFriction::Planar
        } else {
            IntegrationFriction::Simplified
        },
        soft_bodies: IntegrationSoftBodies::PinnedDefaults,
    }
}

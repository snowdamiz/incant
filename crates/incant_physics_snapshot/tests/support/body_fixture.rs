use incant_physics_snapshot::*;
fn pose(d: Dimension) -> LocalPose {
    LocalPose {
        translation: [0; 3],
        rotation: if d == Dimension::Planar {
            [1f32.to_bits(), 0, 0, 0]
        } else {
            [0, 0, 0, 1f32.to_bits()]
        },
    }
}
pub fn state(d: Dimension) -> RigidBodyState {
    let local = LocalMass {
        center: [0; 3],
        inverse_mass: 0,
        inverse_principal_inertia: [0; 3],
        principal_frame: if d == Dimension::Planar {
            [0; 4]
        } else {
            pose(d).rotation
        },
    };
    RigidBodyState {
        position: pose(d),
        next_position: pose(d),
        velocity: BodyVelocity {
            linear: [0; 3],
            angular: [0; 3],
        },
        ccd_velocity: BodyVelocity {
            linear: [0; 3],
            angular: [0; 3],
        },
        forces: BodyForces {
            force: [0; 3],
            torque: [0; 3],
            gravity_scale: 1f32.to_bits(),
            user_force: [0; 3],
            user_torque: [0; 3],
            gyroscopic: d == Dimension::Spatial,
        },
        damping: [0; 2],
        mass: BodyMass {
            world_center: [0; 3],
            effective_inverse_mass: [0; 3],
            effective_inverse_inertia: [0; 6],
            local,
            locked_axes: 0,
            additional: None,
            max_extent: 0,
        },
        activation: BodyActivation {
            linear_threshold: (-1f32).to_bits(),
            angular_threshold: (-1f32).to_bits(),
            time_until_sleep: 0.5f32.to_bits(),
            time_since_can_sleep: 0,
            sleeping: false,
            previous_pose: pose(d),
        },
        ccd: BodyCcd {
            thickness: f32::MAX.to_bits(),
            active: false,
            enabled: false,
            soft_prediction: 0,
            allow_fast_rotation: false,
        },
        ids: [u32::MAX; 4],
        changes: 0,
        kind: BodyKind::Dynamic,
        dominance: 0,
        enabled: true,
        additional_iterations: [0; 2],
        soft_body: Handle {
            slot: u32::MAX,
            generation: u32::MAX,
        },
        soft_cluster: u32::MAX,
        soft_motion_margin: 0,
        user_data: [0; 4],
    }
}

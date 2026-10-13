//! Trusted one-way fixed body observer shared by body and island native tests.
use incant_physics_snapshot::*;
use serde_json::Value;
pub fn word(v: &Value) -> u32 {
    v.as_u64().unwrap().try_into().unwrap()
}
fn float(v: &Value) -> u32 {
    (v.as_f64().unwrap() as f32).to_bits()
}
pub fn vector(v: &Value, d: Dimension) -> [u32; 3] {
    let mut out = [0; 3];
    for i in 0..d as usize {
        out[i] = float(&v[i]);
    }
    out
}
pub fn angular(v: &Value, d: Dimension) -> [u32; 3] {
    if d == Dimension::Planar {
        [float(v), 0, 0]
    } else {
        vector(v, d)
    }
}
fn rotation(v: &Value, d: Dimension) -> [u32; 4] {
    if d == Dimension::Planar {
        [float(&v["re"]), float(&v["im"]), 0, 0]
    } else {
        [float(&v[0]), float(&v[1]), float(&v[2]), float(&v[3])]
    }
}
pub fn pose(v: &Value, d: Dimension) -> LocalPose {
    LocalPose {
        translation: vector(&v["translation"], d),
        rotation: rotation(&v["rotation"], d),
    }
}
fn velocity(v: &Value, d: Dimension) -> BodyVelocity {
    BodyVelocity {
        linear: vector(&v["linvel"], d),
        angular: angular(&v["angvel"], d),
    }
}
fn local(v: &Value, d: Dimension) -> LocalMass {
    LocalMass {
        center: vector(&v["local_com"], d),
        inverse_mass: float(&v["inv_mass"]),
        inverse_principal_inertia: angular(&v["inv_principal_inertia"], d),
        principal_frame: if d == Dimension::Planar {
            [0; 4]
        } else {
            rotation(&v["principal_inertia_local_frame"], d)
        },
    }
}
fn mass(v: &Value, d: Dimension) -> BodyMass {
    let inertia = &v["effective_world_inv_inertia"];
    let additional = &v["additional_local_mprops"];
    BodyMass {
        world_center: vector(&v["world_com"], d),
        effective_inverse_mass: vector(&v["effective_inv_mass"], d),
        effective_inverse_inertia: if d == Dimension::Planar {
            [float(inertia), 0, 0, 0, 0, 0]
        } else {
            ["m11", "m12", "m13", "m22", "m23", "m33"].map(|k| float(&inertia[k]))
        },
        local: local(&v["local_mprops"], d),
        locked_axes: word(&v["flags"]),
        additional: if additional.is_null() {
            None
        } else if let Some(m) = additional.get("Mass") {
            Some(AdditionalMass::Mass(float(m)))
        } else {
            Some(AdditionalMass::Properties(local(
                &additional["MassProps"],
                d,
            )))
        },
        max_extent: float(&v["max_extent"]),
    }
}
pub fn body(v: &Value, d: Dimension) -> RigidBodyState {
    let a = &v["activation"];
    let f = &v["forces"];
    let c = &v["ccd"];
    let ids = &v["ids"];
    let u: u128 = v["user_data"].as_str().unwrap().parse().unwrap();
    RigidBodyState {
        position: pose(&v["pos"]["position"], d),
        next_position: pose(&v["pos"]["next_position"], d),
        velocity: velocity(&v["vels"], d),
        ccd_velocity: velocity(&v["ccd_vels"], d),
        forces: BodyForces {
            force: vector(&f["force"], d),
            torque: angular(&f["torque"], d),
            gravity_scale: float(&f["gravity_scale"]),
            user_force: vector(&f["user_force"], d),
            user_torque: angular(&f["user_torque"], d),
            gyroscopic: if d == Dimension::Planar {
                false
            } else {
                f["gyroscopic_forces_enabled"].as_bool().unwrap()
            },
        },
        damping: [
            float(&v["damping"]["linear_damping"]),
            float(&v["damping"]["angular_damping"]),
        ],
        mass: mass(&v["mprops"], d),
        activation: BodyActivation {
            linear_threshold: float(&a["normalized_linear_threshold"]),
            angular_threshold: float(&a["angular_threshold"]),
            time_until_sleep: float(&a["time_until_sleep"]),
            time_since_can_sleep: float(&a["time_since_can_sleep"]),
            sleeping: a["sleeping"].as_bool().unwrap(),
            previous_pose: pose(&a["sleep_prev_pose"], d),
        },
        ccd: BodyCcd {
            thickness: float(&c["ccd_thickness"]),
            active: c["ccd_active"].as_bool().unwrap(),
            enabled: c["ccd_enabled"].as_bool().unwrap(),
            soft_prediction: float(&c["soft_ccd_prediction"]),
            allow_fast_rotation: c["allow_fast_rotation"].as_bool().unwrap(),
        },
        ids: [
            "active_island_id",
            "active_set_id",
            "island_id",
            "island_index",
        ]
        .map(|k| word(&ids[k])),
        changes: word(&v["changes"]),
        kind: match v["body_type"].as_str().unwrap() {
            "Dynamic" => BodyKind::Dynamic,
            "Fixed" => BodyKind::Fixed,
            "KinematicPositionBased" => BodyKind::KinematicPosition,
            "KinematicVelocityBased" => BodyKind::KinematicVelocity,
            other => panic!("unsupported observed native kind: {other}"),
        },
        dominance: v["dominance"].as_i64().unwrap().try_into().unwrap(),
        enabled: v["enabled"].as_bool().unwrap(),
        additional_iterations: [
            v["additional_solver_iterations"].as_u64().unwrap(),
            v["additional_pgs_iterations"].as_u64().unwrap(),
        ],
        soft_body: Handle {
            slot: word(&v["soft_body"]["index"]),
            generation: word(&v["soft_body"]["generation"]),
        },
        soft_cluster: word(&v["soft_cluster"]),
        soft_motion_margin: float(&v["soft_motion_margin"]),
        user_data: std::array::from_fn(|i| (u >> (i * 32)) as u32),
    }
}

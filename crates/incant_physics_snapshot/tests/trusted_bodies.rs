//! Actual native bodies observed one way; never deserialize native state.
use incant_physics_snapshot::*;
use serde_json::Value;
#[path = "support/ordered_capture.rs"]
mod ordered_capture;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn word(v: &Value) -> u32 {
    v.as_u64().unwrap().try_into().unwrap()
}
fn float(v: &Value) -> u32 {
    (v.as_f64().unwrap() as f32).to_bits()
}
fn vector(v: &Value, d: Dimension) -> [u32; 3] {
    let mut out = [0; 3];
    for i in 0..d as usize {
        out[i] = float(&v[i]);
    }
    out
}
fn angular(v: &Value, d: Dimension) -> [u32; 3] {
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
fn pose(v: &Value, d: Dimension) -> LocalPose {
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
fn body(v: &Value, d: Dimension) -> RigidBodyState {
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
fn layout(v: &Value, id: Identity) -> (ValidatedArena, Vec<Handle>) {
    let mut handles = vec![];
    let slots = v["items"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(slot, item)| {
            if let Some(free) = item.get("Free") {
                ArenaSlot::Free {
                    next: (!free["next_free"].is_null()).then(|| word(&free["next_free"])),
                }
            } else {
                let generation = word(&item["Occupied"]["generation"]);
                let row = handles.len() as u32;
                handles.push(Handle {
                    slot: slot as u32,
                    generation,
                });
                ArenaSlot::Occupied { generation, row }
            }
        })
        .collect();
    (
        validate_arena(
            ArenaRecord {
                identity: id,
                generation: word(&v["generation"]),
                free_head: (!v["free_list_head"].is_null()).then(|| word(&v["free_list_head"])),
                live: word(&v["len"]),
                slots,
            },
            &mut budget(),
        )
        .unwrap(),
        handles,
    )
}
fn vec2(x: f32, y: f32) -> rapier2d::math::Vector {
    rapier2d::math::Vector::new(x, y)
}
fn vec3(x: f32, y: f32) -> rapier3d::math::Vector {
    rapier3d::math::Vector::new(x, y, x * 0.25)
}
fn ang2(x: f32) -> f32 {
    x
}
fn ang3(x: f32) -> rapier3d::math::AngVector {
    rapier3d::math::AngVector::new(x, x * 0.25, -x * 0.5)
}
fn mass2() -> parry2d::mass_properties::MassProperties {
    parry2d::mass_properties::MassProperties::new(vec2(0.1, 0.2), 2., 0.5)
}
fn mass3() -> parry3d::mass_properties::MassProperties {
    parry3d::mass_properties::MassProperties::with_principal_inertia_frame(
        vec3(0.1, 0.2),
        2.,
        vec3(0.5, 0.75),
        rapier3d::math::Rotation::from_rotation_z(0.3),
    )
}
macro_rules! cases {($name:ident,$rapier:ident,$dim:ident,$vec:path,$ang:path,$mass:path)=>{mod $name {
    use super::*;
    use $rapier::prelude::*;
    fn check(w: &PhysicsWorld) -> (Vec<(Handle, RigidBodyState)>, Vec<u8>) {
        let d = Dimension::$dim;
        let identity = Identity {
            dimension: d,
            build: [68; 32],
        };
        let bv = ordered_capture::capture(&w.bodies);
        let cv = ordered_capture::capture(&w.colliders);
        let journals = PendingJournals {
            modified_bodies: bv["modified_bodies"][0].as_array().unwrap().len() as u32,
            modified_colliders: cv["modified_colliders"][0].as_array().unwrap().len() as u32,
            removed_colliders: cv["removed_colliders"].as_array().unwrap().len() as u32,
        };
        let (ba, bh) = layout(&bv["bodies"], identity);
        let (ca, ch) = layout(&cv["colliders"], identity);
        let mut attachments = AttachmentRecord {
            identity,
            bodies: vec![],
            colliders: vec![],
            attached: vec![],
        };
        let mut rows = vec![];
        for h in &bh {
            let b = &w.bodies[RigidBodyHandle::from_raw_parts(h.slot, h.generation)];
            attachments.bodies.push(AttachmentSpan {
                start: attachments.attached.len() as u32,
                count: b.colliders().len() as u32,
            });
            attachments.attached.extend(b.colliders().iter().map(|v| {
                let (slot, generation) = v.into_raw_parts();
                Handle { slot, generation }
            }));
            let observed = ordered_capture::capture(b);
            let row = body(&observed, d);
            assert_eq!(
                row.position,
                pose(&ordered_capture::capture(b.position()), d)
            );
            assert_eq!(
                row.next_position,
                pose(&ordered_capture::capture(b.next_position()), d)
            );
            assert_eq!(
                row.velocity.linear,
                vector(&ordered_capture::capture(&b.linvel()), d)
            );
            assert_eq!(
                row.velocity.angular,
                angular(&ordered_capture::capture(&b.angvel()), d)
            );
            assert_eq!(
                row.mass.local.inverse_mass,
                b.mass_properties().local_mprops.inv_mass.to_bits()
            );
            assert_eq!(
                row.mass.max_extent,
                b.mass_properties().max_extent().to_bits()
            );
            assert_eq!(
                row.forces.user_force,
                vector(&ordered_capture::capture(&b.user_force()), d)
            );
            rows.push(row);
        }
        for h in ch {
            let co = &w.colliders[ColliderHandle::from_raw_parts(h.slot, h.generation)];
            attachments.colliders.push(co.parent().map(|b| {
                let (slot, generation) = b.into_raw_parts();
                ParentAttachment {
                    body: Handle { slot, generation },
                    local_pose: pose(
                        &ordered_capture::capture(co.position_wrt_parent().unwrap()),
                        d,
                    ),
                }
            }));
        }
        let a = validate_attachments(attachments, &ba, &ca, &mut budget()).unwrap();
        let c = BodyContext {
            attachments: &a,
            journals,
        };
        let table = BodyStateTable {
            identity,
            bodies: rows.clone(),
        };
        if journals != PendingJournals::default() {
            assert!(matches!(
                validate_body_state(table, c),
                Err(Error::PendingMutation)
            ));
            return (vec![], vec![]);
        }
        let v = validate_body_state(table.clone(), c).unwrap();
        let bytes = encode_body_state(&v, &mut budget()).unwrap();
        let restored = decode_body_state(&bytes, c, &mut budget()).unwrap();
        assert_eq!(restored.record(), &table);
        assert_eq!(encode_body_state(&restored, &mut budget()).unwrap(), bytes);
        for (h, row) in bh.iter().zip(&rows) {
            assert_eq!(restored.body(*h), Some(row));
        }
        (bh.into_iter().zip(rows).collect(), bytes)
    }
    fn get(rows: &[(Handle, RigidBodyState)], h: RigidBodyHandle) -> RigidBodyState {
        let (slot, generation) = h.into_raw_parts();
        rows.iter()
            .find(|(k, _)| *k == Handle { slot, generation })
            .unwrap()
            .1
    }
    #[test]
    fn actual_forces_activation_ccd_mass_types_and_generation() {
        let mut w = PhysicsWorld::default();
        w.gravity = $vec(0., 0.);
        let h = w.bodies.insert(
            RigidBodyBuilder::dynamic()
                .pose(Pose::from_translation($vec(1., 2.)))
                .linvel($vec(0.5, -0.25))
                .angvel($ang(0.2))
                .linear_damping(0.2)
                .angular_damping(0.3)
                .gravity_scale(-2.)
                .additional_mass_properties($mass())
                .additional_solver_iterations(2)
                .additional_pgs_iterations(3)
                .user_data(u128::MAX),
        );
        w.colliders
            .insert_with_parent(ColliderBuilder::ball(0.5).density(1.5), h, &mut w.bodies);
        let free = w.bodies.insert(
            RigidBodyBuilder::dynamic()
                .can_sleep(false)
                .gravity_scale(-0.0)
                .linear_damping(-0.0),
        );
        let fixed = w.bodies.insert(RigidBodyBuilder::fixed().enabled(false));
        let k = w
            .bodies
            .insert(RigidBodyBuilder::kinematic_position_based());
        let kv = w
            .bodies
            .insert(RigidBodyBuilder::kinematic_velocity_based().linvel($vec(0.3, 0.4)));
        assert!(check(&w).0.is_empty());
        w.step();
        let first = check(&w).0;
        assert_eq!(get(&first, h).user_data, [u32::MAX; 4]);
        assert!(matches!(
            get(&first, h).mass.additional,
            Some(AdditionalMass::Properties(_))
        ));
        assert_eq!(get(&first, free).forces.gravity_scale, (-0f32).to_bits());
        assert_eq!(get(&first, free).damping[0], (-0f32).to_bits());
        assert_eq!(get(&first, free).ccd.thickness, f32::MAX.to_bits());
        assert_eq!(
            get(&first, free).activation.linear_threshold,
            (-1f32).to_bits()
        );
        assert_eq!(get(&first, fixed).mass.local.inverse_mass, 0);
        assert!(!get(&first, fixed).enabled);
        assert_eq!(get(&first, kv).kind, BodyKind::KinematicVelocity);
        w.bodies[h].add_force($vec(3., -2.), true);
        w.bodies[h].add_torque($ang(0.7), true);
        w.bodies[h].apply_impulse($vec(0.2, 0.3), true);
        w.bodies[h].apply_torque_impulse($ang(0.1), true);
        w.bodies[h].enable_ccd(true);
        w.bodies[h].set_soft_ccd_prediction(0.2);
        w.bodies[k].set_next_kinematic_position(Pose::from_translation($vec(2., 1.)));
        assert!(check(&w).0.is_empty());
        w.step();
        let forced = check(&w).0;
        assert_ne!(get(&first, h).velocity, get(&forced, h).velocity);
        assert_ne!(get(&forced, h).forces.force, [0; 3]);
        assert_ne!(get(&forced, h).forces.user_torque, [0; 3]);
        assert!(get(&forced, h).ccd.enabled);
        assert_ne!(get(&forced, k).velocity.linear, [0; 3]);
        w.bodies[h].reset_forces(false);
        w.bodies[h].reset_torques(false);
        w.bodies[h].sleep();
        w.step();
        let asleep = check(&w).0;
        assert!(get(&asleep, h).activation.sleeping);
        w.bodies[h].wake_up(true);
        w.bodies[h].set_locked_axes(
            LockedAxes::TRANSLATION_LOCKED_X | LockedAxes::ROTATION_LOCKED_Z,
            true,
        );
        w.bodies[h].set_additional_mass(3., true);
        w.step();
        let awake = check(&w).0;
        assert!(!get(&awake, h).activation.sleeping);
        assert_eq!(get(&awake, h).mass.effective_inverse_mass[0], 0);
        assert!(
            matches!(get(&awake,h).mass.additional,Some(AdditionalMass::Mass(v)) if v==3f32.to_bits())
        );
        w.bodies[h].set_enabled(false);
        w.bodies[k].set_body_type(RigidBodyType::Fixed, true);
        w.step();
        let disabled = check(&w).0;
        assert!(!get(&disabled, h).enabled);
        assert_eq!(get(&disabled, k).kind, BodyKind::Fixed);
        w.bodies[h].set_enabled(true);
        w.bodies[h].set_dominance_group(-128);
        w.step();
        assert_eq!(get(&check(&w).0, h).dominance, -128);
        w.bodies.remove(
            free,
            &mut w.islands,
            &mut w.colliders,
            &mut w.impulse_joints,
            &mut w.multibody_joints,
            &mut w.soft_bodies,
            true,
        );
        let reused = w.bodies.insert(RigidBodyBuilder::dynamic());
        w.step();
        let last = check(&w).0;
        assert_eq!(free.into_raw_parts().0, reused.into_raw_parts().0);
        assert_ne!(free.into_raw_parts().1, reused.into_raw_parts().1);
        assert!(!last.iter().any(|(v, _)| *v
            == Handle {
                slot: free.into_raw_parts().0,
                generation: free.into_raw_parts().1
            }));
        println!(
            "BODY_NATIVE dimension={} forces=true activation=true mass_variants=true kinematics=true disabled=true removed_reused=true",
            Dimension::$dim as u8
        );
    }
    #[test]
    fn actual_automatic_ccd_and_sleep_history_are_retained() {
        let mut w = PhysicsWorld::default();
        w.gravity = $vec(0., 0.);
        let fast = w
            .bodies
            .insert(RigidBodyBuilder::dynamic().linvel($vec(1000., 0.)));
        w.colliders
            .insert_with_parent(ColliderBuilder::ball(0.25), fast, &mut w.bodies);
        let slow = w.bodies.insert(
            RigidBodyBuilder::dynamic().pose(Pose::from_translation($vec(0., 20.))),
        );
        w.colliders
            .insert_with_parent(ColliderBuilder::ball(0.5), slow, &mut w.bodies);
        w.step();
        let first = check(&w).0;
        assert!(get(&first, fast).ccd.active);
        assert!(!get(&first, fast).ccd.enabled);
        assert_ne!(get(&first, fast).ccd_velocity.linear, [0; 3]);
        let initial = get(&first, slow).activation;
        for _ in 0..90 {
            w.step();
            check(&w);
        }
        let last = check(&w).0;
        assert!(get(&last, slow).activation.sleeping);
        assert_ne!(
            get(&last, slow).activation.time_since_can_sleep,
            initial.time_since_can_sleep
        );
        assert_ne!(
            get(&last, fast).activation.previous_pose,
            get(&first, fast).activation.previous_pose
        );
        println!(
            "BODY_NATIVE dimension={} automatic_ccd=true ccd_without_full_enabled=true natural_sleep=true history_steps=91",
            Dimension::$dim as u8
        );
    }
}};}
cases!(planar, rapier2d, Planar, vec2, ang2, mass2);
cases!(spatial, rapier3d, Spatial, vec3, ang3, mass3);

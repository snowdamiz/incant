//! Actual native bodies observed one way; never deserialize native state.
use incant_physics_snapshot::*;
use serde_json::Value;
#[path = "support/ordered_capture.rs"]
mod ordered_capture;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
#[path = "support/body_capture.rs"]
mod body_capture;
use body_capture::{angular, body, pose, vector, word};
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

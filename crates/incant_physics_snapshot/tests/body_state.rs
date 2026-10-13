use incant_physics_snapshot::*;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn id(d: Dimension) -> Identity {
    Identity {
        dimension: d,
        build: [73; 32],
    }
}
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
fn state(d: Dimension) -> RigidBodyState {
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
fn with(d: Dimension, f: impl FnOnce(BodyContext<'_>, BodyStateTable)) {
    let arena = validate_arena(
        ArenaRecord {
            identity: id(d),
            generation: 3,
            free_head: None,
            live: 2,
            slots: vec![
                ArenaSlot::Occupied {
                    generation: 1,
                    row: 1,
                },
                ArenaSlot::Occupied {
                    generation: 3,
                    row: 0,
                },
            ],
        },
        &mut budget(),
    )
    .unwrap();
    let empty = validate_arena(
        ArenaRecord {
            identity: id(d),
            generation: 0,
            free_head: Some(0),
            live: 0,
            slots: vec![ArenaSlot::Free { next: None }],
        },
        &mut budget(),
    )
    .unwrap();
    let a = validate_attachments(
        AttachmentRecord {
            identity: id(d),
            bodies: vec![AttachmentSpan { start: 0, count: 0 }; 2],
            colliders: vec![],
            attached: vec![],
        },
        &arena,
        &empty,
        &mut budget(),
    )
    .unwrap();
    f(
        BodyContext {
            attachments: &a,
            journals: PendingJournals::default(),
        },
        BodyStateTable {
            identity: id(d),
            bodies: vec![state(d); 2],
        },
    );
}
fn check(c: BodyContext<'_>, r: BodyStateTable) -> Vec<u8> {
    let v = validate_body_state(r.clone(), c).unwrap();
    let bytes = encode_body_state(&v, &mut budget()).unwrap();
    assert_eq!(bytes.len(), 52 + 464 * r.bodies.len());
    let loaded = decode_body_state(&bytes, c, &mut budget()).unwrap();
    assert_eq!(loaded.record(), &r);
    assert_eq!(encode_body_state(&loaded, &mut budget()).unwrap(), bytes);
    bytes
}
#[test]
fn all_bits_sentinels_options_types_and_handle_order_survive() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        with(d, |c, mut r| {
            for kind in [
                BodyKind::Dynamic,
                BodyKind::Fixed,
                BodyKind::KinematicPosition,
                BodyKind::KinematicVelocity,
            ] {
                r.bodies[0].kind = kind;
                r.bodies[0].position.translation[0] = (-0f32).to_bits();
                r.bodies[0].velocity.linear[0] = 1;
                r.bodies[0].mass.local.inverse_mass = (-0f32).to_bits();
                r.bodies[0].soft_motion_margin = (-0f32).to_bits();
                r.bodies[0].additional_iterations = [u64::MAX, 1 << 54];
                r.bodies[0].user_data = [u32::MAX; 4];
                r.bodies[0].dominance = -128;
                for a in [
                    None,
                    Some(AdditionalMass::Mass((-0f32).to_bits())),
                    Some(AdditionalMass::Properties(r.bodies[0].mass.local)),
                ] {
                    r.bodies[0].mass.additional = a;
                    check(c, r.clone());
                }
            }
            let v = validate_body_state(r.clone(), c).unwrap();
            assert_eq!(
                v.body(Handle {
                    slot: 1,
                    generation: 3
                }),
                Some(&r.bodies[0])
            );
            assert_eq!(
                v.body(Handle {
                    slot: 0,
                    generation: 1
                }),
                Some(&r.bodies[1])
            );
            for h in [
                Handle {
                    slot: 0,
                    generation: 3,
                },
                Handle {
                    slot: 1,
                    generation: 2,
                },
                Handle {
                    slot: u32::MAX,
                    generation: u32::MAX,
                },
            ] {
                assert!(v.body(h).is_none());
            }
        });
    }
}
#[test]
fn numerical_and_domain_refusals_do_not_normalize() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        with(d, |c, r| {
            for case in 0..22 {
                let mut x = r.clone();
                let b = &mut x.bodies[0];
                match case {
                    0 => b.position.translation[0] = f32::INFINITY.to_bits(),
                    1 => b.position.rotation = [0; 4],
                    2 => b.next_position.rotation = [0; 4],
                    3 => b.activation.previous_pose.rotation = [0; 4],
                    4 => b.velocity.linear[0] = f32::NAN.to_bits(),
                    5 => b.ccd_velocity.angular[0] = f32::INFINITY.to_bits(),
                    6 => b.forces.gravity_scale = f32::NEG_INFINITY.to_bits(),
                    7 => b.forces.user_torque[0] = f32::NAN.to_bits(),
                    8 => b.damping[0] = (-1f32).to_bits(),
                    9 => b.mass.local.inverse_mass = (-1f32).to_bits(),
                    10 => b.mass.local.inverse_principal_inertia[0] = (-1f32).to_bits(),
                    11 => b.mass.effective_inverse_mass[0] = (-1f32).to_bits(),
                    12 => b.mass.effective_inverse_inertia[0] = (-1f32).to_bits(),
                    13 => b.mass.additional = Some(AdditionalMass::Mass(f32::INFINITY.to_bits())),
                    14 => b.mass.max_extent = (-1f32).to_bits(),
                    15 => b.activation.time_until_sleep = (-1f32).to_bits(),
                    16 => b.activation.time_since_can_sleep = f32::INFINITY.to_bits(),
                    17 => b.ccd.thickness = f32::INFINITY.to_bits(),
                    18 => b.ccd.soft_prediction = (-1f32).to_bits(),
                    19 => b.changes = 256,
                    20 => b.mass.locked_axes = 64,
                    _ => b.dominance = 128,
                };
                assert!(validate_body_state(x, c).is_err(), "case {case} {d:?}");
            }
            let mut x = r.clone();
            x.bodies[0].ids[0] = MAX_ITEMS as u32;
            assert!(matches!(validate_body_state(x, c), Err(Error::Body)));
            // Signed off-diagonals are legal; no PSD/derived tensor claim.
            if d == Dimension::Spatial {
                let mut x = r.clone();
                x.bodies[0].mass.effective_inverse_inertia[1] = (-0.5f32).to_bits();
                check(c, x);
            }
        });
    }
}
#[test]
fn unsupported_soft_and_planar_padding_are_explicit() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        with(d, |c, r| {
            for case in 0..4 {
                let mut x = r.clone();
                match case {
                    0 => x.bodies[0].soft_body.slot = 0,
                    1 => x.bodies[0].soft_body.generation = 0,
                    2 => x.bodies[0].soft_cluster = 0,
                    _ => x.bodies[0].soft_motion_margin = 1,
                };
                assert!(matches!(
                    validate_body_state(x, c),
                    Err(Error::UnsupportedBody)
                ));
            }
        });
    }
    with(Dimension::Planar, |c, r| {
        for case in 0..7 {
            let mut x = r.clone();
            let b = &mut x.bodies[0];
            match case {
                0 => b.velocity.linear[2] = (-0f32).to_bits(),
                1 => b.ccd_velocity.angular[1] = 1,
                2 => b.forces.gyroscopic = true,
                3 => b.mass.local.principal_frame[0] = 1,
                4 => b.mass.effective_inverse_inertia[1] = 1,
                5 => b.mass.local.inverse_principal_inertia[1] = 1,
                _ => b.forces.user_torque[2] = 1,
            };
            assert!(validate_body_state(x, c).is_err());
        }
    });
}
#[test]
fn identities_pending_journals_lengths_and_budgets_precede_allocation() {
    with(Dimension::Spatial, |c, r| {
        let bytes = check(c, r.clone());
        for journals in [
            PendingJournals {
                modified_bodies: 1,
                ..Default::default()
            },
            PendingJournals {
                modified_colliders: 1,
                ..Default::default()
            },
            PendingJournals {
                removed_colliders: 1,
                ..Default::default()
            },
        ] {
            let mut b = Budget::new(0).unwrap();
            assert!(matches!(
                decode_body_state(&bytes, BodyContext { journals, ..c }, &mut b),
                Err(Error::PendingMutation)
            ));
            assert_eq!(b.remaining(), 0);
        }
        let mut x = r.clone();
        x.identity.build[0] ^= 1;
        assert!(matches!(validate_body_state(x, c), Err(Error::Identity)));
        let mut x = r.clone();
        x.bodies.pop();
        assert!(matches!(validate_body_state(x, c), Err(Error::Length)));
        for offset in [12, 44, 48] {
            let mut x = bytes.clone();
            x[offset] ^= 1;
            let mut b = Budget::new(0).unwrap();
            assert!(decode_body_state(&x, c, &mut b).is_err());
            assert_eq!(b.remaining(), 0);
        }
        let size = 2 * size_of::<RigidBodyState>();
        let mut b = Budget::new(size - 1).unwrap();
        assert!(matches!(
            decode_body_state(&bytes, c, &mut b),
            Err(Error::Limit)
        ));
        let mut b = Budget::new(size).unwrap();
        let v = decode_body_state(&bytes, c, &mut b).unwrap();
        assert_eq!(b.remaining(), 0);
        assert!(matches!(encode_body_state(&v, &mut b), Err(Error::Limit)));
        let mut b = Budget::new(size + bytes.len()).unwrap();
        let v = decode_body_state(&bytes, c, &mut b).unwrap();
        assert_eq!(encode_body_state(&v, &mut b).unwrap(), bytes);
        assert_eq!(b.remaining(), 0);
    });
}
#[test]
fn hostile_wire_tags_truncations_and_every_byte_are_checked() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        with(d, |c, r| {
            let bytes = check(c, r);
            for end in 0..bytes.len() {
                assert!(decode_body_state(&bytes[..end], c, &mut budget()).is_err());
            }
            let mut trailing = bytes.clone();
            trailing.push(0);
            assert!(decode_body_state(&trailing, c, &mut budget()).is_err());
            for (word, value) in [
                (101, 4),
                (101, u32::MAX),
                (39, 2),
                (83, 2),
                (66, 3),
                (67, 1),
            ] {
                let mut x = bytes.clone();
                x[52 + word * 4..56 + word * 4].copy_from_slice(&value.to_le_bytes());
                assert!(
                    decode_body_state(&x, c, &mut budget()).is_err(),
                    "word {word}"
                );
            }
            for i in 0..bytes.len() {
                let mut x = bytes.clone();
                x[i] ^= 0x80;
                if let Ok(v) = decode_body_state(&x, c, &mut budget()) {
                    assert_eq!(encode_body_state(&v, &mut budget()).unwrap(), x);
                }
            }
        });
    }
}

#[test]
fn empty_body_table_and_long_dense_rows_are_bounded() {
    for n in [0usize, 4096] {
        let d = Dimension::Spatial;
        let identity = id(d);
        let slots = if n == 0 {
            vec![ArenaSlot::Free { next: None }]
        } else {
            (0..n)
                .map(|i| ArenaSlot::Occupied {
                    generation: 0,
                    row: (n - 1 - i) as u32,
                })
                .collect()
        };
        let b = validate_arena(
            ArenaRecord {
                identity,
                generation: 0,
                free_head: (n == 0).then_some(0),
                live: n as u32,
                slots,
            },
            &mut budget(),
        )
        .unwrap();
        let empty = validate_arena(
            ArenaRecord {
                identity,
                generation: 0,
                free_head: Some(0),
                live: 0,
                slots: vec![ArenaSlot::Free { next: None }],
            },
            &mut budget(),
        )
        .unwrap();
        let a = validate_attachments(
            AttachmentRecord {
                identity,
                bodies: vec![AttachmentSpan { start: 0, count: 0 }; n],
                colliders: vec![],
                attached: vec![],
            },
            &b,
            &empty,
            &mut budget(),
        )
        .unwrap();
        let c = BodyContext {
            attachments: &a,
            journals: PendingJournals::default(),
        };
        let mut bodies = vec![state(d); n];
        for (i, b) in bodies.iter_mut().enumerate() {
            b.user_data[0] = i as u32;
        }
        let bytes = check(c, BodyStateTable { identity, bodies });
        let v = decode_body_state(&bytes, c, &mut budget()).unwrap();
        for i in 0..n {
            assert_eq!(
                v.body(Handle {
                    slot: i as u32,
                    generation: 0
                })
                .unwrap()
                .user_data[0],
                (n - 1 - i) as u32
            );
        }
    }
}

use incant_physics_snapshot::*;
#[path = "support/body_fixture.rs"]
mod body_fixture;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn id(d: Dimension) -> Identity {
    Identity {
        dimension: d,
        build: [83; 32],
    }
}
fn arena(d: Dimension, n: usize) -> ValidatedArena {
    validate_arena(
        ArenaRecord {
            identity: id(d),
            generation: (n as u32).max(7),
            free_head: (n == 0).then_some(0),
            live: n as u32,
            slots: if n == 0 {
                vec![ArenaSlot::Free { next: None }]
            } else {
                (0..n)
                    .map(|i| ArenaSlot::Occupied {
                        generation: i as u32,
                        row: (n - 1 - i) as u32,
                    })
                    .collect()
            },
        },
        &mut budget(),
    )
    .unwrap()
}
fn shape(d: Dimension) -> ValidatedGeometry {
    validate_geometry(GeometryRecord {
        identity: id(d),
        kind: GeometryKind::Primitive,
        parts: vec![Part {
            translation: [0; 3],
            rotation: [0, 0, 0, 1f32.to_bits()],
            shape: Primitive::Ball {
                radius: 1f32.to_bits(),
            },
        }],
    })
    .unwrap()
}
fn state(d: Dimension) -> ColliderState {
    ColliderState {
        kind: ColliderKind::Solid,
        position: body_fixture::state(d).position,
        mass: ColliderMass::Density(1f32.to_bits()),
        changes: 0,
        material: ColliderMaterial {
            friction: 0.7f32.to_bits(),
            restitution: 0,
            friction_rule: CombineRule::Average,
            restitution_rule: CombineRule::Max,
        },
        active_collision_types: 15,
        collision_groups: ColliderGroups {
            memberships: u32::MAX,
            filter: 1,
            test: GroupTest::And,
        },
        solver_groups: ColliderGroups {
            memberships: 0,
            filter: u32::MAX,
            test: GroupTest::Or,
        },
        active_hooks: 0,
        active_events: 0,
        enabled: ColliderEnabled::Enabled,
        contact_skin: 0,
        force_event_threshold: 0,
        deformable: false,
        user_data: [0; 4],
    }
}
fn with(d: Dimension, enabled: bool, f: impl FnOnce(ColliderContext<'_>, ColliderStateTable)) {
    let ba = arena(d, 1);
    let ca = arena(d, 3);
    let a = validate_attachments(
        AttachmentRecord {
            identity: id(d),
            bodies: vec![AttachmentSpan { start: 0, count: 2 }],
            colliders: vec![
                Some(ParentAttachment {
                    body: Handle {
                        slot: 0,
                        generation: 0,
                    },
                    local_pose: state(d).position,
                }),
                None,
                Some(ParentAttachment {
                    body: Handle {
                        slot: 0,
                        generation: 0,
                    },
                    local_pose: state(d).position,
                }),
            ],
            attached: vec![
                Handle {
                    slot: 0,
                    generation: 0,
                },
                Handle {
                    slot: 2,
                    generation: 2,
                },
            ],
        },
        &ba,
        &ca,
        &mut budget(),
    )
    .unwrap();
    let mut body = body_fixture::state(d);
    body.enabled = enabled;
    let b = validate_body_state(
        BodyStateTable {
            identity: id(d),
            bodies: vec![body],
        },
        BodyContext {
            attachments: &a,
            journals: PendingJournals::default(),
        },
    )
    .unwrap();
    let shapes = [shape(d), shape(d), shape(d)];
    let mut colliders = vec![state(d); 3];
    colliders[0].enabled = if enabled {
        ColliderEnabled::Enabled
    } else {
        ColliderEnabled::DisabledByParent
    };
    colliders[1].kind = ColliderKind::Sensor;
    colliders[2].enabled = ColliderEnabled::Disabled;
    f(
        ColliderContext {
            bodies: &b,
            geometries: &shapes,
            journals: PendingJournals::default(),
        },
        ColliderStateTable {
            identity: id(d),
            colliders,
        },
    );
}
fn check(c: ColliderContext<'_>, r: ColliderStateTable) -> Vec<u8> {
    let v = validate_collider_state(r.clone(), c).unwrap();
    let bytes = encode_collider_state(&v, &mut budget()).unwrap();
    assert_eq!(bytes.len(), 52 + 168 * r.colliders.len());
    let out = decode_collider_state(&bytes, c, &mut budget()).unwrap();
    assert_eq!(out.record(), &r);
    assert_eq!(encode_collider_state(&out, &mut budget()).unwrap(), bytes);
    bytes
}
#[test]
fn properties_bits_mass_modes_and_arena_order_roundtrip() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        with(d, true, |c, mut r| {
            r.colliders[0].position.translation[0] = (-0f32).to_bits();
            r.colliders[0].mass = ColliderMass::Mass((-0f32).to_bits());
            r.colliders[1].mass = ColliderMass::Properties(body_fixture::state(d).mass.local);
            r.colliders[2].mass = ColliderMass::Density(1); // positive subnormal, no normalization
            r.colliders[0].material.friction = (-0f32).to_bits();
            r.colliders[0].material.restitution = 1.5f32.to_bits(); // no invented <=1 restriction
            r.colliders[0].active_collision_types = 0xee2f;
            r.colliders[0].active_hooks = 7;
            r.colliders[0].active_events = 3;
            r.colliders[0].force_event_threshold = 2.5f32.to_bits();
            r.colliders[0].user_data = [u32::MAX, 0x87654321, 0xabcdef12, u32::MAX];
            for rule in [
                CombineRule::Average,
                CombineRule::Min,
                CombineRule::Multiply,
                CombineRule::Max,
            ] {
                r.colliders[0].material.friction_rule = rule;
                r.colliders[0].material.restitution_rule = rule;
                check(c, r.clone());
            }
            let v = validate_collider_state(r.clone(), c).unwrap();
            for (slot, row) in [(0, 2), (1, 1), (2, 0)] {
                let h = Handle {
                    slot,
                    generation: slot,
                };
                assert_eq!(v.collider(h), Some(&r.colliders[row]));
                assert_eq!(
                    v.effective_force_threshold(h),
                    Some(if row == 0 {
                        2.5f32.to_bits()
                    } else {
                        f32::MAX.to_bits()
                    })
                );
            }
            for h in [
                Handle {
                    slot: 2,
                    generation: 1,
                },
                Handle {
                    slot: u32::MAX,
                    generation: u32::MAX,
                },
            ] {
                assert!(v.collider(h).is_none());
            }
        });
    }
}
#[test]
fn complete_state_refuses_nonfinite_negative_masks_and_deformables() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        with(d, true, |c, r| {
            for case in 0..18 {
                let mut x = r.clone();
                let v = &mut x.colliders[0];
                match case {
                    0 => v.position.translation[0] = f32::INFINITY.to_bits(),
                    1 => v.position.rotation = [0; 4],
                    2 => v.mass = ColliderMass::Density(f32::NAN.to_bits()),
                    3 => v.mass = ColliderMass::Mass((-1f32).to_bits()),
                    4 => {
                        let mut m = body_fixture::state(d).mass.local;
                        m.inverse_mass = (-1f32).to_bits();
                        v.mass = ColliderMass::Properties(m);
                    }
                    5 => v.material.friction = f32::NEG_INFINITY.to_bits(),
                    6 => v.material.restitution = (-1f32).to_bits(),
                    7 => v.contact_skin = f32::NAN.to_bits(),
                    8 => v.force_event_threshold = f32::INFINITY.to_bits(),
                    9 => v.active_collision_types = 16,
                    10 => v.active_collision_types = 1 << 16,
                    11 => v.active_hooks = 8,
                    12 => v.active_events = 4,
                    13 => v.changes = 1024,
                    14 => v.deformable = true,
                    15 => v.changes = 512,
                    16 => v.changes = 1,
                    _ => v.force_event_threshold = (-1f32).to_bits(),
                }
                let result = validate_collider_state(x, c);
                if (14..=15).contains(&case) {
                    assert!(matches!(result, Err(Error::UnsupportedCollider)));
                } else if case == 16 {
                    assert!(matches!(result, Err(Error::PendingMutation)));
                } else {
                    assert!(result.is_err(), "{d:?} case {case}");
                }
            }
            if d == Dimension::Planar {
                for case in 0..3 {
                    let mut x = r.clone();
                    match case {
                        0 => x.colliders[0].position.translation[2] = (-0f32).to_bits(),
                        1 => x.colliders[0].position.rotation[2] = 1,
                        _ => {
                            let mut m = body_fixture::state(d).mass.local;
                            m.principal_frame[0] = 1;
                            x.colliders[0].mass = ColliderMass::Properties(m);
                        }
                    }
                    assert!(validate_collider_state(x, c).is_err());
                }
            }
        });
    }
}
#[test]
fn enabled_tristate_is_retained_history_not_recomputed_from_parent() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        for enabled in [false, true] {
            with(d, enabled, |c, r| {
                for status in [
                    ColliderEnabled::Enabled,
                    ColliderEnabled::DisabledByParent,
                    ColliderEnabled::Disabled,
                ] {
                    let mut x = r.clone();
                    for row in &mut x.colliders {
                        row.enabled = status;
                    }
                    check(c, x);
                }
            });
        }
    }
}
#[test]
fn bounds_context_and_budget_fail_before_record_allocation() {
    with(Dimension::Spatial, true, |c, r| {
        let bytes = check(c, r.clone());
        for offset in [12, 44, 48] {
            let mut x = bytes.clone();
            x[offset] ^= 1;
            let mut b = Budget::new(0).unwrap();
            assert!(decode_collider_state(&x, c, &mut b).is_err());
            assert_eq!(b.remaining(), 0);
        }
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
            assert!(matches!(
                decode_collider_state(
                    &bytes,
                    ColliderContext { journals, ..c },
                    &mut Budget::new(0).unwrap()
                ),
                Err(Error::PendingMutation)
            ));
        }
        assert!(matches!(
            validate_collider_state(
                r.clone(),
                ColliderContext {
                    geometries: &c.geometries[..2],
                    ..c
                }
            ),
            Err(Error::Length)
        ));
        let foreign = [
            shape(Dimension::Planar),
            shape(Dimension::Planar),
            shape(Dimension::Planar),
        ];
        assert!(matches!(
            validate_collider_state(
                r.clone(),
                ColliderContext {
                    geometries: &foreign,
                    ..c
                }
            ),
            Err(Error::Identity)
        ));
        let mut x = r.clone();
        x.identity.build[0] ^= 1;
        assert!(matches!(
            validate_collider_state(x, c),
            Err(Error::Identity)
        ));
        let mut x = r;
        x.colliders.pop();
        assert!(matches!(validate_collider_state(x, c), Err(Error::Length)));
        let size = 3 * size_of::<ColliderState>();
        assert!(matches!(
            decode_collider_state(&bytes, c, &mut Budget::new(size - 1).unwrap()),
            Err(Error::Limit)
        ));
        let mut b = Budget::new(size + bytes.len()).unwrap();
        let v = decode_collider_state(&bytes, c, &mut b).unwrap();
        assert_eq!(encode_collider_state(&v, &mut b).unwrap(), bytes);
        assert_eq!(b.remaining(), 0);
        let mut x = bytes;
        x[44..48].copy_from_slice(&((MAX_ITEMS + 1) as u32).to_le_bytes());
        assert!(matches!(
            decode_collider_state(&x, c, &mut Budget::new(0).unwrap()),
            Err(Error::Length)
        ));
    });
}
#[test]
fn every_enum_padding_truncation_and_accepted_bit_is_exact() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        with(d, true, |c, r| {
            let bytes = check(c, r);
            for end in 0..bytes.len() {
                assert!(decode_collider_state(&bytes[..end], c, &mut budget()).is_err());
            }
            let mut x = bytes.clone();
            x.push(0);
            assert!(decode_collider_state(&x, c, &mut budget()).is_err());
            // Every scalar enum and the inactive density/mass payload are checked.
            for (word, value) in [
                (0, 2u32),
                (8, 3),
                (10, 1),
                (23, 4),
                (24, 4),
                (28, 2),
                (31, 2),
                (34, 3),
                (37, 2),
            ] {
                let mut x = bytes.clone();
                x[52 + word * 4..56 + word * 4].copy_from_slice(&value.to_le_bytes());
                assert!(
                    decode_collider_state(&x, c, &mut budget()).is_err(),
                    "word {word}"
                );
            }
            for i in 0..bytes.len() {
                let mut x = bytes.clone();
                x[i] ^= 0x80;
                if let Ok(v) = decode_collider_state(&x, c, &mut budget()) {
                    assert_eq!(encode_collider_state(&v, &mut budget()).unwrap(), x);
                }
            }
        });
    }
}
#[test]
fn empty_and_dense_standalone_tables_keep_exact_row_order() {
    for n in [0, 4096] {
        let d = Dimension::Spatial;
        let ba = arena(d, 0);
        let ca = arena(d, n);
        let a = validate_attachments(
            AttachmentRecord {
                identity: id(d),
                bodies: vec![],
                colliders: vec![None; n],
                attached: vec![],
            },
            &ba,
            &ca,
            &mut budget(),
        )
        .unwrap();
        let b = validate_body_state(
            BodyStateTable {
                identity: id(d),
                bodies: vec![],
            },
            BodyContext {
                attachments: &a,
                journals: PendingJournals::default(),
            },
        )
        .unwrap();
        let shapes: Vec<_> = (0..n).map(|_| shape(d)).collect();
        let c = ColliderContext {
            bodies: &b,
            geometries: &shapes,
            journals: PendingJournals::default(),
        };
        let mut rows = vec![state(d); n];
        for (i, r) in rows.iter_mut().enumerate() {
            r.user_data[0] = i as u32;
        }
        let bytes = check(
            c,
            ColliderStateTable {
                identity: id(d),
                colliders: rows,
            },
        );
        let v = decode_collider_state(&bytes, c, &mut budget()).unwrap();
        for i in 0..n {
            assert_eq!(
                v.collider(Handle {
                    slot: i as u32,
                    generation: i as u32
                })
                .unwrap()
                .user_data[0],
                (n - 1 - i) as u32
            );
        }
    }
}

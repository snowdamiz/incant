use incant_physics_snapshot::*;
#[path = "support/body_fixture.rs"]
mod body_fixture;
#[path = "support/manifold_fixture.rs"]
mod fixture;
use fixture::{budget, h, id};
fn record(d: Dimension) -> IslandTable {
    IslandTable {
        identity: id(d),
        active_epoch: u32::MAX,
        awake: Some(1),
        sleep_scan_stamp: 77,
        pending_split: Some(0),
        bootstrapped: true,
        pending_removals: 0,
        joint_links: 0,
        joint_locations: 0,
        active_live: 1,
        persistent_live: 1,
        active: vec![None, Some(RecordSpan { start: 0, count: 2 }), None],
        active_free: vec![2, 0],
        active_bodies: vec![h(1), h(0)],
        persistent: vec![
            Some(PersistentIslandRow {
                bodies: RecordSpan { start: 0, count: 2 },
                contacts: RecordSpan { start: 0, count: 1 },
                constraint_remove_count: 7,
                split_denied_until: 78,
                sleeping: false,
            }),
            None,
        ],
        persistent_free: vec![1],
        persistent_bodies: vec![h(0), h(1)],
        contacts: vec![IslandContact {
            edge: 0,
            bodies: [h(0), h(1)],
        }],
        locations: vec![Some(IslandLocation {
            island: 0,
            index: 0,
        })],
    }
}
fn with(d: Dimension, f: impl FnOnce(IslandContext<'_>, IslandTable)) {
    with_rows(d, |_| {}, f);
}
fn with_rows(
    d: Dimension,
    change: impl FnOnce(&mut [RigidBodyState]),
    f: impl FnOnce(IslandContext<'_>, IslandTable),
) {
    fixture::with(d, |c, t| {
        let m = validate_manifolds(t, c, &mut budget()).unwrap();
        let mut rows = vec![body_fixture::state(d); 2];
        rows[0].ids = [1, 1, 0, 0];
        rows[1].ids = [1, 0, 0, 1];
        change(&mut rows);
        let b = validate_body_state(
            BodyStateTable {
                identity: id(d),
                bodies: rows,
            },
            BodyContext {
                attachments: c.attachments,
                journals: c.journals,
            },
        )
        .unwrap();
        f(
            IslandContext {
                bodies: &b,
                manifolds: &m,
                domain: IslandDomain::default(),
            },
            record(d),
        );
    });
}
fn bytes(c: IslandContext<'_>, r: IslandTable) -> Vec<u8> {
    let v = validate_islands(r, c, &mut budget()).unwrap();
    encode_islands(&v, &mut budget()).unwrap()
}
#[test]
fn exact_order_holes_counters_and_checked_reverse_views() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        with(d, |c, r| {
            let data = bytes(c, r.clone());
            let v = decode_islands(&data, c, &mut budget()).unwrap();
            assert_eq!(v.record(), &r);
            assert_eq!(encode_islands(&v, &mut budget()).unwrap(), data);
            assert_eq!(
                v.body_membership(h(0)),
                Some([
                    Some(IslandLocation {
                        island: 1,
                        index: 1
                    }),
                    Some(IslandLocation {
                        island: 0,
                        index: 0
                    })
                ])
            );
            assert_eq!(v.contact_location(0), r.locations[0]);
            assert_eq!(v.contact_location(u32::MAX), None);
            assert_eq!(
                v.body_membership(Handle {
                    slot: 0,
                    generation: 2
                }),
                None
            );
            let mut reordered = r;
            reordered.active_free.reverse();
            assert_ne!(bytes(c, reordered), data);
        });
    }
}
#[test]
fn membership_free_stack_spans_and_generation_forgeries_refuse() {
    with(Dimension::Spatial, |c, r| {
        for n in 0..15 {
            let mut v = r.clone();
            match n {
                0 => v.active_free[0] = 0,
                1 => v.active_free[0] = 1,
                2 => v.active_live = 2,
                3 => v.active_bodies.swap(0, 1),
                4 => v.active_bodies[0] = v.active_bodies[1],
                5 => v.active_bodies[0].generation = 2,
                6 => v.persistent_bodies.swap(0, 1),
                7 => v.persistent_free.clear(),
                8 => v.persistent[0].as_mut().unwrap().bodies.start = 1,
                9 => v.active[1].as_mut().unwrap().count = 0,
                10 => v.awake = Some(2),
                11 => v.pending_split = Some(1),
                12 => v.persistent[0].as_mut().unwrap().sleeping = true,
                13 => v.active_bodies.pop().map(|_| ()).unwrap(),
                _ => v.persistent[0].as_mut().unwrap().contacts.count = 0,
            }
            assert!(
                validate_islands(v, c, &mut budget()).is_err(),
                "mutation {n}"
            );
        }
    });
}
#[test]
fn contact_locations_endpoints_and_full_touching_coverage_refuse_forgery() {
    with(Dimension::Spatial, |c, r| {
        for n in 0..8 {
            let mut v = r.clone();
            match n {
                0 => v.contacts[0].edge = 1,
                1 => v.contacts[0].bodies.swap(0, 1),
                2 => v.contacts[0].bodies[0].generation = 2,
                3 => v.locations[0] = None,
                4 => v.locations[0].as_mut().unwrap().index = 1,
                5 => v.locations[0].as_mut().unwrap().island = 1,
                6 => v.locations.clear(),
                _ => {
                    v.contacts.clear();
                    v.locations.clear();
                    v.persistent[0].as_mut().unwrap().contacts.count = 0;
                }
            }
            assert!(
                validate_islands(v, c, &mut budget()).is_err(),
                "mutation {n}"
            );
        }
    });
}
#[test]
fn unsupported_domain_and_phase_refuse_before_decoder_allocation() {
    with(Dimension::Spatial, |c, r| {
        let data = bytes(c, r.clone());
        for domain in [
            IslandDomain {
                impulse_joints: 1,
                ..Default::default()
            },
            IslandDomain {
                multibody_joints: 1,
                ..Default::default()
            },
            IslandDomain {
                soft_bodies: 1,
                ..Default::default()
            },
        ] {
            let mut b = budget();
            assert!(matches!(
                decode_islands(&data, IslandContext { domain, ..c }, &mut b),
                Err(Error::UnsupportedIsland)
            ));
            assert_eq!(b.remaining(), MAX_ALLOCATION);
        }
        for n in 0..4 {
            let mut v = r.clone();
            match n {
                0 => v.bootstrapped = false,
                1 => v.joint_links = 1,
                2 => v.joint_locations = 1,
                _ => v.pending_removals = 1,
            }
            let e = if n == 3 {
                Error::PendingMutation
            } else {
                Error::UnsupportedIsland
            };
            assert!(matches!(validate_islands(v,c,&mut budget()),Err(x) if x==e));
            let mut d = data.clone();
            let offset = [60, 68, 72, 64][n];
            d[offset..offset + 4].copy_from_slice(&(if n == 0 { 0u32 } else { 1 }).to_le_bytes());
            let mut b = budget();
            assert!(matches!(decode_islands(&d,c,&mut b),Err(x) if x==e));
            assert_eq!(b.remaining(), MAX_ALLOCATION);
        }
    });
}
#[test]
fn checked_body_witness_not_just_handle_shape_binds_membership() {
    for n in 0..5 {
        with_rows(
            Dimension::Spatial,
            |rows| match n {
                0 => rows[0].ids[0] = 0,
                1 => rows[0].ids[3] = 1,
                2 => rows[0].enabled = false,
                3 => rows[0].kind = BodyKind::Fixed,
                _ => rows[0].ids = [u32::MAX; 4],
            },
            |c, r| {
                assert!(matches!(
                    validate_islands(r, c, &mut budget()),
                    Err(Error::Island)
                ));
            },
        );
    }
    // Equal-but-separate attachment witnesses cannot be accidentally combined.
    with(Dimension::Spatial, |c, r| {
        fixture::with(Dimension::Spatial, |foreign, _| {
            let b = validate_body_state(
                c.bodies.record().clone(),
                BodyContext {
                    attachments: foreign.attachments,
                    journals: Default::default(),
                },
            )
            .unwrap();
            assert!(matches!(
                validate_islands(r, IslandContext { bodies: &b, ..c }, &mut budget()),
                Err(Error::Identity)
            ));
        });
    });
}
#[test]
fn hostile_header_truncation_padding_and_aggregate_budget() {
    with(Dimension::Spatial, |c, r| {
        let data = bytes(c, r.clone());
        for len in 0..data.len() {
            assert!(decode_islands(&data[..len], c, &mut budget()).is_err());
        }
        for offset in [0, 8, 10, 12, 76, 80, 84, 88, 92, 96, 100, 104, 108, 112] {
            let mut d = data.clone();
            d[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            let mut b = budget();
            assert!(decode_islands(&d, c, &mut b).is_err());
            assert_eq!(b.remaining(), MAX_ALLOCATION);
        }
        // Absent active slot has canonical zero payload; malformed location half-sentinel is refused.
        for offset in [116, 120, data.len() - 4] {
            let mut d = data.clone();
            d[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            assert!(decode_islands(&d, c, &mut budget()).is_err());
        }
        let mut b = Budget::new(8).unwrap();
        assert!(matches!(
            validate_islands(r.clone(), c, &mut b),
            Err(Error::Limit)
        ));
        let mut b = Budget::new(MAX_ALLOCATION).unwrap();
        let v = decode_islands(&data, c, &mut b).unwrap();
        let remaining = b.remaining();
        let encoded = encode_islands(&v, &mut b).unwrap();
        assert_eq!(remaining - b.remaining(), encoded.len());
        let mut too_short = Budget::new(MAX_ALLOCATION - remaining - 1).unwrap();
        assert!(matches!(
            decode_islands(&data, c, &mut too_short),
            Err(Error::Limit)
        ));
    });
}
#[test]
fn sleep_scan_wrap_model_is_a_continuation_caveat_not_a_native_restore_test() {
    // Exact pinned branch arithmetic: retained stamp 7 is touched on wrap to zero;
    // a newly zero-filled omitted entry is not. This only models that branch.
    fn observe(slot: &mut (u32, bool), stamp: u32, eligible: bool) -> bool {
        if slot.0 != stamp {
            *slot = (stamp, eligible);
            true
        } else {
            slot.1 &= eligible;
            false
        }
    }
    let stamp = u32::MAX.wrapping_add(1);
    let mut retained = (7, true);
    let mut reset = (0, false);
    assert!(observe(&mut retained, stamp, true));
    assert!(!observe(&mut reset, stamp, true));
    assert_ne!(retained, reset);
}
#[test]
fn fixed_active_history_and_manual_sleep_are_distinct_from_persistent_membership() {
    with_rows(
        Dimension::Spatial,
        |rows| {
            rows[0].kind = BodyKind::Fixed;
            rows[0].ids[2..].fill(u32::MAX);
            rows[1].ids[3] = 0;
        },
        |c, mut r| {
            r.persistent_bodies.remove(0);
            r.persistent[0].as_mut().unwrap().bodies.count = 1;
            let v = validate_islands(r, c, &mut budget()).unwrap();
            assert_eq!(
                v.body_membership(h(0)),
                Some([
                    Some(IslandLocation {
                        island: 1,
                        index: 1
                    }),
                    None
                ])
            );
        },
    );
    with_rows(
        Dimension::Spatial,
        |rows| rows[0].activation.sleeping = true,
        |c, r| {
            assert!(validate_islands(r, c, &mut budget()).is_ok());
        },
    );
}

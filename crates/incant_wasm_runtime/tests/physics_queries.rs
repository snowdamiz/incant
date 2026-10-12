#[path = "support/physics.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_abi::{self as abi, physics as wire};
use incant_wasm_runtime::{BoundaryError, Call, Capability};
fn query(
    cap: &mut Capability,
    m: &mut [u8],
    e: i64,
    rows: &[abi::PhysicsQuery],
) -> Result<i32, BoundaryError> {
    let n = wire::encode_batch(rows, m).unwrap();
    cap.invoke(
        m,
        e,
        Call::PhysicsQuery {
            ptr: 0,
            len: n as i32,
        },
    )
}
fn read(cap: &mut Capability, m: &mut [u8], e: i64, n: i32) -> Vec<u8> {
    m.fill(0xa5);
    assert_eq!(
        cap.invoke(
            m,
            e,
            Call::ReadPhysics {
                ptr: 1,
                capacity: n + 1
            }
        )
        .unwrap(),
        n
    );
    assert_eq!(m[0], 0xa5);
    assert_eq!(m[n as usize + 1], 0xa5);
    m[1..n as usize + 1].to_vec()
}
fn prefix(cap: &mut Capability, m: &mut [u8], e: i64) {
    cap.invoke(
        m,
        e,
        Call::Column {
            chunk: 0,
            ptr: 1024,
            velocity: false,
            write: false,
        },
    )
    .unwrap();
    m[1024..1032].copy_from_slice(&42f64.to_le_bytes());
    cap.invoke(
        m,
        e,
        Call::Column {
            chunk: 0,
            ptr: 1024,
            velocity: false,
            write: true,
        },
    )
    .unwrap();
    m[..16].copy_from_slice(&id(20).0);
    cap.invoke(m, e, Call::Despawn(0)).unwrap();
    cap.invoke(
        m,
        e,
        Call::Log {
            level: 1,
            ptr: 0,
            len: 0,
        },
    )
    .unwrap();
}
#[test]
fn mixed_copied_queries_match_native_solver_and_pending_writes_stay_invisible() {
    for slope in [false, true] {
        let mut w = NativeWorld::new(scene(slope));
        w.step().unwrap();
        let mut cap = Capability::default();
        let e = cap.begin(&mut w, DT).unwrap();
        let mut m = vec![0; 16384];
        w.with_numeric_physics(|_, q| {
            let expected3 = q.raycast(&ray3()).unwrap().unwrap();
            let expected2 = q.raycast_2d(&ray2()).unwrap().unwrap();
            let character = q.compute_character_motion(&character(), DT).unwrap();
            cap.bind_physics(e, q.clone()).unwrap();
            let n = query(&mut cap, &mut m, e, &rays()).unwrap();
            let before = read(&mut cap, &mut m, e, n);
            let results = wire::decode_results(&before)
                .unwrap()
                .iter()
                .collect::<Vec<_>>();
            assert_eq!(results[0].0.entity, expected3.entity_id.0);
            assert_eq!(results[0].0.distance, expected3.distance);
            assert_eq!(results[0].0.point, expected3.point);
            assert_eq!(results[0].0.normal, expected3.normal);
            assert_eq!(results[2].0.flags, 0);
            assert_eq!(results[3].0.entity, expected2.entity.0);
            assert_eq!(&results[3].0.point[..2], &expected2.point);
            assert_eq!(results[5].0.flags, 0);
            assert_eq!(results[6].0.point, character.translation);
            assert_eq!(
                results[6].1,
                character
                    .collisions
                    .iter()
                    .map(|id| id.0)
                    .collect::<Vec<_>>()
            );
            prefix(&mut cap, &mut m, e);
            let n = query(&mut cap, &mut m, e, &rays()).unwrap();
            assert_eq!(read(&mut cap, &mut m, e, n), before);
            for dimension in [2, 3] {
                let n = cap
                    .invoke(&mut m, e, Call::PhysicsEvents { dimension })
                    .unwrap();
                let bytes = read(&mut cap, &mut m, e, n);
                let events = wire::decode_events(&bytes).unwrap();
                assert!(!events.is_empty());
                assert_eq!(events.scene, id(99).0);
                assert_eq!(events.dimension, dimension as u32);
                let n = cap
                    .invoke(&mut m, e, Call::PhysicsEvents { dimension })
                    .unwrap();
                assert_eq!(read(&mut cap, &mut m, e, n), bytes);
            }
            cap.revoke_physics();
        })
        .unwrap();
        cap.finish(&mut w).unwrap();
        assert!(w.inspect(id(20)).is_none());
    }
}
#[test]
fn world_identity_stale_reentrant_expired_and_unwind_controls_restore_owners() {
    for mode in ["different", "reentrant", "expired", "unwind"] {
        let mut w = NativeWorld::new(scene(false));
        let before = w.snapshot();
        let mut other = NativeWorld::new(scene(false));
        let mut cap = Capability::default();
        let e = cap.begin(&mut w, DT).unwrap();
        let mut m = vec![0; 16384];
        prefix(&mut cap, &mut m, e);
        match mode {
            "different" => {
                other
                    .with_numeric_physics(|_, q| {
                        assert!(cap.bind_physics(e, q).is_err());
                    })
                    .unwrap();
            }
            "reentrant" => {
                w.with_numeric_physics(|_, q| {
                    cap.bind_physics(e, q.clone()).unwrap();
                    assert!(cap.bind_physics(e, q).is_err());
                    cap.revoke_physics();
                })
                .unwrap();
            }
            "expired" => {
                let old = w.with_numeric_physics(|_, q| q).unwrap();
                assert!(!old.is_active());
                assert!(cap.bind_physics(e, old).is_err());
            }
            _ => {
                let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    w.with_numeric_physics(|_, q| {
                        cap.bind_physics(e, q).unwrap();
                        panic!("guard control");
                    })
                    .unwrap();
                }));
                assert!(caught.is_err());
                assert!(query(&mut cap, &mut m, e, &rays()).is_err());
            }
        }
        assert!(cap.finish(&mut w).is_err());
        assert_eq!(w.snapshot(), before);
        w.with_numeric_physics(|_, q| assert!(q.raycast(&ray3()).unwrap().is_some()))
            .unwrap();
        w.step().unwrap();
    }
    let mut w = NativeWorld::new(scene(false));
    let mut cap = Capability::default();
    let mut m = vec![0; 16384];
    let e = cap.begin(&mut w, DT).unwrap();
    let old = w
        .with_numeric_physics(|_, q| {
            assert!(cap.bind_physics(e - 1, q.clone()).is_err());
            cap.bind_physics(e, q.clone()).unwrap();
            let n = query(&mut cap, &mut m, e, &rays()).unwrap();
            let before = read(&mut cap, &mut m, e, n);
            assert!(
                cap.invoke(&mut m, e - 1, Call::PhysicsEvents { dimension: 4 })
                    .is_err()
            );
            assert_eq!(read(&mut cap, &mut m, e, n), before);
            cap.revoke_physics();
            q
        })
        .unwrap();
    cap.finish(&mut w).unwrap();
    assert!(!old.is_active());
    assert!(old.raycast(&ray3()).is_err());
    assert!(
        cap.invoke(&mut m, e, Call::PhysicsEvents { dimension: 2 })
            .is_err()
    );
    let next = cap.begin(&mut w, DT).unwrap();
    assert_ne!(e, next);
    w.with_numeric_physics(|_, q| {
        cap.bind_physics(next, q).unwrap();
        assert!(old.raycast(&ray3()).is_err());
        query(&mut cap, &mut m, next, &rays()).unwrap();
        cap.revoke_physics();
    })
    .unwrap();
    cap.finish(&mut w).unwrap();
}
#[test]
fn malformed_and_native_failures_clear_previous_reply_and_reject_whole_prefix() {
    for mode in 0..12 {
        let mut w = NativeWorld::new(scene(false));
        w.step().unwrap();
        let before = w.snapshot();
        let state = w.physics_state(id(11));
        let events = w.physics_events().to_vec();
        let mut cap = Capability::default();
        let mut m = vec![0; 16384];
        let e = cap.begin(&mut w, DT).unwrap();
        w.with_numeric_physics(|_, q| {
            cap.bind_physics(e, q).unwrap();
            prefix(&mut cap, &mut m, e);
            query(&mut cap, &mut m, e, &rays()).unwrap();
            let n = wire::encode_batch(&rays(), &mut m).unwrap();
            let result = match mode {
                0 => cap.invoke(
                    &mut m,
                    e,
                    Call::PhysicsQuery {
                        ptr: -1,
                        len: n as i32,
                    },
                ),
                1 => cap.invoke(&mut m, e, Call::PhysicsQuery { ptr: 0, len: -1 }),
                2 => {
                    m[n - 8] = 1;
                    cap.invoke(
                        &mut m,
                        e,
                        Call::PhysicsQuery {
                            ptr: 0,
                            len: n as i32,
                        },
                    )
                }
                3 => {
                    let mut rows = rays();
                    rows[6].entity = id(12).0;
                    query(&mut cap, &mut m, e, &rows)
                }
                4 => {
                    let mut rows = rays();
                    rows[6].a = [9999., 0., 0.];
                    query(&mut cap, &mut m, e, &rows)
                }
                5 => cap.invoke(&mut m, e, Call::PhysicsEvents { dimension: 4 }),
                6 => cap.invoke(
                    &mut m,
                    e,
                    Call::ReadPhysics {
                        ptr: 0,
                        capacity: 1,
                    },
                ),
                7 => cap.invoke(
                    &mut m,
                    e,
                    Call::ReadPhysics {
                        ptr: 16000,
                        capacity: 10000,
                    },
                ),
                8 => {
                    m[16 + 24..16 + 32].copy_from_slice(&1_000_000.000001f64.to_le_bytes());
                    cap.invoke(
                        &mut m,
                        e,
                        Call::PhysicsQuery {
                            ptr: 0,
                            len: n as i32,
                        },
                    )
                }
                9 => {
                    m[16 + 48..16 + 56].copy_from_slice(&f64::NAN.to_le_bytes());
                    cap.invoke(
                        &mut m,
                        e,
                        Call::PhysicsQuery {
                            ptr: 0,
                            len: n as i32,
                        },
                    )
                }
                10 => {
                    m[0] = 0;
                    cap.invoke(
                        &mut m,
                        e,
                        Call::PhysicsQuery {
                            ptr: 0,
                            len: n as i32,
                        },
                    )
                }
                _ => {
                    cap.revoke_physics();
                    cap.invoke(
                        &mut m,
                        e,
                        Call::ReadPhysics {
                            ptr: 0,
                            capacity: 16000,
                        },
                    )
                }
            };
            assert!(result.is_err(), "{mode}");
            m.fill(0xa5);
            assert!(
                cap.invoke(
                    &mut m,
                    e,
                    Call::ReadPhysics {
                        ptr: 0,
                        capacity: 16000
                    }
                )
                .is_err()
            );
            assert!(m.iter().all(|b| *b == 0xa5));
            cap.revoke_physics();
        })
        .unwrap();
        assert!(cap.finish(&mut w).is_err());
        assert_eq!(w.snapshot(), before, "{mode}");
        assert_eq!(w.physics_state(id(11)), state);
        assert_eq!(w.physics_events(), events);
        assert!(cap.logs().is_err());
    }
}
#[test]
fn read_before_query_character_and_shared_units_are_exact() {
    for mode in ["empty", "units", "character"] {
        let mut w = NativeWorld::new(scene(false));
        let before = w.snapshot();
        let mut cap = Capability::default();
        let e = cap.begin(&mut w, DT).unwrap();
        let mut m = vec![0; 16384];
        w.with_numeric_physics(|_, q| {
            cap.bind_physics(e, q).unwrap();
            prefix(&mut cap, &mut m, e);
            match mode {
                "empty" => assert!(
                    cap.invoke(
                        &mut m,
                        e,
                        Call::ReadPhysics {
                            ptr: 0,
                            capacity: 10000
                        }
                    )
                    .is_err()
                ),
                "units" => {
                    for _ in 0..16 {
                        query(&mut cap, &mut m, e, &[rays()[0]; 64]).unwrap();
                    }
                    assert!(
                        cap.invoke(&mut m, e, Call::PhysicsEvents { dimension: 2 })
                            .is_err()
                    );
                }
                _ => {
                    for _ in 0..2 {
                        query(&mut cap, &mut m, e, &[rays()[6]; 64]).unwrap();
                    }
                    assert!(query(&mut cap, &mut m, e, &[rays()[6]]).is_err());
                }
            }
            cap.revoke_physics();
        })
        .unwrap();
        assert!(cap.finish(&mut w).is_err());
        assert_eq!(w.snapshot(), before);
    }
}
#[test]
fn physics_navigation_and_localization_share_the_exact_native_unit_pool() {
    let mut w = NativeWorld::new(scene(false));
    let before = w.snapshot();
    let mut cap = Capability::default();
    let e = cap.begin(&mut w, DT).unwrap();
    let mut m = vec![0; 16384];
    w.with_numeric_physics(|_, q| {
        cap.bind_physics(e, q).unwrap();
        prefix(&mut cap, &mut m, e);
        let n = abi::steering::encode_request(
            abi::steering::Request {
                agents: &[],
                obstacles: &[],
                time_horizon: 2.,
                obstacle_time_horizon: 2.,
                margin: 0.,
            },
            &mut m,
        )
        .unwrap();
        cap.invoke(
            &mut m,
            e,
            Call::Steer {
                ptr: 0,
                len: n as i32,
            },
        )
        .unwrap();
        cap.invoke(&mut m, e, Call::Locale).unwrap();
        for _ in 0..15 {
            query(&mut cap, &mut m, e, &[rays()[0]; 64]).unwrap();
        }
        query(&mut cap, &mut m, e, &[rays()[0]; 59]).unwrap();
        assert!(matches!(
            cap.invoke(&mut m, e, Call::PhysicsEvents { dimension: 2 }),
            Err(BoundaryError::Limit("query units"))
        ));
        cap.revoke_physics();
    })
    .unwrap();
    assert!(cap.finish(&mut w).is_err());
    assert_eq!(w.snapshot(), before);
}
#[test]
fn memory_and_host_call_failures_poison_physics_and_preserve_destination() {
    for oversized_memory in [false, true] {
        let mut w = NativeWorld::new(scene(false));
        let before = w.snapshot();
        let mut cap = Capability::default();
        let e = cap.begin(&mut w, DT).unwrap();
        let mut m = vec![0; 16384];
        w.with_numeric_physics(|_, q| {
            cap.bind_physics(e, q).unwrap();
            prefix(&mut cap, &mut m, e);
            query(&mut cap, &mut m, e, &rays()).unwrap();
            if oversized_memory {
                m.resize(abi::MEMORY + 1, 0xa5);
            } else {
                while cap.invoke(&mut m, e, Call::ChunkCount).is_ok() {}
            }
            m.fill(0xa5);
            assert!(matches!(
                cap.invoke(
                    &mut m,
                    e,
                    Call::ReadPhysics {
                        ptr: 0,
                        capacity: 10000
                    }
                ),
                Err(BoundaryError::Limit(_))
            ));
            assert!(m.iter().all(|b| *b == 0xa5));
            cap.revoke_physics();
        })
        .unwrap();
        assert!(cap.finish(&mut w).is_err());
        assert_eq!(w.snapshot(), before);
    }
}
#[test]
fn transfer_exhaustion_has_no_partial_final_copy_or_commit() {
    let mut w = NativeWorld::new(scene(false));
    let before = w.snapshot();
    let mut cap = Capability::default();
    let e = cap.begin(&mut w, DT).unwrap();
    let mut m = vec![0; 16384];
    w.with_numeric_physics(|_, q| {
        cap.bind_physics(e, q).unwrap();
        prefix(&mut cap, &mut m, e);
        let len = query(&mut cap, &mut m, e, &[rays()[0]; 64]).unwrap();
        let mut reads = 0;
        loop {
            m.fill(0xa5);
            match cap.invoke(
                &mut m,
                e,
                Call::ReadPhysics {
                    ptr: 0,
                    capacity: len,
                },
            ) {
                Ok(n) => {
                    assert_eq!(n, len);
                    reads += 1;
                }
                Err(error) => {
                    assert!(matches!(error, BoundaryError::Limit("transfer quota")));
                    assert!(m.iter().all(|b| *b == 0xa5));
                    break;
                }
            }
        }
        assert!(reads > 10_000);
        cap.revoke_physics();
    })
    .unwrap();
    assert!(cap.finish(&mut w).is_err());
    assert_eq!(w.snapshot(), before);
}

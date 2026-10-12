#![cfg(feature = "static-conformance")]
#[path = "../../incant_wasm_runtime/tests/support/physics.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_dev::DevModuleHost;
use incant_wasm_runtime::{StaticModule, embedded};
fn guest(name: &str) -> Vec<u8> {
    std::fs::read(std::path::Path::new(env!("INCANT_RUST_SDK_GUESTS")).join(format!("{name}.wasm")))
        .unwrap()
}
fn hit<const N: usize>(id: StableId, d: f64, p: [f64; N], normal: [f64; N]) -> f64 {
    let mut n = d + f64::from(id.0[15]);
    for (i, (&p, &v)) in p.iter().zip(&normal).enumerate() {
        n += p * (i + 1) as f64 + v * (i + 5) as f64;
    }
    n
}
fn oracle(w: &mut NativeWorld) {
    let (checksum, motion, event_score) = w
        .with_numeric_physics(|_, q| {
            let mut checksum = 0.;
            for (i, query) in [
                ray3(),
                RayQuery {
                    include_sensors: true,
                    ..ray3()
                },
                RayQuery {
                    filter: 0,
                    ..ray3()
                },
            ]
            .into_iter()
            .enumerate()
            {
                let score = q
                    .raycast(&query)
                    .unwrap()
                    .map_or(-1., |h| hit(h.entity_id, h.distance, h.point, h.normal));
                checksum += (i + 1) as f64 * score;
            }
            for (i, query) in [
                ray2(),
                RayQuery2D {
                    include_sensors: true,
                    ..ray2()
                },
                RayQuery2D {
                    origin: [1.5, 3.],
                    ..ray2()
                },
            ]
            .into_iter()
            .enumerate()
            {
                let score = q
                    .raycast_2d(&query)
                    .unwrap()
                    .map_or(-1., |h| hit(h.entity, h.distance, h.point, h.normal));
                checksum += (i + 4) as f64 * score;
            }
            let c = q.compute_character_motion(&character(), DT).unwrap();
            let mut n =
                f64::from(u8::from(c.grounded)) + 2. * f64::from(u8::from(c.sliding_down_slope));
            for (i, v) in c.translation.iter().enumerate() {
                n += v * (i + 1) as f64;
            }
            for id in &c.collisions {
                n += f64::from(id.0[15]);
            }
            checksum += 7. * n;
            let events2 = q
                .trigger_events_2d()
                .unwrap()
                .iter()
                .map(|e| {
                    f64::from(e.first.0[15])
                        + f64::from(e.second.0[15]) * 7.
                        + f64::from(u8::from(e.entered)) * 101.
                })
                .sum::<f64>();
            let events3 = q
                .trigger_events()
                .unwrap()
                .iter()
                .map(|e| {
                    f64::from(e.first.0[15])
                        + f64::from(e.second.0[15]) * 7.
                        + f64::from(u8::from(e.entered)) * 101.
                })
                .sum::<f64>();
            (checksum, c.translation, events2 + events3 * 1000.)
        })
        .unwrap();
    w.with_numeric_columns(|columns| {
        columns.for_each_chunk(|chunk| {
            for (entity, t) in chunk
                .ids
                .iter()
                .zip(chunk.transforms.as_chunks_mut::<10>().0)
            {
                if *entity == id(20) {
                    t[0] = checksum;
                    t[1] = event_score;
                    t[2] += 1.;
                }
                if *entity == id(11) {
                    for i in 0..3 {
                        t[i] += motion[i];
                    }
                }
                if *entity == id(2) {
                    t[0] += 10.;
                }
            }
        })
    });
    w.apply(&mut FrameCommands::default()).unwrap();
}
fn compare(a: &mut NativeWorld, b: &mut NativeWorld) {
    assert_eq!(a.snapshot(), b.snapshot());
    assert_eq!(a.physics_state(id(11)), b.physics_state(id(11)));
    assert_eq!(a.physics_2d_state(id(2)), b.physics_2d_state(id(2)));
    assert_eq!(a.physics_events(), b.physics_events());
    assert_eq!(a.physics_2d_events(), b.physics_2d_events());
}
#[test]
fn real_guests_match_direct_native_oracles_for_rays_character_slopes_and_trigger_steps() {
    for slope in [false, true] {
        let mut a = NativeWorld::new(scene(slope));
        let mut b = NativeWorld::new(scene(slope));
        let mut expected = NativeWorld::new(scene(slope));
        let mut native = StaticModule::new(embedded::linked("rust-physics").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest("rust-physics")).unwrap();
        for w in [&mut a, &mut b, &mut expected] {
            w.step().unwrap();
        }
        assert!(expected.physics_events().iter().any(|e| e.entered));
        assert!(expected.physics_2d_events().iter().any(|e| e.entered));
        let mut exited2 = false;
        let mut exited3 = false;
        for turn in 0..4 {
            oracle(&mut expected);
            native.tick(&mut a, DT).unwrap();
            dev.tick(&mut b, DT).unwrap();
            compare(&mut a, &mut expected);
            compare(&mut b, &mut expected);
            assert_eq!(
                native.logs().unwrap().entries(),
                dev.logs().unwrap().entries()
            );
            assert_eq!(native.logs().unwrap().entries().len(), turn + 1);
            if turn == 1 {
                dev.reload(&guest("rust-physics")).unwrap();
                for w in [&mut a, &mut b, &mut expected] {
                    w.with_numeric_columns(|cols| {
                        cols.for_each_chunk(|c| {
                            for (entity, t) in
                                c.ids.iter().zip(c.transforms.as_chunks_mut::<10>().0)
                            {
                                if *entity == id(11) {
                                    t[2] += 10.;
                                }
                            }
                        })
                    });
                    w.apply(&mut FrameCommands::default()).unwrap();
                }
            }
            for w in [&mut a, &mut b, &mut expected] {
                w.step().unwrap();
            }
            exited2 |= expected.physics_2d_events().iter().any(|e| !e.entered);
            exited3 |= expected.physics_events().iter().any(|e| !e.entered);
            compare(&mut a, &mut expected);
            compare(&mut b, &mut expected);
        }
        assert!(exited2 && exited3);
    }
}
#[test]
fn real_sdk_errors_native_queries_and_late_failures_restore_complete_world_and_solver_state() {
    for mode in [
        "capacity",
        "request-capacity",
        "events-capacity",
        "nan",
        "rounding",
        "character-rounding",
        "owner",
        "velocity",
        "units",
        "character-units",
        "late",
        "callback",
        "trap",
    ] {
        let name = format!("rust-physics-{mode}");
        let mut a = NativeWorld::new(scene(false));
        let mut b = NativeWorld::new(scene(false));
        let mut expected = NativeWorld::new(scene(false));
        for w in [&mut a, &mut b, &mut expected] {
            w.step().unwrap();
        }
        let mut native = StaticModule::new(embedded::linked(&name).unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&guest(&name)).unwrap();
        assert!(native.tick(&mut a, DT).is_err(), "{mode}");
        assert!(dev.tick(&mut b, DT).is_err(), "{mode}");
        assert!(native.is_failed() && dev.is_failed());
        assert!(native.logs().is_err() && dev.logs().is_err());
        compare(&mut a, &mut expected);
        compare(&mut b, &mut expected);
        for w in [&mut a, &mut b, &mut expected] {
            w.with_numeric_physics(|_, q| {
                assert!(q.raycast(&ray3()).unwrap().is_some());
                q.compute_character_motion(&character(), DT).unwrap();
            })
            .unwrap();
            w.step().unwrap();
        }
        compare(&mut a, &mut expected);
        compare(&mut b, &mut expected);
    }
}
#[test]
fn actual_raw_ingress_destinations_stale_epoch_and_reply_replacement() {
    let bytes = wat::parse_str(include_str!(
        "../../incant_wasm_runtime/fixtures/physics-queries.wat"
    ))
    .unwrap();
    for mode in 1..=12 {
        let mut a = NativeWorld::new(scene(false));
        let mut b = NativeWorld::new(scene(false));
        let before = a.snapshot();
        let mut native = StaticModule::new(embedded::linked("physics-queries").unwrap()).unwrap();
        let mut dev = DevModuleHost::new(&bytes).unwrap();
        let x = native.tick(&mut a, mode as f64 / 64.);
        let y = dev.tick(&mut b, mode as f64 / 64.);
        if mode == 1 {
            x.unwrap();
            y.unwrap();
            compare(&mut a, &mut b);
            let after = a.snapshot();
            let changed: Vec<_> = after.iter().zip(&before).filter(|(a, b)| a != b).collect();
            assert_eq!(changed.len(), 1);
            assert_eq!(changed[0].0.transform.translation[0], 99.);
        } else {
            assert!(x.is_err(), "{mode}");
            assert!(y.is_err(), "{mode}");
            assert_eq!(a.snapshot(), before);
            assert_eq!(b.snapshot(), before);
        }
    }
}

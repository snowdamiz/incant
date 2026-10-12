#[path = "../../incant_runtime/tests/support/navigation_queries.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_abi::{self as abi, navigation_paths as wire};
use incant_wasm_runtime::{Call, Capability};
const DT: f64 = 1. / 60.;
fn request(grid: bool, work: u32) -> Vec<u8> {
    if grid {
        wire::encode_grid(abi::GridPathRequest {
            owner: id(1).0,
            start: [0, 0],
            end: [2, 0],
            max_expansions: work,
            flags: 1,
        })
        .unwrap()
        .to_vec()
    } else {
        wire::encode_path(abi::PathRequest {
            owner: id(1).0,
            start: [-8., 0., 0.],
            end: [8., 0., 0.],
            snap_distance: 1.,
            max_visited: work,
            reserved: 0,
        })
        .unwrap()
        .to_vec()
    }
}
fn query(
    cap: &mut Capability,
    memory: &mut [u8],
    e: i64,
    grid: bool,
    bytes: &[u8],
) -> Result<i32, incant_wasm_runtime::BoundaryError> {
    memory[..bytes.len()].copy_from_slice(bytes);
    cap.invoke(
        memory,
        e,
        if grid {
            Call::FindGridPath {
                ptr: 0,
                len: bytes.len() as i32,
            }
        } else {
            Call::FindPath {
                ptr: 0,
                len: bytes.len() as i32,
            }
        },
    )
}
fn read(cap: &mut Capability, memory: &mut [u8], e: i64, len: i32) -> Vec<u8> {
    memory.fill(0xa5);
    cap.invoke(
        memory,
        e,
        Call::ReadPath {
            ptr: 1,
            capacity: len + 1,
        },
    )
    .unwrap();
    assert_eq!(memory[0], 0xa5);
    assert_eq!(memory[len as usize + 1], 0xa5);
    memory[1..len as usize + 1].to_vec()
}
fn prefix(cap: &mut Capability, m: &mut [u8], e: i64) {
    cap.invoke(
        m,
        e,
        Call::Column {
            chunk: 0,
            ptr: 512,
            velocity: false,
            write: false,
        },
    )
    .unwrap();
    m[512..520].copy_from_slice(&42f64.to_le_bytes());
    cap.invoke(
        m,
        e,
        Call::Column {
            chunk: 0,
            ptr: 512,
            velocity: false,
            write: true,
        },
    )
    .unwrap();
    m[..16].copy_from_slice(&id(4).0);
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
fn rejected(cap: &mut Capability, w: &mut NativeWorld, before: &[EntitySnapshot]) {
    assert!(cap.finish(w).is_err());
    assert_eq!(w.snapshot(), before);
    assert_eq!(
        w.navigation_path(id(1), &path()).unwrap(),
        NativeWorld::new(scene())
            .navigation_path(id(1), &path())
            .unwrap()
    );
    assert!(cap.logs().is_err());
    assert!(cap.is_failed());
}
#[test]
fn copied_detours_grid_cost_links_reverse_and_misses_match_native_oracles() {
    for scene in [scene(), linked_scene(true), linked_scene(false)] {
        let mut world = NativeWorld::new(scene);
        let before = world.snapshot();
        let mut cap = Capability::default();
        let mut m = vec![0; abi::PATH_MAX_RESULT_BYTES + 2];
        let e = cap.begin(&mut world, DT).unwrap();
        for reverse in [false, true] {
            let mut q = path();
            if reverse {
                std::mem::swap(&mut q.start, &mut q.end);
            }
            let native = world.navigation_path(id(1), &q).unwrap();
            let b = wire::encode_path(abi::PathRequest {
                owner: id(1).0,
                start: q.start.map(f64::from),
                end: q.end.map(f64::from),
                snap_distance: 1.,
                max_visited: 1000,
                reserved: 0,
            })
            .unwrap();
            let n = query(&mut cap, &mut m, e, false, &b).unwrap();
            let bytes = read(&mut cap, &mut m, e, n);
            let result = wire::decode_path_result(&bytes).unwrap();
            assert_eq!(result.is_some(), native.is_some());
            if let (Some(result), Some(native)) = (result, native) {
                assert_eq!(
                    result.points().collect::<Vec<_>>(),
                    native
                        .points
                        .iter()
                        .map(|p| p.map(f64::from))
                        .collect::<Vec<_>>()
                );
                assert_eq!(result.corridor().collect::<Vec<_>>(), native.corridor);
                assert_eq!(
                    (result.generation, result.visited),
                    (native.generation, native.visited)
                );
                let links = result.traversals().collect::<Vec<_>>();
                assert_eq!(links.len(), native.traversals.len());
                for (a, b) in links.iter().zip(native.traversals) {
                    assert_eq!(a.link_id, id(8).0);
                    assert_eq!(
                        (a.from_index, a.to_index, a.reversed),
                        (b.from_index, b.to_index, u32::from(b.reversed))
                    );
                }
            }
        }
        let n = query(&mut cap, &mut m, e, true, &request(true, 100)).unwrap();
        let bytes = read(&mut cap, &mut m, e, n);
        let result = wire::decode_grid_result(&bytes).unwrap().unwrap();
        let native = world.grid_path(id(1), &grid()).unwrap().unwrap();
        assert_eq!((result.generation, result.cost), (native.generation, 4000));
        assert_eq!(result.cells().collect::<Vec<_>>(), native.cells);
        let mut miss = request(true, 100);
        miss[8 + abi::GridPathRequest::END_OFFSET..8 + abi::GridPathRequest::END_OFFSET + 4]
            .copy_from_slice(&1u32.to_le_bytes());
        let n = query(&mut cap, &mut m, e, true, &miss).unwrap();
        assert!(
            wire::decode_grid_result(&read(&mut cap, &mut m, e, n))
                .unwrap()
                .is_none()
        );
        cap.finish(&mut world).unwrap();
        assert_eq!(world.snapshot(), before);
    }
}
#[test]
fn malformed_requests_bad_owners_and_native_exhaustion_erase_reply_and_reject_caught_prefix() {
    for mode in 0..13 {
        let mut world = NativeWorld::new(scene());
        let before = world.snapshot();
        let mut cap = Capability::default();
        let mut m = vec![0; 4096];
        let e = cap.begin(&mut world, DT).unwrap();
        query(&mut cap, &mut m, e, true, &request(true, 100)).unwrap();
        prefix(&mut cap, &mut m, e);
        let grid = mode >= 8;
        let mut b = request(grid, 1000);
        match mode {
            0 => b[0] = 0,
            1 => {
                b.pop();
            }
            2 => b[8 + abi::PathRequest::RESERVED_OFFSET] = 1,
            3 | 4 => b[8 + abi::PathRequest::START_OFFSET..8 + abi::PathRequest::START_OFFSET + 8]
                .copy_from_slice(
                    &(if mode == 3 { f64::NAN } else { 100_000.000_001 }).to_le_bytes(),
                ),
            5 => b[8 + abi::PathRequest::SNAP_DISTANCE_OFFSET
                ..8 + abi::PathRequest::SNAP_DISTANCE_OFFSET + 8]
                .copy_from_slice(&10.000_000_001f64.to_le_bytes()),
            6 => b[8..24].copy_from_slice(&id(4).0),
            7 => b[8 + abi::PathRequest::MAX_VISITED_OFFSET
                ..8 + abi::PathRequest::MAX_VISITED_OFFSET + 4]
                .copy_from_slice(&1u32.to_le_bytes()),
            8 => b[8 + abi::GridPathRequest::FLAGS_OFFSET] = 2,
            9 => b[8 + abi::GridPathRequest::END_OFFSET..8 + abi::GridPathRequest::END_OFFSET + 4]
                .copy_from_slice(&65536u32.to_le_bytes()),
            10 => b[8 + abi::GridPathRequest::END_OFFSET..8 + abi::GridPathRequest::END_OFFSET + 4]
                .copy_from_slice(&3u32.to_le_bytes()),
            11 => b[8 + abi::GridPathRequest::MAX_EXPANSIONS_OFFSET
                ..8 + abi::GridPathRequest::MAX_EXPANSIONS_OFFSET + 4]
                .copy_from_slice(&1u32.to_le_bytes()),
            _ => b[8..24].copy_from_slice(&id(4).0),
        }
        assert!(query(&mut cap, &mut m, e, grid, &b).is_err(), "mode {mode}");
        let untouched = m.clone();
        assert!(
            cap.invoke(
                &mut m,
                e,
                Call::ReadPath {
                    ptr: 0,
                    capacity: 300
                }
            )
            .is_err()
        );
        assert_eq!(m, untouched);
        rejected(&mut cap, &mut world, &before);
    }
}
#[test]
fn ingress_destinations_stale_epochs_and_missing_reply_have_no_partial_copy() {
    for call in [
        Call::FindPath { ptr: -1, len: 88 },
        Call::FindGridPath { ptr: 4090, len: 48 },
        Call::FindPath { ptr: 0, len: -1 },
        Call::ReadPath {
            ptr: -1,
            capacity: 300,
        },
        Call::ReadPath {
            ptr: 0,
            capacity: 1,
        },
        Call::ReadPath {
            ptr: 4000,
            capacity: 300,
        },
        Call::ReadPath {
            ptr: 0,
            capacity: i32::MAX,
        },
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.snapshot();
        let mut cap = Capability::default();
        let mut m = vec![0; 4096];
        let stale = cap.begin(&mut world, DT).unwrap();
        cap.finish(&mut world).unwrap();
        let e = cap.begin(&mut world, DT).unwrap();
        assert!(cap.invoke(&mut m, stale, call).is_err());
        query(&mut cap, &mut m, e, true, &request(true, 100)).unwrap();
        prefix(&mut cap, &mut m, e);
        let untouched = m.clone();
        assert!(cap.invoke(&mut m, e, call).is_err());
        assert_eq!(m, untouched);
        rejected(&mut cap, &mut world, &before);
    }
    let mut world = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut m = vec![0; 4096];
    let stale = cap.begin(&mut world, DT).unwrap();
    query(&mut cap, &mut m, stale, true, &request(true, 100)).unwrap();
    cap.finish(&mut world).unwrap();
    let e = cap.begin(&mut world, DT).unwrap();
    assert!(
        cap.invoke(&mut m, stale, Call::FindGridPath { ptr: 0, len: 48 })
            .is_err()
    );
    assert!(
        cap.invoke(
            &mut m,
            e,
            Call::ReadPath {
                ptr: 0,
                capacity: 300
            }
        )
        .is_err()
    );
    assert!(cap.finish(&mut world).is_err());
}
#[test]
fn combined_navigation_queries_and_requested_search_work_are_charged_before_search() {
    for expansions in [false, true] {
        let mut world = NativeWorld::new(scene());
        let before = world.snapshot();
        let mut cap = Capability::default();
        let mut m = vec![0; 4096];
        let e = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut m, e);
        if expansions {
            query(&mut cap, &mut m, e, false, &request(false, 32768)).unwrap();
            query(&mut cap, &mut m, e, true, &request(true, 32768)).unwrap();
        } else {
            for i in 0..32 {
                if i % 3 == 0 {
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
                } else {
                    query(&mut cap, &mut m, e, i % 3 == 1, &request(i % 3 == 1, 1000)).unwrap();
                }
            }
        }
        assert!(query(&mut cap, &mut m, e, true, &request(true, 100)).is_err());
        rejected(&mut cap, &mut world, &before);
    }
}

#[test]
fn stale_failure_keeps_current_reply_and_does_not_poison_successful_commit() {
    let mut w = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut m = vec![0; 4096];
    let stale = cap.begin(&mut w, DT).unwrap();
    cap.finish(&mut w).unwrap();
    let e = cap.begin(&mut w, DT).unwrap();
    let n = query(&mut cap, &mut m, e, true, &request(true, 100)).unwrap();
    let expected = read(&mut cap, &mut m, e, n);
    for call in [
        Call::FindPath { ptr: -1, len: 88 },
        Call::FindGridPath { ptr: -1, len: 48 },
        Call::ReadPath {
            ptr: -1,
            capacity: 1,
        },
    ] {
        assert!(cap.invoke(&mut m, stale, call).is_err());
        assert_eq!(read(&mut cap, &mut m, e, n), expected);
    }
    cap.finish(&mut w).unwrap();
    assert!(!cap.is_failed());
}

#[test]
fn global_memory_and_host_call_limits_poison_authorized_navigation() {
    for memory_limit in [false, true] {
        let mut w = NativeWorld::new(scene());
        let before = w.snapshot();
        let mut cap = Capability::default();
        let mut m = vec![0; 4096];
        let e = cap.begin(&mut w, DT).unwrap();
        query(&mut cap, &mut m, e, true, &request(true, 100)).unwrap();
        prefix(&mut cap, &mut m, e);
        if memory_limit {
            m.resize(abi::MEMORY + 1, 0x6c);
        } else {
            while cap.invoke(&mut m, e, Call::ChunkCount).is_ok() {}
        }
        let prefix = m[..4096].to_vec();
        assert!(
            cap.invoke(
                &mut m,
                e,
                Call::ReadPath {
                    ptr: 0,
                    capacity: 300
                }
            )
            .is_err()
        );
        assert_eq!(&m[..4096], prefix);
        if memory_limit {
            assert!(m[4096..].iter().all(|&b| b == 0x6c));
        }
        rejected(&mut cap, &mut w, &before);
    }
}

#[test]
fn repeated_large_copied_reads_exhaust_transfer_quota_without_partial_final_write() {
    let mut cfg = config();
    cfg.mesh = None;
    cfg.grid = Some(NavigationGrid {
        dimensions: [1024, 1],
        costs: vec![1; 1024],
    });
    let mut w = NativeWorld::new(
        CookedScene::new(id(0), 60, vec![entity(1), entity(4)])
            .unwrap()
            .with_navigation(vec![cfg])
            .unwrap(),
    );
    let before = w.snapshot();
    let mut cap = Capability::default();
    let mut m = vec![0; 9000];
    let e = cap.begin(&mut w, DT).unwrap();
    prefix(&mut cap, &mut m, e);
    let b = wire::encode_grid(abi::GridPathRequest {
        owner: id(1).0,
        start: [0, 0],
        end: [1023, 0],
        max_expansions: 65536,
        flags: 0,
    })
    .unwrap();
    let n = query(&mut cap, &mut m, e, true, &b).unwrap();
    let mut reads = 0;
    loop {
        m.fill(0xa5);
        match cap.invoke(
            &mut m,
            e,
            Call::ReadPath {
                ptr: 1,
                capacity: n,
            },
        ) {
            Ok(_) => reads += 1,
            Err(_) => break,
        }
    }
    assert!(reads > 10_000);
    assert!(m.iter().all(|&b| b == 0xa5));
    assert!(cap.finish(&mut w).is_err());
    assert_eq!(w.snapshot(), before);
    assert!(cap.logs().is_err());
}

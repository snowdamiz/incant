use incant_wasm_abi::{self as abi, navigation_paths as wire};
fn path() -> Vec<u8> {
    let mut b = abi::PATH_RESULT_MAGIC.to_vec();
    b.extend(
        abi::PathResultHeader {
            found: 1,
            visited: 3,
            generation: u64::MAX,
            points: 2,
            corridor: 1,
            traversals: 1,
            reserved: 0,
        }
        .encode(),
    );
    b.extend(
        abi::PathPoint {
            position: [-2., 0., 0.],
        }
        .encode(),
    );
    b.extend(
        abi::PathPoint {
            position: [2., 0., 0.],
        }
        .encode(),
    );
    b.extend(9u32.to_le_bytes());
    b.extend(
        abi::PathTraversal {
            link_id: u128::MAX.to_be_bytes(),
            from_index: 0,
            to_index: 1,
            reversed: 1,
            reserved: 0,
        }
        .encode(),
    );
    b
}
fn grid() -> Vec<u8> {
    let mut b = abi::GRID_PATH_RESULT_MAGIC.to_vec();
    b.extend(
        abi::GridPathResultHeader {
            found: 1,
            cells: 2,
            generation: u64::MAX,
            cost: u64::MAX,
        }
        .encode(),
    );
    b.extend(
        abi::GridPathCell {
            position: [65534, 65535],
        }
        .encode(),
    );
    b.extend(
        abi::GridPathCell {
            position: [65535, 65535],
        }
        .encode(),
    );
    b
}
#[test]
fn unaligned_copied_routes_preserve_exact_u64s_coordinates_and_traversal_identity() {
    let mut b = vec![7];
    b.extend(path());
    let p = wire::decode_path_result(&b[1..]).unwrap().unwrap();
    assert_eq!(p.generation, u64::MAX);
    assert_eq!(p.visited, 3);
    assert_eq!(p.corridor().collect::<Vec<_>>(), [9]);
    assert_eq!(
        p.points().collect::<Vec<_>>(),
        [[-2., 0., 0.], [2., 0., 0.]]
    );
    let t = p.traversals().next().unwrap();
    assert_eq!(t.link_id, u128::MAX.to_be_bytes());
    assert_eq!(t.reversed, 1);
    let mut b = vec![7];
    b.extend(grid());
    let p = wire::decode_grid_result(&b[1..]).unwrap().unwrap();
    assert_eq!((p.generation, p.cost), (u64::MAX, u64::MAX));
    assert_eq!(
        p.cells().collect::<Vec<_>>(),
        [[65534, 65535], [65535, 65535]]
    );
}
#[test]
fn exact_frames_and_late_fields_fail_before_exposing_any_route() {
    let p = path();
    let g = grid();
    for end in 0..p.len() {
        assert!(wire::decode_path_result(&p[..end]).is_err());
    }
    for end in 0..g.len() {
        assert!(wire::decode_grid_result(&g[..end]).is_err());
    }
    for offset in [
        0,
        8,
        8 + abi::PathResultHeader::RESERVED_OFFSET,
        8 + abi::PathResultHeader::POINTS_OFFSET,
        40 + 24 + abi::PathPoint::POSITION_OFFSET + 6,
        40 + 48 + 4 + abi::PathTraversal::TO_INDEX_OFFSET,
        40 + 48 + 4 + abi::PathTraversal::REVERSED_OFFSET,
        40 + 48 + 4 + abi::PathTraversal::RESERVED_OFFSET,
    ] {
        let mut bad = p.clone();
        bad[offset] = 255;
        assert!(wire::decode_path_result(&bad).is_err(), "offset {offset}");
    }
    for offset in [0, 8, 12, 32 + 8 + 2] {
        let mut bad = g.clone();
        bad[offset] = 255;
        assert!(wire::decode_grid_result(&bad).is_err(), "offset {offset}");
    }
    let mut bad = g.clone();
    bad[40..48].copy_from_slice(&g[32..40]);
    assert!(wire::decode_grid_result(&bad).is_err());
    let mut bad = p.clone();
    bad.push(0);
    assert!(wire::decode_path_result(&bad).is_err());
    let mut miss = abi::PATH_RESULT_MAGIC.to_vec();
    miss.extend(
        abi::PathResultHeader {
            found: 0,
            visited: 0,
            generation: 0,
            points: 0,
            corridor: 0,
            traversals: 0,
            reserved: 0,
        }
        .encode(),
    );
    assert!(wire::decode_path_result(&miss).unwrap().is_none());
    miss[8 + abi::PathResultHeader::GENERATION_OFFSET] = 1;
    assert!(wire::decode_path_result(&miss).is_err());
}
#[test]
fn request_domains_validate_original_doubles_before_native_narrowing() {
    let q = abi::PathRequest {
        owner: [255; 16],
        start: [-100000., 0., 0.],
        end: [100000., 0., 0.],
        snap_distance: 0.001,
        max_visited: 32768,
        reserved: 0,
    };
    let b = wire::encode_path(q).unwrap();
    assert_eq!(wire::decode_path(&b).unwrap(), q);
    for end in 0..b.len() {
        assert!(wire::decode_path(&b[..end]).is_err());
    }
    for mode in 0..6 {
        let mut q = q;
        match mode {
            0 => q.start[0] = -100_000.000_001,
            1 => q.snap_distance = 0.000_999_999_999,
            2 => q.snap_distance = 10.000_000_001,
            3 => q.end[0] = f64::INFINITY,
            4 => q.max_visited = 32769,
            _ => q.reserved = 1,
        }
        assert!(wire::encode_path(q).is_err());
    }
    let q = abi::GridPathRequest {
        owner: [255; 16],
        start: [0, 0],
        end: [65535, 65535],
        max_expansions: 65536,
        flags: 1,
    };
    let b = wire::encode_grid(q).unwrap();
    assert_eq!(wire::decode_grid(&b).unwrap(), q);
    for mode in 0..4 {
        let mut q = q;
        match mode {
            0 => q.end[0] = 65536,
            1 => q.flags = 2,
            2 => q.max_expansions = 0,
            _ => q.max_expansions = 65537,
        }
        assert!(wire::encode_grid(q).is_err());
    }
}
#[test]
fn largest_supported_route_arrays_fit_advertised_bounds() {
    assert_eq!(
        abi::PATH_MAX_RESULT_BYTES,
        8 + abi::PathResultHeader::SIZE + 4096 * (24 + 4 + 32)
    );
    assert_eq!(
        abi::GRID_PATH_MAX_RESULT_BYTES,
        8 + abi::GridPathResultHeader::SIZE + 4096 * 8
    );
    let mut b = abi::PATH_RESULT_MAGIC.to_vec();
    b.extend(
        abi::PathResultHeader {
            found: 1,
            visited: 32768,
            generation: 1,
            points: 4096,
            corridor: 4096,
            traversals: 0,
            reserved: 0,
        }
        .encode(),
    );
    for _ in 0..4096 {
        b.extend(abi::PathPoint { position: [0.; 3] }.encode());
    }
    for _ in 0..4096 {
        b.extend(0u32.to_le_bytes());
    }
    let p = wire::decode_path_result(&b).unwrap().unwrap();
    assert_eq!(p.points().len(), 4096);
    assert_eq!(p.corridor().len(), 4096);
    let mut b = abi::GRID_PATH_RESULT_MAGIC.to_vec();
    b.extend(
        abi::GridPathResultHeader {
            found: 1,
            cells: 4096,
            generation: 1,
            cost: 4_095_000,
        }
        .encode(),
    );
    for x in 0..4096 {
        b.extend(abi::GridPathCell { position: [x, 0] }.encode());
    }
    assert_eq!(
        wire::decode_grid_result(&b).unwrap().unwrap().cells().len(),
        4096
    );
}

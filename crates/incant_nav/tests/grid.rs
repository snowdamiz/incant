use incant_nav::{GridPathRequest, NavigationError, NavigationGrid};

fn request(start: [u16; 2], end: [u16; 2], diagonal: bool) -> GridPathRequest {
    GridPathRequest {
        start,
        end,
        diagonal,
        max_expansions: 65536,
    }
}
#[test]
fn weighted_routes_choose_cheaper_detours_and_zero_length_paths_cost_nothing() {
    let grid = NavigationGrid {
        dimensions: [3, 3],
        costs: vec![1, 1, 1, 1, 100, 1, 1, 1, 1],
    };
    for (diagonal, cost) in [(false, 4000), (true, 2828)] {
        let req = request([0, 1], [2, 1], diagonal);
        let path = grid.find_path(&req).unwrap().unwrap();
        assert_eq!(path.cost, cost);
        assert!(!path.cells.contains(&[1, 1]));
        for _ in 0..20 {
            assert_eq!(grid.find_path(&req).unwrap().unwrap(), path);
        }
    }
    let path = grid
        .find_path(&request([1, 1], [1, 1], true))
        .unwrap()
        .unwrap();
    assert_eq!(path.cells, [[1, 1]]);
    assert_eq!(path.cost, 0);
}
#[test]
fn diagonal_moves_cannot_cut_blocked_corners_and_blocked_endpoints_return_none() {
    let mut grid = NavigationGrid {
        dimensions: [2, 2],
        costs: vec![1, 0, 0, 1],
    };
    assert!(
        grid.find_path(&request([0, 0], [1, 1], true))
            .unwrap()
            .is_none()
    );
    grid.costs[1] = 1;
    let path = grid
        .find_path(&request([0, 0], [1, 1], true))
        .unwrap()
        .unwrap();
    assert_eq!(path.cells, [[0, 0], [1, 0], [1, 1]]);
    assert_eq!(path.cost, 2000);
    grid.costs[2] = 1;
    assert_eq!(
        grid.find_path(&request([0, 0], [1, 1], true))
            .unwrap()
            .unwrap()
            .cost,
        1414
    );
    grid.costs[0] = 0;
    assert!(
        grid.find_path(&request([0, 0], [0, 0], false))
            .unwrap()
            .is_none()
    );
}
#[test]
fn invalid_inputs_and_exhausted_search_are_distinct_from_no_route() {
    let grid = NavigationGrid {
        dimensions: [8, 8],
        costs: vec![1; 64],
    };
    let mut req = request([0, 0], [7, 7], false);
    req.max_expansions = 1;
    assert!(matches!(
        grid.find_path(&req),
        Err(NavigationError::Limit(_))
    ));
    req.max_expansions = 0;
    assert!(grid.find_path(&req).is_err());
    req.max_expansions = 65537;
    assert!(grid.find_path(&req).is_err());
    assert!(matches!(
        grid.find_path(&request([0, 0], [8, 7], false)),
        Err(NavigationError::Invalid(_))
    ));
    for bad in [
        NavigationGrid {
            dimensions: [0, 1],
            costs: vec![],
        },
        NavigationGrid {
            dimensions: [1025, 1],
            costs: vec![1; 1025],
        },
        NavigationGrid {
            dimensions: [257, 256],
            costs: vec![1; 257 * 256],
        },
        NavigationGrid {
            dimensions: [1, 1],
            costs: vec![],
        },
        NavigationGrid {
            dimensions: [1, 1],
            costs: vec![1001],
        },
    ] {
        assert!(bad.validate().is_err());
    }
    assert!(
        NavigationGrid {
            dimensions: [256, 256],
            costs: vec![1; 65536]
        }
        .validate()
        .is_ok()
    );
}
#[test]
fn overlong_corridor_fails_explicitly_instead_of_returning_a_truncated_route() {
    let mut grid = NavigationGrid {
        dimensions: [128, 128],
        costs: vec![0; 128 * 128],
    };
    for y in 0..127 {
        if y % 2 == 0 {
            grid.costs[y * 128..(y + 1) * 128].fill(1);
        } else {
            grid.costs[y * 128 + if (y / 2) % 2 == 0 { 127 } else { 0 }] = 1;
        }
    }
    let error = grid
        .find_path(&request([0, 0], [0, 126], true))
        .unwrap_err();
    assert!(matches!(error, NavigationError::Limit(ref message) if message.contains("4096")));
}

// Independent exhaustive relaxation oracle: no heuristic, priority queue or
// implementation neighbor iterator. Small random grids also verify path edges.
fn reference(grid: &NavigationGrid, start: usize, end: usize, diagonal: bool) -> Option<u64> {
    if grid.costs[start] == 0 || grid.costs[end] == 0 {
        return None;
    }
    let width = usize::from(grid.dimensions[0]);
    let mut best = vec![u64::MAX; grid.costs.len()];
    best[start] = 0;
    for _ in 0..grid.costs.len() {
        let mut changed = false;
        for a in 0..grid.costs.len() {
            if best[a] == u64::MAX {
                continue;
            }
            for b in 0..grid.costs.len() {
                let dx = (a % width).abs_diff(b % width);
                let dy = (a / width).abs_diff(b / width);
                if grid.costs[b] == 0 || dx > 1 || dy > 1 || dx + dy == 0 {
                    continue;
                }
                if dx + dy == 2
                    && (!diagonal
                        || grid.costs[a / width * width + b % width] == 0
                        || grid.costs[b / width * width + a % width] == 0)
                {
                    continue;
                }
                let candidate =
                    best[a] + u64::from(grid.costs[b]) * if dx + dy == 2 { 1414 } else { 1000 };
                if candidate < best[b] {
                    best[b] = candidate;
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    (best[end] != u64::MAX).then_some(best[end])
}
#[test]
fn weighted_astar_matches_exhaustive_costs_on_varied_grids() {
    let mut seed = 0x95b17a3e_u32;
    for _ in 0..60 {
        let mut grid = NavigationGrid {
            dimensions: [8, 8],
            costs: vec![],
        };
        for _ in 0..64 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            grid.costs.push(((seed >> 16) % 8) as u16);
        }
        for diagonal in [false, true] {
            let actual = grid.find_path(&request([0, 0], [7, 7], diagonal)).unwrap();
            assert_eq!(
                actual.as_ref().map(|p| p.cost),
                reference(&grid, 0, 63, diagonal)
            );
            if let Some(path) = actual {
                assert_eq!(path.cells.first(), Some(&[0, 0]));
                assert_eq!(path.cells.last(), Some(&[7, 7]));
                let mut total = 0;
                for pair in path.cells.windows(2) {
                    let [a, b] = [pair[0], pair[1]];
                    let dx = a[0].abs_diff(b[0]);
                    let dy = a[1].abs_diff(b[1]);
                    assert!(dx <= 1 && dy <= 1 && dx + dy > 0);
                    if dx + dy == 2 {
                        assert!(diagonal);
                        assert!(grid.costs[usize::from(a[1]) * 8 + usize::from(b[0])] > 0);
                        assert!(grid.costs[usize::from(b[1]) * 8 + usize::from(a[0])] > 0);
                    }
                    total += u64::from(grid.costs[usize::from(b[1]) * 8 + usize::from(b[0])])
                        * if dx + dy == 2 { 1414 } else { 1000 };
                }
                assert_eq!(total, path.cost);
            }
        }
    }
}

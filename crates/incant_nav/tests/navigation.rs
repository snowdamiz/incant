use incant_nav::*;
use std::collections::BTreeMap;

fn settings() -> NavigationSettings {
    NavigationSettings {
        min: [-10., -2., -6.],
        max: [10., 5., 6.],
        cell_size: 0.25,
        cell_height: 0.1,
        tile_cells: 16,
        agent_radius: 0.5,
        agent_height: 1.8,
        max_climb: 0.3,
        max_slope_degrees: 45.,
    }
}
fn floor() -> NavigationGeometry {
    NavigationGeometry {
        vertices: vec![
            [-10., 0., -6.],
            [10., 0., -6.],
            [10., 0., 6.],
            [-10., 0., 6.],
        ],
        triangles: vec![[0, 2, 1], [0, 3, 2]],
    }
}
fn wall() -> NavigationGeometry {
    NavigationGeometry {
        vertices: vec![
            [-1., 0., -2.],
            [1., 0., -2.],
            [1., 0., 2.],
            [-1., 0., 2.],
            [-1., 3., -2.],
            [1., 3., -2.],
            [1., 3., 2.],
            [-1., 3., 2.],
        ],
        triangles: vec![
            [4, 6, 5],
            [4, 7, 6],
            [0, 1, 2],
            [0, 2, 3],
            [0, 4, 5],
            [0, 5, 1],
            [1, 5, 6],
            [1, 6, 2],
            [2, 6, 7],
            [2, 7, 3],
            [3, 7, 4],
            [3, 4, 0],
        ],
    }
}
fn block(min: [f32; 3], max: [f32; 3]) -> NavigationGeometry {
    let mut geometry = wall();
    for p in &mut geometry.vertices {
        *p = [
            min[0] + (p[0] + 1.) / 2. * (max[0] - min[0]),
            min[1] + p[1] / 3. * (max[1] - min[1]),
            min[2] + (p[2] + 2.) / 4. * (max[2] - min[2]),
        ];
    }
    geometry
}
fn query() -> PathRequest {
    PathRequest {
        start: [-8., 0., 0.],
        end: [8., 0., 0.],
        snap_distance: 1.,
        max_visited: 1000,
    }
}
#[test]
fn actual_recast_tiles_connect_and_funnel_a_flat_floor() {
    let mut nav = NavigationMesh::default();
    let sources = BTreeMap::from([("floor".into(), floor())]);
    let report = nav.rebuild(&settings(), &sources).unwrap();
    assert_eq!(report.rebuilt.len(), 15);
    assert!(report.polygons > 0);
    let path = nav.find_path(&query()).unwrap().unwrap();
    assert!(path.corridor.len() > 1);
    assert_eq!(path.points.len(), 2, "{path:?}");
    assert!((path.points[0][0] + 8.).abs() < 0.001);
    assert!((path.points[1][0] - 8.).abs() < 0.001);
    let same = nav.rebuild(&settings(), &sources).unwrap();
    assert!(same.rebuilt.is_empty());
    assert_eq!(same.reused.len(), 15);
    assert_eq!(same.generation, report.generation);
    assert_eq!(nav.find_path(&query()).unwrap().unwrap(), path);
}
#[test]
fn obstacle_rebuilds_only_affected_tiles_and_path_respects_radius() {
    let mut nav = NavigationMesh::default();
    let mut sources = BTreeMap::from([("floor".into(), floor())]);
    nav.rebuild(&settings(), &sources).unwrap();
    sources.insert("wall".into(), wall());
    let changed = nav.rebuild(&settings(), &sources).unwrap();
    assert!(
        !changed.rebuilt.is_empty() && changed.rebuilt.len() < 15,
        "{changed:?}"
    );
    let path = nav.find_path(&query()).unwrap().unwrap();
    assert!(path.points.len() > 2, "{path:?}");
    for pair in path.points.windows(2) {
        for i in 0..=100 {
            let t = i as f32 / 100.;
            let x = pair[0][0] + t * (pair[1][0] - pair[0][0]);
            let z = pair[0][2] + t * (pair[1][2] - pair[0][2]);
            let distance = (x.abs() - 1.).max(0.).hypot((z.abs() - 2.).max(0.));
            assert!(
                distance >= 0.49,
                "path enters inflated obstacle: {x},{z}: {path:?}"
            );
        }
    }
    let mut independent = NavigationMesh::default();
    independent.rebuild(&settings(), &sources).unwrap();
    assert_eq!(nav.polygons(), independent.polygons());
    assert_eq!(
        path.points,
        independent.find_path(&query()).unwrap().unwrap().points
    );
    sources.remove("wall");
    nav.rebuild(&settings(), &sources).unwrap();
    assert_eq!(nav.find_path(&query()).unwrap().unwrap().points.len(), 2);
}
#[test]
fn invalid_rebuild_is_atomic_and_path_failure_is_explicit() {
    let mut nav = NavigationMesh::default();
    let mut sources = BTreeMap::from([("floor".into(), floor())]);
    nav.rebuild(&settings(), &sources).unwrap();
    let before = nav.find_path(&query()).unwrap();
    let generation = nav.generation();
    let mut bad = wall();
    bad.triangles.push([0, 1, 999]);
    sources.insert("bad".into(), bad);
    assert!(nav.rebuild(&settings(), &sources).is_err());
    assert_eq!(nav.generation(), generation);
    assert_eq!(nav.find_path(&query()).unwrap(), before);
    let mut request = query();
    request.start = [0., 50., 0.];
    assert!(nav.find_path(&request).unwrap().is_none());
    request = query();
    request.max_visited = 1;
    assert!(matches!(
        nav.find_path(&request),
        Err(NavigationError::Limit(_))
    ));
    request = query();
    request.end[0] = f32::NAN;
    assert!(nav.find_path(&request).is_err());
    let mut oversized = settings();
    oversized.min = [-100_000., -2., -100_000.];
    oversized.max = [100_000., 5., 100_000.];
    assert!(oversized.validate().is_err());
}

#[test]
fn low_ceiling_and_narrow_passages_cannot_produce_a_route() {
    let mut nav = NavigationMesh::default();
    let mut sources = BTreeMap::from([
        ("floor".into(), floor()),
        ("ceiling".into(), block([-3., 1.4, -3.], [3., 2., 3.])),
    ]);
    nav.rebuild(&settings(), &sources).unwrap();
    let mut q = query();
    q.start = [0., 0., 0.];
    q.snap_distance = 0.2;
    assert!(nav.find_path(&q).unwrap().is_none());
    sources.remove("ceiling");
    sources.insert("north".into(), block([-1., 0., 0.3], [1., 3., 6.]));
    sources.insert("south".into(), block([-1., 0., -6.], [1., 3., -0.3]));
    nav.rebuild(&settings(), &sources).unwrap();
    assert!(nav.find_path(&query()).unwrap().is_none());
    sources.insert("north".into(), block([-1., 0., 1.5], [1., 3., 6.]));
    sources.insert("south".into(), block([-1., 0., -6.], [1., 3., -1.5]));
    nav.rebuild(&settings(), &sources).unwrap();
    assert!(nav.find_path(&query()).unwrap().is_some());
}

#[test]
fn removed_bounds_drop_tiles_and_failed_work_limits_keep_prior_mesh() {
    let mut nav = NavigationMesh::default();
    let sources = BTreeMap::from([("floor".into(), floor())]);
    nav.rebuild(&settings(), &sources).unwrap();
    let mut s = settings();
    s.max[0] = 2.;
    let shrunk = nav.rebuild(&s, &sources).unwrap();
    assert_eq!(shrunk.removed.len(), 6);
    assert!(nav.find_path(&query()).unwrap().is_none());
    let before = nav.polygons().to_vec();
    let generation = nav.generation();
    let mut expensive = floor();
    expensive.triangles = expensive.triangles.repeat(16000);
    assert!(matches!(
        nav.rebuild(&s, &BTreeMap::from([("expensive".into(), expensive)])),
        Err(NavigationError::Limit(_))
    ));
    assert_eq!(nav.polygons(), before);
    assert_eq!(nav.generation(), generation);
}

#[test]
fn terrain_height_detail_survives_funnel_smoothing_and_steep_faces_are_excluded() {
    let mut geometry = NavigationGeometry {
        vertices: vec![],
        triangles: vec![],
    };
    // A low ridge in the middle; a straight endpoint-only path would cut through it.
    for (x, y) in [(-10., 0.), (-4., 0.), (0., 1.5), (4., 0.), (10., 0.)] {
        geometry.vertices.extend([[x, y, -6.], [x, y, 6.]]);
    }
    for i in 0..4 {
        let a = i * 2;
        geometry
            .triangles
            .extend([[a, a + 1, a + 3], [a, a + 3, a + 2]]);
    }
    let mut nav = NavigationMesh::default();
    nav.rebuild(
        &settings(),
        &BTreeMap::from([("ridge".into(), geometry.clone())]),
    )
    .unwrap();
    let path = nav.find_path(&query()).unwrap().unwrap();
    assert!(path.points.iter().any(|p| p[1] > 1.3), "{path:?}");
    for pair in path.points.windows(2) {
        for i in 0..=10 {
            let t = i as f32 / 10.;
            let x = pair[0][0] + t * (pair[1][0] - pair[0][0]);
            let y = pair[0][1] + t * (pair[1][1] - pair[0][1]);
            let expected = (1. - x.abs() / 4.).max(0.) * 1.5;
            assert!(
                (y - expected).abs() < 0.35,
                "height {x}: {y} vs {expected}: {path:?}"
            );
        }
    }
    let mut steep = settings();
    steep.max_slope_degrees = 10.;
    nav.rebuild(&steep, &BTreeMap::from([("ridge".into(), geometry)]))
        .unwrap();
    assert!(nav.find_path(&query()).unwrap().is_none());
    nav.rebuild(&settings(), &BTreeMap::new()).unwrap();
    assert!(nav.polygons().is_empty());
    assert!(nav.find_path(&query()).unwrap().is_none());
}

#[test]
fn climbable_steps_connect_across_tiles_but_tall_steps_and_stacked_floors_do_not() {
    let mut nav = NavigationMesh::default();
    let mut s = settings();
    let left = block([-10., -1., -6.], [-2., 0., 6.]);
    let low = block([-2., -1., -6.], [10., 0.2, 6.]);
    nav.rebuild(
        &s,
        &BTreeMap::from([("left".into(), left.clone()), ("right".into(), low)]),
    )
    .unwrap();
    let mut q = query();
    q.end[1] = 0.2;
    q.snap_distance = 0.25;
    let path = nav.find_path(&q).unwrap().unwrap();
    assert!(path.points.last().unwrap()[1] > 0.15);
    let tall = block([-2., -1., -6.], [10., 1., 6.]);
    q.end[1] = 1.;
    nav.rebuild(
        &s,
        &BTreeMap::from([("left".into(), left), ("right".into(), tall)]),
    )
    .unwrap();
    assert!(nav.find_path(&q).unwrap().is_none());
    let lower = floor();
    let mut upper = floor();
    for p in &mut upper.vertices {
        p[1] = 3.;
    }
    s.max[1] = 7.;
    nav.rebuild(
        &s,
        &BTreeMap::from([("lower".into(), lower), ("upper".into(), upper)]),
    )
    .unwrap();
    q.end[1] = 3.;
    assert!(nav.find_path(&q).unwrap().is_none());
    q.start[1] = 3.;
    assert!(nav.find_path(&q).unwrap().is_some());
}

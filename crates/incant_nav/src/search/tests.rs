use super::*;
use crate::{
    NavigationGeometry, NavigationPath, NavigationPolygon, NavigationSettings, OffMeshLink,
    PathRequest,
};
use std::{collections::BTreeMap, sync::Arc};

fn floor(min: f32, max: f32) -> NavigationGeometry {
    NavigationGeometry {
        vertices: vec![[min, 0., -2.], [max, 0., -2.], [max, 0., 2.], [min, 0., 2.]],
        triangles: vec![[0, 2, 1], [0, 3, 2]],
    }
}

fn fixture() -> (
    NavigationSettings,
    BTreeMap<String, NavigationGeometry>,
    Vec<OffMeshLink>,
    PathRequest,
) {
    let settings = NavigationSettings {
        min: [-6., -1., -2.],
        max: [6., 3., 2.],
        cell_size: 0.1,
        cell_height: 0.05,
        agent_radius: 0.2,
        agent_height: 1.7,
        max_climb: 0.2,
        ..Default::default()
    };
    let sources = BTreeMap::from([
        ("west".into(), floor(-6., -2.)),
        ("east".into(), floor(2., 6.)),
    ]);
    let links = vec![OffMeshLink {
        id: "00000000000000000000000001".into(),
        start: [-2.6, 0., 0.],
        end: [2.6, 0., 0.],
        snap_distance: 0.2,
        bidirectional: false,
        enabled: true,
        extra_cost: 0.,
    }];
    let query = PathRequest {
        start: [-5., 0., 0.],
        end: [5., 0., 0.],
        snap_distance: 0.2,
        max_visited: 1000,
    };
    (settings, sources, links, query)
}

// Original search feeds the same existing surface/shortcut algorithms. Requests
// here are valid; separate boundary controls exercise public admission failures.
fn reference(
    mesh: &NavigationMesh,
    q: &PathRequest,
) -> Result<Option<NavigationPath>, NavigationError> {
    let Some((first, start)) = mesh.nearest(q.start, q.snap_distance) else {
        return Ok(None);
    };
    let Some((last, end)) = mesh.nearest(q.end, q.snap_distance) else {
        return Ok(None);
    };
    let Some(route) = mesh.search_portals_uncached(first, last, start, end, q.max_visited)? else {
        return Ok(None);
    };
    let mut path = mesh.surface_route(route, start, end)?;
    if path.traversals.is_empty() {
        mesh.shortcut(&mut path, q.max_visited);
    }
    Ok(Some(path))
}

fn error<T>(result: Result<T, NavigationError>) -> Result<T, String> {
    result.map_err(|e| e.to_string())
}

#[test]
fn cached_paths_match_original_queries_links_and_exact_visit_refusals() {
    let (settings, sources, mut links, query) = fixture();
    let mut mesh = NavigationMesh::default();
    for mode in 0..3 {
        links[0].enabled = mode != 0;
        links[0].bidirectional = mode == 2;
        mesh.rebuild_with_links(&settings, &sources, &links)
            .unwrap();
        for (start, end) in [
            (query.start, query.end),
            (query.end, query.start),
            (query.start, [-4., 0., 0.]),
            ([0., 0., 0.], query.end),
        ] {
            for snap_distance in [0.001, 0.2, 2.] {
                for max_visited in [1, 2, 3, 1000] {
                    let q = PathRequest {
                        start,
                        end,
                        snap_distance,
                        max_visited,
                    };
                    assert_eq!(error(mesh.find_path(&q)), error(reference(&mesh, &q)));
                }
            }
        }
    }
}

#[test]
fn ties_keep_original_state_indices_and_query_scratch_is_not_reused() {
    let mut mesh = NavigationMesh::default();
    mesh.polygons = (0..5)
        .map(|_| NavigationPolygon {
            vertices: vec![],
            triangles: vec![],
        })
        .collect();
    let portal = |to, x| crate::build::Portal {
        to,
        a: [x, 0., -1.],
        b: [x, 0., 1.],
    };
    mesh.portals = vec![
        vec![portal(1, 1.), portal(2, 1.)],
        vec![portal(3, 2.)],
        vec![portal(3, 2.)],
        vec![],
        vec![],
    ];
    mesh.search = Arc::new(PreparedSearch::new(&mesh));
    let route = mesh
        .search_portals(0, 3, [0.; 3], [3., 0., 0.], 10)
        .unwrap()
        .unwrap();
    assert_eq!(route.corridor, [0, 1, 3]);
    for first in 0..5 {
        for last in 0..5 {
            for start in [[0.; 3], [1., 0., 0.], [-1., 0., 0.]] {
                for budget in [1, 2, 3, 32] {
                    assert_eq!(
                        error(mesh.search_portals(first, last, start, [3., 0., 0.], budget)),
                        error(mesh.search_portals_uncached(
                            first,
                            last,
                            start,
                            [3., 0., 0.],
                            budget
                        ))
                    );
                }
            }
        }
    }
}

#[test]
fn generation_replacement_and_failed_link_resolution_preserve_old_snapshots() {
    let (settings, mut sources, mut links, q) = fixture();
    let mut mesh = NavigationMesh::default();
    mesh.rebuild_with_links(&settings, &sources, &links)
        .unwrap();
    let old = mesh.clone();
    let original = old.find_path(&q).unwrap().unwrap();
    assert!(Arc::ptr_eq(&mesh.search, &old.search));
    mesh.rebuild_with_links(&settings, &sources, &links)
        .unwrap();
    assert!(Arc::ptr_eq(&mesh.search, &old.search));
    assert_eq!(mesh.generation(), old.generation());

    // Changed tiles are built before a bad enabled link fails to resolve.
    sources.insert("west".into(), floor(-4., -2.));
    links[0].end = [50., 0., 0.];
    assert!(
        mesh.rebuild_with_links(&settings, &sources, &links)
            .is_err()
    );
    assert!(Arc::ptr_eq(&mesh.search, &old.search));
    assert_eq!(mesh.generation(), old.generation());
    assert_eq!(mesh.find_path(&q).unwrap(), Some(original.clone()));

    links[0].end = [2.6, 0., 0.];
    mesh.rebuild_with_links(&settings, &sources, &links)
        .unwrap();
    assert!(!Arc::ptr_eq(&mesh.search, &old.search));
    assert!(mesh.generation() > old.generation());
    assert!(mesh.find_path(&q).unwrap().is_none());
    assert_eq!(old.find_path(&q).unwrap(), Some(original));

    let q = PathRequest {
        start: [-3.5, 0., 0.],
        ..q
    };
    let linked = mesh.find_path(&q).unwrap().unwrap();
    let linked_snapshot = mesh.clone();
    links[0].enabled = false;
    let report = mesh
        .rebuild_with_links(&settings, &sources, &links)
        .unwrap();
    assert!(report.rebuilt.is_empty());
    assert!(mesh.find_path(&q).unwrap().is_none());
    assert_eq!(linked_snapshot.find_path(&q).unwrap(), Some(linked));
}

#[test]
fn over_limit_graph_preserves_endpoint_and_same_polygon_admission_order() {
    let mut mesh = NavigationMesh::default();
    mesh.settings = Some(NavigationSettings::default());
    mesh.polygons = [0., 4.]
        .map(|x| {
            let triangle = [[x, 0., 0.], [x + 2., 0., 0.], [x, 0., 2.]];
            NavigationPolygon {
                vertices: triangle.to_vec(),
                triangles: vec![triangle],
            }
        })
        .to_vec();
    mesh.portals = vec![
        vec![
            crate::build::Portal {
                to: 1,
                a: [1., 0., 0.],
                b: [1., 0., 1.]
            };
            262_145
        ],
        vec![],
    ];
    mesh.search = Arc::new(PreparedSearch::new(&mesh));
    assert_eq!(
        mesh.search.len(),
        0,
        "over-limit sentinel must not allocate states"
    );
    let q = PathRequest {
        start: [0.2, 0., 0.2],
        end: [0.3, 0., 0.3],
        snap_distance: 0.1,
        max_visited: 1,
    };
    assert_eq!(mesh.find_path(&q).unwrap().unwrap().visited, 1);
    assert!(
        matches!(mesh.find_path(&PathRequest { end: [4.2, 0., 0.2], ..q.clone() }), Err(NavigationError::Limit(s)) if s == "navigation exceeds 262144 directed portal states")
    );
    assert!(
        mesh.find_path(&PathRequest {
            end: [100., 0., 0.],
            ..q.clone()
        })
        .unwrap()
        .is_none()
    );
    assert!(matches!(
        mesh.find_path(&PathRequest {
            max_visited: 0,
            ..q
        }),
        Err(NavigationError::Invalid(_))
    ));
}

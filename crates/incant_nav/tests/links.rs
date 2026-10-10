use incant_nav::*;
use std::collections::BTreeMap;
fn geometry(min: f32, max: f32) -> NavigationGeometry {
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
        ("west".into(), geometry(-6., -2.)),
        ("middle".into(), geometry(-1., 1.)),
        ("east".into(), geometry(2., 6.)),
    ]);
    let links = vec![
        OffMeshLink {
            id: "00000000000000000000000001".into(),
            start: [-2.6, 0., 0.],
            end: [-0.6, 0., 0.],
            snap_distance: 0.2,
            bidirectional: false,
            enabled: true,
            extra_cost: 0.,
        },
        OffMeshLink {
            id: "00000000000000000000000002".into(),
            start: [0.6, 0., 0.],
            end: [2.6, 0., 0.],
            snap_distance: 0.2,
            bidirectional: false,
            enabled: true,
            extra_cost: 0.,
        },
    ];
    let query = PathRequest {
        start: [-5., 0., 0.],
        end: [5., 0., 0.],
        snap_distance: 0.2,
        max_visited: 1000,
    };
    (settings, sources, links, query)
}
#[test]
fn one_way_connections_return_explicit_traversals_without_smoothing_over_gaps() {
    let (settings, sources, mut links, query) = fixture();
    let mut mesh = NavigationMesh::default();
    mesh.rebuild(&settings, &sources).unwrap();
    assert!(mesh.find_path(&query).unwrap().is_none());
    let report = mesh
        .rebuild_with_links(&settings, &sources, &links)
        .unwrap();
    assert_eq!(report.active_links, 2);
    assert!(report.rebuilt.is_empty());
    let path = mesh.find_path(&query).unwrap().unwrap();
    assert_eq!(path.traversals.len(), 2);
    for (transition, link) in path.traversals.iter().zip(&links) {
        assert_eq!(transition.link_id, link.id);
        assert!(!transition.reversed);
        assert_eq!(transition.to_index, transition.from_index + 1);
        assert!((path.points[transition.from_index as usize][0] - link.start[0]).abs() < 1e-5);
        assert!((path.points[transition.to_index as usize][0] - link.end[0]).abs() < 1e-5);
    }
    let reverse = PathRequest {
        start: query.end,
        end: query.start,
        ..query.clone()
    };
    assert!(mesh.find_path(&reverse).unwrap().is_none());
    for link in &mut links {
        link.bidirectional = true;
    }
    mesh.rebuild_with_links(&settings, &sources, &links)
        .unwrap();
    let reverse_path = mesh.find_path(&reverse).unwrap().unwrap();
    assert!(reverse_path.traversals.iter().all(|t| t.reversed));
    assert_eq!(reverse_path.traversals[0].link_id, links[1].id);
    assert_eq!(reverse_path.traversals[1].link_id, links[0].id);
}
#[test]
fn link_edits_are_atomic_reuse_tiles_and_do_not_depend_on_input_order() {
    let (settings, sources, mut links, query) = fixture();
    let mut mesh = NavigationMesh::default();
    let original = mesh
        .rebuild_with_links(&settings, &sources, &links)
        .unwrap();
    let path = mesh.find_path(&query).unwrap();
    links.reverse();
    let same = mesh
        .rebuild_with_links(&settings, &sources, &links)
        .unwrap();
    assert_eq!(same.generation, original.generation);
    assert!(same.rebuilt.is_empty());
    assert_eq!(mesh.find_path(&query).unwrap(), path);
    links[0].end = [50., 0., 0.];
    assert!(
        mesh.rebuild_with_links(&settings, &sources, &links)
            .is_err()
    );
    assert_eq!(mesh.generation(), original.generation);
    assert_eq!(mesh.find_path(&query).unwrap(), path);
    // A disabled link may have an unattached endpoint, but cannot be traversed.
    links[0].enabled = false;
    let disabled = mesh
        .rebuild_with_links(&settings, &sources, &links)
        .unwrap();
    assert_eq!(disabled.active_links, 1);
    assert!(disabled.generation > original.generation);
    assert!(disabled.rebuilt.is_empty());
    assert!(mesh.find_path(&query).unwrap().is_none());
    let mut duplicate = links.clone();
    duplicate.push(links[0].clone());
    assert!(
        mesh.rebuild_with_links(&settings, &sources, &duplicate)
            .is_err()
    );
    assert_eq!(mesh.generation(), disabled.generation);
}
#[test]
fn a_distance_penalty_selects_the_cheaper_connection_and_invalid_costs_reject() {
    let (settings, sources, mut links, query) = fixture();
    let mut alternative = links[0].clone();
    alternative.id = "00000000000000000000000003".into();
    alternative.end = links[1].end;
    alternative.extra_cost = 100.;
    links.push(alternative);
    let mut mesh = NavigationMesh::default();
    mesh.rebuild_with_links(&settings, &sources, &links)
        .unwrap();
    assert_eq!(mesh.find_path(&query).unwrap().unwrap().traversals.len(), 2);
    links[0].extra_cost = 20.;
    links[1].extra_cost = 20.;
    links[2].extra_cost = 0.;
    mesh.rebuild_with_links(&settings, &sources, &links)
        .unwrap();
    let path = mesh.find_path(&query).unwrap().unwrap();
    assert_eq!(path.traversals.len(), 1);
    assert_eq!(path.traversals[0].link_id, links[2].id);
    for cost in [-1., f32::NAN, f32::INFINITY, 100_001.] {
        links[2].extra_cost = cost;
        assert!(
            mesh.rebuild_with_links(&settings, &sources, &links)
                .is_err()
        );
        assert_eq!(mesh.find_path(&query).unwrap(), Some(path.clone()));
    }
}

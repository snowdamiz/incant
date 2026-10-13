//! Inert mutations of actual, already-joined native observations.
use incant_physics_snapshot::*;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn graph_with_edges(base: &GraphRecord, endpoints: &[[u32; 2]]) -> ValidatedGraph {
    let mut r = base.clone();
    r.edges.clear();
    for n in &mut r.nodes {
        n.head = [None; 2];
    }
    for node in endpoints {
        let index = r.edges.len() as u32;
        let next = std::array::from_fn(|side| r.nodes[node[side] as usize].head[side]);
        r.edges.push(GraphEdge {
            node: *node,
            next,
            prev: [None; 2],
            row: index,
        });
        for side in 0..2 {
            if let Some(old) = next[side] {
                r.edges[old as usize].prev[side] = Some(index);
            }
            r.nodes[node[side] as usize].head[side] = Some(index);
        }
    }
    validate_graph(r, &mut budget()).unwrap()
}
pub fn check(r: &NarrowOwnerRecord, c: NarrowOwnerContext<'_>) {
    // Deliberately permute packed payload rows independently of physical graph
    // and arena slots. Re-encoding must not substitute physical index for row.
    let mut g = c.intersections.record().clone();
    let mut reordered = r.clone();
    reordered.nodes.reverse();
    reordered.intersections.reverse();
    let n = g.nodes.len() as u32;
    for node in &mut g.nodes {
        node.row = n - 1 - node.row;
    }
    let n = g.edges.len() as u32;
    for edge in &mut g.edges {
        edge.row = n - 1 - edge.row;
    }
    // Coarena payload order is independently permuted by check_narrow, which
    // holds the exact arena witness. This helper permutes node/edge rows only.
    let g = validate_graph(g, &mut budget()).unwrap();
    let changed = validate_narrow_owner(
        reordered.clone(),
        NarrowOwnerContext {
            intersections: &g,
            ..c
        },
        &mut budget(),
    )
    .unwrap();
    let bytes = encode_narrow_owner(&changed, &mut budget()).unwrap();
    assert_eq!(
        decode_narrow_owner(&bytes, changed.context(), &mut budget())
            .unwrap()
            .record(),
        &reordered
    );
    for i in 0..r.intersections.len() {
        assert_eq!(
            changed.intersection(i as u32),
            Some(&r.intersections[c.intersections.record().edges[i].row as usize])
        );
    }
    if !r.nodes.is_empty() {
        let g = graph_with_edges(c.intersections.record(), &[[0, 0]]);
        let mut bad = r.clone();
        bad.intersections = vec![IntersectionState {
            intersecting: false,
            start_event_emitted: false,
        }];
        assert!(matches!(
            validate_narrow_owner(
                bad,
                NarrowOwnerContext {
                    intersections: &g,
                    ..c
                },
                &mut budget()
            ),
            Err(Error::Narrow)
        ));
    }
    if let Some(edge) = c.intersections.record().edges.first() {
        let g = graph_with_edges(
            c.intersections.record(),
            &[edge.node, [edge.node[1], edge.node[0]]],
        );
        let mut bad = r.clone();
        bad.intersections = vec![
            IntersectionState {
                intersecting: false,
                start_event_emitted: false
            };
            2
        ];
        assert!(matches!(
            validate_narrow_owner(
                bad,
                NarrowOwnerContext {
                    intersections: &g,
                    ..c
                },
                &mut budget()
            ),
            Err(Error::Narrow)
        ));
    }
    let checked = validate_narrow_owner(r.clone(), c, &mut budget()).unwrap();
    let bytes = encode_narrow_owner(&checked, &mut budget()).unwrap();
    for offset in [44, 48, 52] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut b = budget();
        let before = b.remaining();
        assert!(decode_narrow_owner(&bad, c, &mut b).is_err());
        assert_eq!(
            b.remaining(),
            before,
            "oversized frame rejected before allocation"
        );
    }
    // Every single-bit mutation must reject or re-encode byte-exactly; history
    // mutations that stay valid are deliberately not normalized or erased.
    for offset in 0..bytes.len() {
        for bit in 0..8 {
            let mut changed = bytes.clone();
            changed[offset] ^= 1 << bit;
            if let Ok(decoded) = decode_narrow_owner(&changed, c, &mut budget()) {
                assert_eq!(
                    encode_narrow_owner(&decoded, &mut budget()).unwrap(),
                    changed
                );
            }
        }
    }
}

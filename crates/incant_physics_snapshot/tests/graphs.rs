use incant_physics_snapshot::*;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn identity() -> Identity {
    Identity {
        dimension: Dimension::Spatial,
        build: [37; 32],
    }
}
fn add(record: &mut GraphRecord, a: usize, b: usize) {
    let id = record.edges.len() as u32;
    let next = [record.nodes[a].head[0], record.nodes[b].head[1]];
    for (side, old) in next.into_iter().enumerate() {
        if let Some(old) = old {
            record.edges[old as usize].prev[side] = Some(id);
        }
    }
    record.nodes[a].head[0] = Some(id);
    record.nodes[b].head[1] = Some(id);
    record.edges.push(GraphEdge {
        node: [a as u32, b as u32],
        next,
        prev: [None; 2],
        row: id,
    });
}
fn fixture() -> GraphRecord {
    let mut record = GraphRecord {
        identity: identity(),
        nodes: (0..3)
            .map(|i| GraphNode {
                head: [None; 2],
                row: (i + 2) % 3,
            })
            .collect(),
        edges: vec![],
    };
    for (a, b) in [(0, 1), (1, 2), (2, 0), (0, 1), (1, 1)] {
        add(&mut record, a, b);
    }
    for (i, edge) in record.edges.iter_mut().enumerate() {
        edge.row = 4 - i as u32;
    }
    record
}
fn rejected(record: GraphRecord, error: Error) {
    assert_eq!(validate_graph(record, &mut budget()).err(), Some(error));
}
fn write(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn exact_adjacency_preserves_cycles_self_loops_parallel_edges_and_payload_order() {
    let record = fixture();
    let checked = validate_graph(record.clone(), &mut budget()).unwrap();
    let bytes = encode_graph(&checked, &mut budget()).unwrap();
    let loaded = decode_graph(&bytes, identity(), &mut budget()).unwrap();
    assert_eq!(loaded.record(), &record);
    assert_eq!(encode_graph(&loaded, &mut budget()).unwrap(), bytes);
    for (node, expected) in [(0, vec![3, 0, 2]), (1, vec![4, 1, 3, 0]), (2, vec![2, 1])] {
        assert_eq!(loaded.incident(node).unwrap().collect::<Vec<_>>(), expected);
    }
    assert_eq!(
        loaded
            .adjacency(1, Side::Outgoing)
            .unwrap()
            .collect::<Vec<_>>(),
        [4, 1]
    );
    assert_eq!(
        loaded
            .adjacency(1, Side::Incoming)
            .unwrap()
            .collect::<Vec<_>>(),
        [4, 3, 0]
    );
    assert!(loaded.adjacency(3, Side::Outgoing).is_none());
    assert!(loaded.incident(u32::MAX).is_none());
    let mut iter = loaded.incident(0).unwrap();
    while iter.next().is_some() {}
    assert_eq!(iter.next(), None);
    assert_eq!(iter.next(), None);
}

#[test]
fn empty_isolated_and_single_self_loop_graphs_are_admitted() {
    for n in [0, 1, 4] {
        let r = GraphRecord {
            identity: identity(),
            nodes: (0..n)
                .map(|row| GraphNode {
                    head: [None; 2],
                    row,
                })
                .collect(),
            edges: vec![],
        };
        let v = validate_graph(r.clone(), &mut budget()).unwrap();
        let bytes = encode_graph(&v, &mut budget()).unwrap();
        assert_eq!(
            decode_graph(&bytes, identity(), &mut budget())
                .unwrap()
                .record(),
            &r
        );
    }
    let mut r = GraphRecord {
        identity: identity(),
        nodes: vec![GraphNode {
            head: [None; 2],
            row: 0,
        }],
        edges: vec![],
    };
    add(&mut r, 0, 0);
    let v = validate_graph(r, &mut budget()).unwrap();
    assert_eq!(v.incident(0).unwrap().collect::<Vec<_>>(), [0]);
    let mut r = fixture();
    r.nodes.clear();
    rejected(r, Error::Length);
}

#[test]
fn malformed_endpoints_heads_and_each_link_side_are_rejected() {
    for side in 0..2 {
        let mut r = fixture();
        r.nodes[0].head[side] = Some(5);
        rejected(r, Error::Adjacency);
        let mut r = fixture();
        r.edges[0].node[side] = 3;
        rejected(r, Error::Adjacency);
        let mut r = fixture();
        r.edges[0].next[side] = Some(5);
        rejected(r, Error::Adjacency);
        let mut r = fixture();
        r.edges[0].prev[side] = Some(5);
        rejected(r, Error::Adjacency);
        let mut r = fixture();
        r.edges[0].node[side] = (r.edges[0].node[side] + 1) % 3;
        rejected(r, Error::Adjacency);
    }
}

#[test]
fn list_cycles_shared_tails_and_unreachable_edges_cannot_pass_coverage() {
    let mut r = fixture();
    r.edges[0].next[0] = Some(3);
    r.edges[3].prev[0] = Some(0);
    rejected(r, Error::Adjacency);
    let mut r = fixture();
    r.edges[3].next[0] = Some(3);
    rejected(r, Error::Adjacency);
    let mut r = fixture();
    r.nodes[1].head[0] = r.nodes[0].head[0];
    rejected(r, Error::Adjacency);
    let mut r = fixture();
    r.nodes[0].head[0] = None;
    rejected(r, Error::Adjacency);
    let mut r = fixture();
    for node in &mut r.nodes {
        node.head = [None; 2];
    }
    rejected(r, Error::Adjacency);
    let mut r = fixture();
    r.edges[0].prev[0] = None;
    rejected(r, Error::Adjacency);
    let mut r = fixture();
    r.edges[0].prev[1] = Some(1);
    rejected(r, Error::Adjacency);
    let mut r = fixture();
    r.edges[4].prev[1] = Some(3);
    rejected(r, Error::Adjacency);
    let mut r = fixture();
    r.edges[4].next[1] = Some(0);
    r.edges[0].prev[1] = Some(4);
    rejected(r, Error::Adjacency);
}

#[test]
fn both_dense_payload_tables_must_be_bijections() {
    let mut r = fixture();
    r.nodes[0].row = r.nodes[1].row;
    rejected(r, Error::Ownership);
    let mut r = fixture();
    r.nodes[0].row = 3;
    rejected(r, Error::Ownership);
    let mut r = fixture();
    r.edges[0].row = r.edges[1].row;
    rejected(r, Error::Ownership);
    let mut r = fixture();
    r.edges[0].row = 5;
    rejected(r, Error::Ownership);
}

#[test]
fn graph_wire_admission_rejects_all_truncations_and_invalid_headers_before_allocation() {
    let v = validate_graph(fixture(), &mut budget()).unwrap();
    let bytes = encode_graph(&v, &mut budget()).unwrap();
    for n in 0..bytes.len() {
        assert!(decode_graph(&bytes[..n], identity(), &mut budget()).is_err());
    }
    for offset in [0, 8, 10, 11, 12, 52] {
        let mut bad = bytes.clone();
        bad[offset] ^= 0x80;
        let mut zero = Budget::new(0).unwrap();
        assert!(decode_graph(&bad, identity(), &mut zero).is_err());
        assert_eq!(zero.remaining(), 0);
    }
    for offset in [44, 48] {
        let mut bad = bytes.clone();
        write(&mut bad, offset, u32::MAX);
        assert_eq!(
            decode_graph(&bad, identity(), &mut Budget::new(0).unwrap()).err(),
            Some(Error::Length)
        );
    }
    let mut bad = bytes.clone();
    bad.push(0);
    assert_eq!(
        decode_graph(&bad, identity(), &mut budget()).err(),
        Some(Error::Length)
    );
    let mut other = identity();
    other.dimension = Dimension::Planar;
    assert_eq!(
        decode_graph(&bytes, other, &mut budget()).err(),
        Some(Error::Identity)
    );
}

#[test]
fn graph_budget_charges_records_scratch_and_output_cumulatively() {
    let v = validate_graph(fixture(), &mut budget()).unwrap();
    let bytes = encode_graph(&v, &mut budget()).unwrap();
    let charge = 3 * size_of::<GraphNode>() + 5 * size_of::<GraphEdge>() + 3 + 3 * 5;
    let mut exact = Budget::new(charge + bytes.len()).unwrap();
    let loaded = decode_graph(&bytes, identity(), &mut exact).unwrap();
    assert_eq!(encode_graph(&loaded, &mut exact).unwrap(), bytes);
    assert_eq!(exact.remaining(), 0);
    assert_eq!(
        decode_graph(&bytes, identity(), &mut Budget::new(charge - 1).unwrap()).err(),
        Some(Error::Limit)
    );
    let mut bad = fixture();
    bad.edges[0].next[0] = Some(u32::MAX);
    let mut zero = Budget::new(0).unwrap();
    assert_eq!(validate_graph(bad, &mut zero).err(), Some(Error::Adjacency));
    assert_eq!(zero.remaining(), 0);
}

#[test]
fn long_adjacency_and_hostile_byte_mutations_remain_bounded() {
    let mut r = GraphRecord {
        identity: identity(),
        nodes: vec![GraphNode {
            head: [None; 2],
            row: 0,
        }],
        edges: vec![],
    };
    for _ in 0..4096 {
        add(&mut r, 0, 0);
    }
    let v = validate_graph(r, &mut budget()).unwrap();
    assert_eq!(v.incident(0).unwrap().count(), 4096);
    assert_eq!(v.adjacency(0, Side::Incoming).unwrap().count(), 4096);
    let v = validate_graph(fixture(), &mut budget()).unwrap();
    let bytes = encode_graph(&v, &mut budget()).unwrap();
    for index in 0..bytes.len() {
        let mut bad = bytes.clone();
        bad[index] ^= 0x80;
        if let Ok(value) = decode_graph(&bad, identity(), &mut budget()) {
            assert_eq!(encode_graph(&value, &mut budget()).unwrap(), bad);
        }
    }
    let mut seed = 7u32;
    for len in 0..1024 {
        let bytes: Vec<_> = (0..len)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                (seed >> 24) as u8
            })
            .collect();
        assert!(decode_graph(&bytes, identity(), &mut budget()).is_err());
    }
}

use incant_physics_snapshot::*;

fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn identity() -> Identity {
    Identity {
        dimension: Dimension::Spatial,
        build: [42; 32],
    }
}
fn leaf(id: u32, extent: f32) -> Node {
    Node {
        mins: [(-extent).to_bits(); 3],
        children: id,
        maxs: [extent.to_bits(); 3],
        data: 1,
    }
}
fn fixture() -> BvhRecord {
    let mut slots = vec![None; 10];
    slots[2] = Some(0);
    slots[4] = Some(2);
    slots[7] = Some(3);
    BvhRecord {
        identity: identity(),
        nodes: vec![
            Wide {
                left: leaf(2, 1.),
                right: Node {
                    children: 1,
                    data: 2,
                    ..leaf(0, 3.)
                },
            },
            Wide {
                left: leaf(4, 2.),
                right: leaf(7, 3.),
            },
            Wide::default(),
            Wide::default(),
        ],
        parents: vec![0, 1, 0, 1],
        leaves: LeafMap { occupied: 3, slots },
        free: vec![3, 2],
        rebuild_frame: u32::MAX,
        rebuild_start: 9,
    }
}
fn refused(record: BvhRecord, error: Error) {
    assert_eq!(validate(record, &mut budget()).err(), Some(error));
}

#[test]
fn exact_sparse_slots_free_order_signed_zero_and_counter_roundtrip() {
    let mut input = fixture();
    input.nodes[0].left.mins[0] = (-0.0f32).to_bits();
    let checked = validate(input.clone(), &mut budget()).unwrap();
    let bytes = encode(&checked, &mut budget()).unwrap();
    let loaded = decode(&bytes, identity(), &mut budget()).unwrap();
    assert_eq!(loaded.record(), &input);
    assert_eq!(encode(&loaded, &mut budget()).unwrap(), bytes);
    let mut swapped = input.clone();
    swapped.free.swap(0, 1);
    assert_ne!(
        encode(&validate(swapped, &mut budget()).unwrap(), &mut budget()).unwrap(),
        bytes
    );
}

#[test]
fn empty_and_single_leaf_keep_sparse_map_holes() {
    for occupied in [0, 1] {
        let mut record = BvhRecord {
            identity: identity(),
            nodes: vec![],
            parents: vec![],
            leaves: LeafMap {
                occupied,
                slots: vec![None; 20],
            },
            free: vec![],
            rebuild_frame: 42,
            rebuild_start: 17,
        };
        if occupied == 1 {
            record.nodes.push(Wide {
                left: leaf(19, 1.),
                right: Node::default(),
            });
            record.parents.push(0);
            record.leaves.slots[19] = Some(0);
        }
        let data = encode(
            &validate(record.clone(), &mut budget()).unwrap(),
            &mut budget(),
        )
        .unwrap();
        assert_eq!(
            decode(&data, identity(), &mut budget()).unwrap().record(),
            &record
        );
    }
}

#[test]
fn every_truncation_and_trailing_bytes_refuse() {
    let bytes = encode(&validate(fixture(), &mut budget()).unwrap(), &mut budget()).unwrap();
    for len in 0..bytes.len() {
        assert!(
            decode(&bytes[..len], identity(), &mut budget()).is_err(),
            "length{len}"
        );
    }
    let mut extra = bytes;
    extra.push(0);
    assert_eq!(
        decode(&extra, identity(), &mut budget()).err(),
        Some(Error::Length)
    );
}

#[test]
fn headers_lengths_identity_and_reserved_fields_fail_before_allocation() {
    let bytes = encode(&validate(fixture(), &mut budget()).unwrap(), &mut budget()).unwrap();
    for offset in [0, 8, 10, 11, 12, 16, 48, 52, 56, 60, 64, 76] {
        let mut bad = bytes.clone();
        bad[offset] ^= 0xff;
        let mut zero = Budget::new(0).unwrap();
        assert!(
            decode(&bad, identity(), &mut zero).is_err(),
            "offset{offset}"
        );
        assert_eq!(zero.remaining(), 0);
    }
    let mut wrong = identity();
    wrong.dimension = Dimension::Planar;
    assert_eq!(
        decode(&bytes, wrong, &mut budget()).err(),
        Some(Error::Identity)
    );
    let mut bad = bytes;
    bad[48..52].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        decode(&bad, identity(), &mut budget()).err(),
        Some(Error::Length)
    );
}

#[test]
fn whole_record_and_scratch_allocation_admission_is_cumulative() {
    let bytes = encode(&validate(fixture(), &mut budget()).unwrap(), &mut budget()).unwrap();
    assert_eq!(
        decode(&bytes, identity(), &mut Budget::new(0).unwrap()).err(),
        Some(Error::Limit)
    );
    let mut allowance = budget();
    decode(&bytes, identity(), &mut allowance).unwrap();
    let consumed = MAX_ALLOCATION - allowance.remaining();
    let mut exact = Budget::new(consumed).unwrap();
    decode(&bytes, identity(), &mut exact).unwrap();
    assert_eq!(exact.remaining(), 0);
    assert_eq!(
        decode(&bytes, identity(), &mut exact).err(),
        Some(Error::Limit)
    );
    assert_eq!(
        decode(&bytes, identity(), &mut Budget::new(consumed - 1).unwrap()).err(),
        Some(Error::Limit)
    );
    assert!(Budget::new(MAX_ALLOCATION + 1).is_err());
}

#[test]
fn cycles_shared_subtrees_and_free_live_aliases_refuse() {
    let mut r = fixture();
    r.nodes[0].right.children = 0;
    refused(r, Error::Ownership);
    let mut r = fixture();
    r.nodes[1].left = Node {
        children: 1,
        data: 2,
        ..leaf(0, 2.)
    };
    refused(r, Error::Metadata);
    let mut r = fixture();
    r.nodes[0].left = r.nodes[0].right;
    refused(r, Error::Parent);
    let mut r = fixture();
    r.nodes[0].left = r.nodes[0].right;
    r.parents[1] = 0;
    refused(r, Error::Ownership);
    let mut r = fixture();
    r.nodes[0].right.children = 2;
    refused(r, Error::Ownership);
    let mut r = fixture();
    r.nodes[0].right.children = u32::MAX;
    refused(r, Error::Ownership);
}

#[test]
fn reverse_leaf_counts_and_parent_identity_are_exact() {
    let mut r = fixture();
    r.leaves.occupied = 2;
    refused(r, Error::Leaf);
    let mut r = fixture();
    r.leaves.slots[4] = Some(3);
    refused(r, Error::Leaf);
    let mut r = fixture();
    r.leaves.slots[4] = None;
    r.leaves.slots[5] = Some(2);
    refused(r, Error::Leaf);
    let mut r = fixture();
    r.nodes[1].right.children = 4;
    refused(r, Error::Leaf);
    let mut r = fixture();
    r.nodes[1].right.children = u32::MAX;
    refused(r, Error::Leaf);
    let mut r = fixture();
    r.parents[1] = 0;
    refused(r, Error::Parent);
    let mut r = fixture();
    r.parents[0] = 1;
    refused(r, Error::Parent);
}

#[test]
fn free_partition_requires_unique_zero_slots_and_no_unreachable_storage() {
    let mut r = fixture();
    r.free = vec![2, 2];
    refused(r, Error::Free);
    let mut r = fixture();
    r.free = vec![0, 2];
    refused(r, Error::Free);
    let mut r = fixture();
    r.free = vec![2, 99];
    refused(r, Error::Free);
    let mut r = fixture();
    r.free = vec![2];
    refused(r, Error::Ownership);
    let mut r = fixture();
    r.nodes[2].left = leaf(9, 1.);
    refused(r, Error::Free);
    let mut r = fixture();
    r.parents[2] = u32::MAX;
    refused(r, Error::Parent);
}

#[test]
fn aabbs_metadata_and_pending_flags_never_get_normalized() {
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, 1e10] {
        let mut r = fixture();
        r.nodes[1].left.maxs[0] = value.to_bits();
        refused(r, Error::Bounds);
    }
    let mut r = fixture();
    r.nodes[1].left.mins[0] = 3f32.to_bits();
    refused(r, Error::Bounds);
    let mut r = fixture();
    r.nodes[0].right.maxs[1] = 2f32.to_bits();
    refused(r, Error::Bounds);
    let mut r = fixture();
    r.nodes[0].right.data = 3;
    refused(r, Error::Metadata);
    let mut r = fixture();
    r.nodes[0].left.data = 0x8000_0001;
    refused(r, Error::Metadata);
    let mut r = fixture();
    r.nodes[1].right.data |= 3 << 30;
    refused(r.clone(), Error::Metadata);
    r.nodes[0].right.data |= 3 << 30;
    let original = r.clone();
    let v = validate(r, &mut budget()).unwrap();
    assert_eq!(v.record(), &original);
    let mut r = fixture();
    r.rebuild_start = MAX_ITEMS as u32 + 1;
    refused(r, Error::Metadata);
}

#[test]
fn planar_unused_axis_and_single_leaf_sentinel_are_canonical() {
    let mut r = fixture();
    r.identity.dimension = Dimension::Planar;
    refused(r.clone(), Error::Bounds);
    for n in &mut r.nodes {
        for node in [&mut n.left, &mut n.right] {
            node.mins[2] = 0;
            node.maxs[2] = 0;
        }
    }
    assert!(validate(r.clone(), &mut budget()).is_ok());
    r.nodes[0].left.mins[2] = (-0.0f32).to_bits();
    refused(r, Error::Bounds);
    let r = BvhRecord {
        identity: identity(),
        nodes: vec![Wide {
            left: leaf(0, 1.),
            right: leaf(0, 1.),
        }],
        parents: vec![0],
        leaves: LeafMap {
            occupied: 1,
            slots: vec![Some(0)],
        },
        free: vec![],
        rebuild_frame: 0,
        rebuild_start: 0,
    };
    refused(r, Error::Node);
}

#[test]
fn arbitrary_bytes_and_single_byte_mutations_are_total() {
    let mut seed = 123u32;
    for len in 0..1024 {
        let bytes: Vec<_> = (0..len)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (seed >> 24) as u8
            })
            .collect();
        assert!(decode(&bytes, identity(), &mut budget()).is_err());
    }
    let original = encode(&validate(fixture(), &mut budget()).unwrap(), &mut budget()).unwrap();
    for offset in 0..original.len() {
        let mut bytes = original.clone();
        bytes[offset] ^= 0x80;
        if let Ok(valid) = decode(&bytes, identity(), &mut budget()) {
            assert_eq!(encode(&valid, &mut budget()).unwrap(), bytes);
        }
    }
}

#[test]
fn depth_is_bounded_without_recursive_validation() {
    for depth in [MAX_DEPTH, MAX_DEPTH + 1] {
        let mut r = BvhRecord {
            identity: identity(),
            nodes: vec![],
            parents: vec![],
            leaves: LeafMap {
                occupied: depth + 1,
                slots: vec![None; depth as usize + 1],
            },
            free: vec![],
            rebuild_frame: 0,
            rebuild_start: 0,
        };
        for i in 0..depth {
            let right = if i + 1 == depth {
                leaf(depth, 1.)
            } else {
                Node {
                    children: i + 1,
                    data: depth - i,
                    ..leaf(0, 1.)
                }
            };
            r.nodes.push(Wide {
                left: leaf(i, 1.),
                right,
            });
            r.parents.push(if i == 0 { 0 } else { (i - 1) * 2 + 1 });
            r.leaves.slots[i as usize] = Some(i * 2);
        }
        r.leaves.slots[depth as usize] = Some((depth - 1) * 2 + 1);
        let result = validate(r, &mut budget());
        if depth == MAX_DEPTH {
            assert!(result.is_ok());
        } else {
            assert_eq!(result.err(), Some(Error::Depth));
        }
    }
}

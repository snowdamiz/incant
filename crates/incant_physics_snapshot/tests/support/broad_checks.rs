use incant_physics_snapshot::*;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
pub fn check(r: &BroadOwnerRecord, c: BroadOwnerContext<'_>, arena: &ValidatedArena) {
    let v = validate_broad_owner(r.clone(), c, &mut budget()).unwrap();
    let b = encode_broad_owner(&v, &mut budget()).unwrap();
    for n in 0..b.len() {
        assert!(decode_broad_owner(&b[..n], c, &mut budget()).is_err());
    }
    let mut extra = b.clone();
    extra.push(0);
    assert!(decode_broad_owner(&extra, c, &mut budget()).is_err());
    for offset in [44, 48, 52, 56, 60, 72, 76, 80] {
        let mut bad = b.clone();
        bad[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
        let mut budget = budget();
        let before = budget.remaining();
        assert!(decode_broad_owner(&bad, c, &mut budget).is_err());
        assert_eq!(before, budget.remaining());
    }
    for mutate in [
        |r: &mut BroadOwnerRecord| r.pairs[0].colliders.swap(0, 1),
        |r: &mut BroadOwnerRecord| r.pairs[1] = r.pairs[0],
        |r: &mut BroadOwnerRecord| r.pairs[0].colliders[0].generation += 1,
        |r: &mut BroadOwnerRecord| r.neighbors[0] = r.neighbors[1],
        |r: &mut BroadOwnerRecord| r.adjacency[0].start = 1,
        |r: &mut BroadOwnerRecord| r.previous_leaves.push(u32::MAX),
        |r: &mut BroadOwnerRecord| {
            r.pending_aabbs.push(Handle {
                slot: 0,
                generation: u32::MAX,
            })
        },
    ] {
        let mut bad = r.clone();
        mutate(&mut bad);
        assert!(validate_broad_owner(bad, c, &mut budget()).is_err());
    }
    assert!(
        r.pending_aabbs.is_empty(),
        "stationary three-pair hostile fixture"
    );
    let empty = validate_pending_bvh(
        BvhRecord {
            identity: r.identity,
            nodes: vec![],
            parents: vec![],
            leaves: LeafMap::default(),
            free: vec![],
            rebuild_frame: 0,
            rebuild_start: 0,
        },
        PendingBvhContext {
            colliders: c.tree.context().colliders,
            pending_aabbs: &[],
        },
        &mut budget(),
    )
    .unwrap();
    assert!(matches!(
        validate_broad_owner(
            r.clone(),
            BroadOwnerContext { tree: &empty, ..c },
            &mut budget()
        ),
        Err(Error::BroadTree)
    ));
    let other = validate_arena(arena.record().clone(), &mut budget()).unwrap();
    let wrong = validate_coarena(c.adjacency.record().clone(), &other, &mut budget()).unwrap();
    assert!(matches!(
        validate_broad_owner(
            r.clone(),
            BroadOwnerContext {
                adjacency: &wrong,
                ..c
            },
            &mut budget()
        ),
        Err(Error::Identity)
    ));
    let mut co = c.adjacency.record().clone();
    let mut reordered = r.clone();
    let mut neighbors = vec![];
    reordered.adjacency.clear();
    for s in r.adjacency.iter().rev() {
        let start = neighbors.len() as u32;
        neighbors.extend_from_slice(&r.neighbors[s.start as usize..(s.start + s.count) as usize]);
        reordered.adjacency.push(RecordSpan {
            start,
            count: s.count,
        });
    }
    reordered.neighbors = neighbors;
    let n = co.rows.len() as u32;
    for row in &mut co.rows {
        row.row = n - 1 - row.row;
    }
    let co = validate_coarena(co, arena, &mut budget()).unwrap();
    let v2 = validate_broad_owner(
        reordered,
        BroadOwnerContext {
            adjacency: &co,
            ..c
        },
        &mut budget(),
    )
    .unwrap();
    for p in &r.pairs {
        for h in p.colliders {
            assert_eq!(v.neighbors(h), v2.neighbors(h));
        }
    }
    // A valid ordered list can be reversed without changing set membership.
    let mut reversed = r.clone();
    for s in &r.adjacency {
        reversed.neighbors[s.start as usize..(s.start + s.count) as usize].reverse();
    }
    let v = validate_broad_owner(reversed.clone(), c, &mut budget()).unwrap();
    let bytes = encode_broad_owner(&v, &mut budget()).unwrap();
    assert_eq!(
        decode_broad_owner(&bytes, c, &mut budget())
            .unwrap()
            .record(),
        &reversed
    );
    assert!(decode_broad_owner(&b, c, &mut Budget::new(0).unwrap()).is_err());
    for offset in 0..b.len() {
        for bit in 0..8 {
            let mut changed = b.clone();
            changed[offset] ^= 1 << bit;
            if let Ok(v) = decode_broad_owner(&changed, c, &mut budget()) {
                assert_eq!(encode_broad_owner(&v, &mut budget()).unwrap(), changed);
            }
        }
    }
}

/// Adversarial changes target the inert witness only, never Rapier serde.
pub fn pending(r: &BvhRecord, c: PendingBvhContext<'_>) {
    let v = validate_pending_bvh(r.clone(), c, &mut budget()).unwrap();
    let bytes = encode_pending_bvh(&v, &mut budget()).unwrap();
    if validate(r.clone(), &mut budget()).is_err() {
        assert!(
            decode(&bytes, r.identity, &mut budget()).is_err(),
            "strict entry must not inherit pending exceptions"
        );
    }
    let mut stale = c.pending_aabbs.to_vec();
    stale[0].generation = stale[0].generation.wrapping_add(1);
    assert!(matches!(
        validate_pending_bvh(
            r.clone(),
            PendingBvhContext {
                pending_aabbs: &stale,
                ..c
            },
            &mut budget()
        ),
        Err(Error::Generation)
    ));
    assert!(
        validate_pending_bvh(
            r.clone(),
            PendingBvhContext {
                pending_aabbs: &[],
                ..c
            },
            &mut budget()
        )
        .is_err()
    );
    assert!(validate_pending_bvh(r.clone(), c, &mut Budget::new(0).unwrap()).is_err());
    let packed = r.leaves.slots[c.pending_aabbs[0].slot as usize].unwrap();
    for kind in 0..4 {
        let mut bad = r.clone();
        let n = if packed & 1 == 0 {
            &mut bad.nodes[packed as usize / 2].left
        } else {
            &mut bad.nodes[packed as usize / 2].right
        };
        match kind {
            0 => n.data = (n.data & 0x3fff_ffff) | 1 << 30,
            1 => n.mins[0] = f32::NAN.to_bits(),
            2 => n.data = (n.data & 0xc000_0000) | 2,
            _ => n.children = u32::MAX,
        }
        assert!(validate_pending_bvh(bad, c, &mut budget()).is_err());
    }
    if r.nodes.len() > 1 {
        let mut bad = r.clone();
        bad.parents[1] = u32::MAX;
        assert!(validate_pending_bvh(bad, c, &mut budget()).is_err());
        let mut bad = r.clone();
        bad.free.push(1);
        assert!(validate_pending_bvh(bad, c, &mut budget()).is_err());
    }
    // Nonpending siblings retain strict containment even under a pending parent.
    for (slot, packed) in r.leaves.slots.iter().enumerate() {
        let Some(packed) = *packed else { continue };
        if packed / 2 == 0 || c.pending_aabbs.iter().any(|h| h.slot as usize == slot) {
            continue;
        }
        let mut bad = r.clone();
        let n = if packed & 1 == 0 {
            &mut bad.nodes[packed as usize / 2].left
        } else {
            &mut bad.nodes[packed as usize / 2].right
        };
        n.mins[0] = 900_000_000f32.to_bits();
        n.maxs[0] = 900_000_001f32.to_bits();
        assert!(matches!(
            validate_pending_bvh(bad, c, &mut budget()),
            Err(Error::Bounds)
        ));
    }
    // Duplicate native set_aabb calls are historical queue entries, not an
    // invalid duplicate ownership claim. Keep their exact ordering on both sides.
    let mut duplicate = c.pending_aabbs.to_vec();
    duplicate.push(duplicate[0]);
    let doubled = validate_pending_bvh(
        r.clone(),
        PendingBvhContext {
            pending_aabbs: &duplicate,
            ..c
        },
        &mut budget(),
    )
    .unwrap();
    assert_eq!(encode_pending_bvh(&doubled, &mut budget()).unwrap(), bytes);
    for n in 0..bytes.len() {
        assert!(decode_pending_bvh(&bytes[..n], c, &mut budget()).is_err());
    }
}

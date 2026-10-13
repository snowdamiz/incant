use incant_physics_snapshot::*;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn identity() -> Identity {
    Identity {
        dimension: Dimension::Spatial,
        build: [41; 32],
    }
}
fn handle(slot: u32, generation: u32) -> Handle {
    Handle { slot, generation }
}
fn arena(slots: Vec<ArenaSlot>, free_head: Option<u32>, live: u32) -> ValidatedArena {
    validate_arena(
        ArenaRecord {
            identity: identity(),
            generation: 3,
            free_head,
            live,
            slots,
        },
        &mut budget(),
    )
    .unwrap()
}
fn occupied(generation: u32, row: u32) -> ArenaSlot {
    ArenaSlot::Occupied { generation, row }
}
fn layouts() -> (ValidatedArena, ValidatedArena) {
    (
        arena(
            vec![
                occupied(2, 1),
                ArenaSlot::Free { next: None },
                occupied(1, 0),
            ],
            Some(1),
            2,
        ),
        arena(
            vec![
                occupied(2, 1),
                ArenaSlot::Free { next: Some(5) },
                occupied(1, 0),
                occupied(3, 2),
                occupied(0, 3),
                ArenaSlot::Free { next: None },
            ],
            Some(1),
            4,
        ),
    )
}
fn pose() -> LocalPose {
    LocalPose {
        translation: [(-0f32).to_bits(), 1, 2f32.to_bits()],
        rotation: [(-0f32).to_bits(), 0, 0, (-1f32).to_bits()],
    }
}
fn fixture() -> AttachmentRecord {
    AttachmentRecord {
        identity: identity(),
        bodies: vec![
            AttachmentSpan { start: 0, count: 2 },
            AttachmentSpan { start: 2, count: 1 },
        ],
        colliders: vec![
            Some(ParentAttachment {
                body: handle(2, 1),
                local_pose: pose(),
            }),
            Some(ParentAttachment {
                body: handle(0, 2),
                local_pose: pose(),
            }),
            Some(ParentAttachment {
                body: handle(2, 1),
                local_pose: pose(),
            }),
            None,
        ],
        attached: vec![handle(3, 3), handle(2, 1), handle(0, 2)],
    }
}
fn rejects(record: AttachmentRecord, expected: Error) {
    let (b, c) = layouts();
    assert_eq!(
        validate_attachments(record, &b, &c, &mut budget()).err(),
        Some(expected)
    );
}
fn word(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn exact_order_bits_and_checked_handle_queries() {
    let (b, c) = layouts();
    let record = fixture();
    let checked = validate_attachments(record.clone(), &b, &c, &mut budget()).unwrap();
    assert_eq!(
        checked.body_colliders(handle(2, 1)),
        Some(&record.attached[..2])
    );
    assert_eq!(
        checked.body_colliders(handle(0, 2)),
        Some(&record.attached[2..])
    );
    assert_eq!(checked.collider_parent(handle(4, 0)), Some(None));
    assert_eq!(
        checked
            .collider_parent(handle(3, 3))
            .flatten()
            .unwrap()
            .local_pose,
        pose()
    );
    for bad in [
        handle(1, 3),
        handle(2, 0),
        handle(2, u32::MAX),
        handle(u32::MAX, 1),
    ] {
        assert_eq!(checked.body_colliders(bad), None);
        assert_eq!(checked.collider_parent(bad), None);
    }
    let bytes = encode_attachments(&checked, &mut budget()).unwrap();
    let loaded = decode_attachments(&bytes, &b, &c, &mut budget()).unwrap();
    assert_eq!(loaded.record(), &record);
    assert_eq!(encode_attachments(&loaded, &mut budget()).unwrap(), bytes);
}

#[test]
fn empty_and_standalone_are_not_missing_owners() {
    let empty = arena(vec![ArenaSlot::Free { next: None }], Some(0), 0);
    let collider = arena(vec![occupied(0, 0)], None, 1);
    for (c, rows) in [(&empty, vec![]), (&collider, vec![None])] {
        let record = AttachmentRecord {
            identity: identity(),
            bodies: vec![],
            colliders: rows,
            attached: vec![],
        };
        let checked = validate_attachments(record.clone(), &empty, c, &mut budget()).unwrap();
        let bytes = encode_attachments(&checked, &mut budget()).unwrap();
        assert_eq!(
            decode_attachments(&bytes, &empty, c, &mut budget())
                .unwrap()
                .record(),
            &record
        );
    }
}

#[test]
fn strict_reciprocity_and_complete_unique_membership() {
    let mut r = fixture();
    r.attached[1] = r.attached[0];
    rejects(r, Error::Ownership);
    let mut r = fixture();
    r.attached.swap(1, 2);
    rejects(r, Error::Ownership);
    let mut r = fixture();
    r.colliders[2] = None;
    rejects(r, Error::Ownership);
    let mut r = fixture();
    r.colliders[3] = r.colliders[0];
    rejects(r, Error::Ownership);
    let mut r = fixture();
    r.colliders[0].as_mut().unwrap().body = handle(0, 2);
    rejects(r, Error::Ownership);
    for span in [
        AttachmentSpan { start: 1, count: 1 },
        AttachmentSpan { start: 0, count: 1 },
        AttachmentSpan {
            start: 0,
            count: u32::MAX,
        },
    ] {
        let mut r = fixture();
        r.bodies[0] = span;
        rejects(r, Error::Ownership);
    }
    let mut r = fixture();
    r.bodies[1].count = 0;
    rejects(r, Error::Ownership);
}

#[test]
fn stale_free_sentinel_and_foreign_layout_refusal() {
    for bad in [
        handle(1, 3),
        handle(2, 0),
        handle(2, u32::MAX),
        handle(u32::MAX, 1),
    ] {
        let mut r = fixture();
        r.attached[0] = bad;
        rejects(r, Error::Generation);
        let mut r = fixture();
        r.colliders[0].as_mut().unwrap().body = bad;
        rejects(r, Error::Generation);
    }
    let (b, c) = layouts();
    assert_eq!(
        validate_attachments(fixture(), &c, &b, &mut budget()).err(),
        Some(Error::Length)
    );
    let mut wrong = b.record().clone();
    wrong.identity.build[0] ^= 1;
    let wrong = validate_arena(wrong, &mut budget()).unwrap();
    assert_eq!(
        validate_attachments(fixture(), &wrong, &c, &mut budget()).err(),
        Some(Error::Identity)
    );
    let mut wrong = c.record().clone();
    wrong.slots[2] = occupied(2, 0);
    let wrong = validate_arena(wrong, &mut budget()).unwrap();
    assert_eq!(
        validate_attachments(fixture(), &b, &wrong, &mut budget()).err(),
        Some(Error::Generation)
    );
    let mut wrong = c.record().clone();
    wrong.slots.swap(0, 2);
    let wrong = validate_arena(wrong, &mut budget()).unwrap();
    assert!(validate_attachments(fixture(), &b, &wrong, &mut budget()).is_err());
    // Equal copies are structurally interchangeable, not authenticated owners.
    let same = validate_arena(b.record().clone(), &mut budget()).unwrap();
    assert!(validate_attachments(fixture(), &same, &c, &mut budget()).is_ok());
}

#[test]
fn native_rotation_domains_preserve_bits_without_normalizing() {
    let (b, c) = layouts();
    for t in [
        0,
        (-0f32).to_bits(),
        1,
        f32::MAX.to_bits(),
        (-f32::MAX).to_bits(),
    ] {
        let mut r = fixture();
        r.colliders[0].as_mut().unwrap().local_pose.translation[0] = t;
        let checked = validate_attachments(r.clone(), &b, &c, &mut budget()).unwrap();
        assert_eq!(checked.record(), &r);
    }
    for value in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut r = fixture();
        r.colliders[0].as_mut().unwrap().local_pose.translation[1] = value.to_bits();
        rejects(r, Error::Geometry);
        let mut r = fixture();
        r.colliders[0].as_mut().unwrap().local_pose.rotation[0] = value.to_bits();
        rejects(r, Error::Geometry);
    }
    let mut r = fixture();
    r.colliders[0].as_mut().unwrap().local_pose.rotation = [0; 4];
    rejects(r, Error::Geometry);
    let mut bp = b.record().clone();
    bp.identity.dimension = Dimension::Planar;
    let mut cp = c.record().clone();
    cp.identity.dimension = Dimension::Planar;
    let bp = validate_arena(bp, &mut budget()).unwrap();
    let cp = validate_arena(cp, &mut budget()).unwrap();
    let mut r = fixture();
    r.identity.dimension = Dimension::Planar;
    for p in r.colliders.iter_mut().flatten() {
        p.local_pose = LocalPose {
            translation: [(-0f32).to_bits(), 1, 0],
            rotation: [(-1f32).to_bits(), (-0f32).to_bits(), 0, 0],
        };
    }
    let checked = validate_attachments(r.clone(), &bp, &cp, &mut budget()).unwrap();
    let bytes = encode_attachments(&checked, &mut budget()).unwrap();
    assert_eq!(
        decode_attachments(&bytes, &bp, &cp, &mut budget())
            .unwrap()
            .record(),
        &r
    );
    for lane in 2..4 {
        let mut bad = r.clone();
        bad.colliders[0].as_mut().unwrap().local_pose.rotation[lane] = (-0f32).to_bits();
        assert_eq!(
            validate_attachments(bad, &bp, &cp, &mut budget()).err(),
            Some(Error::Geometry)
        );
    }
    r.colliders[0].as_mut().unwrap().local_pose.translation[2] = (-0f32).to_bits();
    assert_eq!(
        validate_attachments(r, &bp, &cp, &mut budget()).err(),
        Some(Error::Geometry)
    );
}

#[test]
fn bounded_frame_rejects_truncation_counts_tags_padding_and_identity() {
    let (b, c) = layouts();
    let checked = validate_attachments(fixture(), &b, &c, &mut budget()).unwrap();
    let bytes = encode_attachments(&checked, &mut budget()).unwrap();
    for n in 0..bytes.len() {
        assert!(decode_attachments(&bytes[..n], &b, &c, &mut budget()).is_err());
    }
    for (offset, value) in [(44, u32::MAX), (48, u32::MAX), (52, u32::MAX), (56, 1)] {
        let mut bad = bytes.clone();
        word(&mut bad, offset, value);
        let mut budget = Budget::new(0).unwrap();
        assert!(decode_attachments(&bad, &b, &c, &mut budget).is_err());
        assert_eq!(budget.remaining(), 0);
    }
    for (offset, value) in [(0, 0), (8, 2), (10, 2), (11, 2), (12, 99)] {
        let mut bad = bytes.clone();
        bad[offset] = value;
        assert!(decode_attachments(&bad, &b, &c, &mut budget()).is_err());
    }
    let first = 60 + 2 * 8;
    let mut bad = bytes.clone();
    word(&mut bad, first, 2);
    assert_eq!(
        decode_attachments(&bad, &b, &c, &mut budget()).err(),
        Some(Error::Header)
    );
    let mut bad = bytes.clone();
    word(&mut bad, first + 3 * 40 + 4, 1);
    assert_eq!(
        decode_attachments(&bad, &b, &c, &mut budget()).err(),
        Some(Error::Header)
    );
    let mut bad = bytes;
    bad.push(0);
    assert_eq!(
        decode_attachments(&bad, &b, &c, &mut budget()).err(),
        Some(Error::Length)
    );
}

#[test]
fn aggregate_allocation_budget_includes_scratch_and_encoded_output() {
    let (b, c) = layouts();
    assert_eq!(
        validate_attachments(fixture(), &b, &c, &mut Budget::new(3).unwrap()).err(),
        Some(Error::Limit)
    );
    let mut exact = Budget::new(4).unwrap();
    let checked = validate_attachments(fixture(), &b, &c, &mut exact).unwrap();
    assert_eq!(exact.remaining(), 0);
    let bytes = encode_attachments(&checked, &mut budget()).unwrap();
    assert_eq!(
        encode_attachments(&checked, &mut Budget::new(bytes.len() - 1).unwrap()).err(),
        Some(Error::Limit)
    );
    let charge = 2 * size_of::<AttachmentSpan>()
        + 4 * size_of::<Option<ParentAttachment>>()
        + 3 * size_of::<Handle>()
        + 4;
    assert_eq!(
        decode_attachments(&bytes, &b, &c, &mut Budget::new(charge - 1).unwrap()).err(),
        Some(Error::Limit)
    );
    let mut exact = Budget::new(charge).unwrap();
    decode_attachments(&bytes, &b, &c, &mut exact).unwrap();
    assert_eq!(exact.remaining(), 0);
    let mut shared = Budget::new(charge + bytes.len()).unwrap();
    let loaded = decode_attachments(&bytes, &b, &c, &mut shared).unwrap();
    assert_eq!(encode_attachments(&loaded, &mut shared).unwrap(), bytes);
    assert_eq!(shared.remaining(), 0);
}

#[test]
fn long_ordered_membership_and_hostile_bytes_are_bounded() {
    let n = 4096u32;
    let b = arena(vec![occupied(1, 0)], None, 1);
    let c = arena((0..n).map(|row| occupied(1, row)).collect(), None, n);
    let record = AttachmentRecord {
        identity: identity(),
        bodies: vec![AttachmentSpan { start: 0, count: n }],
        colliders: vec![
            Some(ParentAttachment {
                body: handle(0, 1),
                local_pose: pose()
            });
            n as usize
        ],
        attached: (0..n).rev().map(|slot| handle(slot, 1)).collect(),
    };
    let checked = validate_attachments(record.clone(), &b, &c, &mut budget()).unwrap();
    let bytes = encode_attachments(&checked, &mut budget()).unwrap();
    assert_eq!(
        decode_attachments(&bytes, &b, &c, &mut budget())
            .unwrap()
            .record(),
        &record
    );
    let (b, c) = layouts();
    let checked = validate_attachments(fixture(), &b, &c, &mut budget()).unwrap();
    let bytes = encode_attachments(&checked, &mut budget()).unwrap();
    for index in 0..bytes.len() {
        let mut bad = bytes.clone();
        bad[index] ^= 0xff;
        if let Ok(loaded) = decode_attachments(&bad, &b, &c, &mut budget()) {
            assert_eq!(encode_attachments(&loaded, &mut budget()).unwrap(), bad);
        }
    }
    let mut seed = 0x41c5_7331u32;
    for n in 0..4096 {
        let mut bad = vec![0; n % 512];
        for value in &mut bad {
            seed ^= seed << 13;
            seed ^= seed >> 17;
            seed ^= seed << 5;
            *value = seed as u8;
        }
        assert!(decode_attachments(&bad, &b, &c, &mut budget()).is_err());
    }
}

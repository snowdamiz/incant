use incant_physics_snapshot::*;

fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn identity() -> Identity {
    Identity {
        dimension: Dimension::Spatial,
        build: [29; 32],
    }
}
fn fixture() -> ArenaRecord {
    ArenaRecord {
        identity: identity(),
        generation: 8,
        live: 2,
        free_head: Some(3),
        slots: vec![
            ArenaSlot::Occupied {
                generation: 2,
                row: 1,
            },
            ArenaSlot::Free { next: None },
            ArenaSlot::Occupied {
                generation: 8,
                row: 0,
            },
            ArenaSlot::Free { next: Some(1) },
        ],
    }
}
fn coarena() -> CoarenaRecord {
    CoarenaRecord {
        identity: identity(),
        rows: vec![
            CoarenaRow {
                generation: 2,
                row: 3,
            },
            CoarenaRow {
                generation: 3,
                row: 2,
            },
            CoarenaRow {
                generation: 7,
                row: 1,
            },
            CoarenaRow {
                generation: u32::MAX,
                row: 0,
            },
        ],
    }
}
fn rejected(record: ArenaRecord, error: Error) {
    assert_eq!(validate_arena(record, &mut budget()).err(), Some(error));
}
fn write(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn exact_slots_free_reuse_order_generations_and_payload_bijection() {
    let input = fixture();
    let checked = validate_arena(input.clone(), &mut budget()).unwrap();
    let bytes = encode_arena(&checked, &mut budget()).unwrap();
    let loaded = decode_arena(&bytes, identity(), &mut budget()).unwrap();
    assert_eq!(loaded.record(), &input);
    assert_eq!(encode_arena(&loaded, &mut budget()).unwrap(), bytes);
    assert_eq!(
        loaded.lookup(Handle {
            slot: 0,
            generation: 2
        }),
        Some(1)
    );
    assert_eq!(
        loaded.lookup(Handle {
            slot: 2,
            generation: 8
        }),
        Some(0)
    );
    for h in [
        Handle {
            slot: 0,
            generation: 8,
        },
        Handle {
            slot: 1,
            generation: 8,
        },
        Handle {
            slot: u32::MAX,
            generation: 8,
        },
        Handle {
            slot: 2,
            generation: u32::MAX,
        },
    ] {
        assert_eq!(loaded.lookup(h), None);
    }
    let mut reordered = input.clone();
    reordered.free_head = Some(1);
    reordered.slots[1] = ArenaSlot::Free { next: Some(3) };
    reordered.slots[3] = ArenaSlot::Free { next: None };
    assert_ne!(
        bytes,
        encode_arena(
            &validate_arena(reordered, &mut budget()).unwrap(),
            &mut budget()
        )
        .unwrap()
    );
}

#[test]
fn all_free_all_live_and_last_supported_generation_are_exact() {
    for full in [false, true] {
        let record = ArenaRecord {
            identity: identity(),
            generation: u32::MAX - 1,
            live: u32::from(full),
            free_head: (!full).then_some(0),
            slots: vec![if full {
                ArenaSlot::Occupied {
                    generation: u32::MAX - 1,
                    row: 0,
                }
            } else {
                ArenaSlot::Free { next: None }
            }],
        };
        let bytes = encode_arena(
            &validate_arena(record.clone(), &mut budget()).unwrap(),
            &mut budget(),
        )
        .unwrap();
        assert_eq!(
            decode_arena(&bytes, identity(), &mut budget())
                .unwrap()
                .record(),
            &record
        );
    }
    let mut r = fixture();
    r.slots.clear();
    r.live = 0;
    r.free_head = None;
    rejected(r, Error::Length);
    let mut r = fixture();
    r.generation = u32::MAX;
    rejected(r, Error::Generation);
    let mut r = fixture();
    r.slots[0] = ArenaSlot::Occupied {
        generation: u32::MAX,
        row: 1,
    };
    rejected(r, Error::Generation);
    let mut r = fixture();
    r.slots[0] = ArenaSlot::Occupied {
        generation: 9,
        row: 1,
    };
    rejected(r, Error::Generation);
}

#[test]
fn free_chain_cycles_orphans_bounds_and_live_aliases_refuse() {
    let mut r = fixture();
    r.slots[1] = ArenaSlot::Free { next: Some(3) };
    rejected(r, Error::Free);
    let mut r = fixture();
    r.free_head = None;
    rejected(r, Error::Free);
    let mut r = fixture();
    r.free_head = Some(1);
    rejected(r, Error::Free);
    let mut r = fixture();
    r.free_head = Some(4);
    rejected(r, Error::Free);
    let mut r = fixture();
    r.slots[1] = ArenaSlot::Free { next: Some(0) };
    rejected(r, Error::Ownership);
    let mut r = fixture();
    r.slots[1] = ArenaSlot::Free {
        next: Some(u32::MAX),
    };
    rejected(r, Error::Free);
}

#[test]
fn stored_counts_dense_row_duplicates_and_missing_rows_refuse() {
    let mut r = fixture();
    r.live = 1;
    rejected(r, Error::Ownership);
    let mut r = fixture();
    r.live = 3;
    rejected(r, Error::Ownership);
    let mut r = fixture();
    r.live = 5;
    rejected(r, Error::Length);
    let mut r = fixture();
    r.slots[0] = ArenaSlot::Occupied {
        generation: 2,
        row: 0,
    };
    rejected(r, Error::Ownership);
    let mut r = fixture();
    r.slots[0] = ArenaSlot::Occupied {
        generation: 2,
        row: 2,
    };
    rejected(r, Error::Ownership);
}

#[test]
fn arena_headers_lengths_and_slot_tags_refuse_before_any_live_object() {
    let bytes = encode_arena(
        &validate_arena(fixture(), &mut budget()).unwrap(),
        &mut budget(),
    )
    .unwrap();
    for n in 0..bytes.len() {
        assert!(decode_arena(&bytes[..n], identity(), &mut budget()).is_err());
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode_arena(&extra, identity(), &mut budget()).is_err());
    for offset in [0, 8, 10, 11, 12, 60] {
        let mut bad = bytes.clone();
        bad[offset] ^= 0x80;
        let mut zero = Budget::new(0).unwrap();
        assert!(decode_arena(&bad, identity(), &mut zero).is_err());
        assert_eq!(zero.remaining(), 0);
    }
    for offset in [44, 48, 52, 56] {
        let mut bad = bytes.clone();
        write(&mut bad, offset, if offset == 56 { 4 } else { u32::MAX });
        let mut zero = Budget::new(0).unwrap();
        assert!(decode_arena(&bad, identity(), &mut zero).is_err());
        assert_eq!(zero.remaining(), 0);
    }
    let mut bad = bytes.clone();
    write(&mut bad, 64, 2);
    assert_eq!(
        decode_arena(&bad, identity(), &mut budget()).err(),
        Some(Error::Node)
    );
    let mut bad = bytes.clone();
    write(&mut bad, 64 + 12 + 8, 1);
    assert_eq!(
        decode_arena(&bad, identity(), &mut budget()).err(),
        Some(Error::Node)
    );
    let mut other = identity();
    other.dimension = Dimension::Planar;
    assert_eq!(
        decode_arena(&bytes, other, &mut budget()).err(),
        Some(Error::Identity)
    );
}

#[test]
fn coarena_preserves_and_classifies_current_stale_and_invalidated_rows() {
    let arena = validate_arena(fixture(), &mut budget()).unwrap();
    let input = coarena();
    let checked = validate_coarena(input.clone(), &arena, &mut budget()).unwrap();
    let bytes = encode_coarena(&checked, &mut budget()).unwrap();
    let loaded = decode_coarena(&bytes, &arena, &mut budget()).unwrap();
    assert_eq!(loaded.record(), &input);
    assert_eq!(encode_coarena(&loaded, &mut budget()).unwrap(), bytes);
    assert_eq!(loaded.state(0), Some(RowState::Current));
    assert_eq!(loaded.state(1), Some(RowState::Stale));
    assert_eq!(loaded.state(2), Some(RowState::Stale));
    assert_eq!(loaded.state(3), Some(RowState::Invalidated));
    assert_eq!(loaded.state(4), None);
    for i in 0..4 {
        assert_eq!(loaded.unknown_generation_row(i), Some(3 - i));
    }
    assert_eq!(
        loaded.lookup(Handle {
            slot: 2,
            generation: 7
        }),
        Some(1)
    );
    assert_eq!(
        arena.lookup(Handle {
            slot: 2,
            generation: 7
        }),
        None
    );
    assert_eq!(
        loaded.lookup(Handle {
            slot: 2,
            generation: 8
        }),
        None
    );
    assert_eq!(
        loaded.lookup(Handle {
            slot: 3,
            generation: u32::MAX
        }),
        None
    );
    let empty = validate_coarena(
        CoarenaRecord {
            identity: identity(),
            rows: vec![],
        },
        &arena,
        &mut budget(),
    )
    .unwrap();
    assert!(
        decode_coarena(
            &encode_coarena(&empty, &mut budget()).unwrap(),
            &arena,
            &mut budget()
        )
        .unwrap()
        .record()
        .rows
        .is_empty()
    );
}

#[test]
fn coarena_identity_future_generation_row_alias_and_wrong_witness_refuse() {
    let arena = validate_arena(fixture(), &mut budget()).unwrap();
    let mut r = coarena();
    r.identity.build[0] ^= 1;
    assert_eq!(
        validate_coarena(r, &arena, &mut budget()).err(),
        Some(Error::Identity)
    );
    let mut r = coarena();
    r.rows[1].generation = 9;
    assert_eq!(
        validate_coarena(r, &arena, &mut budget()).err(),
        Some(Error::Generation)
    );
    let mut r = coarena();
    r.rows[0].generation = 3;
    assert_eq!(
        validate_coarena(r, &arena, &mut budget()).err(),
        Some(Error::Generation)
    );
    let mut r = coarena();
    r.rows[0].row = 2;
    assert_eq!(
        validate_coarena(r, &arena, &mut budget()).err(),
        Some(Error::Ownership)
    );
    let mut r = coarena();
    r.rows[0].row = 4;
    assert_eq!(
        validate_coarena(r, &arena, &mut budget()).err(),
        Some(Error::Ownership)
    );
    let mut r = coarena();
    r.rows.push(CoarenaRow {
        generation: u32::MAX,
        row: 4,
    });
    assert_eq!(
        validate_coarena(r, &arena, &mut budget()).err(),
        Some(Error::Length)
    );
    let bytes = encode_coarena(
        &validate_coarena(coarena(), &arena, &mut budget()).unwrap(),
        &mut budget(),
    )
    .unwrap();
    let mut other = fixture();
    other.generation = 2;
    other.slots[2] = ArenaSlot::Occupied {
        generation: 2,
        row: 0,
    };
    let other = validate_arena(other, &mut budget()).unwrap();
    assert_eq!(
        decode_coarena(&bytes, &other, &mut budget()).err(),
        Some(Error::Generation)
    );
    let mut other = fixture();
    other.identity.build[3] ^= 1;
    let other = validate_arena(other, &mut budget()).unwrap();
    assert_eq!(
        decode_coarena(&bytes, &other, &mut budget()).err(),
        Some(Error::Identity)
    );
}

#[test]
fn coarena_headers_truncation_padding_and_lengths_refuse() {
    let arena = validate_arena(fixture(), &mut budget()).unwrap();
    let bytes = encode_coarena(
        &validate_coarena(coarena(), &arena, &mut budget()).unwrap(),
        &mut budget(),
    )
    .unwrap();
    for n in 0..bytes.len() {
        assert!(decode_coarena(&bytes[..n], &arena, &mut budget()).is_err());
    }
    for offset in [0, 8, 10, 11, 12, 48] {
        let mut bad = bytes.clone();
        bad[offset] ^= 0x80;
        assert!(decode_coarena(&bad, &arena, &mut Budget::new(0).unwrap()).is_err());
    }
    let mut bad = bytes.clone();
    write(&mut bad, 44, u32::MAX);
    assert_eq!(
        decode_coarena(&bad, &arena, &mut Budget::new(0).unwrap()).err(),
        Some(Error::Length)
    );
    let mut bad = bytes.clone();
    bad.push(0);
    assert_eq!(
        decode_coarena(&bad, &arena, &mut budget()).err(),
        Some(Error::Length)
    );
}

#[test]
fn all_sections_share_a_cumulative_requested_allocation_budget() {
    let arena = validate_arena(fixture(), &mut budget()).unwrap();
    let a = encode_arena(&arena, &mut budget()).unwrap();
    let c = encode_coarena(
        &validate_coarena(coarena(), &arena, &mut budget()).unwrap(),
        &mut budget(),
    )
    .unwrap();
    let charge = 4 * size_of::<ArenaSlot>() + 4 + 2 + 4 * size_of::<CoarenaRow>() + 4;
    let mut exact = Budget::new(charge).unwrap();
    let decoded = decode_arena(&a, identity(), &mut exact).unwrap();
    let _co = decode_coarena(&c, &decoded, &mut exact).unwrap();
    assert_eq!(exact.remaining(), 0);
    assert!(encode_arena(&decoded, &mut exact).is_err());
    let mut short = Budget::new(charge - 1).unwrap();
    let decoded = decode_arena(&a, identity(), &mut short).unwrap();
    assert_eq!(
        decode_coarena(&c, &decoded, &mut short).err(),
        Some(Error::Limit)
    );
    assert_eq!(
        decode_arena(&a, identity(), &mut Budget::new(0).unwrap()).err(),
        Some(Error::Limit)
    );
}

#[test]
fn arbitrary_input_and_single_byte_mutations_are_bounded_and_exact_when_admitted() {
    let arena = validate_arena(fixture(), &mut budget()).unwrap();
    let a = encode_arena(&arena, &mut budget()).unwrap();
    let c = encode_coarena(
        &validate_coarena(coarena(), &arena, &mut budget()).unwrap(),
        &mut budget(),
    )
    .unwrap();
    for index in 0..a.len() {
        let mut bad = a.clone();
        bad[index] ^= 0x80;
        if let Ok(value) = decode_arena(&bad, identity(), &mut budget()) {
            assert_eq!(encode_arena(&value, &mut budget()).unwrap(), bad);
        }
    }
    for index in 0..c.len() {
        let mut bad = c.clone();
        bad[index] ^= 0x80;
        if let Ok(value) = decode_coarena(&bad, &arena, &mut budget()) {
            assert_eq!(encode_coarena(&value, &mut budget()).unwrap(), bad);
        }
    }
    let mut state = 17u32;
    for len in 0..1024 {
        let bytes: Vec<_> = (0..len)
            .map(|_| {
                state = state.wrapping_mul(1664525).wrapping_add(1013904223);
                (state >> 24) as u8
            })
            .collect();
        assert!(decode_arena(&bytes, identity(), &mut budget()).is_err());
        assert!(decode_coarena(&bytes, &arena, &mut budget()).is_err());
    }
}

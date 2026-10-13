use incant_physics_snapshot::*;

fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn identity(dimension: Dimension) -> Identity {
    Identity {
        dimension,
        build: [81; 32],
    }
}
fn record(d: Dimension) -> ContactPointsRecord {
    let v = [
        -0.0f32,
        -2.25,
        if d == Dimension::Planar { 0. } else { 3.5 },
    ]
    .map(f32::to_bits);
    let mut data = ContactWarmstart {
        impulse: 17f32.to_bits(),
        tangent_impulse: [(-2f32).to_bits(), 0],
        warmstart_impulse: 13f32.to_bits(),
        warmstart_tangent_impulse: [0x8000_0000, 0],
        solver_dp1: v,
        solver_dp2: v,
        ..Default::default()
    };
    if d == Dimension::Spatial {
        data.tangent_impulse[1] = 0x8000_0001;
        data.warmstart_tangent_impulse[1] = f32::MAX.to_bits();
        data.warmstart_twist_impulse = (-7f32).to_bits();
        data.warmstart_tangent_world = v;
    }
    let point = TrackedContactPoint {
        local_p1: v,
        local_p2: v,
        distance: (-0.01f32).to_bits(),
        features: [0x4000_0003, 0xc000_0001],
        data,
    };
    let solver = SolverContactPoint {
        anchor1: v,
        anchor2: v,
        distance: point.distance,
        tangent_velocity: v,
        contact_id: NEW_CONTACT_BIT | 1,
        padding: 0,
    };
    ContactPointsRecord {
        identity: identity(d),
        points: vec![
            point,
            TrackedContactPoint {
                features: [0, 0],
                ..point
            },
        ],
        solver: vec![
            solver,
            SolverContactPoint {
                contact_id: 0,
                ..solver
            },
        ],
    }
}
fn encode(r: ContactPointsRecord) -> Vec<u8> {
    encode_contact_points(
        &validate_contact_points(r, &mut budget()).unwrap(),
        &mut budget(),
    )
    .unwrap()
}
fn put(bytes: &mut [u8], offset: usize, value: u32) {
    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
}

#[test]
fn exact_fields_order_flags_and_checked_lookup() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        let r = record(d);
        let bytes = encode(r.clone());
        assert_eq!(bytes.len(), 56 + 2 * 100 + 2 * 48);
        let value = decode_contact_points(&bytes, r.identity, &mut budget()).unwrap();
        assert_eq!(value.record(), &r);
        assert_eq!(value.solver_point(0), Some(&r.points[1]));
        assert_eq!(value.solver_point(1), Some(&r.points[0]));
        assert_eq!(value.solver_point(2), None);
        assert_eq!(value.solver_point(usize::MAX), None);
        assert!(value.record().solver[0].is_new());
        assert!(!value.record().solver[1].is_new());
        assert_eq!(encode_contact_points(&value, &mut budget()).unwrap(), bytes);
    }
}
#[test]
fn empty_unselected_and_repeated_solver_rows_are_preserved() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        let mut r = record(d);
        r.solver[1] = r.solver[0];
        let value = validate_contact_points(r.clone(), &mut budget()).unwrap();
        assert_eq!(value.solver_point(0), value.solver_point(1));
        r.solver.clear();
        assert_eq!(
            decode_contact_points(&encode(r.clone()), r.identity, &mut budget())
                .unwrap()
                .record(),
            &r
        );
        r.points.clear();
        assert_eq!(encode(r.clone()).len(), 56);
        r.solver.push(SolverContactPoint::default());
        assert!(matches!(
            validate_contact_points(r, &mut budget()),
            Err(Error::Contact)
        ));
    }
}
#[test]
fn every_numeric_word_refuses_nonfinite_before_exposure() {
    let r = record(Dimension::Spatial);
    let original = encode(r.clone());
    // First point: all words except the two packed feature IDs are floats.
    // First solver row: all words except contact ID and padding are floats.
    for offset in (0..25)
        .filter(|w| ![7, 8].contains(w))
        .map(|w| 56 + w * 4)
        .chain((0..10).map(|w| 256 + w * 4))
    {
        for invalid in [
            f32::INFINITY.to_bits(),
            f32::NEG_INFINITY.to_bits(),
            0x7fc0_0123,
        ] {
            let mut bytes = original.clone();
            put(&mut bytes, offset, invalid);
            assert!(
                matches!(
                    decode_contact_points(&bytes, r.identity, &mut budget()),
                    Err(Error::Contact)
                ),
                "offset {offset}"
            );
        }
    }
    // All finite values, including negative impulses, max finite and subnormal,
    // are preserved here. Physical consistency is a future composition check.
    for bits in [
        0x8000_0000,
        1,
        0x8000_0001,
        f32::MAX.to_bits(),
        (-f32::MAX).to_bits(),
    ] {
        let mut bytes = original.clone();
        put(&mut bytes, 56 + 9 * 4, bits);
        assert_eq!(
            encode_contact_points(
                &decode_contact_points(&bytes, r.identity, &mut budget()).unwrap(),
                &mut budget()
            )
            .unwrap(),
            bytes
        );
    }
}
#[test]
fn inactive_dimension_fields_and_native_padding_are_canonical() {
    let r = record(Dimension::Planar);
    let original = encode(r.clone());
    for word in [2, 5, 11, 14, 15, 16, 17, 18, 21, 24] {
        for invalid in [0x8000_0000, 1, 1f32.to_bits()] {
            let mut bytes = original.clone();
            put(&mut bytes, 56 + word * 4, invalid);
            assert!(
                matches!(
                    decode_contact_points(&bytes, r.identity, &mut budget()),
                    Err(Error::Contact)
                ),
                "point word {word}"
            );
        }
    }
    for word in [2, 5, 9, 11] {
        let mut bytes = original.clone();
        put(&mut bytes, 256 + word * 4, 0x8000_0000);
        assert!(matches!(
            decode_contact_points(&bytes, r.identity, &mut budget()),
            Err(Error::Contact)
        ));
    }
    let r = record(Dimension::Spatial);
    let mut bytes = encode(r.clone());
    put(&mut bytes, 256 + 11 * 4, 0x8000_0000);
    assert!(matches!(
        decode_contact_points(&bytes, r.identity, &mut budget()),
        Err(Error::Contact)
    ));
}
#[test]
fn features_preserve_constructor_encodings_without_shape_claims() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        for feature in [0, 0x4000_0000, 0x7fff_ffff, 0xc000_0000, 0xffff_ffff] {
            let mut r = record(d);
            r.points[0].features = [feature; 2];
            assert_eq!(
                decode_contact_points(&encode(r.clone()), r.identity, &mut budget())
                    .unwrap()
                    .record(),
                &r
            );
        }
        for feature in [1, 0x3fff_ffff] {
            let mut r = record(d);
            r.points[0].features[0] = feature;
            assert!(matches!(
                validate_contact_points(r, &mut budget()),
                Err(Error::Contact)
            ));
        }
        let mut r = record(d);
        r.points[0].features = [0x8000_0000, 0xbfff_ffff];
        assert_eq!(
            validate_contact_points(r, &mut budget()).is_ok(),
            d == Dimension::Spatial
        );
    }
}
#[test]
fn solver_indices_and_native_dimension_capacities_are_checked() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        for id in [2, NEW_CONTACT_BIT | 2, 0x7fff_ffff, u32::MAX] {
            let mut r = record(d);
            r.solver[0].contact_id = id;
            assert!(matches!(
                validate_contact_points(r, &mut budget()),
                Err(Error::Contact)
            ));
        }
        let mut r = record(d);
        r.points.push(r.points[0]);
        assert_eq!(
            validate_contact_points(r, &mut budget()).is_ok(),
            d == Dimension::Spatial
        );
        let mut r = record(d);
        r.solver.push(r.solver[0]);
        assert_eq!(
            validate_contact_points(r, &mut budget()).is_ok(),
            d == Dimension::Spatial
        );
    }
    let mut r = record(Dimension::Spatial);
    r.points.resize(MAX_ITEMS + 1, r.points[0]);
    assert!(matches!(
        validate_contact_points(r, &mut budget()),
        Err(Error::Limit)
    ));
    let mut r = record(Dimension::Spatial);
    r.points.resize(MAX_ITEMS, r.points[0]);
    r.solver.resize(MAX_ITEMS, r.solver[0]);
    assert!(
        matches!(validate_contact_points(r, &mut budget()), Err(Error::Limit)),
        "individually bounded tables exceed the aggregate wire limit"
    );
}
#[test]
fn header_identity_counts_and_all_truncations_precede_allocation() {
    let r = record(Dimension::Spatial);
    let bytes = encode(r.clone());
    for end in 0..bytes.len() {
        let mut b = budget();
        assert!(decode_contact_points(&bytes[..end], r.identity, &mut b).is_err());
        assert_eq!(b.remaining(), MAX_ALLOCATION);
    }
    for (offset, value) in [
        (0, 0),
        (8, 2),
        (12, 99),
        (44, u32::MAX),
        (48, u32::MAX),
        (52, 1),
    ] {
        let mut bad = bytes.clone();
        put(&mut bad, offset, value);
        let mut b = budget();
        assert!(decode_contact_points(&bad, r.identity, &mut b).is_err());
        assert_eq!(b.remaining(), MAX_ALLOCATION);
    }
    let mut bad = bytes.clone();
    bad.push(0);
    let mut b = budget();
    assert!(decode_contact_points(&bad, r.identity, &mut b).is_err());
    assert_eq!(b.remaining(), MAX_ALLOCATION);
    for expected in [
        Identity {
            build: [82; 32],
            ..r.identity
        },
        identity(Dimension::Planar),
    ] {
        assert!(matches!(
            decode_contact_points(&bytes, expected, &mut budget()),
            Err(Error::Identity)
        ));
    }
}
#[test]
fn exact_cumulative_allocation_budget_and_large_ordered_table() {
    let mut r = record(Dimension::Spatial);
    r.points.resize(4096, r.points[0]);
    r.solver = (0..4096)
        .rev()
        .map(|i| SolverContactPoint {
            contact_id: i | NEW_CONTACT_BIT,
            ..r.solver[0]
        })
        .collect();
    let bytes = encode(r.clone());
    let payload = r.points.len() * size_of::<TrackedContactPoint>()
        + r.solver.len() * size_of::<SolverContactPoint>();
    assert!(matches!(
        decode_contact_points(&bytes, r.identity, &mut Budget::new(payload - 1).unwrap()),
        Err(Error::Limit)
    ));
    let mut b = Budget::new(payload + bytes.len()).unwrap();
    let value = decode_contact_points(&bytes, r.identity, &mut b).unwrap();
    assert_eq!(value.record(), &r);
    assert_eq!(b.remaining(), bytes.len());
    assert_eq!(encode_contact_points(&value, &mut b).unwrap(), bytes);
    assert_eq!(b.remaining(), 0);
    assert!(matches!(
        encode_contact_points(&value, &mut b),
        Err(Error::Limit)
    ));
    for (i, s) in r.solver.iter().enumerate() {
        assert_eq!(
            value.solver_point(i),
            r.points.get(s.point_index() as usize)
        );
    }
}
#[test]
fn hostile_byte_mutations_never_expose_noncanonical_success() {
    let r = record(Dimension::Spatial);
    let bytes = encode(r.clone());
    for index in 0..bytes.len() {
        for mask in [1, 0x80, 0xff] {
            let mut changed = bytes.clone();
            changed[index] ^= mask;
            if let Ok(value) = decode_contact_points(&changed, r.identity, &mut budget()) {
                assert_eq!(
                    encode_contact_points(&value, &mut budget()).unwrap(),
                    changed
                );
                for i in 0..value.record().solver.len() {
                    assert!(value.solver_point(i).is_some());
                }
            }
        }
    }
}

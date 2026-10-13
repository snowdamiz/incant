use incant_physics_snapshot::*;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn part(shape: Primitive) -> Part {
    Part {
        translation: [0; 3],
        rotation: [0, 0, 0, 1f32.to_bits()],
        shape,
    }
}
fn record(dimension: Dimension, kind: GeometryKind, parts: Vec<Part>) -> GeometryRecord {
    GeometryRecord {
        identity: Identity {
            dimension,
            build: [31; 32],
        },
        kind,
        parts,
    }
}
fn roundtrip(input: GeometryRecord) {
    let identity = input.identity;
    let valid = validate_geometry(input.clone()).unwrap();
    let data = encode_geometry(&valid, &mut budget()).unwrap();
    let loaded = decode_geometry(&data, identity, &mut budget()).unwrap();
    assert_eq!(loaded.record(), &input);
    assert_eq!(encode_geometry(&loaded, &mut budget()).unwrap(), data);
}
#[test]
fn only_native_primitive_and_compound_shapes_roundtrip_exactly() {
    for dimension in [Dimension::Planar, Dimension::Spatial] {
        let extents = if dimension == Dimension::Planar {
            [1f32.to_bits(), 2f32.to_bits(), 0]
        } else {
            [1f32.to_bits(), 2f32.to_bits(), 3f32.to_bits()]
        };
        for shape in [
            Primitive::Box {
                half_extents: extents,
            },
            Primitive::Ball {
                radius: 0.5f32.to_bits(),
            },
            Primitive::CapsuleY {
                half_height: (-0.0f32).to_bits(),
                radius: 0.25f32.to_bits(),
            },
        ] {
            roundtrip(record(
                dimension,
                GeometryKind::Primitive,
                vec![part(shape)],
            ));
        }
    }
    let mut parts = vec![
        part(Primitive::Ball {
            radius: 1f32.to_bits(),
        }),
        part(Primitive::CapsuleY {
            half_height: 2f32.to_bits(),
            radius: 0.5f32.to_bits(),
        }),
    ];
    parts[0].translation = [(-2f32).to_bits(), 0, 4f32.to_bits()];
    parts[0].rotation = [0, 0, 0, (-1f32).to_bits()];
    roundtrip(record(
        Dimension::Spatial,
        GeometryKind::SpatialCompound,
        parts,
    ));
    let mut tile = part(Primitive::Box {
        half_extents: [0.0000005f32.to_bits(), 500_000f32.to_bits(), 0],
    });
    tile.translation = [500_000f32.to_bits(), 500_000f32.to_bits(), 0];
    roundtrip(record(
        Dimension::Planar,
        GeometryKind::PlanarTiles,
        vec![tile],
    ));
}
#[test]
fn geometry_tags_reserved_words_and_truncation_refuse() {
    let r = record(
        Dimension::Spatial,
        GeometryKind::Primitive,
        vec![part(Primitive::Ball {
            radius: 1f32.to_bits(),
        })],
    );
    let bytes = encode_geometry(&validate_geometry(r.clone()).unwrap(), &mut budget()).unwrap();
    for len in 0..bytes.len() {
        assert!(decode_geometry(&bytes[..len], r.identity, &mut budget()).is_err());
    }
    for offset in [0, 8, 10, 11, 12, 44, 48, 52, 56, 64, 68] {
        let mut bad = bytes.clone();
        bad[offset] = 0xff;
        assert!(
            decode_geometry(&bad, r.identity, &mut budget()).is_err(),
            "offset{offset}"
        );
    }
    let mut extra = bytes;
    extra.push(0);
    assert!(decode_geometry(&extra, r.identity, &mut budget()).is_err());
}
#[test]
fn unsupported_dimensions_counts_poses_and_nested_compounds_are_closed() {
    let p = part(Primitive::Ball {
        radius: 1f32.to_bits(),
    });
    for r in [
        record(Dimension::Planar, GeometryKind::SpatialCompound, vec![p]),
        record(Dimension::Spatial, GeometryKind::PlanarTiles, vec![p]),
        record(Dimension::Spatial, GeometryKind::Primitive, vec![]),
        record(Dimension::Spatial, GeometryKind::Primitive, vec![p, p]),
        record(
            Dimension::Spatial,
            GeometryKind::SpatialCompound,
            vec![p; 65],
        ),
        record(Dimension::Planar, GeometryKind::PlanarTiles, vec![p]),
    ] {
        assert_eq!(validate_geometry(r).err(), Some(Error::Geometry));
    }
    // There is no nested/custom shape variant in the inert API or wire registry.
    for shape in [
        Primitive::Ball {
            radius: f32::NAN.to_bits(),
        },
        Primitive::Ball { radius: 0 },
        Primitive::CapsuleY {
            half_height: (-1f32).to_bits(),
            radius: 1f32.to_bits(),
        },
        Primitive::Box {
            half_extents: [f32::INFINITY.to_bits(); 3],
        },
    ] {
        assert!(
            validate_geometry(record(
                Dimension::Spatial,
                GeometryKind::Primitive,
                vec![part(shape)]
            ))
            .is_err()
        );
    }
    let mut translated = p;
    translated.translation[0] = 1f32.to_bits();
    assert!(
        validate_geometry(record(
            Dimension::Spatial,
            GeometryKind::Primitive,
            vec![translated]
        ))
        .is_err()
    );
    let mut nonunit = p;
    nonunit.rotation = [0; 4];
    assert!(
        validate_geometry(record(
            Dimension::Spatial,
            GeometryKind::SpatialCompound,
            vec![nonunit]
        ))
        .is_err()
    );
    let mut nonfinite = p;
    nonfinite.rotation[0] = f32::NAN.to_bits();
    assert!(
        validate_geometry(record(
            Dimension::Spatial,
            GeometryKind::SpatialCompound,
            vec![nonfinite]
        ))
        .is_err()
    );
}
#[test]
fn tile_and_primitive_bounds_have_distinct_admission_and_shared_budget() {
    let shape = Primitive::Box {
        half_extents: [0.0000005f32.to_bits(), 1f32.to_bits(), 0],
    };
    assert!(
        validate_geometry(record(
            Dimension::Planar,
            GeometryKind::Primitive,
            vec![part(shape)]
        ))
        .is_err()
    );
    let r = record(
        Dimension::Planar,
        GeometryKind::PlanarTiles,
        vec![part(shape)],
    );
    let identity = r.identity;
    let bytes = encode_geometry(&validate_geometry(r).unwrap(), &mut budget()).unwrap();
    assert_eq!(
        decode_geometry(
            &bytes,
            identity,
            &mut Budget::new(size_of::<Part>() - 1).unwrap()
        )
        .err(),
        Some(Error::Limit)
    );
    let mut exact = Budget::new(size_of::<Part>()).unwrap();
    decode_geometry(&bytes, identity, &mut exact).unwrap();
    assert_eq!(exact.remaining(), 0);
    assert_eq!(
        decode_geometry(&bytes, identity, &mut exact).err(),
        Some(Error::Limit)
    );
}

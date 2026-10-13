use incant_physics_snapshot::*;

fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn fixture(dimension: Dimension) -> (GeometryRecord, CompoundRecord) {
    let id = Identity {
        dimension,
        build: [29; 32],
    };
    let active = |x: f32, y: f32, z: f32| {
        [
            x.to_bits(),
            y.to_bits(),
            if dimension == Dimension::Spatial {
                z.to_bits()
            } else {
                0
            },
        ]
    };
    let aabbs = vec![
        CompoundAabb {
            mins: active(0., 0., -1.),
            maxs: active(2., 2., 1.),
        },
        CompoundAabb {
            mins: active(2., 0., -1.),
            maxs: active(4., 2., 1.),
        },
    ];
    let root_aabb = CompoundAabb {
        mins: aabbs[0].mins,
        maxs: aabbs[1].maxs,
    };
    let node = |i: usize| Node {
        mins: aabbs[i].mins,
        maxs: aabbs[i].maxs,
        children: i as u32,
        data: 1,
    };
    let tree = BvhRecord {
        identity: id,
        nodes: vec![Wide {
            left: node(0),
            right: node(1),
        }],
        parents: vec![0],
        leaves: LeafMap {
            occupied: 2,
            slots: vec![Some(0), Some(1)],
        },
        free: vec![],
        rebuild_frame: 7,
        rebuild_start: 1,
    };
    let geometry = GeometryRecord {
        identity: id,
        kind: if dimension == Dimension::Spatial {
            GeometryKind::SpatialCompound
        } else {
            GeometryKind::PlanarTiles
        },
        parts: [1., 3.]
            .map(|x| Part {
                translation: active(x, 1., 0.),
                rotation: [0, 0, 0, 1f32.to_bits()],
                shape: Primitive::Box {
                    half_extents: active(1., 1., 1.),
                },
            })
            .to_vec(),
    };
    (
        geometry,
        CompoundRecord {
            identity: id,
            child_aabbs: aabbs,
            root_aabb,
            tree,
            flags: 0,
            pseudo_normals: false,
        },
    )
}

#[test]
fn attached_storage_roundtrips_exactly_and_declarations_keep_v1() {
    for dimension in [Dimension::Planar, Dimension::Spatial] {
        let (g, c) = fixture(dimension);
        let plain = validate_geometry(g.clone()).unwrap();
        let v1 = encode_geometry(&plain, &mut budget()).unwrap();
        assert_eq!(&v1[8..10], &1u16.to_le_bytes());
        assert!(
            decode_geometry(&v1, g.identity, &mut budget())
                .unwrap()
                .compound()
                .is_none()
        );
        let joined = attach_compound(plain, c.clone(), &mut budget()).unwrap();
        let bytes = encode_geometry(&joined, &mut budget()).unwrap();
        assert_eq!(&bytes[8..10], &2u16.to_le_bytes());
        let loaded = decode_geometry(&bytes, g.identity, &mut budget()).unwrap();
        assert_eq!(loaded.record(), &g);
        let compound = loaded.compound().unwrap();
        assert_eq!(compound.identity(), g.identity);
        assert_eq!(compound.geometry().record(), &g);
        assert_eq!(compound.tree().record(), &c.tree);
        assert_eq!(compound.child_aabbs(), c.child_aabbs);
        assert_eq!(compound.root_aabb(), c.root_aabb);
        assert_eq!(encode_geometry(&loaded, &mut budget()).unwrap(), bytes);
    }
}

#[test]
fn storage_must_join_every_child_once_with_exact_leaf_bounds() {
    for case in 0..10 {
        let (g, mut c) = fixture(Dimension::Spatial);
        match case {
            0 => c.identity.build[0] ^= 1,
            1 => c.tree.identity.build[0] ^= 1,
            2 => c.flags = 1,
            3 => c.pseudo_normals = true,
            4 => {
                c.child_aabbs.pop();
            }
            5 => {
                c.tree.leaves.slots.push(None);
            }
            6 => c.child_aabbs.swap(0, 1),
            7 => c.root_aabb.maxs[0] = 1f32.to_bits(),
            8 => c.child_aabbs[0].mins[0] = f32::NAN.to_bits(),
            9 => c.tree.nodes[0].left.children = 1,
            _ => unreachable!(),
        }
        assert!(
            attach_compound(validate_geometry(g).unwrap(), c, &mut budget()).is_err(),
            "case {case}"
        );
    }
    let (g, c) = fixture(Dimension::Planar);
    let joined = attach_compound(validate_geometry(g).unwrap(), c.clone(), &mut budget()).unwrap();
    assert!(attach_compound(joined, c, &mut budget()).is_err());
}

#[test]
fn compound_extension_is_bounded_closed_and_transactionally_decoded() {
    let (g, c) = fixture(Dimension::Spatial);
    let joined = attach_compound(validate_geometry(g.clone()).unwrap(), c, &mut budget()).unwrap();
    let bytes = encode_geometry(&joined, &mut budget()).unwrap();
    for len in 0..bytes.len() {
        assert!(
            decode_geometry(&bytes[..len], g.identity, &mut budget()).is_err(),
            "len {len}"
        );
    }
    // Extension flags, boolean, child count and tree length precede exact bounds.
    let extension = 56 + g.parts.len() * 44;
    for offset in [
        8,
        52,
        extension,
        extension + 4,
        extension + 8,
        extension + 12,
    ] {
        let mut bad = bytes.clone();
        bad[offset] = 0xff;
        assert!(
            decode_geometry(&bad, g.identity, &mut budget()).is_err(),
            "offset {offset}"
        );
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(decode_geometry(&extra, g.identity, &mut budget()).is_err());
    let mut shared = Budget::new(size_of::<Part>() * g.parts.len()).unwrap();
    assert!(matches!(
        decode_geometry(&bytes, g.identity, &mut shared),
        Err(Error::Limit)
    ));
    assert_eq!(shared.remaining(), 0);
    let mut generous = budget();
    decode_geometry(&bytes, g.identity, &mut generous).unwrap();
    let used = MAX_ALLOCATION - generous.remaining();
    let mut exact = Budget::new(used).unwrap();
    decode_geometry(&bytes, g.identity, &mut exact).unwrap();
    assert_eq!(exact.remaining(), 0);
    assert!(matches!(
        decode_geometry(&bytes, g.identity, &mut exact),
        Err(Error::Limit)
    ));
}

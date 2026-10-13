use incant_physics_snapshot::*;
#[path = "support/manifold_fixture.rs"]
mod fixture;
use fixture::*;
fn state(d: Dimension) -> PairStateTable {
    PairStateTable {
        identity: id(d),
        pairs: vec![PairState {
            event_status: 3,
            solver_color: 255,
            solver_color_bodies: [u32::MAX; 2],
            recycle: Some(RecycleState {
                relative_pose: pose(d),
                rotations: [pose(d).rotation; 2],
                max_extent: 1f32.to_bits(),
                max_drift: 0x8000_0000,
            }),
            workspace: WorkspaceKind::None,
            timestamp: false,
            detectors: RecordSpan { start: 0, count: 0 },
        }],
        detectors: vec![],
    }
}
fn check(r: PairStateTable, m: &ValidatedManifolds<'_>) -> Vec<u8> {
    let v = validate_pair_state(r.clone(), m, &mut budget()).unwrap();
    let b = encode_pair_state(&v, &mut budget()).unwrap();
    let decoded = decode_pair_state(&b, m, &mut budget()).unwrap();
    assert_eq!(decoded.record(), &r);
    assert_eq!(encode_pair_state(&decoded, &mut budget()).unwrap(), b);
    b
}
fn compound(f: impl FnOnce(&ValidatedManifolds<'_>, PairStateTable)) {
    with(Dimension::Spatial, |c, mut t| {
        let mut g = c.geometries[0].record().clone();
        g.kind = GeometryKind::SpatialCompound;
        g.parts.push(g.parts[0]);
        g.parts[1].translation = [2f32.to_bits(), 0, 0];
        let geometry = [validate_geometry(g).unwrap(), geometry(Dimension::Spatial)];
        t.pairs[0].plain.count = 2;
        t.pairs[0].clusters.start = 2;
        t.manifolds.push(t.manifolds[0]);
        t.points.push(t.points[0]);
        t.solver.push(t.solver[0]);
        for (i, m) in t.manifolds.iter_mut().enumerate() {
            m.points.start = i as u32;
            m.solver.start = i as u32;
            m.subshapes[0] = i as u32;
            m.subshape_poses = Some([
                Some(LocalPose {
                    translation: geometry[0].record().parts[i].translation,
                    rotation: geometry[0].record().parts[i].rotation,
                }),
                None,
            ]);
        }
        let m = validate_manifolds(
            t,
            ManifoldContext {
                geometries: &geometry,
                ..c
            },
            &mut budget(),
        )
        .unwrap();
        let mut r = state(Dimension::Spatial);
        r.pairs[0].workspace = WorkspaceKind::CompositeShape;
        r.pairs[0].timestamp = true;
        r.pairs[0].detectors.count = 2;
        r.detectors = vec![
            SubDetector {
                key: [1, 0],
                manifold: 1,
                timestamp: true,
            },
            SubDetector {
                key: [0, 0],
                manifold: 0,
                timestamp: true,
            },
        ];
        f(&m, r);
    });
}
#[test]
fn fixed_bits_optional_recycle_and_native_map_order_are_exact() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        with(d, |c, t| {
            let m = validate_manifolds(t, c, &mut budget()).unwrap();
            let mut r = state(d);
            r.pairs[0]
                .recycle
                .as_mut()
                .unwrap()
                .relative_pose
                .translation[0] = 0x8000_0000;
            for present in [true, false] {
                if !present {
                    r.pairs[0].recycle = None;
                }
                let b = check(r.clone(), &m);
                assert_eq!(b.len(), 56 + 104);
            }
        });
    }
    compound(|m, r| {
        check(r.clone(), m);
        let v = validate_pair_state(r.clone(), m, &mut budget()).unwrap();
        assert_eq!(v.detectors(0).unwrap(), r.detectors);
        assert_eq!(v.detector_manifold(0, 0), Some(&m.record().manifolds[1]));
        assert_eq!(v.detector_manifold(0, 1), Some(&m.record().manifolds[0]));
        assert!(v.detector_manifold(0, 2).is_none());
        assert!(v.detectors(u32::MAX).is_none());
        let mut reversed = r;
        reversed.detectors.reverse();
        assert_ne!(
            check(reversed, m),
            encode_pair_state(&v, &mut budget()).unwrap()
        );
    });
}
#[test]
fn every_detector_ref_timestamp_key_and_partition_is_checked() {
    compound(|m, r| {
        for change in 0..8 {
            let mut x = r.clone();
            match change {
                0 => x.detectors[0].manifold = 2,
                1 => x.detectors[0].manifold = 0,
                2 => x.detectors[0].key = [0, 0],
                3 => x.detectors[0].key[1] = 1,
                4 => x.detectors[0].timestamp = false,
                5 => x.pairs[0].detectors.start = 1,
                6 => x.pairs[0].detectors.count = 1,
                _ => x.detectors.push(x.detectors[0]),
            };
            assert!(
                validate_pair_state(x, m, &mut budget()).is_err(),
                "case {change}"
            );
        }
    });
}
#[test]
fn workspace_kind_and_shape_correspondence_never_downgrade() {
    compound(|m, r| {
        for kind in [WorkspaceKind::None, WorkspaceKind::CompositeComposite] {
            let mut x = r.clone();
            x.pairs[0].workspace = kind;
            assert!(validate_pair_state(x, m, &mut budget()).is_err());
        }
    });
    with(Dimension::Spatial, |c, t| {
        let m = validate_manifolds(t, c, &mut budget()).unwrap();
        for change in 0..3 {
            let mut r = state(Dimension::Spatial);
            match change {
                0 => r.pairs[0].workspace = WorkspaceKind::CompositeShape,
                1 => r.pairs[0].workspace = WorkspaceKind::CompositeComposite,
                _ => r.pairs[0].timestamp = true,
            };
            assert!(matches!(
                validate_pair_state(r, &m, &mut budget()),
                Err(Error::Workspace)
            ));
        }
    });
}
#[test]
fn metadata_flags_rotations_and_finite_nonnegative_recycle_bounds() {
    for dim in [Dimension::Planar, Dimension::Spatial] {
        with(dim, |c, t| {
            let m = validate_manifolds(t, c, &mut budget()).unwrap();
            for change in 0..9 {
                let mut r = state(dim);
                let p = &mut r.pairs[0];
                let s = p.recycle.as_mut().unwrap();
                match change {
                    0 => p.event_status = 4,
                    1 => p.solver_color = 129,
                    2 => p.solver_color_bodies[0] = MAX_ITEMS as u32,
                    3 => s.rotations[0] = [0; 4],
                    4 => s.relative_pose.translation[0] = f32::NAN.to_bits(),
                    5 => s.max_extent = f32::INFINITY.to_bits(),
                    6 => s.max_drift = (-1f32).to_bits(),
                    7 => s.rotations[1][0] = f32::NAN.to_bits(),
                    _ => s.relative_pose.rotation = [0; 4],
                };
                assert!(validate_pair_state(r, &m, &mut budget()).is_err());
            }
            let mut r = state(dim);
            let s = r.pairs[0].recycle.as_mut().unwrap();
            s.max_extent = f32::MAX.to_bits();
            s.max_drift = 1;
            check(r, &m);
        });
    }
}
#[test]
fn hostile_headers_unknown_variants_boolean_tags_and_cumulative_budget() {
    compound(|m, r| {
        let bytes = check(r.clone(), m);
        for end in 0..bytes.len() {
            let mut b = budget();
            assert!(decode_pair_state(&bytes[..end], m, &mut b).is_err());
            assert_eq!(b.remaining(), MAX_ALLOCATION);
        }
        for offset in [44, 48, 52] {
            let mut x = bytes.clone();
            x[offset..offset + 4].copy_from_slice(&u32::MAX.to_le_bytes());
            let mut b = budget();
            assert!(decode_pair_state(&x, m, &mut b).is_err());
            assert_eq!(b.remaining(), MAX_ALLOCATION);
        }
        // Workspace kind and both native boolean fields are closed, not lossy casts.
        for (offset, value) in [(144, 3), (144, u32::MAX), (148, 2), (172, 2), (72, 2)] {
            let mut x = bytes.clone();
            x[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            assert!(decode_pair_state(&x, m, &mut budget()).is_err());
        }
        let payload = size_of::<PairState>() + 2 * size_of::<SubDetector>();
        let scratch = m.record().manifolds.len() + 16;
        assert!(matches!(
            decode_pair_state(&bytes, m, &mut Budget::new(payload + scratch - 1).unwrap()),
            Err(Error::Limit)
        ));
        let mut b = Budget::new(payload + scratch + bytes.len()).unwrap();
        let v = decode_pair_state(&bytes, m, &mut b).unwrap();
        assert_eq!(encode_pair_state(&v, &mut b).unwrap(), bytes);
        assert_eq!(b.remaining(), 0);
        for i in 0..bytes.len() {
            for mask in [1, 0x80, 0xff] {
                let mut x = bytes.clone();
                x[i] ^= mask;
                if let Ok(v) = decode_pair_state(&x, m, &mut budget()) {
                    assert_eq!(encode_pair_state(&v, &mut budget()).unwrap(), x);
                }
            }
        }
    });
}
#[test]
fn inactive_optional_payload_and_independent_identity_are_not_reinterpreted() {
    with(Dimension::Spatial, |c, t| {
        let m = validate_manifolds(t, c, &mut budget()).unwrap();
        let mut r = state(Dimension::Spatial);
        r.pairs[0].recycle = None;
        let mut bytes = check(r.clone(), &m);
        bytes[76] = 1;
        assert!(matches!(
            decode_pair_state(&bytes, &m, &mut budget()),
            Err(Error::Header)
        ));
        r.identity.build[0] ^= 1;
        let mut b = budget();
        assert!(matches!(
            validate_pair_state(r, &m, &mut b),
            Err(Error::Identity)
        ));
        assert_eq!(b.remaining(), MAX_ALLOCATION);
    });
}

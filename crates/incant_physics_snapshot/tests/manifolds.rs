use incant_physics_snapshot::*;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn id(d: Dimension) -> Identity {
    Identity {
        dimension: d,
        build: [91; 32],
    }
}
fn h(slot: u32) -> Handle {
    Handle {
        slot,
        generation: 3,
    }
}
fn arena(d: Dimension) -> ValidatedArena {
    validate_arena(
        ArenaRecord {
            identity: id(d),
            generation: 3,
            free_head: None,
            live: 2,
            slots: vec![
                ArenaSlot::Occupied {
                    generation: 3,
                    row: 0,
                },
                ArenaSlot::Occupied {
                    generation: 3,
                    row: 1,
                },
            ],
        },
        &mut budget(),
    )
    .unwrap()
}
fn graph(d: Dimension, edges: &[[u32; 2]]) -> ValidatedGraph {
    let mut r = GraphRecord {
        identity: id(d),
        nodes: vec![
            GraphNode {
                head: [None; 2],
                row: 0,
            },
            GraphNode {
                head: [None; 2],
                row: 1,
            },
        ],
        edges: vec![],
    };
    for (i, n) in edges.iter().enumerate() {
        // Compute each side separately, including self loops.
        let next = [
            r.nodes[n[0] as usize].head[0],
            r.nodes[n[1] as usize].head[1],
        ];
        for side in 0..2 {
            if let Some(old) = next[side] {
                r.edges[old as usize].prev[side] = Some(i as u32);
            }
            r.nodes[n[side] as usize].head[side] = Some(i as u32);
        }
        r.edges.push(GraphEdge {
            node: *n,
            next,
            prev: [None; 2],
            row: i as u32,
        });
    }
    validate_graph(r, &mut budget()).unwrap()
}
fn pose(d: Dimension) -> LocalPose {
    LocalPose {
        translation: [0; 3],
        rotation: if d == Dimension::Planar {
            [1f32.to_bits(), 0, 0, 0]
        } else {
            [0, 0, 0, 1f32.to_bits()]
        },
    }
}
fn geometry(d: Dimension) -> ValidatedGeometry {
    validate_geometry(GeometryRecord {
        identity: id(d),
        kind: GeometryKind::Primitive,
        parts: vec![Part {
            translation: [0; 3],
            rotation: [0, 0, 0, 1f32.to_bits()],
            shape: Primitive::Ball {
                radius: 1f32.to_bits(),
            },
        }],
    })
    .unwrap()
}
fn table(d: Dimension) -> ManifoldTable {
    ManifoldTable {
        identity: id(d),
        nodes: vec![h(0), h(1)],
        pairs: vec![ContactPairManifolds {
            colliders: [h(0), h(1)],
            plain: RecordSpan { start: 0, count: 1 },
            clusters: RecordSpan { start: 1, count: 0 },
        }],
        manifolds: vec![ContactManifoldRecord {
            points: RecordSpan { start: 0, count: 1 },
            solver: RecordSpan { start: 0, count: 1 },
            local_normals: [[0, 1f32.to_bits(), 0], [0, (-1f32).to_bits(), 0]],
            subshapes: [0; 2],
            subshape_poses: None,
            bodies: [Some(h(0)), Some(h(1))],
            solver_flags: 1,
            solver_color: 0,
            solver_body_ids: [0, 1],
            graph_position: 0,
            normal: [0, 1f32.to_bits(), 0],
            relative_dominance: -256,
            user_data: u32::MAX,
            friction: 2f32.to_bits(),
            restitution: 3f32.to_bits(),
        }],
        points: vec![TrackedContactPoint::default()],
        solver: vec![SolverContactPoint {
            contact_id: NEW_CONTACT_BIT,
            ..Default::default()
        }],
    }
}
fn with(d: Dimension, f: impl FnOnce(ManifoldContext<'_>, ManifoldTable)) {
    let b = arena(d);
    let c = arena(d);
    let g = graph(d, &[[0, 1]]);
    let geometry = [geometry(d), geometry(d)];
    let a = validate_attachments(
        AttachmentRecord {
            identity: id(d),
            bodies: vec![
                AttachmentSpan { start: 0, count: 1 },
                AttachmentSpan { start: 1, count: 1 },
            ],
            colliders: vec![
                Some(ParentAttachment {
                    body: h(0),
                    local_pose: pose(d),
                }),
                Some(ParentAttachment {
                    body: h(1),
                    local_pose: pose(d),
                }),
            ],
            attached: vec![h(0), h(1)],
        },
        &b,
        &c,
        &mut budget(),
    )
    .unwrap();
    f(
        ManifoldContext {
            graph: &g,
            attachments: &a,
            geometries: &geometry,
            journals: PendingJournals::default(),
        },
        table(d),
    );
}
fn encoded(t: ManifoldTable, c: ManifoldContext<'_>) -> Vec<u8> {
    encode_manifolds(
        &validate_manifolds(t, c, &mut budget()).unwrap(),
        &mut budget(),
    )
    .unwrap()
}
#[test]
fn exact_metadata_float_bits_optional_allocation_and_checked_rows() {
    for d in [Dimension::Planar, Dimension::Spatial] {
        with(d, |c, mut t| {
            for poses in [None, Some([None; 2])] {
                t.manifolds[0].subshape_poses = poses;
                t.manifolds[0].normal[0] = 0x8000_0000;
                let bytes = encoded(t.clone(), c);
                assert_eq!(bytes.len(), 68 + 16 + 32 + 188 + 100 + 48);
                let v = decode_manifolds(&bytes, c, &mut budget()).unwrap();
                assert_eq!(v.record(), &t);
                assert_eq!(v.edge_pair(0), Some(&t.pairs[0]));
                assert_eq!(v.edge_pair(1), None);
                assert_eq!(v.manifold_points(0), Some(t.points.as_slice()));
                assert_eq!(v.manifold_solver(0), Some(t.solver.as_slice()));
                assert_eq!(v.manifold_points(u32::MAX), None);
                assert_eq!(encoded(v.record().clone(), c), bytes);
            }
            // Equal unrelated witnesses establish structure, not owner authentication.
            let another = graph(d, &[[0, 1]]);
            assert!(
                validate_manifolds(
                    t,
                    ManifoldContext {
                        graph: &another,
                        ..c
                    },
                    &mut budget()
                )
                .is_ok()
            );
        });
    }
}
#[test]
fn pending_journals_are_explicit_refusal_before_decoder_allocation() {
    with(Dimension::Spatial, |c, t| {
        let bytes = encoded(t.clone(), c);
        for journals in [
            PendingJournals {
                modified_bodies: 1,
                ..Default::default()
            },
            PendingJournals {
                modified_colliders: 1,
                ..Default::default()
            },
            PendingJournals {
                removed_colliders: 1,
                ..Default::default()
            },
        ] {
            let c = ManifoldContext { journals, ..c };
            let mut b = budget();
            assert!(matches!(
                decode_manifolds(&bytes, c, &mut b),
                Err(Error::PendingMutation)
            ));
            assert_eq!(b.remaining(), MAX_ALLOCATION);
            assert!(matches!(
                validate_manifolds(t.clone(), c, &mut b),
                Err(Error::PendingMutation)
            ));
        }
    });
}
#[test]
fn graph_generations_duplicate_nodes_pair_orientation_self_and_parallel_edges_refuse() {
    with(Dimension::Spatial, |c, t| {
        for change in 0..4 {
            let mut r = t.clone();
            match change {
                0 => r.nodes[1] = r.nodes[0],
                1 => r.nodes[0].generation -= 1,
                2 => r.pairs[0].colliders.swap(0, 1),
                _ => r.nodes[0].slot = u32::MAX,
            };
            assert!(validate_manifolds(r, c, &mut budget()).is_err());
        }
        let self_graph = graph(Dimension::Spatial, &[[0, 0]]);
        let mut r = t.clone();
        r.pairs[0].colliders = [h(0); 2];
        assert!(matches!(
            validate_manifolds(
                r,
                ManifoldContext {
                    graph: &self_graph,
                    ..c
                },
                &mut budget()
            ),
            Err(Error::Ownership)
        ));
        let parallel = graph(Dimension::Spatial, &[[0, 1], [1, 0]]);
        let mut r = t.clone();
        let mut second = r.pairs[0];
        second.colliders.swap(0, 1);
        r.pairs.push(second);
        assert!(matches!(
            validate_manifolds(
                r,
                ManifoldContext {
                    graph: &parallel,
                    ..c
                },
                &mut budget()
            ),
            Err(Error::Ownership)
        ));
        let mut reversed = c.graph.record().clone();
        reversed.nodes[0].row = 1;
        reversed.nodes[1].row = 0;
        let reversed = validate_graph(reversed, &mut budget()).unwrap();
        let mut r = t;
        r.nodes.swap(0, 1);
        assert!(
            validate_manifolds(
                r,
                ManifoldContext {
                    graph: &reversed,
                    ..c
                },
                &mut budget()
            )
            .is_ok()
        );
    });
}
#[test]
fn ordered_partitions_body_binding_and_local_point_references_are_complete() {
    with(Dimension::Spatial, |c, t| {
        for change in 0..8 {
            let mut r = t.clone();
            match change {
                0 => r.pairs[0].plain.start = 1,
                1 => r.pairs[0].clusters.start = 0,
                2 => r.manifolds[0].points.count = 0,
                3 => r.manifolds[0].solver.start = 1,
                4 => r.manifolds[0].bodies[0] = None,
                5 => r.manifolds[0].bodies[0] = Some(h(1)),
                6 => r.solver[0].contact_id = NEW_CONTACT_BIT | 1,
                _ => r.manifolds.push(r.manifolds[0]),
            };
            assert!(
                validate_manifolds(r, c, &mut budget()).is_err(),
                "case {change}"
            );
        }
    });
}
#[test]
fn normals_materials_and_solver_scalar_domains_preserve_limits_without_normalizing() {
    with(Dimension::Spatial, |c, t| {
        for change in 0..12 {
            let mut r = t.clone();
            let m = &mut r.manifolds[0];
            match change {
                0 => m.local_normals[0] = [0; 3],
                1 => m.normal = [0; 3],
                2 => m.local_normals[1][0] = f32::NAN.to_bits(),
                3 => m.friction = (-1f32).to_bits(),
                4 => m.restitution = f32::INFINITY.to_bits(),
                5 => m.relative_dominance = 257,
                6 => m.solver_flags = 2,
                7 => m.solver_color = 129,
                8 => m.solver_body_ids[0] = MAX_ITEMS as u32,
                9 => m.graph_position = 130 << 22,
                10 => m.graph_position = MAX_ITEMS as u32,
                _ => m.normal[1] = 2f32.to_bits(),
            };
            assert!(
                matches!(
                    validate_manifolds(r, c, &mut budget()),
                    Err(Error::Manifold)
                ),
                "case {change}"
            );
        }
        let mut r = t;
        r.points.clear();
        r.solver.clear();
        let m = &mut r.manifolds[0];
        m.points.count = 0;
        m.solver.count = 0;
        m.local_normals = [[0; 3]; 2];
        m.normal = [0; 3];
        m.friction = 0x8000_0000;
        m.solver_color = 255;
        m.solver_body_ids = [u32::MAX; 2];
        m.graph_position = u32::MAX;
        m.relative_dominance = 256;
        assert!(validate_manifolds(r, c, &mut budget()).is_ok());
    });
    with(Dimension::Planar, |c, mut t| {
        t.manifolds[0].normal[2] = 0x8000_0000;
        assert!(matches!(
            validate_manifolds(t, c, &mut budget()),
            Err(Error::Manifold)
        ));
    });
}
#[test]
fn plain_child_pose_and_cluster_collider_frame_are_distinct() {
    with(Dimension::Spatial, |c, mut t| {
        let mut record = c.geometries[0].record().clone();
        record.kind = GeometryKind::SpatialCompound;
        record.parts.push(record.parts[0]);
        record.parts[1].translation = [2f32.to_bits(), 0, 0];
        let child = LocalPose {
            translation: record.parts[1].translation,
            rotation: record.parts[1].rotation,
        };
        let geometries = [
            validate_geometry(record).unwrap(),
            geometry(Dimension::Spatial),
        ];
        let c = ManifoldContext {
            geometries: &geometries,
            ..c
        };
        t.manifolds[0].subshapes[0] = 1;
        t.manifolds[0].subshape_poses = Some([Some(child), None]);
        assert!(validate_manifolds(t.clone(), c, &mut budget()).is_ok());
        let mut wrong = t.clone();
        wrong.manifolds[0].subshape_poses = None;
        assert!(matches!(
            validate_manifolds(wrong.clone(), c, &mut budget()),
            Err(Error::Geometry)
        ));
        wrong.pairs[0].plain.count = 0;
        wrong.pairs[0].clusters = RecordSpan { start: 0, count: 1 };
        assert!(validate_manifolds(wrong.clone(), c, &mut budget()).is_ok());
        wrong.manifolds[0].subshapes[0] = 2;
        assert!(matches!(
            validate_manifolds(wrong, c, &mut budget()),
            Err(Error::Geometry)
        ));
        t.manifolds[0].subshape_poses.as_mut().unwrap()[0]
            .as_mut()
            .unwrap()
            .translation[0] = 3f32.to_bits();
        assert!(matches!(
            validate_manifolds(t, c, &mut budget()),
            Err(Error::Geometry)
        ));
    });
    with(Dimension::Planar, |c, mut t| {
        t.pairs[0].plain.count = 0;
        t.pairs[0].clusters = RecordSpan { start: 0, count: 1 };
        assert!(matches!(
            validate_manifolds(t, c, &mut budget()),
            Err(Error::Manifold)
        ));
    });
}
#[test]
fn complete_header_admission_all_truncations_optional_tags_and_budget() {
    with(Dimension::Spatial, |c, t| {
        let bytes = encoded(t.clone(), c);
        for end in 0..bytes.len() {
            let mut b = budget();
            assert!(decode_manifolds(&bytes[..end], c, &mut b).is_err());
            assert_eq!(b.remaining(), MAX_ALLOCATION);
        }
        for (offset, v) in [
            (0, 0),
            (8, 2),
            (12, 123),
            (44, u32::MAX),
            (48, 0),
            (52, u32::MAX),
            (64, 1),
        ] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&v.to_le_bytes());
            let mut b = budget();
            assert!(decode_manifolds(&bad, c, &mut b).is_err());
            assert_eq!(b.remaining(), MAX_ALLOCATION);
        }
        // First manifold starts after nodes and pair. Outer pose option, first
        // child option and inactive pose payload each retain canonical tags.
        for offset in [116 + 48, 116 + 52, 116 + 56] {
            let mut bad = bytes.clone();
            bad[offset..offset + 4].copy_from_slice(&2u32.to_le_bytes());
            assert!(matches!(
                decode_manifolds(&bad, c, &mut budget()),
                Err(Error::Header)
            ));
        }
        let payload = 2 * size_of::<Handle>()
            + size_of::<ContactPairManifolds>()
            + size_of::<ContactManifoldRecord>()
            + size_of::<TrackedContactPoint>()
            + size_of::<SolverContactPoint>();
        let scratch = 2 + size_of::<[u32; 2]>();
        assert!(matches!(
            decode_manifolds(&bytes, c, &mut Budget::new(payload + scratch - 1).unwrap()),
            Err(Error::Limit)
        ));
        let mut b = Budget::new(payload + scratch + bytes.len()).unwrap();
        let v = decode_manifolds(&bytes, c, &mut b).unwrap();
        assert_eq!(encode_manifolds(&v, &mut b).unwrap(), bytes);
        assert_eq!(b.remaining(), 0);
    });
}
#[test]
fn byte_mutations_preserve_every_admitted_byte() {
    with(Dimension::Spatial, |c, t| {
        let bytes = encoded(t, c);
        for i in 0..bytes.len() {
            for mask in [1, 0x80, 0xff] {
                let mut bad = bytes.clone();
                bad[i] ^= mask;
                if let Ok(v) = decode_manifolds(&bad, c, &mut budget()) {
                    assert_eq!(encode_manifolds(&v, &mut budget()).unwrap(), bad);
                }
            }
        }
    });
}

#[test]
fn independent_identity_and_geometry_layout_witnesses_fail_before_allocation() {
    with(Dimension::Spatial, |c, t| {
        let mut wrong = c.geometries[0].record().clone();
        wrong.identity.build[0] ^= 1;
        let geometries = [
            validate_geometry(wrong).unwrap(),
            geometry(Dimension::Spatial),
        ];
        for c in [
            ManifoldContext {
                geometries: &geometries,
                ..c
            },
            ManifoldContext {
                geometries: &[],
                ..c
            },
        ] {
            let mut b = budget();
            assert!(validate_manifolds(t.clone(), c, &mut b).is_err());
            assert_eq!(b.remaining(), MAX_ALLOCATION);
        }
        let mut wrong = t;
        wrong.identity.build[0] ^= 1;
        let mut b = budget();
        assert!(matches!(
            validate_manifolds(wrong, c, &mut b),
            Err(Error::Identity)
        ));
        assert_eq!(b.remaining(), MAX_ALLOCATION);
    });
}

#[test]
fn clustered_native_capacity_is_exact_and_ordered_points_are_not_deduplicated() {
    with(Dimension::Spatial, |c, mut t| {
        t.pairs[0].plain.count = 0;
        t.pairs[0].clusters = RecordSpan { start: 0, count: 1 };
        t.points.resize(255, TrackedContactPoint::default());
        t.manifolds[0].points.count = 255;
        t.solver[0].contact_id = NEW_CONTACT_BIT | 254;
        let bytes = encoded(t.clone(), c);
        assert_eq!(
            decode_manifolds(&bytes, c, &mut budget())
                .unwrap()
                .record()
                .points
                .len(),
            255
        );
        t.points.push(TrackedContactPoint::default());
        t.manifolds[0].points.count = 256;
        assert!(matches!(
            validate_manifolds(t, c, &mut budget()),
            Err(Error::Manifold)
        ));
    });
}

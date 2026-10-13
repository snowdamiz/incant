use incant_physics_snapshot::*;
pub fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
pub fn id(d: Dimension) -> Identity {
    Identity {
        dimension: d,
        build: [91; 32],
    }
}
pub fn h(slot: u32) -> Handle {
    Handle {
        slot,
        generation: 3,
    }
}
pub fn arena(d: Dimension) -> ValidatedArena {
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
pub fn graph(d: Dimension, edges: &[[u32; 2]]) -> ValidatedGraph {
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
pub fn pose(d: Dimension) -> LocalPose {
    LocalPose {
        translation: [0; 3],
        rotation: if d == Dimension::Planar {
            [1f32.to_bits(), 0, 0, 0]
        } else {
            [0, 0, 0, 1f32.to_bits()]
        },
    }
}
pub fn geometry(d: Dimension) -> ValidatedGeometry {
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
pub fn table(d: Dimension) -> ManifoldTable {
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
pub fn with(d: Dimension, f: impl FnOnce(ManifoldContext<'_>, ManifoldTable)) {
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

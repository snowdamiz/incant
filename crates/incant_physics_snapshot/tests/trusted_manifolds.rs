//! Actual completed pinned native worlds, observed one-way. No live deserialize.
use incant_physics_snapshot::*;
use serde_json::Value;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn word(v: &Value) -> u32 {
    v.as_u64().unwrap().try_into().unwrap()
}
fn link(v: &Value) -> Option<u32> {
    let n = word(v);
    (n != u32::MAX).then_some(n)
}
fn handle((slot, generation): (u32, u32)) -> Handle {
    Handle { slot, generation }
}
fn layout(value: &Value, identity: Identity) -> (ValidatedArena, Vec<Handle>) {
    let mut handles = vec![];
    let slots = value["items"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(slot, item)| {
            if let Some(free) = item.get("Free") {
                ArenaSlot::Free {
                    next: (!free["next_free"].is_null()).then(|| word(&free["next_free"])),
                }
            } else {
                let occupied = &item["Occupied"];
                let generation = word(&occupied["generation"]);
                let row = handles.len() as u32;
                handles.push(Handle {
                    slot: slot as u32,
                    generation,
                });
                ArenaSlot::Occupied { generation, row }
            }
        })
        .collect();
    let record = ArenaRecord {
        identity,
        generation: word(&value["generation"]),
        free_head: (!value["free_list_head"].is_null()).then(|| word(&value["free_list_head"])),
        live: word(&value["len"]),
        slots,
    };
    (validate_arena(record, &mut budget()).unwrap(), handles)
}
fn bits2(v: rapier2d::math::Vector) -> [u32; 3] {
    [v.x.to_bits(), v.y.to_bits(), 0]
}
fn bits3(v: rapier3d::math::Vector) -> [u32; 3] {
    v.to_array().map(f32::to_bits)
}
fn data2(d: &rapier2d::geometry::ContactData) -> ContactWarmstart {
    ContactWarmstart {
        impulse: d.impulse.to_bits(),
        tangent_impulse: [d.tangent_impulse[0].to_bits(), 0],
        warmstart_impulse: d.warmstart_impulse.to_bits(),
        warmstart_tangent_impulse: [d.warmstart_tangent_impulse[0].to_bits(), 0],
        solver_dp1: bits2(d.solver_dp1),
        solver_dp2: bits2(d.solver_dp2),
        ..Default::default()
    }
}
fn data3(d: &rapier3d::geometry::ContactData) -> ContactWarmstart {
    ContactWarmstart {
        impulse: d.impulse.to_bits(),
        tangent_impulse: [
            d.tangent_impulse[0].to_bits(),
            d.tangent_impulse[1].to_bits(),
        ],
        warmstart_impulse: d.warmstart_impulse.to_bits(),
        warmstart_tangent_impulse: [
            d.warmstart_tangent_impulse[0].to_bits(),
            d.warmstart_tangent_impulse[1].to_bits(),
        ],
        warmstart_twist_impulse: d.warmstart_twist_impulse.to_bits(),
        warmstart_tangent_world: bits3(d.warmstart_tangent_world),
        solver_dp1: bits3(d.solver_dp1),
        solver_dp2: bits3(d.solver_dp2),
    }
}
fn vector2(x: f32, y: f32) -> rapier2d::math::Vector {
    rapier2d::math::Vector::new(x, y)
}
fn vector3(x: f32, y: f32) -> rapier3d::math::Vector {
    rapier3d::math::Vector::new(x, y, 0.)
}
fn box2(x: f32, y: f32) -> rapier2d::geometry::ColliderBuilder {
    rapier2d::geometry::ColliderBuilder::cuboid(x, y)
}
fn box3(x: f32, y: f32) -> rapier3d::geometry::ColliderBuilder {
    rapier3d::geometry::ColliderBuilder::cuboid(x, y, x)
}

fn planar_bits(pose: &rapier2d::math::Pose) -> LocalPose {
    LocalPose {
        translation: [
            pose.translation.x.to_bits(),
            pose.translation.y.to_bits(),
            0,
        ],
        rotation: [pose.rotation.re.to_bits(), pose.rotation.im.to_bits(), 0, 0],
    }
}
fn spatial_bits(pose: &rapier3d::math::Pose) -> LocalPose {
    LocalPose {
        translation: pose.translation.to_array().map(f32::to_bits),
        rotation: pose.rotation.to_array().map(f32::to_bits),
    }
}

fn graph(v: &Value, identity: Identity) -> ValidatedGraph {
    validate_graph(
        GraphRecord {
            identity,
            nodes: v["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, n)| GraphNode {
                    head: [link(&n["next"][0]), link(&n["next"][1])],
                    row: i as u32,
                })
                .collect(),
            edges: v["edges"]
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(|(i, e)| GraphEdge {
                    node: [word(&e["node"][0]), word(&e["node"][1])],
                    next: [link(&e["next"][0]), link(&e["next"][1])],
                    prev: [link(&e["prev"][0]), link(&e["prev"][1])],
                    row: i as u32,
                })
                .collect(),
        },
        &mut budget(),
    )
    .unwrap()
}
macro_rules! cases {
    ($name:ident,$rapier:ident,$dim:ident,$vector:path,$cuboid:path,$bits:path,$data:path,$posebits:path) => {
        mod $name {
            use super::*;
            use $rapier::prelude::*;
            fn identity() -> Identity {
                Identity {
                    dimension: Dimension::$dim,
                    build: [92; 32],
                }
            }
            fn geometry(c: &Collider) -> ValidatedGeometry {
                // Capture actual public native geometry; never substitute a
                // fixture-side assumed shape for the collider's current shape.
                let shape = c.shape();
                let parts = if let Some(compound) = shape.as_compound() {
                    compound
                        .shapes()
                        .iter()
                        .map(|(pose, s)| {
                            let p = $posebits(pose);
                            if Dimension::$dim == Dimension::Planar {
                                assert_eq!(p.rotation, [1f32.to_bits(), 0, 0, 0]);
                            }
                            Part {
                                translation: p.translation,
                                rotation: if Dimension::$dim == Dimension::Planar {
                                    [0, 0, 0, 1f32.to_bits()]
                                } else {
                                    p.rotation
                                },
                                shape: Primitive::Box {
                                    half_extents: $bits(s.as_cuboid().unwrap().half_extents),
                                },
                            }
                        })
                        .collect()
                } else {
                    vec![Part {
                        translation: [0; 3],
                        rotation: [0, 0, 0, 1f32.to_bits()],
                        shape: Primitive::Box {
                            half_extents: $bits(shape.as_cuboid().unwrap().half_extents),
                        },
                    }]
                };
                let kind = if shape.as_compound().is_none() {
                    GeometryKind::Primitive
                } else if Dimension::$dim == Dimension::Planar {
                    GeometryKind::PlanarTiles
                } else {
                    GeometryKind::SpatialCompound
                };
                validate_geometry(GeometryRecord {
                    identity: identity(),
                    kind,
                    parts,
                })
                .unwrap()
            }
            fn push(t: &mut ManifoldTable, m: &$rapier::geometry::ContactManifold) {
                let points = RecordSpan {
                    start: t.points.len() as u32,
                    count: m.points.len() as u32,
                };
                let solver = RecordSpan {
                    start: t.solver.len() as u32,
                    count: m.data.solver_contacts.len() as u32,
                };
                t.points
                    .extend(m.points.iter().map(|p| TrackedContactPoint {
                        local_p1: $bits(p.local_p1),
                        local_p2: $bits(p.local_p2),
                        distance: p.dist.to_bits(),
                        features: [p.fid1.0, p.fid2.0],
                        data: $data(&p.data),
                    }));
                t.solver.extend(m.data.solver_contacts.iter().map(|s| {
                    let v = serde_json::to_value(s).unwrap();
                    let padding = if Dimension::$dim == Dimension::Spatial {
                        (v["padding"][0].as_f64().unwrap() as f32).to_bits()
                    } else {
                        0
                    };
                    SolverContactPoint {
                        anchor1: $bits(s.anchor1),
                        anchor2: $bits(s.anchor2),
                        distance: s.dist.to_bits(),
                        tangent_velocity: $bits(s.tangent_velocity),
                        contact_id: s.contact_id[0],
                        padding,
                    }
                }));
                let d = &m.data;
                let private = serde_json::to_value(d).unwrap();
                t.manifolds.push(ContactManifoldRecord {
                    points,
                    solver,
                    local_normals: [$bits(m.local_n1), $bits(m.local_n2)],
                    subshapes: [m.subshape1, m.subshape2],
                    subshape_poses: m.subshape_poses.as_ref().map(|p| {
                        [
                            p.pos1.as_ref().map($posebits),
                            p.pos2.as_ref().map($posebits),
                        ]
                    }),
                    bodies: [d.rigid_body1, d.rigid_body2].map(|h| h.map(|h| handle(h.into_raw_parts()))),
                    solver_flags: d.solver_flags.bits(),
                    solver_color: word(&private["solver_color"]),
                    solver_body_ids: [
                        word(&private["solver_body_ids"][0]),
                        word(&private["solver_body_ids"][1]),
                    ],
                    graph_position: word(&private["graph_pos"]),
                    normal: $bits(d.normal),
                    relative_dominance: i32::from(d.relative_dominance),
                    user_data: d.user_data,
                    friction: d.friction.to_bits(),
                    restitution: d.restitution.to_bits(),
                });
            }
            fn observe(w: &PhysicsWorld, pending: bool) -> Option<ManifoldTable> {
                let bv = serde_json::to_value(&w.bodies).unwrap();
                let cv = serde_json::to_value(&w.colliders).unwrap();
                let (ba, bh) = layout(&bv["bodies"], identity());
                let (ca, ch) = layout(&cv["colliders"], identity());
                let mut a = AttachmentRecord {
                    identity: identity(),
                    bodies: vec![],
                    colliders: vec![],
                    attached: vec![],
                };
                for h in bh {
                    let body = &w.bodies[RigidBodyHandle::from_raw_parts(h.slot, h.generation)];
                    a.bodies.push(AttachmentSpan {
                        start: a.attached.len() as u32,
                        count: body.colliders().len() as u32,
                    });
                    a.attached
                        .extend(body.colliders().iter().map(|h| handle(h.into_raw_parts())));
                }
                let mut shapes = vec![];
                for h in ch {
                    let co = &w.colliders[ColliderHandle::from_raw_parts(h.slot, h.generation)];
                    a.colliders.push(co.parent().map(|h| ParentAttachment {
                        body: handle(h.into_raw_parts()),
                        local_pose: $posebits(co.position_wrt_parent().unwrap()),
                    }));
                    shapes.push(geometry(co));
                }
                let a = validate_attachments(a, &ba, &ca, &mut budget()).unwrap();
                let raw = w.narrow_phase.contact_graph().raw_graph();
                let g = graph(&serde_json::to_value(raw).unwrap(), identity());
                let journals = PendingJournals {
                    modified_bodies: bv["modified_bodies"][0].as_array().unwrap().len() as u32,
                    modified_colliders: cv["modified_colliders"][0].as_array().unwrap().len() as u32,
                    removed_colliders: cv["removed_colliders"].as_array().unwrap().len() as u32,
                };
                let mut t = ManifoldTable {
                    identity: identity(),
                    nodes: raw
                        .raw_nodes()
                        .iter()
                        .map(|n| handle(n.weight.into_raw_parts()))
                        .collect(),
                    pairs: vec![],
                    manifolds: vec![],
                    points: vec![],
                    solver: vec![],
                };
                for e in raw.raw_edges() {
                    let p = &e.weight;
                    let rigid = p.rigid().unwrap();
                    let plain = RecordSpan {
                        start: t.manifolds.len() as u32,
                        count: rigid.manifolds.len() as u32,
                    };
                    for m in &rigid.manifolds {
                        push(&mut t, m);
                    }
                    let clusters = RecordSpan {
                        start: t.manifolds.len() as u32,
                        count: rigid.solver_clusters.len() as u32,
                    };
                    for m in &rigid.solver_clusters {
                        push(&mut t, m);
                    }
                    t.pairs.push(ContactPairManifolds {
                        colliders: [
                            handle(p.collider1.into_raw_parts()),
                            handle(p.collider2.into_raw_parts()),
                        ],
                        plain,
                        clusters,
                    });
                }
                let c = ManifoldContext {
                    graph: &g,
                    attachments: &a,
                    geometries: &shapes,
                    journals,
                };
                if pending {
                    assert_ne!(journals, PendingJournals::default());
                    assert!(matches!(
                        validate_manifolds(t, c, &mut budget()),
                        Err(Error::PendingMutation)
                    ));
                    return None;
                }
                assert_eq!(journals, PendingJournals::default());
                let v = validate_manifolds(t.clone(), c, &mut budget()).unwrap();
                let bytes = encode_manifolds(&v, &mut budget()).unwrap();
                let loaded = decode_manifolds(&bytes, c, &mut budget()).unwrap();
                assert_eq!(loaded.record(), &t);
                assert_eq!(encode_manifolds(&loaded, &mut budget()).unwrap(), bytes);
                for (i, p) in t.pairs.iter().enumerate() {
                    assert_eq!(loaded.edge_pair(i as u32), Some(p));
                }
                for (i, m) in t.manifolds.iter().enumerate() {
                    assert_eq!(
                        loaded.manifold_points(i as u32).unwrap(),
                        &t.points[m.points.start as usize..(m.points.start + m.points.count) as usize]
                    );
                }
                Some(t)
            }
            fn fixture(compound: bool) -> (PhysicsWorld, RigidBodyHandle, ColliderHandle) {
                let mut w = PhysicsWorld::default();
                w.integration_parameters.dt = 1. / 60.;
                w.integration_parameters.contact_clustering = true;
                let ground = if compound {
                    ColliderBuilder::compound(
                        [0.4, 1.6]
                            .map(|x| {
                                let mut pose = $rapier::math::Pose::IDENTITY;
                                pose.translation = $vector(x, 0.5);
                                (pose, $cuboid(0.6, 0.5).build().shared_shape().clone())
                            })
                            .to_vec(),
                    )
                } else {
                    $cuboid(4., 0.5).translation($vector(1., 0.5))
                };
                // Standalone ground explicitly exercises a None body owner.
                w.insert_collider(ground.friction(0.9), None);
                let (body, co) = w.insert(
                    RigidBodyBuilder::dynamic()
                        .translation($vector(1., 1.49))
                        .linvel($vector(0.05, 0.)),
                    $cuboid(0.95, 0.5).friction(0.9),
                );
                (w, body, co)
            }
            #[test]
            fn actual_plain_owners_generations_journals_and_graph_removal() {
                let (mut w, body, co) = fixture(false);
                let (_, other) = w.insert(
                    RigidBodyBuilder::dynamic().translation($vector(3.5, 1.49)),
                    $cuboid(0.4, 0.5),
                );
                observe(&w, true);
                let mut warm = false;
                for _ in 0..60 {
                    w.step();
                    let t = observe(&w, false).unwrap();
                    warm |= t
                        .points
                        .iter()
                        .any(|p| f32::from_bits(p.data.warmstart_impulse) > 0.);
                    assert!(t.pairs.iter().all(|p| p.clusters.count == 0));
                    assert_eq!(t.pairs.len(), 2);
                }
                assert!(warm);
                w.bodies[body].set_linvel($vector(0.1, 0.), true);
                observe(&w, true);
                w.step();
                observe(&w, false);
                w.remove_collider(co).unwrap();
                observe(&w, true);
                w.step();
                let removed = observe(&w, false).unwrap();
                assert_eq!(removed.pairs.len(), 1);
                assert!(removed.nodes.contains(&handle(other.into_raw_parts())));
                assert!(!removed.nodes.contains(&handle(co.into_raw_parts())));
                let new = w.insert_collider($cuboid(0.95, 0.5), Some(body));
                assert_eq!(new.into_raw_parts().0, co.into_raw_parts().0);
                assert_ne!(new.into_raw_parts().1, co.into_raw_parts().1);
                observe(&w, true);
                w.step();
                let t = observe(&w, false).unwrap();
                assert!(t.nodes.contains(&handle(new.into_raw_parts())));
                assert!(!t.nodes.contains(&handle(co.into_raw_parts())));
                eprintln!(
                    "MANIFOLD_NATIVE dimension={} plain_warm={} removed_reused=true pending_refused=true",
                    Dimension::$dim as u8,
                    warm
                );
            }
            #[test]
            fn actual_compound_plain_and_cluster_coordinate_frames() {
                let (mut w, _, _) = fixture(true);
                let mut cluster_seen = false;
                let mut changed = false;
                let mut mixed = false;
                let mut warm = false;
                for _ in 0..60 {
                    w.step();
                    let table = observe(&w, false).unwrap();
                    for pair in w.contact_pairs() {
                        let r = pair.rigid().unwrap();
                        assert!(r.manifolds.len() >= 2);
                        for cluster in &r.solver_clusters {
                            cluster_seen = true;
                            assert!(cluster.subshape_pos1().is_none() && cluster.subshape_pos2().is_none());
                            let mut matched_children = std::collections::BTreeSet::new();
                            for p in &cluster.points {
                                let mut matched = false;
                                for plain in &r.manifolds {
                                    for source in &plain.points {
                                        let a = plain
                                            .subshape_pos1()
                                            .map_or(source.local_p1, |pose| *pose * source.local_p1);
                                        let b = plain
                                            .subshape_pos2()
                                            .map_or(source.local_p2, |pose| *pose * source.local_p2);
                                        if $bits(a) == $bits(p.local_p1)
                                            && $bits(b) == $bits(p.local_p2)
                                        {
                                            matched = true;
                                            changed |= $bits(a) != $bits(source.local_p1)
                                                || $bits(b) != $bits(source.local_p2);
                                            matched_children.insert([plain.subshape1, plain.subshape2]);
                                        }
                                    }
                                }
                                assert!(
                                    matched,
                                    "cluster point exactly transforms an observed plain point"
                                );
                            }
                            mixed |= matched_children.len() > 1;
                        }
                    }
                    warm |= table
                        .points
                        .iter()
                        .any(|p| f32::from_bits(p.data.warmstart_impulse) > 0.);
                }
                assert!(warm);
                if Dimension::$dim == Dimension::Spatial {
                    assert!(cluster_seen && changed && mixed);
                } else {
                    assert!(!cluster_seen);
                }
                eprintln!(
                    "MANIFOLD_NATIVE dimension={} compound=true cluster={} transformed={} mixed_children={} warm={}",
                    Dimension::$dim as u8,
                    cluster_seen,
                    changed,
                    mixed,
                    warm
                );
            }
        }

    }
}
cases!(
    planar,
    rapier2d,
    Planar,
    vector2,
    box2,
    bits2,
    data2,
    planar_bits
);
cases!(
    spatial,
    rapier3d,
    Spatial,
    vector3,
    box3,
    bits3,
    data3,
    spatial_bits
);

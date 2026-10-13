//! Native owner observation plus explicitly trusted continuation diagnostics.
//! No hostile native deserialization or production live-save admission.
use incant_physics_snapshot::*;
use serde_json::Value;
#[path = "support/body_capture.rs"]
mod body_capture;
#[path = "support/collider_capture.rs"]
mod collider_capture;
#[path = "support/island_capture.rs"]
mod island_capture;
#[path = "support/rigid_owner_capture.rs"]
mod rigid_owner_capture;
#[macro_use]
#[path = "support/island_cases.rs"]
mod island_cases;
#[macro_use]
#[path = "support/collider_cases.rs"]
mod collider_cases;
#[macro_use]
#[path = "support/continuation_cases.rs"]
mod continuation_cases;
#[macro_use]
#[path = "support/narrow_cases.rs"]
mod narrow_cases;
#[path = "support/narrow_checks.rs"]
mod narrow_checks;
#[path = "support/ordered_capture.rs"]
mod ordered_capture;
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
fn float(v: &Value) -> u32 {
    (v.as_f64().unwrap() as f32).to_bits()
}
fn observed_rotation(v: &Value, d: Dimension) -> [u32; 4] {
    if d == Dimension::Planar {
        [float(&v["re"]), float(&v["im"]), 0, 0]
    } else {
        [float(&v[0]), float(&v[1]), float(&v[2]), float(&v[3])]
    }
}
fn observed_pose(v: &Value, d: Dimension) -> LocalPose {
    LocalPose {
        translation: if d == Dimension::Planar {
            [float(&v["translation"][0]), float(&v["translation"][1]), 0]
        } else {
            [
                float(&v["translation"][0]),
                float(&v["translation"][1]),
                float(&v["translation"][2]),
            ]
        },
        rotation: observed_rotation(&v["rotation"], d),
    }
}
fn captured_pair(value: &Value, r: &mut PairStateTable) {
    let rigid = value["contacts"]
        .get("Rigid")
        .expect("only supported rigid native pair");
    let rec = &rigid["recycle_state"];
    let recycle = (!rec.is_null()).then(|| RecycleState {
        relative_pose: observed_pose(&rec["pos12"], r.identity.dimension),
        rotations: [
            observed_rotation(&rec["rot1"], r.identity.dimension),
            observed_rotation(&rec["rot2"], r.identity.dimension),
        ],
        max_extent: float(&rec["max_extent"]),
        max_drift: float(&rec["max_drift"]),
    });
    let workspace = &rigid["workspace"];
    let start = r.detectors.len() as u32;
    let (kind, timestamp) = if workspace.is_null() {
        (WorkspaceKind::None, false)
    } else {
        let mut entries = workspace.as_object().unwrap().iter();
        let (tag, work) = entries.next().unwrap();
        assert!(entries.next().is_none());
        let kind = match tag.as_str() {
            "CompositeShapeShapeContactManifoldsWorkspace" => WorkspaceKind::CompositeShape,
            "CompositeShapeCompositeShapeContactManifoldsWorkspace" => {
                WorkspaceKind::CompositeComposite
            }
            _ => panic!("unsupported native workspace: {tag}"),
        };
        for entry in work["sub_detectors"].as_array().unwrap() {
            let key = if kind == WorkspaceKind::CompositeShape {
                [word(&entry[0]), 0]
            } else {
                [word(&entry[0][0]), word(&entry[0][1])]
            };
            r.detectors.push(SubDetector {
                key,
                manifold: word(&entry[1]["manifold_id"]),
                timestamp: entry[1]["timestamp"].as_bool().unwrap(),
            });
        }
        (kind, work["timestamp"].as_bool().unwrap())
    };
    r.pairs.push(PairState {
        event_status: word(&value["event_status"]),
        solver_color: word(&rigid["solver_color"]),
        solver_color_bodies: [
            word(&rigid["solver_color_bodies"][0]),
            word(&rigid["solver_color_bodies"][1]),
        ],
        recycle,
        workspace: kind,
        timestamp,
        detectors: RecordSpan {
            start,
            count: r.detectors.len() as u32 - start,
        },
    });
}
fn check_owner_corruption(r: RigidOwnerRecord, c: RigidOwnerContext<'_>, bytes: &[u8]) {
    let mut mutations = vec![];
    let mut bad = r.clone();
    bad.sleep_scan_stamp = bad.sleep_scan_stamp.wrapping_add(1);
    mutations.push(bad);
    let mut bad = r.clone();
    bad.sleep_scan_touched = 1;
    mutations.push(bad);
    let mut bad = r.clone();
    bad.force_flagged.push(0);
    mutations.push(bad);
    if !r.sleep_scan.is_empty() {
        let mut bad = r.clone();
        bad.sleep_scan.clear();
        mutations.push(bad);
    }
    if let Some(pair) = c.pairs.record().pairs.iter().find(|p| p.solver_color < 128) {
        let mut bad = r.clone();
        bad.body_color_masks[pair.solver_color_bodies[0] as usize] &= !(1u128 << pair.solver_color);
        mutations.push(bad);
    }
    if !r.pair_hints.is_empty() {
        let mut bad = r.clone();
        bad.pair_hints[0] = 65536;
        mutations.push(bad);
    }
    if !r.references.is_empty() {
        let mut bad = r.clone();
        bad.references[0].edge = u32::MAX;
        mutations.push(bad);
    }
    if !r.force_positions.is_empty() {
        let mut bad = r.clone();
        bad.force_positions[0] = if bad.force_positions[0] == u32::MAX {
            0
        } else {
            u32::MAX
        };
        mutations.push(bad);
    }
    for bad in mutations {
        assert!(validate_rigid_owner(bad, c, &mut budget()).is_err());
    }
    for cut in [0, 1, 43, 99, bytes.len() - 1] {
        assert!(decode_rigid_owner(&bytes[..cut], c, &mut budget()).is_err());
    }
    let mut trailing = bytes.to_vec();
    trailing.push(0);
    assert!(decode_rigid_owner(&trailing, c, &mut budget()).is_err());
    let mut bad = bytes.to_vec();
    bad[68..72].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_rigid_owner(&bad, c, &mut budget()).is_err());
    assert!(matches!(
        decode_rigid_owner(bytes, c, &mut Budget::new(0).unwrap()),
        Err(Error::Limit)
    ));
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
            fn pair_states(w: &PhysicsWorld) -> PairStateTable {
                let mut r = PairStateTable {
                    identity: identity(),
                    pairs: vec![],
                    detectors: vec![],
                };
                for edge in w.narrow_phase.contact_graph().raw_graph().raw_edges() {
                    captured_pair(&ordered_capture::capture(&edge.weight), &mut r);
                }
                r
            }
            fn observe(w: &PhysicsWorld, pending: bool) -> Option<ManifoldTable> {
                observe_with_owner(w,pending).map(|(m,_)|m)
            }
            fn observe_with_owner(w: &PhysicsWorld, pending: bool) -> Option<(ManifoldTable,RigidOwnerRecord)> {
                let bv = ordered_capture::capture(&w.bodies);
                let cv = ordered_capture::capture(&w.colliders);
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
                let g = graph(&ordered_capture::capture(raw), identity());
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
                let state = pair_states(w);
                let checked = validate_pair_state(state.clone(), &v, &mut budget()).unwrap();
                let encoded = encode_pair_state(&checked, &mut budget()).unwrap();
                let decoded = decode_pair_state(&encoded, &v, &mut budget()).unwrap();
                assert_eq!(decoded.record(), &state);
                assert_eq!(encode_pair_state(&decoded, &mut budget()).unwrap(), encoded);
                for (row, p) in state.pairs.iter().enumerate() {
                    for i in 0..p.detectors.count {
                        let detector = &state.detectors[(p.detectors.start + i) as usize];
                        assert_eq!(
                            decoded.detector_manifold(row as u32, i).unwrap(),
                            &t.manifolds[(t.pairs[row].plain.start + detector.manifold) as usize]
                        );
                    }
                }
                let owner=check_islands(w, &a, &v, &shapes, &ca);
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
                Some((t,owner))
            }
            island_cases!($rapier, $vector, $cuboid);
            collider_cases!($rapier, $vector, $cuboid, $posebits);
            continuation_cases!($rapier, $vector, $cuboid);
            narrow_cases!($rapier, $vector, $cuboid);
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
            fn actual_pair_events_recycle_and_workspace_churn() {
                let (mut w, body, co) = fixture(true);
                w.colliders[co]
                    .set_active_events(ActiveEvents::COLLISION_EVENTS | ActiveEvents::CONTACT_FORCE_EVENTS);
                w.colliders[co].set_contact_force_event_threshold(0.);
                let mut recycled = false;
                let mut previous = None;
                let mut events = false;
                for _ in 0..40 {
                    w.step();
                    observe(&w, false);
                    let r = pair_states(&w);
                    events |= r.pairs.iter().any(|p| p.event_status != 0);
                    let state = r.pairs[0].recycle;
                    let position = $bits(w.bodies[body].translation());
                    if let Some((old, old_position)) = previous {
                        recycled |= state.is_some() && state == old && position != old_position;
                    }
                    previous = Some((state, position));
                }
                assert!(events && recycled);
                let old = pair_states(&w);
                w.bodies[body].set_translation($vector(1.25, 1.49), true);
                w.step();
                observe(&w, false);
                let moved = pair_states(&w);
                assert_ne!(old.pairs[0].recycle, moved.pairs[0].recycle);
                let ground = w
                    .colliders
                    .iter()
                    .find(|(_, c)| c.parent().is_none())
                    .unwrap()
                    .0;
                let compound = w.colliders[ground].shared_shape().clone();
                let primitive = w.colliders[co].shared_shape().clone();
                // Swap which ordered collider is composite, keeping the same
                // live handles. Native shape-change invalidates its old workspace.
                w.colliders[ground].set_shape(primitive);
                w.colliders[ground].set_translation($vector(1., 0.5));
                w.colliders[co].set_shape(compound.clone());
                w.bodies[body].set_translation($vector(0., 0.99), true);
                observe(&w, true);
                w.step();
                observe(&w, false);
                assert!(
                    pair_states(&w)
                        .pairs
                        .iter()
                        .any(|p| p.workspace == WorkspaceKind::CompositeShape)
                );
                // Both colliders become composite; a different map key arity is
                // exercised through actual native dispatch, not fabricated rows.
                w.colliders[ground].set_shape(compound);
                w.colliders[ground].set_translation($vector(0., 0.));
                w.bodies[body].set_translation($vector(0., 0.99), true);
                w.step();
                observe(&w, false);
                let both = pair_states(&w);
                assert!(
                    both.pairs
                        .iter()
                        .any(|p| p.workspace == WorkspaceKind::CompositeComposite)
                );
                assert!(!both.detectors.is_empty());
                // Churn the actual query footprint, preserving native detector
                // row order and checking its manifold bijection each step.
                let mut histories = vec![];
                for x in [0.2, 0.8, 1.4, 0.4, 0.] {
                    w.bodies[body].set_translation($vector(x, 0.99), true);
                    w.step();
                    observe(&w, false);
                    histories.push(
                        pair_states(&w)
                            .detectors
                            .into_iter()
                            .map(|d| (d.key, d.manifold))
                            .collect::<Vec<_>>(),
                    );
                }
                assert!(histories.windows(2).any(|p| p[0] != p[1]));
                let mut flips = std::collections::BTreeSet::new();
                for half in [0.3, 3.0] {
                    let shape = SharedShape::compound(
                        [0.4, 1.6]
                            .map(|x| {
                                let mut p = $rapier::math::Pose::IDENTITY;
                                p.translation = $vector(x, 0.5);
                                (p, $cuboid(half, 0.5).build().shared_shape().clone())
                            })
                            .to_vec(),
                    );
                    w.colliders[co].set_shape(shape);
                    w.bodies[body].set_translation($vector(0., 0.99), true);
                    w.step();
                    observe(&w, false);
                    let p = w.contact_pairs().next().unwrap();
                    let a = w.colliders[p.collider1]
                        .shape()
                        .as_compound()
                        .unwrap()
                        .bvh()
                        .root_aabb()
                        .half_extents()
                        .length_squared();
                    let b = w.colliders[p.collider2]
                        .shape()
                        .as_compound()
                        .unwrap()
                        .bvh()
                        .root_aabb()
                        .half_extents()
                        .length_squared();
                    flips.insert(a < b);
                }
                assert_eq!(
                    flips.len(),
                    2,
                    "both pinned composite traversal-size branches"
                );

                eprintln!(
                    "PAIR_STATE_NATIVE dimension={} events={} recycle_reference_reused={} forced_full_changed=true composite_swap=true two_composite=true churn=true both_traversal_sizes=true",
                    Dimension::$dim as u8,
                    events,
                    recycled
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
                            assert!(
                                cluster.subshape_poses.is_none(),
                                "ordinary rebuilt clusters have no outer pose allocation"
                            );
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

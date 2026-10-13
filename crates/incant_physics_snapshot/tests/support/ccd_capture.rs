//! One-way trusted observations; never deserialize bytes to a native solver.
use incant_physics_snapshot::*;
use serde_json::Value;
pub fn handle((slot, generation): (u32, u32)) -> Handle {
    Handle { slot, generation }
}
pub fn layout(v: &Value, id: Identity) -> (ValidatedArena, Vec<Handle>) {
    let mut handles = vec![];
    let word = |v: &Value| u32::try_from(v.as_u64().unwrap()).unwrap();
    let slots = v["items"]
        .as_array()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(slot, item)| {
            if let Some(f) = item.get("Free") {
                ArenaSlot::Free {
                    next: (!f["next_free"].is_null()).then(|| word(&f["next_free"])),
                }
            } else {
                let o = &item["Occupied"];
                let generation = word(&o["generation"]);
                let row = handles.len() as u32;
                handles.push(Handle {
                    slot: slot as u32,
                    generation,
                });
                ArenaSlot::Occupied { generation, row }
            }
        })
        .collect();
    let r = ArenaRecord {
        identity: id,
        generation: word(&v["generation"]),
        free_head: (!v["free_list_head"].is_null()).then(|| word(&v["free_list_head"])),
        live: word(&v["len"]),
        slots,
    };
    (
        validate_arena(r, &mut Budget::new(MAX_ALLOCATION).unwrap()).unwrap(),
        handles,
    )
}
macro_rules! ccd_observer {
    ($d:expr) => {
        fn observe(w: &PhysicsWorld, f: impl FnOnce(CcdContext<'_>, CcdState)) {
            let id = fixture::identity($d);
            let bv = ordered_capture::capture(&w.bodies);
            let cv = ordered_capture::capture(&w.colliders);
            let (ba, bh) = ccd_capture::layout(&bv["bodies"], id);
            let (ca, ch) = ccd_capture::layout(&cv["colliders"], id);
            let mut a = AttachmentRecord {
                identity: id,
                bodies: vec![],
                colliders: vec![],
                attached: vec![],
            };
            let mut bs = vec![];
            let mut cs = vec![];
            let mut geometries = vec![];
            for h in bh {
                let b = &w.bodies[RigidBodyHandle::from_raw_parts(h.slot, h.generation)];
                a.bodies.push(AttachmentSpan {
                    start: a.attached.len() as u32,
                    count: b.colliders().len() as u32,
                });
                a.attached.extend(
                    b.colliders()
                        .iter()
                        .map(|h| ccd_capture::handle(h.into_raw_parts())),
                );
                bs.push(body_capture::body(&ordered_capture::capture(b), $d));
            }
            for h in ch {
                let co = &w.colliders[ColliderHandle::from_raw_parts(h.slot, h.generation)];
                a.colliders.push(co.parent().map(|body| ParentAttachment {
                    body: ccd_capture::handle(body.into_raw_parts()),
                    local_pose: body_capture::pose(
                        &ordered_capture::capture(co.position_wrt_parent().unwrap()),
                        $d,
                    ),
                }));
                cs.push(collider_capture::collider(
                    &ordered_capture::capture(co),
                    $d,
                ));
                geometries.push(
                    validate_geometry(GeometryRecord {
                        identity: id,
                        kind: GeometryKind::Primitive,
                        parts: vec![Part {
                            translation: [0; 3],
                            rotation: [0, 0, 0, 1f32.to_bits()],
                            shape: Primitive::Ball {
                                radius: co.shape().as_ball().unwrap().radius.to_bits(),
                            },
                        }],
                    })
                    .unwrap(),
                );
            }
            let journals = PendingJournals {
                modified_bodies: bv["modified_bodies"][0].as_array().unwrap().len() as u32,
                modified_colliders: cv["modified_colliders"][0].as_array().unwrap().len() as u32,
                removed_colliders: cv["removed_colliders"].as_array().unwrap().len() as u32,
            };
            let a = validate_attachments(a, &ba, &ca, &mut budget()).unwrap();
            let bs = validate_body_state(
                BodyStateTable {
                    identity: id,
                    bodies: bs,
                },
                BodyContext {
                    attachments: &a,
                    journals,
                },
            )
            .unwrap();
            let cs = validate_collider_state(
                ColliderStateTable {
                    identity: id,
                    colliders: cs,
                },
                ColliderContext {
                    bodies: &bs,
                    geometries: &geometries,
                    journals,
                },
            )
            .unwrap();
            let mut config = fixture::config($d);
            config.dt = w.integration_parameters.dt.to_bits();
            config.normalized_prediction_distance = w
                .integration_parameters
                .normalized_prediction_distance
                .to_bits();
            config.max_ccd_substeps = w.integration_parameters.max_ccd_substeps as u64;
            let integration = validate_integration_config(config).unwrap();
            let cache = match w.ccd_solver.checkpoint_fixed_targets() {
                None => CcdCache::Uninitialized,
                Some((prediction, None)) => CcdCache::FullBvh {
                    prediction: prediction.to_bits(),
                },
                Some((prediction, Some(list))) => CcdCache::Targets {
                    prediction: prediction.to_bits(),
                    targets: list
                        .iter()
                        .map(|(h, a)| {
                            let expected = w.colliders[*h].compute_aabb().loosened(prediction);
                            assert_eq!(*a, expected);
                            CcdTarget {
                                collider: ccd_capture::handle(h.into_raw_parts()),
                                mins: bits(a.mins),
                                maxs: bits(a.maxs),
                            }
                        })
                        .collect(),
                },
            };
            f(
                CcdContext {
                    colliders: &cs,
                    integration: &integration,
                },
                CcdState {
                    identity: id,
                    cache,
                },
            );
        }
        fn roundtrip(w: &PhysicsWorld) -> CcdState {
            let mut record = None;
            observe(w, |c, r| {
                let v = validate_ccd_state(r.clone(), c).unwrap();
                let b = encode_ccd_state(&v, &mut budget()).unwrap();
                let d = decode_ccd_state(&b, c, &mut budget()).unwrap();
                assert_eq!(d.record(), &r);
                assert_eq!(encode_ccd_state(&d, &mut budget()).unwrap(), b);
                record = Some(r);
            });
            record.unwrap()
        }
    };
}
pub(crate) use ccd_observer;

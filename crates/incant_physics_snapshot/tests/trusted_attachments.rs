//! Trusted one-way layout observation. Never deserialize a live solver object.
use incant_physics_snapshot::*;
use serde_json::Value;
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn word(value: &Value) -> u32 {
    value.as_u64().unwrap().try_into().unwrap()
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
macro_rules! cases {
    ($name:ident, $rapier:ident, $dim:ident, $pose:path, $bits:path) => {
        mod $name {
            use super::*;
            use $rapier::prelude::*;
            fn rh(h: Handle) -> RigidBodyHandle {
                RigidBodyHandle::from_raw_parts(h.slot, h.generation)
            }
            fn ch(h: Handle) -> ColliderHandle {
                ColliderHandle::from_raw_parts(h.slot, h.generation)
            }
            fn handle((slot, generation): (u32, u32)) -> Handle {
                Handle { slot, generation }
            }
            fn observe(
                b: &RigidBodySet,
                c: &ColliderSet,
            ) -> (ValidatedArena, ValidatedArena, AttachmentRecord) {
                let identity = Identity {
                    dimension: Dimension::$dim,
                    build: [43; 32],
                };
                let (ba, bh) = layout(&serde_json::to_value(b).unwrap()["bodies"], identity);
                let (ca, cc) = layout(&serde_json::to_value(c).unwrap()["colliders"], identity);
                let mut record = AttachmentRecord {
                    identity,
                    bodies: vec![],
                    colliders: vec![],
                    attached: vec![],
                };
                for h in bh {
                    let list = b[rh(h)].colliders();
                    record.bodies.push(AttachmentSpan {
                        start: record.attached.len() as u32,
                        count: list.len() as u32,
                    });
                    record
                        .attached
                        .extend(list.iter().map(|h| handle(h.into_raw_parts())));
                }
                for h in cc {
                    let co = &c[ch(h)];
                    record
                        .colliders
                        .push(co.parent().map(|body| ParentAttachment {
                            body: handle(body.into_raw_parts()),
                            local_pose: $bits(co.position_wrt_parent().unwrap()),
                        }));
                }
                (ba, ca, record)
            }
            fn check(b: &RigidBodySet, c: &ColliderSet) {
                let (ba, ca, record) = observe(b, c);
                let checked =
                    validate_attachments(record.clone(), &ba, &ca, &mut budget()).unwrap();
                let bytes = encode_attachments(&checked, &mut budget()).unwrap();
                let loaded = decode_attachments(&bytes, &ba, &ca, &mut budget()).unwrap();
                assert_eq!(loaded.record(), &record);
                assert_eq!(encode_attachments(&loaded, &mut budget()).unwrap(), bytes);
                for (h, body) in b.iter() {
                    assert_eq!(
                        loaded.body_colliders(handle(h.into_raw_parts())).unwrap(),
                        body.colliders()
                            .iter()
                            .map(|v| handle(v.into_raw_parts()))
                            .collect::<Vec<_>>()
                    );
                }
                for (h, co) in c.iter() {
                    let row = loaded.collider_parent(handle(h.into_raw_parts())).unwrap();
                    assert_eq!(
                        row.map(|p| p.body),
                        co.parent().map(|p| handle(p.into_raw_parts()))
                    );
                    assert_eq!(
                        row.map(|p| p.local_pose),
                        co.position_wrt_parent().map($bits)
                    );
                }
            }
            #[test]
            fn native_attach_reparent_detach_remove_and_reuse_order() {
                let mut b = RigidBodySet::new();
                let mut c = ColliderSet::new();
                let mut islands = IslandManager::new();
                let mut soft = SoftBodySet::new();
                let mut joints = ImpulseJointSet::new();
                let mut multi = MultibodyJointSet::new();
                check(&b, &c);
                let body = b.insert(RigidBodyBuilder::dynamic().sleeping(true).pose($pose(1)));
                let disabled = b.insert(RigidBodyBuilder::fixed().enabled(false).pose($pose(2)));
                let third = b.insert(RigidBodyBuilder::kinematic_position_based());
                let mut handles = vec![];
                for i in 0..5 {
                    handles.push(
                        c.insert_with_parent(
                            ColliderBuilder::ball(0.5)
                                .enabled(i % 2 == 0)
                                .position($pose(i)),
                            body,
                            &mut b,
                        ),
                    );
                    check(&b, &c);
                }
                let standalone = c.insert(ColliderBuilder::ball(0.25).position($pose(4)));
                assert_eq!(c[standalone].parent(), None);
                check(&b, &c);
                let removed = handles[1];
                c.remove(removed, &mut islands, &mut b, &mut soft, false)
                    .unwrap();
                assert_eq!(
                    b[body].colliders(),
                    &[handles[0], handles[4], handles[2], handles[3]]
                );
                check(&b, &c);
                let reused = c.insert_with_parent(
                    ColliderBuilder::ball(0.4).position($pose(3)),
                    disabled,
                    &mut b,
                );
                assert_eq!(reused.into_raw_parts().0, removed.into_raw_parts().0);
                assert_ne!(reused.into_raw_parts().1, removed.into_raw_parts().1);
                check(&b, &c);
                // Existing local pose survives a direct reparent; detaching and
                // later attaching a standalone collider instead creates identity.
                let original = *c[handles[0]].position_wrt_parent().unwrap();
                c.set_parent(handles[0], Some(disabled), &mut b);
                assert_eq!(
                    $bits(c[handles[0]].position_wrt_parent().unwrap()),
                    $bits(&original)
                );
                check(&b, &c);
                c.set_parent(handles[2], None, &mut b);
                check(&b, &c);
                c.set_parent(handles[2], Some(third), &mut b);
                assert_eq!(
                    $bits(c[handles[2]].position_wrt_parent().unwrap()),
                    $bits(&$rapier::math::Pose::IDENTITY)
                );
                c[handles[2]].set_position_wrt_parent($pose(0));
                check(&b, &c);
                c.set_parent(standalone, Some(body), &mut b);
                check(&b, &c);
                b.remove(
                    body,
                    &mut islands,
                    &mut c,
                    &mut joints,
                    &mut multi,
                    &mut soft,
                    false,
                )
                .unwrap();
                assert!(c[standalone].parent().is_none());
                check(&b, &c);
                let new_body = b.insert(RigidBodyBuilder::dynamic());
                assert_eq!(new_body.into_raw_parts().0, body.into_raw_parts().0);
                assert_ne!(new_body.into_raw_parts().1, body.into_raw_parts().1);
                c.set_parent(standalone, Some(new_body), &mut b);
                check(&b, &c);
                b.remove(
                    disabled,
                    &mut islands,
                    &mut c,
                    &mut joints,
                    &mut multi,
                    &mut soft,
                    true,
                )
                .unwrap();
                assert!(c.get(reused).is_none());
                assert!(c.get(handles[0]).is_none());
                check(&b, &c);
                let (ba, ca, record) = observe(&b, &c);
                let checked = validate_attachments(record, &ba, &ca, &mut budget()).unwrap();
                assert_eq!(checked.body_colliders(handle(body.into_raw_parts())), None);
                assert_eq!(
                    checked.collider_parent(handle(removed.into_raw_parts())),
                    None
                );
                for h in b.iter().map(|(h, _)| h).collect::<Vec<_>>() {
                    b.remove(
                        h,
                        &mut islands,
                        &mut c,
                        &mut joints,
                        &mut multi,
                        &mut soft,
                        true,
                    )
                    .unwrap();
                    check(&b, &c);
                }
            }
            #[test]
            fn unchecked_native_missing_parent_and_nonfinite_pose_are_refused() {
                let mut b = RigidBodySet::new();
                let mut c = ColliderSet::new();
                let body = b.insert(RigidBodyBuilder::fixed());
                let collider = c.insert_with_parent(ColliderBuilder::ball(0.5), body, &mut b);
                let mut invalid = $pose(0);
                invalid.translation.x = f32::NAN;
                c[collider].set_position_wrt_parent(invalid);
                let (ba, ca, r) = observe(&b, &c);
                assert_eq!(
                    validate_attachments(r, &ba, &ca, &mut budget()).err(),
                    Some(Error::Geometry)
                );
                c[collider].set_position_wrt_parent($pose(0));
                c.set_parent(
                    collider,
                    Some(RigidBodyHandle::from_raw_parts(42, 7)),
                    &mut b,
                );
                assert_eq!(c[collider].parent().unwrap().into_raw_parts(), (42, 7));
                let (ba, ca, r) = observe(&b, &c);
                assert_eq!(
                    validate_attachments(r, &ba, &ca, &mut budget()).err(),
                    Some(Error::Generation)
                );
            }
        }
    };
}
fn planar_pose(index: u32) -> rapier2d::math::Pose {
    use rapier2d::math::*;
    Pose {
        translation: Vector::new(-0., f32::from_bits(index)),
        rotation: if index == 0 {
            Rotation::from_cos_sin_unchecked(-1., -0.)
        } else {
            Rotation::new(index as f32 * 0.37)
        },
    }
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
fn spatial_pose(index: u32) -> rapier3d::math::Pose {
    use rapier3d::math::*;
    Pose {
        translation: Vector::new(-0., f32::from_bits(index), 3.25),
        rotation: if index == 0 {
            Rotation::from_xyzw(-0., 0., 0., -1.)
        } else {
            Rotation::from_rotation_y(index as f32 * 0.37)
        },
        ..Pose::IDENTITY
    }
}
fn spatial_bits(pose: &rapier3d::math::Pose) -> LocalPose {
    LocalPose {
        translation: pose.translation.to_array().map(f32::to_bits),
        rotation: pose.rotation.to_array().map(f32::to_bits),
    }
}
cases!(planar, rapier2d, Planar, planar_pose, planar_bits);
cases!(spatial, rapier3d, Spatial, spatial_pose, spatial_bits);

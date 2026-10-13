//! Actual live body observations -> inert codec -> private exact fields.
//! Other owners below are trusted typed clones; this is not full-world restore.
use super::*;
use crate::alloc_prelude::*;
use crate::geometry::{Collider, ColliderBuilder, ColliderSet};
use crate::math::Vector;
use crate::pipeline::checked_restore::collider as co_construct;
use crate::pipeline::{CollisionPipeline, PhysicsWorld};
use incant_physics_snapshot::*;
use std::eprintln;
#[path = "collider_observe.rs"]
mod collider_observe;
#[path = "native_observe.rs"]
mod native_observe;
#[path = "body_observe.rs"]
mod observe;
fn id() -> Identity {
    Identity {
        dimension: if crate::math::DIM == 2 {
            Dimension::Planar
        } else {
            Dimension::Spatial
        },
        build: [27; 32],
    }
}
fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
}
fn handle(h: (u32, u32)) -> Handle {
    Handle {
        slot: h.0,
        generation: h.1,
    }
}
fn layout(value: &serde_json::Value) -> ValidatedArena {
    let mut row = 0;
    let slots = value["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            if let Some(f) = item.get("Free") {
                ArenaSlot::Free {
                    next: f["next_free"].as_u64().map(|n| n as u32),
                }
            } else {
                let generation = item["Occupied"]["generation"].as_u64().unwrap() as u32;
                let r = row;
                row += 1;
                ArenaSlot::Occupied { generation, row: r }
            }
        })
        .collect();
    validate_arena(
        ArenaRecord {
            identity: id(),
            generation: value["generation"].as_u64().unwrap() as u32,
            free_head: value["free_list_head"].as_u64().map(|n| n as u32),
            live: row,
            slots,
        },
        &mut budget(),
    )
    .unwrap()
}
fn observed(b: &RigidBody) -> RigidBodyState {
    observe::body(&native_observe::capture(b), id().dimension)
}
fn reconstructed(w: &PhysicsWorld) -> (RigidBodySet, ColliderSet) {
    let ba = layout(&native_observe::capture(&w.bodies)["bodies"]);
    let ca = layout(&native_observe::capture(&w.colliders)["colliders"]);
    let mut record = AttachmentRecord {
        identity: id(),
        bodies: vec![],
        colliders: vec![],
        attached: vec![],
    };
    for (_, b) in w.bodies.iter() {
        record.bodies.push(AttachmentSpan {
            start: record.attached.len() as u32,
            count: b.colliders().len() as u32,
        });
        record
            .attached
            .extend(b.colliders().iter().map(|h| handle(h.into_raw_parts())));
    }
    for (_, c) in w.colliders.iter() {
        record
            .colliders
            .push(c.parent.as_ref().map(|p| ParentAttachment {
                body: handle(p.handle.into_raw_parts()),
                local_pose: pose_bits(p.pos_wrt_parent),
            }));
    }
    let attachments = validate_attachments(record, &ba, &ca, &mut budget()).unwrap();
    let context = BodyContext {
        attachments: &attachments,
        journals: PendingJournals::default(),
    };
    let state = validate_body_state(
        BodyStateTable {
            identity: id(),
            bodies: w.bodies.iter().map(|(_, b)| observed(b)).collect(),
        },
        context,
    )
    .unwrap();
    let bytes = encode_body_state(&state, &mut budget()).unwrap();
    let decoded = decode_body_state(&bytes, context, &mut budget()).unwrap();
    let geometries: Vec<_> = w
        .colliders
        .iter()
        .map(|(_, c)| {
            let shape = match c.shape().as_typed_shape() {
                parry::shape::TypedShape::Ball(b) => Primitive::Ball {
                    radius: b.radius.to_bits(),
                },
                parry::shape::TypedShape::Cuboid(b) => Primitive::Box {
                    half_extents: bits(b.half_extents),
                },
                parry::shape::TypedShape::Capsule(b) => {
                    let h = b.segment.b.y;
                    assert_eq!(b.segment.a, -Vector::Y * h);
                    assert_eq!(b.segment.b, Vector::Y * h);
                    Primitive::CapsuleY {
                        half_height: h.to_bits(),
                        radius: b.radius.to_bits(),
                    }
                }
                _ => panic!("unsupported test geometry"),
            };
            let g = validate_geometry(GeometryRecord {
                identity: id(),
                kind: GeometryKind::Primitive,
                parts: vec![Part {
                    translation: [0; 3],
                    rotation: [0, 0, 0, 1f32.to_bits()],
                    shape,
                }],
            })
            .unwrap();
            decode_geometry(
                &encode_geometry(&g, &mut budget()).unwrap(),
                id(),
                &mut budget(),
            )
            .unwrap()
        })
        .collect();
    let context = ColliderContext {
        bodies: &decoded,
        geometries: &geometries,
        journals: PendingJournals::default(),
    };
    let cs = validate_collider_state(
        ColliderStateTable {
            identity: id(),
            colliders: w
                .colliders
                .iter()
                .map(|(_, c)| {
                    collider_observe::collider(&native_observe::capture(c), id().dimension)
                })
                .collect(),
        },
        context,
    )
    .unwrap();
    let bytes = encode_collider_state(&cs, &mut budget()).unwrap();
    let cs = decode_collider_state(&bytes, context, &mut budget()).unwrap();
    let mut plan = Arena::<RigidBody>::checkpoint_plan(&ba).unwrap();
    plan.append(Arena::<Collider>::checkpoint_plan(&ca).unwrap())
        .unwrap();
    for (c, g) in cs.record().colliders.iter().zip(&geometries) {
        plan.append(Collider::checkpoint_payload_plan(c).unwrap())
            .unwrap();
        plan.append(co_construct::shape_plan(g).unwrap()).unwrap();
    }

    for (row, b) in decoded.record().bodies.iter().enumerate() {
        plan.append(row_plan(b, attachments.record().bodies[row].count as usize).unwrap())
            .unwrap();
    }
    if plan.bytes() > 0 {
        assert!(
            AllocationBudget::new(plan.bytes() - 1, plan.allocations())
                .admit(plan)
                .is_err()
        );
    }
    let mut permit = AllocationBudget::new(plan.bytes(), plan.allocations())
        .admit(plan)
        .unwrap();
    let bodies = Arena::from_checkpoint(&ba, &mut permit, |row, p| {
        let span = attachments.record().bodies[row as usize];
        from_row(
            &decoded.record().bodies[row as usize],
            &attachments.record().attached[span.start as usize..(span.start + span.count) as usize],
            p,
        )
    })
    .unwrap();
    let colliders = Arena::from_checkpoint(&ca, &mut permit, |row, p| {
        let row = row as usize;
        Collider::from_checkpoint_fields(
            &cs.record().colliders[row],
            co_construct::shape(&geometries[row], p)?,
            attachments.record().colliders[row],
            p,
        )
    })
    .unwrap();
    permit.finish().unwrap();
    let colliders = ColliderSet {
        colliders,
        modified_colliders: Default::default(),
        removed_colliders: Default::default(),
    };
    assert_eq!(
        bincode::serialize(&w.colliders).unwrap(),
        bincode::serialize(&colliders).unwrap()
    );
    let result = RigidBodySet {
        bodies,
        modified_bodies: Default::default(),
        default_fixed: RigidBodyBuilder::fixed().build(),
    };
    assert_eq!(
        bincode::serialize(&w.bodies).unwrap(),
        bincode::serialize(&result).unwrap()
    );
    (result, colliders)
}
fn fresh(w: &mut PhysicsWorld) -> PhysicsWorld {
    let (bodies, colliders) = reconstructed(w);
    PhysicsWorld {
        gravity: w.gravity,
        integration_parameters: w.integration_parameters,
        physics_pipeline: w.physics_pipeline.checkpoint_fresh_workspace().unwrap(),
        collision_pipeline: CollisionPipeline::new(),
        islands: w.islands.clone(),
        broad_phase: w.broad_phase.clone(),
        narrow_phase: w.narrow_phase.clone(),
        bodies,
        colliders,
        impulse_joints: w.impulse_joints.clone(),
        multibody_joints: w.multibody_joints.clone(),
        soft_bodies: w.soft_bodies.clone(),
        ccd_solver: w.ccd_solver.clone(),
    }
}
#[test]
fn checked_fields_decoded_native_fields_and_reused_handles_continue() {
    for additional in 0..3 {
        let mut w = PhysicsWorld::default();
        w.insert_collider(
            ColliderBuilder::ball(4.).translation(Vector::Y * -4.5),
            None,
        );
        let removed = w
            .insert(
                RigidBodyBuilder::dynamic().translation(Vector::Y * 5.),
                ColliderBuilder::ball(0.1),
            )
            .0;
        let (h, _) = w.insert(
            RigidBodyBuilder::dynamic()
                .translation(Vector::Y * 0.25)
                .linear_damping(0.13)
                .angular_damping(0.17)
                .dominance_group(-7)
                .ccd_enabled(true),
            ColliderBuilder::ball(0.5),
        );
        w.remove_body(removed).unwrap();
        w.bodies[h].user_data = 0x1234_5678;
        w.insert_collider(
            ColliderBuilder::ball(0.2).translation(Vector::X * 0.25),
            Some(h),
        );
        let transient = w.insert_collider(ColliderBuilder::ball(0.1), Some(h));
        w.insert_collider(
            ColliderBuilder::ball(0.1).translation(-Vector::X * 0.25),
            Some(h),
        );
        w.colliders.remove(
            transient,
            &mut w.islands,
            &mut w.bodies,
            &mut w.soft_bodies,
            true,
        );
        match additional {
            1 => w.bodies[h].set_additional_mass(2., true),
            2 => w.bodies[h].set_additional_mass_properties(MassProperties::default(), true),
            _ => {}
        }
        w.bodies[h].set_gravity_scale(-0.0, true);
        w.bodies[h].add_force(Vector::X * 0.1, true);
        for _ in 0..8 {
            w.step();
        }
        // Exact u128 and signed-zero fields do not pass through JSON numbers.
        w.bodies.bodies.get_mut(h.0).unwrap().user_data = u128::MAX - 7;
        let mut restored = fresh(&mut w);
        for tick in 0..32 {
            for world in [&mut w, &mut restored] {
                match tick {
                    3 => world.bodies[h].sleep(),
                    5 => world.bodies[h].wake_up(true),
                    8 => world.bodies[h].set_body_type(RigidBodyType::KinematicVelocityBased, true),
                    12 => world.bodies[h].set_body_type(RigidBodyType::Dynamic, true),
                    16 => world.bodies[h].set_enabled(false),
                    18 => world.bodies[h].set_enabled(true),
                    20 => world.bodies[h].set_gravity_scale(1., true),
                    _ => {}
                }
                world.step();
            }
            assert_eq!(
                bincode::serialize(&w).unwrap(),
                bincode::serialize(&restored).unwrap(),
                "additional={additional} tick={tick}"
            );
            assert_eq!(
                w.physics_pipeline.checkpoint_observation().unwrap(),
                restored.physics_pipeline.checkpoint_observation().unwrap()
            );
            assert_eq!(w.bodies[h].user_data, u128::MAX - 7);
        }
    }
    eprintln!(
        "CHECKED_FIELDS dimension={} actual_steps=96 other_owners=trusted_clones exact_bincode=true",
        crate::math::DIM
    );
}
#[test]
fn checked_fields_bits_and_generation_failure_do_not_publish() {
    let mut w = PhysicsWorld::default();
    let (h, _) = w.insert(
        RigidBodyBuilder::dynamic().can_sleep(false),
        ColliderBuilder::ball(0.5),
    );
    w.bodies[h].set_gravity_scale(-0.0, true);
    w.step();
    let b = observed(&w.bodies[h]);
    assert_eq!(b.forces.gravity_scale, (-0.0f32).to_bits());
    assert!(f32::from_bits(b.activation.linear_threshold) < 0.);
    let before = bincode::serialize(&w.bodies).unwrap();
    let _ = reconstructed(&w);
    assert_eq!(before, bincode::serialize(&w.bodies).unwrap());
    let plan = row_plan(&b, w.bodies[h].colliders().len()).unwrap();
    let mut p = AllocationBudget::new(0, 0)
        .admit(AllocationPlan::default())
        .unwrap();
    assert!(matches!(
        from_row(
            &b,
            &[handle(w.bodies[h].colliders()[0].into_raw_parts())],
            &mut p
        ),
        Err(ConstructionError::Limit)
    ));
    assert!(plan.bytes() > 0);
    assert_eq!(before, bincode::serialize(&w.bodies).unwrap());
}
#[test]
fn checked_fields_primitive_material_mass_and_disabled_parent_state_survive() {
    use crate::geometry::{
        ColliderEnabled as NativeEnabled, Group, InteractionGroups, InteractionTestMode,
        SharedShape,
    };
    use crate::pipeline::ActiveEvents;
    for kind in 0..3 {
        let mut w = PhysicsWorld::default();
        w.gravity = Vector::ZERO;
        let (h, c) = w.insert(
            RigidBodyBuilder::dynamic().translation(Vector::Y * 2.),
            ColliderBuilder::ball(0.5)
                .friction(0.7)
                .restitution(0.23)
                .friction_combine_rule(CoefficientCombineRule::Min)
                .restitution_combine_rule(CoefficientCombineRule::Max)
                .collision_groups(InteractionGroups::new(
                    Group::GROUP_2,
                    Group::GROUP_3,
                    InteractionTestMode::Or,
                ))
                .solver_groups(InteractionGroups::new(
                    Group::GROUP_3,
                    Group::GROUP_2,
                    InteractionTestMode::And,
                ))
                .active_events(ActiveEvents::COLLISION_EVENTS | ActiveEvents::CONTACT_FORCE_EVENTS)
                .contact_force_event_threshold(f32::MAX)
                .contact_skin(0.03),
        );
        match kind {
            1 => {
                w.colliders[c].set_shape(SharedShape::new(parry::shape::Cuboid::new(
                    Vector::splat(0.3),
                )));
                w.colliders[c].set_mass(2.);
            }
            2 => {
                w.colliders[c].set_shape(SharedShape::capsule_y(0.25, 0.3));
                w.colliders[c].set_mass_properties(MassProperties::default());
            }
            _ => w.colliders[c].set_density(-0.0),
        }
        w.step();
        w.bodies[h].set_enabled(false);
        w.step();
        assert_eq!(
            w.colliders[c].flags.enabled,
            NativeEnabled::DisabledByParent
        );
        w.colliders.colliders.get_mut(c.0).unwrap().user_data = u128::MAX - 3;
        let mut restored = fresh(&mut w);
        assert_eq!(
            restored.colliders[c].flags.enabled,
            NativeEnabled::DisabledByParent
        );
        for tick in 0..16 {
            for world in [&mut w, &mut restored] {
                match tick {
                    1 => world.colliders.set_parent(c, None, &mut world.bodies),
                    3 => world.colliders[c].set_enabled(true),
                    5 => world.colliders[c].set_sensor(true),
                    7 => {
                        world.bodies[h].set_enabled(true);
                        world.colliders.set_parent(c, Some(h), &mut world.bodies);
                    }
                    9 => world.colliders[c].set_friction(-0.0),
                    11 => world.colliders[c].set_enabled(false),
                    13 => world.colliders[c].set_enabled(true),
                    _ => {}
                }
                world.step();
            }
            assert_eq!(
                bincode::serialize(&w).unwrap(),
                bincode::serialize(&restored).unwrap(),
                "shape={kind} tick={tick}"
            );
            assert_eq!(w.colliders[c].user_data, u128::MAX - 3);
        }
    }
    eprintln!(
        "CHECKED_FIELDS dimension={} primitive_steps=48 disabled_parent=true detach_reparent=true materials=true exact_bincode=true other_owners=trusted_clones",
        crate::math::DIM
    );
}

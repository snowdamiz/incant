use super::*;
use crate::dynamics::{FixedJointBuilder, SoftBodyBuilder};
use crate::math::Pose;
use parry::shape::SharedShape;
fn load(o: &mut CheckpointOwner) -> CheckpointOwner {
    let f = capture(o);
    let bytes = f.persisted_bytes(&mut payload(), &mut work()).unwrap();
    let mut loaded =
        CheckpointOwner::from_persisted(&bytes, &mut payload(), &mut work(), &mut allocation())
            .unwrap();
    assert_eq!(
        capture(&mut loaded)
            .persisted_bytes(&mut payload(), &mut work())
            .unwrap(),
        bytes
    );
    loaded
}
#[test]
fn persisted_native_incant_default_enabled_boxes_continue() {
    let mut original = boxes_with_ccd(true);
    assert!(
        original
            .get()
            .unwrap()
            .bodies
            .iter()
            .all(|(_, b)| b.is_ccd_enabled())
    );
    let mut loaded = load(&mut original);
    for tick in 0..32 {
        original.step().unwrap();
        loaded.step().unwrap();
        assert_eq!(
            capture(&mut original).encoded(),
            capture(&mut loaded).encoded(),
            "enabled tick {tick}"
        );
    }
}
fn fast(shape: u8, bullet: bool, fixed_count: usize) -> CheckpointOwner {
    let mut owner = CheckpointOwner::new().unwrap();
    let w = owner.access_mut().unwrap();
    w.gravity = Vector::ZERO;
    #[cfg(feature = "dim2")]
    let wall = SharedShape::cuboid(0.1, 2.);
    #[cfg(feature = "dim3")]
    let wall = SharedShape::cuboid(0.1, 2., 2.);
    w.insert_collider(ColliderBuilder::new(wall.clone()), None);
    for i in 1..fixed_count {
        w.insert_collider(
            ColliderBuilder::new(wall.clone())
                .translation(Vector::X * (10. + i as f32) + Vector::Y * 10.),
            None,
        );
    }
    #[cfg(feature = "dim2")]
    let box_ = SharedShape::cuboid(0.1, 0.1);
    #[cfg(feature = "dim3")]
    let box_ = SharedShape::cuboid(0.1, 0.1, 0.1);
    let shape = match shape {
        0 => SharedShape::ball(0.1),
        1 => box_.clone(),
        2 => SharedShape::capsule_y(0.1, 0.1),
        3 => SharedShape::compound(alloc::vec![
            (Pose::from_translation(Vector::Y * 0.2), box_.clone()),
            (Pose::from_translation(Vector::Y * 0.5), box_),
        ]),
        _ => unreachable!(),
    };
    w.insert(
        RigidBodyBuilder::dynamic()
            .translation(-Vector::X)
            .linvel(Vector::X * 120.)
            .ccd_enabled(bullet),
        ColliderBuilder::new(shape),
    );
    owner.step().unwrap();
    owner
}
#[test]
fn persisted_native_actual_implicit_and_bullet_clamps_preserve_cache() {
    for bullet in [false, true] {
        for shape in 0..4 {
            let mut original = fast(shape, bullet, 1);
            let w = original.get().unwrap();
            let (_, body) = w.bodies.iter().next().unwrap();
            assert!(
                body.is_ccd_active(),
                "real automatic activation shape{shape} bullet{bullet}"
            );
            assert!(
                body.translation().x < 0.,
                "actual clamp must stop before wall"
            );
            assert_eq!(
                w.ccd_solver
                    .checkpoint_fixed_targets()
                    .unwrap()
                    .1
                    .unwrap()
                    .len(),
                1
            );
            let f = capture(&mut original);
            incant_physics_snapshot::with_decoded_world_boundary(
                &f.encoded()[16..],
                f.identity(),
                &mut payload(),
                |v| {
                    assert_eq!(v.pipeline().record().post_solve_ccd_active, Some(true));
                    Ok(())
                },
            )
            .unwrap();
            let mut loaded = load(&mut original);
            for tick in 0..8 {
                original.step().unwrap();
                loaded.step().unwrap();
                assert_eq!(
                    capture(&mut original).encoded(),
                    capture(&mut loaded).encoded(),
                    "clamp shape{shape} bullet{bullet} tick{tick}"
                );
            }
        }
    }
}
#[test]
fn persisted_native_actual_full_bvh_cache_and_empty_list() {
    let mut original = fast(0, false, 513);
    assert!(
        original
            .get()
            .unwrap()
            .ccd_solver
            .checkpoint_fixed_targets()
            .unwrap()
            .1
            .is_none()
    );
    let mut loaded = load(&mut original);
    original.step().unwrap();
    loaded.step().unwrap();
    assert_eq!(
        capture(&mut original).encoded(),
        capture(&mut loaded).encoded()
    );
    let mut original = CheckpointOwner::new().unwrap();
    original.access_mut().unwrap().insert(
        RigidBodyBuilder::dynamic()
            .linvel(Vector::X * 120.)
            .ccd_enabled(true),
        ColliderBuilder::ball(0.1),
    );
    original.step().unwrap();
    assert!(
        original
            .get()
            .unwrap()
            .ccd_solver
            .checkpoint_fixed_targets()
            .unwrap()
            .1
            .unwrap()
            .is_empty()
    );
    let mut loaded = load(&mut original);
    original.step().unwrap();
    loaded.step().unwrap();
    assert_eq!(
        capture(&mut original).encoded(),
        capture(&mut loaded).encoded()
    );
}
#[test]
fn persisted_native_ccd_configuration_and_history_refuse_before_advancing() {
    use super::super::super::world::OmittedOwnerError as F;
    for family in 0..3 {
        for remove in [false, true] {
            let mut source = boxes();
            let mut loaded = load(&mut source);
            let w = loaded.access_mut().unwrap();
            let handles: alloc::vec::Vec<_> = w.bodies.iter().map(|(h, _)| h).collect();
            let reason = match family {
                0 => {
                    let j = w.impulse_joints.insert(
                        handles[0],
                        handles[1],
                        FixedJointBuilder::new(),
                        false,
                    );
                    if remove {
                        w.impulse_joints.remove(j, false).unwrap();
                        assert!(w.impulse_joints.is_empty());
                    }
                    F::ImpulseArena
                }
                1 => {
                    let j = w
                        .multibody_joints
                        .insert(handles[0], handles[1], FixedJointBuilder::new(), false)
                        .unwrap();
                    if remove {
                        w.multibody_joints.remove(j, false);
                        assert!(w.multibody_joints.multibodies().next().is_none());
                    }
                    F::MultibodyArena
                }
                _ => {
                    let h = w.insert_soft_body(SoftBodyBuilder::rope(Vector::ZERO, Vector::X, 3));
                    if remove {
                        w.remove_soft_body(h).unwrap();
                        assert!(w.soft_bodies.is_empty());
                    }
                    F::SoftArena
                }
            };
            let before = serde_json::to_vec(w).unwrap();
            assert_eq!(
                loaded.step(),
                Err(CaptureError::ImportedDomain(DomainError::Family(reason)))
            );
            assert_eq!(serde_json::to_vec(loaded.get().unwrap()).unwrap(), before);
        }
    }
    for change in 0..3 {
        let mut source = boxes();
        let mut loaded = load(&mut source);
        let w = loaded.access_mut().unwrap();
        match change {
            0 => w.integration_parameters.max_ccd_substeps = 2,
            1 => w.integration_parameters.normalized_allowed_linear_error = 0.,
            _ => {
                w.bodies
                    .iter_mut()
                    .next()
                    .unwrap()
                    .1
                    .ccd
                    .allow_fast_rotation = true
            }
        }
        let before = serde_json::to_vec(w).unwrap();
        assert_eq!(
            loaded.step(),
            Err(CaptureError::ImportedDomain(DomainError::Ccd))
        );
        assert_eq!(serde_json::to_vec(loaded.get().unwrap()).unwrap(), before);
    }
}

#[test]
fn persisted_native_pending_change_flags_are_not_disabled_ancestors() {
    let mut original = CheckpointOwner::new().unwrap();
    let w = original.access_mut().unwrap();
    w.integration_parameters.dt = 1. / 64.;
    #[cfg(feature = "dim2")]
    let floor = ColliderBuilder::cuboid(4., 0.5);
    #[cfg(feature = "dim3")]
    let floor = ColliderBuilder::cuboid(4., 0.5, 4.);
    w.insert_collider(floor.translation(-Vector::Y * 0.5), None);
    let (body, _) = w.insert(
        RigidBodyBuilder::dynamic().translation(Vector::Y),
        ColliderBuilder::ball(0.4),
    );
    w.insert_collider(
        ColliderBuilder::ball(1.)
            .sensor(true)
            .translation(Vector::Y * 0.5),
        None,
    );
    for _ in 0..64 {
        original.step().unwrap();
    }
    original.access_mut().unwrap().bodies[body].set_linvel(Vector::X * 0.25, true);
    let mut observed = false;
    for _ in 0..48 {
        original.step().unwrap();
        let f = capture(&mut original);
        incant_physics_snapshot::with_decoded_world_boundary(
            &f.encoded()[16..],
            f.identity(),
            &mut payload(),
            |v| {
                let tree = v.broad().context().tree.record();
                observed |= tree.nodes.iter().flat_map(|w| [w.left, w.right]).any(|n| {
                    if n.leaf_count() <= 1 {
                        return false;
                    }
                    let w = &tree.nodes[n.children as usize];
                    n.flags() == 0 && [w.left, w.right].iter().any(|n| n.flags() == 3)
                });
                Ok(())
            },
        )
        .unwrap();
        let bytes = f.persisted_bytes(&mut payload(), &mut work()).unwrap();
        let mut loaded =
            CheckpointOwner::from_persisted(&bytes, &mut payload(), &mut work(), &mut allocation())
                .unwrap();
        assert_eq!(capture(&mut loaded).encoded(), bytes);
    }
    assert!(observed, "actual pending leaf below unchanged ancestor");
}

#[test]
fn persisted_native_far_fixed_ccd_target_is_in_arithmetic_envelope() {
    let mut owner = CheckpointOwner::new().unwrap();
    let w = owner.access_mut().unwrap();
    w.gravity = Vector::ZERO;
    w.insert_collider(
        ColliderBuilder::ball(0.1).translation(Vector::X * 1.0e8),
        None,
    );
    w.insert(
        RigidBodyBuilder::dynamic().ccd_enabled(true),
        ColliderBuilder::ball(0.1),
    );
    owner.step().unwrap();
    let frame = capture(&mut owner);
    assert_eq!(
        frame.persisted_bytes(&mut payload(), &mut work()),
        Err(PersistedError::Domain(DomainError::Arithmetic))
    );
    assert!(matches!(
        CheckpointOwner::from_persisted(
            frame.encoded(),
            &mut payload(),
            &mut work(),
            &mut allocation()
        ),
        Err(PersistedError::Domain(DomainError::Arithmetic))
    ));
}

#[test]
fn persisted_native_hostile_ccd_configuration_has_no_hidden_interval_or_radius() {
    use incant_physics_snapshot::{
        encode_integration_config, validate_integration_config, with_decoded_world_boundary,
    };
    let mut owner = boxes();
    let f = capture(&mut owner);
    for case in 0..3 {
        let config =
            with_decoded_world_boundary(&f.encoded()[16..], f.identity(), &mut payload(), |v| {
                let mut r = *v.pipeline().context().integration.record();
                match case {
                    0 => r.max_ccd_substeps = 2,
                    1 => r.normalized_allowed_linear_error = 0,
                    _ => r.normalized_allowed_linear_error = 1.0e8f32.to_bits(),
                }
                let r = validate_integration_config(r)?;
                encode_integration_config(&r, &mut payload())
            })
            .unwrap();
        let bytes = adversarial::section(f.encoded(), 19, &config);
        let expected = if case == 2 {
            DomainError::Arithmetic
        } else {
            DomainError::Ccd
        };
        assert!(
            matches!(CheckpointOwner::from_persisted(&bytes, &mut payload(), &mut work(), &mut allocation()),
            Err(PersistedError::Domain(e)) if e == expected)
        );
    }
    assert_eq!(capture(&mut owner).encoded(), f.encoded());
}

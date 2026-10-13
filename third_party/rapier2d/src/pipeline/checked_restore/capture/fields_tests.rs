use super::*;
use crate::alloc_prelude::*;
use crate::dynamics::{LockedAxes, RigidBodyBuilder};
use crate::geometry::{ColliderBuilder, SharedShape};
use crate::math::{Pose, Vector};
use crate::pipeline::PhysicsWorld;
#[path = "../collider_observe.rs"]
mod collider_observe;
#[path = "../native_observe.rs"]
mod native_observe;
#[path = "../body_observe.rs"]
mod observe;

fn id() -> s::Identity {
    s::Identity {
        dimension: if crate::math::DIM == 2 {
            s::Dimension::Planar
        } else {
            s::Dimension::Spatial
        },
        build: [39; 32],
    }
}
fn budget() -> s::Budget {
    s::Budget::new(s::MAX_ALLOCATION).unwrap()
}
fn body_matches(b: &RigidBody) {
    assert_eq!(
        body(b).unwrap(),
        observe::body(&native_observe::capture(b), id().dimension)
    );
}
fn collider_matches(c: &Collider) {
    assert_eq!(
        collider(c).unwrap(),
        collider_observe::collider(&native_observe::capture(c), id().dimension)
    );
}

#[test]
fn direct_fields_match_independent_native_observation_across_completed_steps() {
    let mut w = PhysicsWorld::default();
    w.colliders.insert(
        ColliderBuilder::ball(1.0)
            .translation(-Vector::Y)
            .friction(0.63)
            .build(),
    );
    for (i, kind) in [
        RigidBodyType::Dynamic,
        RigidBodyType::Fixed,
        RigidBodyType::KinematicPositionBased,
        RigidBodyType::KinematicVelocityBased,
    ]
    .into_iter()
    .enumerate()
    {
        let b = w.bodies.insert(
            RigidBodyBuilder::new(kind)
                .translation(Vector::Y * (1.3 + i as f32))
                .linear_damping(0.03)
                .angular_damping(0.07)
                .gravity_scale(0.8)
                .dominance_group(-3)
                .additional_solver_iterations(2)
                .can_sleep(false)
                .additional_mass(0.37)
                .ccd_enabled(true)
                .soft_ccd_prediction(0.05)
                .user_data(u128::MAX - i as u128)
                .build(),
        );
        w.colliders.insert_with_parent(
            ColliderBuilder::capsule_y(0.35, 0.3)
                .restitution(0.15)
                .contact_skin(0.002)
                .contact_force_event_threshold(1.25)
                .user_data(0x9876543210fedcba123456789abcdef0)
                .build(),
            b,
            &mut w.bodies,
        );
    }
    for _ in 0..64 {
        w.step();
        for (_, b) in w.bodies.iter() {
            body_matches(b);
        }
        for (_, c) in w.colliders.iter() {
            collider_matches(c);
        }
    }
}

#[test]
fn direct_fields_preserve_nondefault_variants_and_raw_float_bits() {
    let mut b = RigidBodyBuilder::dynamic().build();
    b.mprops.additional_local_mprops = Some(Box::new(RigidBodyAdditionalMassProps::MassProps(
        b.mprops.local_mprops,
    )));
    b.mprops.flags = LockedAxes::all();
    b.ids.active_island_id = 17;
    b.ids.active_set_id = 23;
    b.ids.island_id = 31;
    b.ids.island_index = 41;
    b.additional_pgs_iterations = 7;
    b.soft_cluster = 12;
    b.soft_motion_margin = 0.37;
    b.ccd.ccd_active = true;
    b.ccd.allow_fast_rotation = true;
    b.enabled = false;
    b.activation.sleeping = true;
    b.activation.time_since_can_sleep = 1.37;
    b.activation.normalized_linear_threshold = -1.0;
    b.forces.force = Vector::splat(-0.0);
    b.forces.user_force = Vector::splat(0.125);
    b.vels.linvel = Vector::splat(-0.0);
    b.ccd_vels.linvel = Vector::splat(0.75);
    b.damping.linear_damping = -0.0;
    body_matches(&b);
    assert_eq!(body(&b).unwrap().velocity.linear[0], (-0.0f32).to_bits());
    // No production float-to-JSON conversion: even invalid input is captured
    // exactly for the joined validator to refuse, never silently normalized.
    b.damping.linear_damping = f32::from_bits(0x7fc01234);
    assert_eq!(body(&b).unwrap().damping[0], 0x7fc01234);

    let mut c = ColliderBuilder::ball(0.5)
        .sensor(true)
        .contact_skin(-0.0)
        .build();
    c.mprops = native::ColliderMassProps::MassProperties(Box::new(c.mass_properties()));
    c.flags.collision_groups.test_mode = native::InteractionTestMode::Or;
    c.flags.collision_groups.memberships = native::Group::GROUP_3;
    c.flags.solver_groups.filter = native::Group::GROUP_7;
    c.flags.active_collision_types = native::ActiveCollisionTypes::all();
    c.flags.active_hooks = crate::pipeline::ActiveHooks::all();
    c.flags.active_events = crate::pipeline::ActiveEvents::all();
    for enabled in [
        native::ColliderEnabled::Enabled,
        native::ColliderEnabled::Disabled,
        native::ColliderEnabled::DisabledByParent,
    ] {
        c.flags.enabled = enabled;
        for r in [
            CoefficientCombineRule::Average,
            CoefficientCombineRule::Min,
            CoefficientCombineRule::Multiply,
            CoefficientCombineRule::Max,
        ] {
            c.material.friction_combine_rule = r;
            c.material.restitution_combine_rule = r;
            collider_matches(&c);
        }
    }
    c.mprops = native::ColliderMassProps::Mass(0.75);
    collider_matches(&c);
    assert_eq!(collider(&c).unwrap().contact_skin, (-0.0f32).to_bits());
}

#[test]
fn primitive_capture_is_exact_and_charged_before_allocation() {
    let shapes = [
        SharedShape::ball(0.4),
        SharedShape::new(parry::shape::Cuboid::new(Vector::splat(0.5))),
        SharedShape::capsule_y(0.35, 0.2),
        SharedShape::capsule_y(0.0, 0.2),
    ];
    for shape in shapes {
        let c = ColliderBuilder::new(shape).build();
        let mut b = budget();
        let before = b.remaining();
        let captured = geometry(&c, id(), &mut b).unwrap();
        assert_eq!(before - b.remaining(), core::mem::size_of::<s::Part>());
        let part = captured.record().parts[0].shape;
        match (part, c.shape().as_typed_shape()) {
            (s::Primitive::Ball { radius }, parry::shape::TypedShape::Ball(v)) => {
                assert_eq!(radius, v.radius.to_bits())
            }
            (s::Primitive::Box { half_extents }, parry::shape::TypedShape::Cuboid(v)) => {
                assert_eq!(half_extents, bits(v.half_extents))
            }
            (
                s::Primitive::CapsuleY {
                    half_height,
                    radius,
                },
                parry::shape::TypedShape::Capsule(v),
            ) => {
                let rebuilt = parry::shape::Capsule::new_y(
                    f32::from_bits(half_height),
                    f32::from_bits(radius),
                );
                assert_eq!(bits(rebuilt.segment.a), bits(v.segment.a));
                assert_eq!(bits(rebuilt.segment.b), bits(v.segment.b));
            }
            _ => panic!("shape changed during capture"),
        }
        let mut small = s::Budget::new(core::mem::size_of::<s::Part>() - 1).unwrap();
        assert!(matches!(
            geometry(&c, id(), &mut small),
            Err(s::Error::Limit)
        ));
    }
}

#[test]
fn unsupported_or_noncanonical_geometry_is_refused() {
    let mut noncanonical = parry::shape::Capsule::new_y(0.5, 0.2);
    noncanonical.segment.a.x = 0.0; // canonical factory has negative zero
    let shapes = [
        SharedShape::capsule_x(0.5, 0.2),
        SharedShape::compound(vec![(Pose::IDENTITY, SharedShape::capsule_x(0.5, 0.2))]),
        SharedShape::new(noncanonical),
        SharedShape::ball(f32::NAN),
    ];
    for shape in shapes {
        assert!(matches!(
            geometry(&ColliderBuilder::new(shape).build(), id(), &mut budget()),
            Err(s::Error::Geometry)
        ));
    }
    let mut other = id();
    other.dimension = if other.dimension == s::Dimension::Planar {
        s::Dimension::Spatial
    } else {
        s::Dimension::Planar
    };
    assert!(matches!(
        geometry(&ColliderBuilder::ball(0.5).build(), other, &mut budget()),
        Err(s::Error::Geometry)
    ));
}

#[test]
fn unsupported_native_variants_return_typed_errors() {
    let mut b = RigidBodyBuilder::dynamic().build();
    b.body_type = RigidBodyType::SoftFrame;
    assert_eq!(body(&b), Err(s::Error::UnsupportedBody));
    let mut c = ColliderBuilder::ball(0.5).build();
    for r in [
        CoefficientCombineRule::ClampedSum,
        CoefficientCombineRule::GeometricMean,
    ] {
        c.material.friction_combine_rule = r;
        assert_eq!(collider(&c), Err(s::Error::UnsupportedCollider));
        c.material.friction_combine_rule = CoefficientCombineRule::Average;
        c.material.restitution_combine_rule = r;
        assert_eq!(collider(&c), Err(s::Error::UnsupportedCollider));
        c.material.restitution_combine_rule = CoefficientCombineRule::Average;
    }
}

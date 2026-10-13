//! Trusted one-way observations of actual pinned PhysicsWorld steps. No live
//! object is ever deserialized, and the validated records never reenter Rapier.
use incant_physics_snapshot::*;

fn budget() -> Budget {
    Budget::new(MAX_ALLOCATION).unwrap()
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

macro_rules! cases {
    ($name:ident, $rapier:ident, $parry:ident, $dim:ident, $vector:path, $cuboid:path, $bits:path, $data:path) => {
        mod $name {
            use super::*;
            use $rapier::prelude::*;
            fn capture(m: &$rapier::geometry::ContactManifold) -> ContactPointsRecord {
                ContactPointsRecord {
                    identity: Identity { dimension: Dimension::$dim, build: [82; 32] },
                    points: m.points.iter().map(|p| TrackedContactPoint {
                        local_p1: $bits(p.local_p1), local_p2: $bits(p.local_p2), distance: p.dist.to_bits(),
                        features: [p.fid1.0, p.fid2.0], data: $data(&p.data),
                    }).collect(),
                    solver: m.data.solver_contacts.iter().map(|s| {
                        // Padding is private in the pinned 3D type. One-way
                        // serialization observes it; it never constructs an object.
                        let value = serde_json::to_value(s).unwrap();
                        let padding = if Dimension::$dim == Dimension::Spatial {
                            let padding = value["padding"].as_array().unwrap();
                            assert_eq!(padding.len(), 1);
                            (padding[0].as_f64().unwrap() as f32).to_bits()
                        } else { assert!(value.get("padding").is_none()); 0 };
                        SolverContactPoint { anchor1: $bits(s.anchor1), anchor2: $bits(s.anchor2), distance: s.dist.to_bits(), tangent_velocity: $bits(s.tangent_velocity), contact_id: s.contact_id[0], padding }
                    }).collect(),
                }
            }
            fn check(r: ContactPointsRecord) -> ContactPointsRecord {
                let checked = validate_contact_points(r.clone(), &mut budget()).unwrap();
                let bytes = encode_contact_points(&checked, &mut budget()).unwrap();
                let decoded = decode_contact_points(&bytes, r.identity, &mut budget()).unwrap();
                assert_eq!(decoded.record(), &r);
                assert_eq!(encode_contact_points(&decoded, &mut budget()).unwrap(), bytes);
                for (i, solver) in r.solver.iter().enumerate() {
                    assert_eq!(decoded.solver_point(i), Some(&r.points[solver.point_index() as usize]));
                }
                r
            }
            fn observe(world: &PhysicsWorld) -> Vec<ContactPointsRecord> {
                world.contact_pairs().flat_map(|pair| {
                    let rigid = pair.rigid().unwrap();
                    // Preserve both plain and solver-cluster point sets in
                    // their own native order, never flatten their owners.
                    rigid.manifolds.iter().chain(&rigid.solver_clusters).map(|m| check(capture(m)))
                }).collect()
            }
            #[test]
            fn real_active_warmstart_reuse_sleep_and_wake_records() {
                let mut world = PhysicsWorld::default();
                world.integration_parameters.dt = 1. / 60.;
                assert!(world.integration_parameters.contact_recycling);
                world.insert(RigidBodyBuilder::fixed().translation($vector(0., -0.5)), $cuboid(8., 0.5).friction(0.9));
                let (body, _) = world.insert(RigidBodyBuilder::dynamic().translation($vector(0., 0.49)).linvel($vector(0.7, 0.)), $cuboid(0.5, 0.5).friction(0.9));
                let mut observed = 0;
                let mut warm = false;
                let mut friction = false;
                let mut new_point = false;
                let mut reused = false;
                let mut previous = None;
                for _ in 0..360 {
                    world.step();
                    assert!(world.quarantine().bodies().is_empty());
                    assert!(world.quarantine().colliders().is_empty());
                    let records = observe(&world);
                    for r in &records {
                        observed += r.points.len();
                        new_point |= r.solver.iter().any(SolverContactPoint::is_new);
                        for p in &r.points {
                            warm |= f32::from_bits(p.data.warmstart_impulse) > 0.;
                            // In 3D the canonical retained friction is world-space;
                            // the twist path does not write legacy tangent_impulse.
                            let tangents: &[u32] = if Dimension::$dim == Dimension::Spatial { &p.data.warmstart_tangent_world } else { &p.data.warmstart_tangent_impulse };
                            friction |= tangents.iter().any(|v| f32::from_bits(*v) != 0.);
                        }
                    }
                    if let Some(pair) = world.contact_pairs().next() {
                        let current = serde_json::to_value(pair.rigid().unwrap()).unwrap()["recycle_state"].clone();
                        let position = $bits(world.bodies[body].translation());
                        if let Some((old, old_position)) = &previous {
                            reused |= !current.is_null() && current == *old && position != *old_position;
                        }
                        previous = Some((current, position));
                    }
                }
                assert!(observed >= 360, "actual contact points observed each step");
                assert!(warm, "actual solved nonzero warm start");
                assert!(friction, "actual solved tangential impulse");
                assert!(new_point, "packed new-contact flag observed");
                assert!(reused, "retained recycle reference across changed body pose");
                world.bodies[body].sleep();
                world.step();
                assert!(world.bodies[body].is_sleeping());
                let sleeping = observe(&world);
                assert!(sleeping.iter().any(|r| !r.solver.is_empty()));
                for _ in 0..5 { world.step(); assert_eq!(observe(&world), sleeping); }
                world.bodies[body].set_linvel($vector(0.5, 0.), true);
                world.step();
                assert!(!world.bodies[body].is_sleeping());
                let awake = observe(&world);
                assert!(awake.iter().any(|r| !r.solver.is_empty()));
                assert_ne!(awake, sleeping);
                eprintln!("CONTACT_OBSERVATION dimension={} points={} nonzero_warm={} friction={} new={} retained_recycle={} sleeping_rows={} wake_rows={}", Dimension::$dim as u8, observed, warm, friction, new_point, reused, sleeping.len(), awake.len());
            }
            #[test]
            fn actual_feature_sentinels_and_unchecked_payload_refusals() {
                use $parry::shape::{PackedFeatureId, FeatureId};
                assert_eq!(PackedFeatureId(1).unpack(), FeatureId::Unknown);
                assert!(!PackedFeatureId(1).is_unknown());
                let mut world = PhysicsWorld::default();
                world.insert(RigidBodyBuilder::fixed().translation($vector(0., -0.5)), $cuboid(2., 0.5));
                world.insert(RigidBodyBuilder::dynamic().translation($vector(0., 0.49)), $cuboid(0.5, 0.5));
                world.step();
                let mut m = world.contact_pairs().next().unwrap().solver_manifolds()[0].clone();
                assert!(!m.points.is_empty() && !m.data.solver_contacts.is_empty());
                let original = check(capture(&m));
                for id in [PackedFeatureId::UNKNOWN, PackedFeatureId::vertex(0x3fff_ffff), PackedFeatureId::face(0x3fff_ffff)] {
                    m.points[0].fid1 = id;
                    assert_eq!(check(capture(&m)).points[0].features[0], id.0);
                }
                // Public native fields permit these invalid values; capture is
                // inert and the boundary refuses instead of normalizing them.
                m.points[0].fid1 = PackedFeatureId(1);
                assert!(matches!(validate_contact_points(capture(&m), &mut budget()), Err(Error::Contact)));
                m.points[0].fid1 = PackedFeatureId(original.points[0].features[0]);
                m.points[0].data.solver_dp1.x = f32::NAN;
                assert!(matches!(validate_contact_points(capture(&m), &mut budget()), Err(Error::Contact)));
                m.points[0].data.solver_dp1.x = f32::from_bits(original.points[0].data.solver_dp1[0]);
                m.data.solver_contacts[0].contact_id[0] = $rapier::geometry::NEW_CONTACT_BIT | m.points.len() as u32;
                assert!(matches!(validate_contact_points(capture(&m), &mut budget()), Err(Error::Contact)));
            }
        }
    }
}
cases!(
    planar, rapier2d, parry2d, Planar, vector2, box2, bits2, data2
);
cases!(
    spatial, rapier3d, parry3d, Spatial, vector3, box3, bits3, data3
);

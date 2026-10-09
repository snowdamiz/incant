//! Native candidate behavior, before exposing any binding to the Incant runtime.
use oxijolt::prelude::math::*;
use oxijolt::prelude::*;

fn step(world: &mut PhysicsWorld) {
    assert!(world.step(1. / 60.).unwrap().is_complete());
}

fn state(world: &PhysicsWorld, id: BodyId) -> Vec<u32> {
    let body = world.body(id).unwrap();
    let p = body.position();
    let q = body.rotation();
    let v = body.linear_velocity();
    let a = body.angular_velocity();
    [
        p.x, p.y, p.z, q.x, q.y, q.z, q.w, v.x, v.y, v.z, a.x, a.y, a.z,
    ]
    .map(f32::to_bits)
    .to_vec()
}

#[test]
fn jolt_contact_raycast_rollback_and_removed_handle() {
    for workers in [1, 4] {
        let mut world =
            PhysicsWorld::new(WorldSettings::default().worker_threads(workers)).unwrap();
        let floor = Shape::new_box(Vec3::new(10., 1., 10.)).unwrap();
        world
            .create_body(
                &floor,
                &BodySettings::new_static().position(RVec3::new(0., -1., 0.)),
            )
            .unwrap();
        let sphere = Shape::new_sphere(0.5).unwrap();
        let ball = world
            .create_body(
                &sphere,
                &BodySettings::new_dynamic().position(RVec3::new(0., 2., 0.)),
            )
            .unwrap();
        let ray = RayCast::new(RVec3::new(0., 10., 0.), Vec3::new(0., -20., 0.));
        let hit = world.cast_ray(&ray, &QueryFilter::new()).unwrap().unwrap();
        assert_eq!(hit.body, ball);
        assert!((hit.fraction - 0.375).abs() < 1e-5);
        let initial = state(&world, ball);
        assert!(world.step(f32::NAN).is_err());
        assert_eq!(state(&world, ball), initial);
        assert!(Shape::new_sphere(-1.).is_err());
        for _ in 0..15 {
            step(&mut world);
        }
        let saved = world.save_state();
        let mut first = Vec::new();
        for _ in 0..120 {
            step(&mut world);
            first.extend(state(&world, ball));
        }
        let y = world.body(ball).unwrap().position().y;
        assert!((0.47..0.53).contains(&y), "resting contact height {y}");
        world.restore_state(&saved).unwrap();
        let mut second = Vec::new();
        for _ in 0..120 {
            step(&mut world);
            second.extend(state(&world, ball));
        }
        assert_eq!(first, second, "pose and velocities replay after restore");
        world.remove_body(ball).unwrap();
        assert!(world.body(ball).is_err());
        assert!(world.remove_body(ball).is_err());
        let replacement = world
            .create_body(&sphere, &BodySettings::new_dynamic())
            .unwrap();
        assert_ne!(replacement, ball);
        assert!(
            world.body(ball).is_err(),
            "old handle must not alias reused slot"
        );
    }
}

#[test]
fn jolt_sensor_reports_entry_exit_without_blocking_motion() {
    let mut world = PhysicsWorld::new(WorldSettings::default().worker_threads(1)).unwrap();
    world.set_event_settings(EventSettings::default().contacts(true));
    let sensor_shape = Shape::new_box(Vec3::new(2., 0.25, 2.)).unwrap();
    let sensor = world
        .create_body(&sensor_shape, &BodySettings::new_static().sensor(true))
        .unwrap();
    let sphere = Shape::new_sphere(0.25).unwrap();
    let ball = world
        .create_body(
            &sphere,
            &BodySettings::new_dynamic().position(RVec3::new(0., 3., 0.)),
        )
        .unwrap();
    let (mut added, mut removed) = (false, false);
    for _ in 0..120 {
        step(&mut world);
        for contact in world.take_events().contacts {
            let pair = contact.pair();
            if (pair.body1 == sensor && pair.body2 == ball)
                || (pair.body1 == ball && pair.body2 == sensor)
            {
                match contact {
                    ContactEvent::Added { settings, .. } => {
                        assert!(settings.is_sensor());
                        added = true;
                    }
                    ContactEvent::Removed(_) => removed = true,
                    _ => {}
                }
            }
        }
    }
    assert!(added && removed, "sensor must report entry and exit");
    assert!(world.body(ball).unwrap().position().y < -10.);
}

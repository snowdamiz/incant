use incant_doc::{
    BodyMotion, Collider, ColliderShape, Entity, Project, RigidBody, Scene, Transform,
};
use incant_physics::{PhysicsRuntime, PreparedPhysics, RayQuery};
use serde_json::json;

fn entity(name: &str, position: [f64; 3], body: Option<RigidBody>, collider: Collider) -> Entity {
    let mut e = Entity::new(name);
    e.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: position,
            ..Default::default()
        }),
    );
    e.components.insert("Collider".into(), json!(collider));
    if let Some(body) = body {
        e.components.insert("RigidBody".into(), json!(body));
    }
    e
}
fn setup() -> (Project, String, String, String) {
    let mut project = Project::empty("Physics behavior");
    let mut scene = Scene::new("World");
    let floor = entity(
        "floor",
        [0., -0.5, 0.],
        None,
        Collider {
            shape: ColliderShape::Box {
                half_extents: [10., 0.5, 10.],
            },
            ..Default::default()
        },
    );
    let body = entity(
        "body",
        [0., 4., 0.],
        Some(RigidBody::default()),
        Collider::default(),
    );
    let ids = (scene.id.clone(), floor.id.clone(), body.id.clone());
    scene.entities.insert(floor.id.clone(), floor);
    scene.entities.insert(body.id.clone(), body);
    project.scenes.insert(scene.id.clone(), scene);
    (project, ids.0, ids.1, ids.2)
}
fn runtime(project: &Project) -> PhysicsRuntime {
    let mut physics = PhysicsRuntime::default();
    physics.sync(PreparedPhysics::new(project).unwrap());
    physics
}
fn ray(scene: &str) -> RayQuery {
    RayQuery {
        scene_id: scene.into(),
        origin: [0., 10., 0.],
        direction: [0., -2., 0.],
        max_distance: 20.,
        include_sensors: false,
        exclude_entity: None,
        memberships: u32::MAX,
        filter: u32::MAX,
    }
}
fn echo(project: &mut Project, scene: &str, physics: &PhysicsRuntime) {
    for (id, state) in physics.states() {
        let e = project
            .scenes
            .get_mut(scene)
            .unwrap()
            .entities
            .get_mut(&id)
            .unwrap();
        if !e.components.contains_key("RigidBody") {
            continue;
        }
        e.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: state.translation,
                rotation: state.rotation,
                ..Default::default()
            }),
        );
        e.components
            .insert("Velocity".into(), json!({"linear":state.velocity}));
        e.components.insert(
            "AngularVelocity".into(),
            json!({"angular":state.angular_velocity}),
        );
    }
}
#[test]
fn gravity_contact_sleep_and_document_echo_preserve_exact_solver_state() {
    let (mut project, scene, _, body) = setup();
    let original = project.clone();
    let mut direct = runtime(&project);
    let mut synced = runtime(&project);
    for _ in 0..360 {
        direct.step(1. / 60.).unwrap();
        synced.step(1. / 60.).unwrap();
        assert_eq!(direct.states(), synced.states());
        echo(&mut project, &scene, &synced);
        synced.sync(PreparedPhysics::new(&project).unwrap());
    }
    let state = &synced.states()[&body];
    assert!((state.translation[1] - 0.5).abs() < 0.02, "{state:?}");
    assert!(state.sleeping);
    let mut replay = runtime(&original);
    for _ in 0..360 {
        replay.step(1. / 60.).unwrap();
    }
    assert_eq!(replay.states(), direct.states());
}
#[test]
fn queries_are_immediate_stable_filtered_and_removed_handles_do_not_alias() {
    let (mut project, scene, floor, body) = setup();
    let mut physics = runtime(&project);
    let mut query = ray(&scene);
    let hit = physics.raycast(&query).unwrap().unwrap();
    assert_eq!(hit.entity_id, body);
    assert!((hit.distance - 5.5).abs() < 1e-6);
    assert_eq!(hit.normal, [0., 1., 0.]);
    query.exclude_entity = Some(body.clone());
    assert_eq!(physics.raycast(&query).unwrap().unwrap().entity_id, floor);
    query.filter = 0;
    assert!(physics.raycast(&query).unwrap().is_none());
    query.filter = u32::MAX;
    query.direction = [0.; 3];
    assert!(physics.raycast(&query).is_err());
    query = ray(&scene);
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .remove(&body);
    physics.sync(PreparedPhysics::new(&project).unwrap());
    assert_eq!(physics.raycast(&query).unwrap().unwrap().entity_id, floor);
    let replacement = entity("replacement", [0., 6., 0.], None, Collider::default());
    let id = replacement.id.clone();
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .insert(id.clone(), replacement);
    physics.sync(PreparedPhysics::new(&project).unwrap());
    assert_eq!(physics.raycast(&query).unwrap().unwrap().entity_id, id);
    let duplicate = entity("same distance", [0., 6., 0.], None, Collider::default());
    let expected = id.clone().min(duplicate.id.clone());
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .insert(duplicate.id.clone(), duplicate);
    physics.sync(PreparedPhysics::new(&project).unwrap());
    assert_eq!(
        physics.raycast(&query).unwrap().unwrap().entity_id,
        expected
    );
}
#[test]
fn sensors_emit_one_entry_and_exit_and_do_not_stop_motion() {
    let (mut project, scene, floor, body) = setup();
    let entities = &mut project.scenes.get_mut(&scene).unwrap().entities;
    entities
        .get_mut(&floor)
        .unwrap()
        .components
        .get_mut("Collider")
        .unwrap()["sensor"] = json!(true);
    let mut physics = runtime(&project);
    let mut events = Vec::new();
    for _ in 0..180 {
        physics.step(1. / 60.).unwrap();
        events.extend_from_slice(physics.events());
    }
    assert_eq!(events.len(), 2, "{events:?}");
    assert!(events[0].entered);
    assert!(!events[1].entered);
    assert!(physics.states()[&body].translation[1] < -10.);
    assert_eq!(events[0].scene_id, scene);
    let mut ids = [floor, body];
    ids.sort();
    assert_eq!([events[0].first.clone(), events[0].second.clone()], ids);
}
#[test]
fn scenes_do_not_collide_and_masks_disable_contacts() {
    let (mut project, scene, floor, body) = setup();
    let other_floor = project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .remove(&floor)
        .unwrap();
    let mut other_scene = Scene::new("Separate world");
    other_scene.entities.insert(floor.clone(), other_floor);
    project.scenes.insert(other_scene.id.clone(), other_scene);
    let mut physics = runtime(&project);
    for _ in 0..120 {
        physics.step(1. / 60.).unwrap();
    }
    assert!(physics.states()[&body].translation[1] < -10.);
    let (mut project, scene, _, body) = setup();
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&body)
        .unwrap()
        .components
        .get_mut("Collider")
        .unwrap()["filter"] = json!(0);
    let mut physics = runtime(&project);
    for _ in 0..120 {
        physics.step(1. / 60.).unwrap();
    }
    assert!(physics.states()[&body].translation[1] < -10.);
}
#[test]
fn kinematics_rotation_shape_variants_and_ccd_are_executed() {
    let (mut project, scene, _, body) = setup();
    let e = project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&body)
        .unwrap();
    e.components.insert(
        "RigidBody".into(),
        json!(RigidBody {
            motion: BodyMotion::Kinematic,
            ..Default::default()
        }),
    );
    e.components
        .insert("Velocity".into(), json!({"linear":[3.,0.,0.]}));
    e.components
        .insert("AngularVelocity".into(), json!({"angular":[0.,1.,0.]}));
    let mut physics = runtime(&project);
    for _ in 0..60 {
        physics.step(1. / 60.).unwrap();
    }
    let state = &physics.states()[&body];
    assert!((state.translation[0] - 3.).abs() < 1e-4);
    assert!((state.translation[1] - 4.).abs() < 1e-6);
    assert!((state.rotation[1] - (0.5_f64).sin()).abs() < 1e-4);
    for shape in [
        ColliderShape::Sphere { radius: 0.5 },
        ColliderShape::Capsule {
            radius: 0.5,
            half_height: 0.25,
        },
    ] {
        let (mut project, scene, _, body) = setup();
        let e = project
            .scenes
            .get_mut(&scene)
            .unwrap()
            .entities
            .get_mut(&body)
            .unwrap();
        e.components.insert(
            "Collider".into(),
            json!(Collider {
                shape,
                ..Default::default()
            }),
        );
        e.components
            .insert("Velocity".into(), json!({"linear":[0.,-1000.,0.]}));
        let mut physics = runtime(&project);
        for _ in 0..120 {
            physics.step(1. / 60.).unwrap();
        }
        assert!(
            physics.states()[&body].translation[1] > 0.45,
            "CCD must stop the high-speed body"
        );
    }
}
#[test]
fn invalid_document_and_step_leave_live_state_unchanged() {
    let (mut project, scene, _, body) = setup();
    let mut physics = runtime(&project);
    physics.step(1. / 60.).unwrap();
    let before = physics.states();
    for dt in [f64::NAN, -1., 0., 2., 1. / 1000.] {
        assert!(physics.step(dt).is_err());
    }
    assert_eq!(physics.states(), before);
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&body)
        .unwrap()
        .components
        .get_mut("Transform")
        .unwrap()["scale"] = json!([2., 1., 1.]);
    assert!(PreparedPhysics::new(&project).is_err());
    assert_eq!(physics.states(), before);
}

#[test]
fn authored_friction_restitution_gravity_and_damping_change_motion() {
    let slide = |friction: f64| {
        let (mut project, scene, floor, body) = setup();
        let entities = &mut project.scenes.get_mut(&scene).unwrap().entities;
        let ground = entities.get_mut(&floor).unwrap();
        ground.components.get_mut("Collider").unwrap()["friction"] = json!(friction);
        ground.components.get_mut("Collider").unwrap()["shape"]["half_extents"] =
            json!([100., 0.5, 100.]);
        let moving = entities.get_mut(&body).unwrap();
        moving.components.get_mut("Transform").unwrap()["translation"] = json!([0., 0.51, 0.]);
        moving.components.get_mut("Collider").unwrap()["friction"] = json!(friction);
        moving
            .components
            .insert("Velocity".into(), json!({"linear":[5.,0.,0.]}));
        let mut world = runtime(&project);
        for _ in 0..120 {
            world.step(1. / 60.).unwrap();
        }
        world.states()[&body].translation[0]
    };
    assert!(slide(1.) < slide(0.) * 0.5);
    let rebound = |restitution: f64| {
        let (mut project, scene, floor, body) = setup();
        let entities = &mut project.scenes.get_mut(&scene).unwrap().entities;
        for id in [&floor, &body] {
            entities
                .get_mut(id)
                .unwrap()
                .components
                .get_mut("Collider")
                .unwrap()["restitution"] = json!(restitution);
        }
        entities
            .get_mut(&body)
            .unwrap()
            .components
            .get_mut("Collider")
            .unwrap()["shape"] = json!({"type":"sphere","radius":0.5});
        let mut world = runtime(&project);
        let mut hit = false;
        let mut maximum: f64 = 0.;
        for _ in 0..180 {
            world.step(1. / 60.).unwrap();
            let y = world.states()[&body].translation[1];
            hit |= y < 0.55;
            if hit {
                maximum = maximum.max(y);
            }
        }
        maximum
    };
    assert!(rebound(1.) > 2.);
    assert!(rebound(0.) < 0.6);
    let (mut project, scene, _, body) = setup();
    let moving = project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&body)
        .unwrap();
    moving.components.insert(
        "RigidBody".into(),
        json!(RigidBody {
            gravity_scale: 0.,
            linear_damping: 5.,
            can_sleep: false,
            ..Default::default()
        }),
    );
    moving
        .components
        .insert("Velocity".into(), json!({"linear":[5.,0.,0.]}));
    let mut world = runtime(&project);
    for _ in 0..120 {
        world.step(1. / 60.).unwrap();
    }
    let state = &world.states()[&body];
    assert_eq!(state.translation[1], 4.);
    assert!(state.velocity[0] < 0.01);
    assert!(!state.sleeping);
}

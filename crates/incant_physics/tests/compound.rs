use incant_doc::{
    Collider, ColliderPart, ColliderShape, Entity, PrimitiveColliderShape, Project, RigidBody,
    Scene, Transform, new_id,
};
use incant_physics::{PhysicsRuntime, PreparedPhysics, RayQuery};
use serde_json::json;

fn part(at: [f64; 3], shape: PrimitiveColliderShape) -> ColliderPart {
    ColliderPart {
        id: new_id(),
        translation: at,
        rotation: [0., 0., 0., 1.],
        shape,
    }
}
fn block(at: [f64; 3], half: [f64; 3]) -> ColliderPart {
    part(at, PrimitiveColliderShape::Box { half_extents: half })
}
fn setup(parts: Vec<ColliderPart>) -> (Project, String, String) {
    let mut project = Project::empty("Compound simulation");
    let mut scene = Scene::new("World");
    let mut entity = Entity::new("Compound");
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity.components.insert(
        "Collider".into(),
        json!(Collider {
            shape: ColliderShape::Compound { parts },
            ..Default::default()
        }),
    );
    let ids = (scene.id.clone(), entity.id.clone());
    scene.entities.insert(entity.id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    (project, ids.0, ids.1)
}
fn runtime(project: &Project) -> PhysicsRuntime {
    let mut physics = PhysicsRuntime::default();
    physics.sync(PreparedPhysics::new(project).unwrap());
    physics
}
fn ray(scene: &str, from: [f64; 3]) -> RayQuery {
    RayQuery {
        scene_id: scene.into(),
        origin: from,
        direction: [0., 0., -1.],
        max_distance: 6.,
        include_sensors: false,
        exclude_entity: None,
        memberships: u32::MAX,
        filter: u32::MAX,
    }
}

#[test]
fn compound_queries_preserve_holes_local_rotations_and_parent_identity() {
    let (mut project, sid, eid) = setup(vec![
        block([-1., 1., 0.], [0.2, 1., 0.2]),
        block([1., 1., 0.], [0.2, 1., 0.2]),
        block([0., 2., 0.], [1.2, 0.2, 0.2]),
    ]);
    let mut physics = runtime(&project);
    assert!(physics.raycast(&ray(&sid, [0., 1., 3.])).unwrap().is_none());
    for origin in [[-1., 1., 3.], [1., 1., 3.], [0., 2., 3.]] {
        let hit = physics.raycast(&ray(&sid, origin)).unwrap().unwrap();
        assert_eq!(hit.entity_id, eid);
        assert!((hit.distance - 2.8).abs() < 1e-5);
    }
    let angle = std::f64::consts::FRAC_PI_2;
    let mut rod = block([0., 1.5, 0.], [1., 0.1, 0.1]);
    rod.rotation = [0., 0., (angle / 2.).sin(), (angle / 2.).cos()];
    project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .get_mut("Collider")
        .unwrap()["shape"] = json!(ColliderShape::Compound { parts: vec![rod] });
    physics.sync(PreparedPhysics::new(&project).unwrap());
    assert!(
        physics
            .raycast(&ray(&sid, [0.5, 1.5, 3.]))
            .unwrap()
            .is_none()
    );
    let hit = physics.raycast(&ray(&sid, [0., 2.3, 3.])).unwrap().unwrap();
    assert_eq!(hit.entity_id, eid);
    assert!((hit.distance - 2.9).abs() < 1e-4);
    project.scenes.get_mut(&sid).unwrap().entities.remove(&eid);
    physics.sync(PreparedPhysics::new(&project).unwrap());
    assert!(
        physics
            .raycast(&ray(&sid, [0., 2.3, 3.]))
            .unwrap()
            .is_none()
    );
}

#[test]
fn dynamic_compound_contacts_and_reordering_preserve_solver_state() {
    let (mut project, sid, eid) = setup(vec![
        part(
            [-0.7, 0., 0.],
            PrimitiveColliderShape::Sphere { radius: 0.3 },
        ),
        part(
            [0.7, 0., 0.],
            PrimitiveColliderShape::Sphere { radius: 0.3 },
        ),
    ]);
    let entities = &mut project.scenes.get_mut(&sid).unwrap().entities;
    let body = entities.get_mut(&eid).unwrap();
    body.components.get_mut("Transform").unwrap()["translation"] = json!([0, 4, 0]);
    body.components
        .insert("RigidBody".into(), json!(RigidBody::default()));
    let mut floor = Entity::new("Floor");
    floor.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., -0.5, 0.],
            ..Default::default()
        }),
    );
    floor.components.insert(
        "Collider".into(),
        json!(Collider {
            shape: ColliderShape::Box {
                half_extents: [10., 0.5, 10.]
            },
            ..Default::default()
        }),
    );
    entities.insert(floor.id.clone(), floor);
    let mut control = runtime(&project);
    let mut reordered = runtime(&project);
    for tick in 0..360 {
        control.step(1. / 60.).unwrap();
        reordered.step(1. / 60.).unwrap();
        assert_eq!(control.states(), reordered.states(), "tick {tick}");
        let state = &reordered.states()[&eid];
        let e = project
            .scenes
            .get_mut(&sid)
            .unwrap()
            .entities
            .get_mut(&eid)
            .unwrap();
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
        e.components.get_mut("Collider").unwrap()["shape"]["parts"]
            .as_array_mut()
            .unwrap()
            .reverse();
        reordered.sync(PreparedPhysics::new(&project).unwrap());
    }
    let state = &reordered.states()[&eid];
    assert!((state.translation[1] - 0.3).abs() < 0.02, "{state:?}");
    assert!(state.sleeping);
}

#[test]
fn overlapping_compound_sensor_parts_emit_one_entity_event_and_honor_query_masks() {
    let (mut project, sid, sensor) = setup(vec![
        part(
            [-0.4, 0., 0.],
            PrimitiveColliderShape::Sphere { radius: 0.8 },
        ),
        part(
            [0.4, 0., 0.],
            PrimitiveColliderShape::Sphere { radius: 0.8 },
        ),
    ]);
    let entities = &mut project.scenes.get_mut(&sid).unwrap().entities;
    let collider = entities
        .get_mut(&sensor)
        .unwrap()
        .components
        .get_mut("Collider")
        .unwrap();
    collider["sensor"] = json!(true);
    collider["memberships"] = json!(4);
    collider["filter"] = json!(4);
    let mut moving = Entity::new("Visitor");
    moving.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [-2., 0., 0.],
            ..Default::default()
        }),
    );
    moving.components.insert(
        "Collider".into(),
        json!(Collider {
            shape: ColliderShape::Sphere { radius: 0.2 },
            ..Default::default()
        }),
    );
    moving.components.insert(
        "RigidBody".into(),
        json!(RigidBody {
            motion: incant_doc::BodyMotion::Kinematic,
            ..Default::default()
        }),
    );
    moving
        .components
        .insert("Velocity".into(), json!({"linear":[1.,0.,0.]}));
    let visitor = moving.id.clone();
    entities.insert(visitor.clone(), moving);
    let mut physics = runtime(&project);
    let mut events = Vec::new();
    for _ in 0..240 {
        physics.step(1. / 60.).unwrap();
        events.extend_from_slice(physics.events());
    }
    assert_eq!(events.len(), 2, "{events:?}");
    assert!(events[0].entered);
    assert!(!events[1].entered);
    for event in events {
        assert_eq!(event.scene_id, sid);
        assert!(event.first == sensor || event.second == sensor);
        assert!(event.first == visitor || event.second == visitor);
    }
    let mut query = ray(&sid, [0., 0., 3.]);
    query.exclude_entity = Some(visitor);
    assert!(physics.raycast(&query).unwrap().is_none());
    query.include_sensors = true;
    assert_eq!(physics.raycast(&query).unwrap().unwrap().entity_id, sensor);
    query.memberships = 1;
    query.filter = 1;
    assert!(physics.raycast(&query).unwrap().is_none());
}

#[test]
fn compound_character_bodies_fail_explicitly_without_mutating_the_world() {
    let (mut project, sid, eid) = setup(vec![block([0., 0., 0.], [0.3, 0.8, 0.3])]);
    project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&eid)
        .unwrap()
        .components
        .insert(
            "RigidBody".into(),
            json!(RigidBody {
                motion: incant_doc::BodyMotion::Kinematic,
                ..Default::default()
            }),
        );
    let physics = runtime(&project);
    let before = physics.states();
    let result = physics.compute_character_motion(
        &incant_physics::CharacterQuery {
            scene_id: sid,
            entity_id: eid,
            translation: [0.1, 0., 0.],
            options: Default::default(),
        },
        1. / 60.,
    );
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("primitive collider")
    );
    assert_eq!(physics.states(), before);
}

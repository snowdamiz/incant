use incant_doc::{
    BodyMotion, Collider, ColliderShape, Entity, Project, RigidBody, Scene, Transform,
};
use incant_physics::{
    CharacterOptions, CharacterQuery, CharacterStep, PhysicsRuntime, PreparedPhysics,
};
use serde_json::json;

fn entity(name: &str, at: [f64; 3], shape: ColliderShape, body: Option<RigidBody>) -> Entity {
    let mut e = Entity::new(name);
    e.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: at,
            ..Default::default()
        }),
    );
    e.components.insert(
        "Collider".into(),
        json!(Collider {
            shape,
            ..Default::default()
        }),
    );
    if let Some(b) = body {
        e.components.insert("RigidBody".into(), json!(b));
    }
    e
}
fn box_at(name: &str, at: [f64; 3], half: [f64; 3]) -> Entity {
    entity(name, at, ColliderShape::Box { half_extents: half }, None)
}
fn scene() -> (Project, CharacterQuery, String) {
    let mut project = Project::empty("Character movement");
    let mut scene = Scene::new("World");
    let floor = box_at("Floor", [0., -0.5, 0.], [10., 0.5, 10.]);
    let floor_id = floor.id.clone();
    let character = entity(
        "Player",
        [0., 0.91, 0.],
        ColliderShape::Capsule {
            half_height: 0.6,
            radius: 0.3,
        },
        Some(RigidBody {
            motion: BodyMotion::Kinematic,
            ..Default::default()
        }),
    );
    let request = CharacterQuery {
        scene_id: scene.id.clone(),
        entity_id: character.id.clone(),
        translation: [0., -0.1, 0.],
        options: CharacterOptions::default(),
    };
    scene.entities.insert(floor.id.clone(), floor);
    scene.entities.insert(character.id.clone(), character);
    project.scenes.insert(scene.id.clone(), scene);
    (project, request, floor_id)
}
fn runtime(p: &Project) -> PhysicsRuntime {
    let mut r = PhysicsRuntime::default();
    r.sync(PreparedPhysics::new(p).unwrap());
    r
}

#[test]
fn character_sweeps_stop_at_walls_slide_ground_and_preserve_live_world() {
    let (mut project, mut query, floor) = scene();
    let wall = box_at("Wall", [2., 2., 0.], [0.1, 2., 10.]);
    let wall_id = wall.id.clone();
    project
        .scenes
        .get_mut(&query.scene_id)
        .unwrap()
        .entities
        .insert(wall.id.clone(), wall);
    query.translation = [4., 0., 1.];
    let mut physics = runtime(&project);
    let before = physics.states();
    let first = physics.compute_character_motion(&query, 1. / 60.).unwrap();
    assert!(first.grounded, "{first:?}");
    assert!((1.5..1.61).contains(&first.translation[0]), "{first:?}");
    assert!((first.translation[2] - 1.).abs() < 0.02, "{first:?}");
    assert!(first.translation[1].abs() < 0.02, "{first:?}");
    assert!(first.collisions.contains(&wall_id));
    // Ground detection is separate from sweep collision callbacks.
    let mut down = query.clone();
    down.translation = [0., -0.1, 0.];
    assert!(
        physics
            .compute_character_motion(&down, 1. / 60.)
            .unwrap()
            .collisions
            .contains(&floor)
    );
    assert_eq!(before, physics.states());
    assert!(physics.events().is_empty());
    assert_eq!(
        first,
        physics.compute_character_motion(&query, 1. / 60.).unwrap()
    );
    physics.step(1. / 60.).unwrap();
    assert_eq!(
        first,
        physics.compute_character_motion(&query, 1. / 60.).unwrap()
    );
    query.options.slide = false;
    let stopped = physics.compute_character_motion(&query, 1. / 60.).unwrap();
    assert!(stopped.translation[2] < first.translation[2]);
}

fn walk(
    mut project: Project,
    query: &CharacterQuery,
    ticks: usize,
) -> (incant_physics::BodyState, bool) {
    let mut physics = runtime(&project);
    let mut grounded = false;
    for _ in 0..ticks {
        let movement = physics.compute_character_motion(query, 1. / 60.).unwrap();
        grounded = movement.grounded;
        let state = &physics.states()[&query.entity_id];
        let entity = project
            .scenes
            .get_mut(&query.scene_id)
            .unwrap()
            .entities
            .get_mut(&query.entity_id)
            .unwrap();
        entity.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: state.translation,
                rotation: state.rotation,
                ..Default::default()
            }),
        );
        entity.components.insert(
            "Velocity".into(),
            json!({"linear": movement.translation.map(|v| v * 60.)}),
        );
        physics.sync(PreparedPhysics::new(&project).unwrap());
        physics.step(1. / 60.).unwrap();
    }
    (physics.states()[&query.entity_id].clone(), grounded)
}
#[test]
fn character_steps_over_low_stairs_but_not_tall_obstacles() {
    let (mut project, mut query, _) = scene();
    let step = box_at("Step", [2., 0.15, 0.], [1.5, 0.15, 2.]);
    let step_id = step.id.clone();
    project
        .scenes
        .get_mut(&query.scene_id)
        .unwrap()
        .entities
        .insert(step.id.clone(), step);
    query.translation = [0.05, -0.01, 0.];
    let (blocked, _) = walk(project.clone(), &query, 40);
    query.options.autostep = Some(CharacterStep {
        max_height: 0.4,
        min_width: 0.2,
        include_dynamic_bodies: false,
    });
    let (climbed, grounded) = walk(project.clone(), &query, 40);
    assert!(blocked.translation[0] < 0.5, "{blocked:?}");
    assert!(climbed.translation[0] > 1.5, "{climbed:?}");
    assert!(
        (1.18..1.24).contains(&climbed.translation[1]),
        "{climbed:?}"
    );
    assert!(grounded);
    let e = project
        .scenes
        .get_mut(&query.scene_id)
        .unwrap()
        .entities
        .get_mut(&step_id)
        .unwrap();
    e.components.get_mut("Transform").unwrap()["translation"][1] = json!(0.5);
    e.components.get_mut("Collider").unwrap()["shape"]["half_extents"][1] = json!(0.5);
    let (tall, _) = walk(project, &query, 40);
    assert!(tall.translation[0] < 0.5, "{tall:?}");
}

#[test]
fn slope_limit_and_ground_snap_change_observable_traversal() {
    let (mut project, mut query, _) = scene();
    let mut ramp = box_at("Ramp", [2., 0.8, 0.], [4., 0.1, 2.]);
    let angle: f64 = 0.45;
    ramp.components.get_mut("Transform").unwrap()["rotation"] =
        json!([0., 0., (angle / 2.).sin(), (angle / 2.).cos()]);
    project
        .scenes
        .get_mut(&query.scene_id)
        .unwrap()
        .entities
        .insert(ramp.id.clone(), ramp);
    query.translation = [0.05, -0.01, 0.];
    query.options.max_slope_climb_angle = 0.2;
    let (blocked, _) = walk(project.clone(), &query, 100);
    query.options.max_slope_climb_angle = 0.8;
    let (climbed, _) = walk(project, &query, 100);
    assert!(blocked.translation[0] < 1.5, "{blocked:?}");
    assert!(
        climbed.translation[0] > 3. && climbed.translation[1] > 2.,
        "{climbed:?}"
    );

    let (mut project, mut query, _) = scene();
    let ledge = box_at("Ledge", [-2., 0.15, 0.], [2., 0.15, 2.]);
    let entities = &mut project.scenes.get_mut(&query.scene_id).unwrap().entities;
    entities.insert(ledge.id.clone(), ledge);
    entities
        .get_mut(&query.entity_id)
        .unwrap()
        .components
        .get_mut("Transform")
        .unwrap()["translation"] = json!([-0.4, 1.21, 0.]);
    query.translation = [0.05, 0., 0.];
    query.options.snap_to_ground = None;
    let (airborne, ground_without_snap) = walk(project.clone(), &query, 40);
    query.options.snap_to_ground = Some(0.4);
    let (snapped, ground_with_snap) = walk(project, &query, 40);
    assert!(!ground_without_snap && ground_with_snap);
    assert!(airborne.translation[1] > 1.2, "{airborne:?}");
    assert!((0.9..0.93).contains(&snapped.translation[1]), "{snapped:?}");
}

#[test]
fn character_queries_honor_masks_ignore_sensors_and_see_unsimulated_edits() {
    let (mut project, mut query, _) = scene();
    let wall = box_at("Wall", [2., 2., 0.], [0.1, 2., 10.]);
    let wall_id = wall.id.clone();
    project
        .scenes
        .get_mut(&query.scene_id)
        .unwrap()
        .entities
        .insert(wall.id.clone(), wall);
    query.translation = [4., 0., 0.];
    let mut physics = runtime(&project);
    assert!(
        physics
            .compute_character_motion(&query, 1. / 60.)
            .unwrap()
            .translation[0]
            < 2.
    );
    let entities = &mut project.scenes.get_mut(&query.scene_id).unwrap().entities;
    entities
        .get_mut(&wall_id)
        .unwrap()
        .components
        .get_mut("Collider")
        .unwrap()["sensor"] = json!(true);
    physics.sync(PreparedPhysics::new(&project).unwrap());
    assert!(
        physics
            .compute_character_motion(&query, 1. / 60.)
            .unwrap()
            .translation[0]
            > 3.9
    );
    let entities = &mut project.scenes.get_mut(&query.scene_id).unwrap().entities;
    entities
        .get_mut(&wall_id)
        .unwrap()
        .components
        .get_mut("Collider")
        .unwrap()["sensor"] = json!(false);
    entities
        .get_mut(&query.entity_id)
        .unwrap()
        .components
        .get_mut("Collider")
        .unwrap()["filter"] = json!(0);
    physics.sync(PreparedPhysics::new(&project).unwrap());
    let unfiltered = physics.compute_character_motion(&query, 1. / 60.).unwrap();
    assert!(unfiltered.translation[0] > 3.9);
    assert!(!unfiltered.grounded);
    assert!(unfiltered.collisions.is_empty());
}

#[test]
fn character_query_validation_is_explicit_and_does_not_consume_pending_solver_changes() {
    let (mut project, query, _) = scene();
    let falling = entity(
        "Falling",
        [5., 4., 0.],
        ColliderShape::Sphere { radius: 0.3 },
        Some(RigidBody::default()),
    );
    let falling_id = falling.id.clone();
    project
        .scenes
        .get_mut(&query.scene_id)
        .unwrap()
        .entities
        .insert(falling.id.clone(), falling);
    let mut physics = runtime(&project);
    let mut direct = runtime(&project);
    physics.compute_character_motion(&query, 1. / 60.).unwrap();
    for _ in 0..60 {
        physics.step(1. / 60.).unwrap();
        direct.step(1. / 60.).unwrap();
        assert_eq!(physics.states(), direct.states());
    }
    assert!(physics.states()[&falling_id].translation[1] < 1.);
    let before = physics.states();
    for bad in [f64::NAN, f64::INFINITY, -0.1, 0.] {
        let mut q = query.clone();
        q.options.offset = bad;
        assert!(physics.compute_character_motion(&q, 1. / 60.).is_err());
    }
    let mut q = query.clone();
    q.translation[0] = f64::NAN;
    assert!(physics.compute_character_motion(&q, 1. / 60.).is_err());
    q = query.clone();
    q.entity_id = falling_id;
    assert!(physics.compute_character_motion(&q, 1. / 60.).is_err());
    q = query.clone();
    q.scene_id = "missing".into();
    assert!(physics.compute_character_motion(&q, 1. / 60.).is_err());
    assert!(physics.compute_character_motion(&query, 0.).is_err());
    assert_eq!(before, physics.states());
}

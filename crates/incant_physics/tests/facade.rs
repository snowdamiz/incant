use incant_doc::{Collider, Entity, Project, Scene, Transform};
use incant_physics::{PhysicsRuntime, PreparedPhysics, RayQuery};
use serde_json::json;

#[test]
fn facade_preserves_distinct_textual_ids_and_lexical_ties() {
    // Project IDs are historically case-sensitive strings even when their ULID
    // decoder maps two representations to the same 128 bits. The legacy facade
    // must not silently merge them or rewrite returned IDs during extraction.
    let mut project = Project::empty("textual identities");
    let mut scene = Scene::new("scene");
    let scene_id = scene.id.clone();
    let upper = "0000000000000000000000000A";
    let lower = "0000000000000000000000000a";
    for id in [lower, upper] {
        let mut entity = Entity::new("collider");
        entity.id = id.into();
        entity
            .components
            .insert("Transform".into(), json!(Transform::default()));
        entity
            .components
            .insert("Collider".into(), json!(Collider::default()));
        scene.entities.insert(entity.id.clone(), entity);
    }
    project.scenes.insert(scene.id.clone(), scene);
    let mut runtime = PhysicsRuntime::default();
    runtime
        .sync(PreparedPhysics::new(&project).unwrap())
        .unwrap();
    assert_eq!(
        runtime
            .states()
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec![upper, lower]
    );
    let mut ray = RayQuery {
        scene_id,
        origin: [0., 2., 0.],
        direction: [0., -1., 0.],
        max_distance: 3.,
        include_sensors: false,
        exclude_entity: None,
        memberships: u32::MAX,
        filter: u32::MAX,
    };
    assert_eq!(runtime.raycast(&ray).unwrap().unwrap().entity_id, upper);
    ray.exclude_entity = Some(upper.into());
    assert_eq!(runtime.raycast(&ray).unwrap().unwrap().entity_id, lower);
    runtime.step(1. / 60.).unwrap();
    assert_eq!(runtime.raycast(&ray).unwrap().unwrap().entity_id, lower);
}

#[test]
fn extracted_query_and_event_schemas_preserve_the_published_contract() {
    let schemas = [
        (
            "PhysicsRayQuery",
            json!(schemars::schema_for!(incant_physics::RayQuery)),
        ),
        (
            "PhysicsRayHit",
            json!(schemars::schema_for!(incant_physics::RayHit)),
        ),
        (
            "PhysicsCharacterQuery",
            json!(schemars::schema_for!(incant_physics::CharacterQuery)),
        ),
        (
            "PhysicsCharacterMovement",
            json!(schemars::schema_for!(incant_physics::CharacterMovement)),
        ),
        (
            "PhysicsTriggerEvent",
            json!(schemars::schema_for!(incant_physics::TriggerEvent)),
        ),
    ];
    for (name, actual) in schemas {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../schemas/{name}.schema.json"));
        let expected: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(actual, expected, "{name}");
    }
}

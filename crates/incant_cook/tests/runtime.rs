#![cfg(feature = "scene")]
use incant_cook::{CookError, cook_scene, stable_id};
use incant_doc::{Entity, Project, Scene, Transform, Velocity, new_id};
use incant_runtime::NativeWorld;
use serde_json::json;
use std::collections::BTreeMap;

fn fixture() -> (Project, String, String) {
    let mut project = Project::empty("Cooked motion");
    let scene_id = new_id();
    let entity_id = new_id();
    let entity = Entity {
        id: entity_id.clone(),
        name: "Mover".into(),
        parent: None,
        provenance: None,
        components: BTreeMap::from([
            (
                "Transform".into(),
                serde_json::to_value(Transform::default()).unwrap(),
            ),
            (
                "Velocity".into(),
                serde_json::to_value(Velocity {
                    linear: [3., 0., 0.],
                })
                .unwrap(),
            ),
        ]),
    };
    project.scenes.insert(
        scene_id.clone(),
        Scene {
            id: scene_id.clone(),
            name: "Main".into(),
            entities: BTreeMap::from([(entity_id.clone(), entity)]),
        },
    );
    (project, scene_id, entity_id)
}

#[test]
fn cooked_world_runs_after_authoring_is_discarded_and_cook_is_read_only() {
    let (project, scene, entity) = fixture();
    let before = serde_json::to_vec(&project).unwrap();
    let bytes = cook_scene(&project, &scene).unwrap();
    assert_eq!(serde_json::to_vec(&project).unwrap(), before);
    assert_eq!(bytes, cook_scene(&project, &scene).unwrap());
    drop(project);
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    for _ in 0..60 {
        world.step().unwrap();
    }
    assert!(
        (world
            .inspect(stable_id(&entity).unwrap())
            .unwrap()
            .transform
            .translation[0]
            - 3.)
            .abs()
            < 1e-12
    );
}

#[test]
fn unsupported_component_is_a_named_error_not_silently_omitted() {
    let (mut project, scene, entity) = fixture();
    let script = new_id();
    project.scripts.insert(
        script.clone(),
        incant_doc::ScriptSource {
            id: script.clone(),
            name: "Unbound".into(),
            path: "scripts/missing.ts".into(),
        },
    );
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&entity)
        .unwrap()
        .components
        .insert("Script".into(), json!({"source":script,"props":{}}));
    let error = cook_scene(&project, &scene).unwrap_err();
    assert!(
        matches!(error, CookError::UnsupportedComponent { entity: id, component } if id == entity && component == "Script")
    );
}

#[test]
fn invalid_authoring_and_missing_scene_do_not_produce_bytes() {
    let (mut project, scene, _) = fixture();
    assert!(matches!(
        cook_scene(&project, &new_id()),
        Err(CookError::MissingScene(_))
    ));
    project.settings.tick_rate = 0;
    assert!(matches!(
        cook_scene(&project, &scene),
        Err(CookError::Document(_))
    ));
    assert!(stable_id("not-a-ulid").is_err());
}

#[test]
fn rigid_body_cook_preserves_full_angular_state_after_project_drop() {
    let (mut project, scene, entity) = fixture();
    let components = &mut project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&entity)
        .unwrap()
        .components;
    components.insert(
        "RigidBody".into(),
        serde_json::to_value(incant_doc::RigidBody {
            motion: incant_doc::BodyMotion::Kinematic,
            gravity_scale: 0.,
            ..Default::default()
        })
        .unwrap(),
    );
    components.insert(
        "Collider".into(),
        serde_json::to_value(incant_doc::Collider::default()).unwrap(),
    );
    components.insert("AngularVelocity".into(), json!({"angular":[0,1,0]}));
    let bytes = cook_scene(&project, &scene).unwrap();
    drop(project);
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    for _ in 0..60 {
        world.step().unwrap();
    }
    let state = world.physics_state(stable_id(&entity).unwrap()).unwrap();
    assert!((state.translation[0] - 3.).abs() < 1e-5);
    assert_eq!(state.velocity, [3., 0., 0.]);
    assert_eq!(state.angular_velocity, [0., 1., 0.]);
    assert!((state.rotation[1] - (0.5_f64).sin()).abs() < 1e-5);
}

#[test]
fn audio_cook_keeps_binary_asset_ids_and_rejects_cross_scene_bus_dependencies() {
    let (mut project, scene, entity) = fixture();
    let asset = incant_doc::Asset {
        id: new_id(),
        name: "Tone".into(),
        path: "tone.wav".into(),
        kind: "audio".into(),
        sha256: "0".repeat(64),
        import_settings: None,
    };
    let clip = asset.id.clone();
    project.assets.insert(clip.clone(), asset);
    let mut bus = Entity::new("Bus");
    let bid = bus.id.clone();
    bus.components
        .insert("AudioBus".into(), json!({"parent":null,"gain_db":-6}));
    project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .insert(bid.clone(), bus);
    let components = &mut project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .get_mut(&entity)
        .unwrap()
        .components;
    components.insert("AudioSource".into(), json!({"clip":clip,"bus":bid,"gain_db":0,"pan":0,"rate":1,"start_seconds":0,"playing":true,"looping":false,"streaming":true,"spatial":null}));
    let original = serde_json::to_vec(&project).unwrap();
    let bytes = cook_scene(&project, &scene).unwrap();
    assert_eq!(serde_json::to_vec(&project).unwrap(), original);
    let world = NativeWorld::from_bytes(&bytes).unwrap();
    let source = world
        .audio_nodes()
        .find_map(|(n, _)| n.source.as_ref())
        .unwrap();
    assert_eq!(source.clip, stable_id(&clip).unwrap());
    assert_eq!(source.bus, Some(stable_id(&bid).unwrap()));
    // Authoring has a project-wide bus graph; a standalone scene cannot silently
    // load a reference to an omitted scene. Fail during cooking instead.
    let bus = project
        .scenes
        .get_mut(&scene)
        .unwrap()
        .entities
        .remove(&bid)
        .unwrap();
    let mut other = Scene::new("Other");
    other.entities.insert(bid, bus);
    project.scenes.insert(other.id.clone(), other);
    project.validate().unwrap();
    assert!(matches!(
        cook_scene(&project, &scene),
        Err(CookError::Scene(_))
    ));
}

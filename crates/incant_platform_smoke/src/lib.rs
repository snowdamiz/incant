//! Small runnable cross-platform probe of the actual document and Bevy crates.
use incant_core::{CharacterQuery, Engine};
use incant_doc::{
    BodyMotion, Collider, ColliderPart, ColliderShape, Entity, PrimitiveColliderShape, Project,
    RigidBody, Scene, Transform,
};
use serde_json::json;

pub fn run() -> Result<String, String> {
    let mut project = Project::empty("Incant platform smoke");
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Hello world");
    let id = entity.id.clone();
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity
        .components
        .insert("Velocity".into(), json!({"linear":[3.,0.,0.]}));
    scene.entities.insert(id.clone(), entity);
    let mut floor = Entity::new("Physics floor");
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
            shape: ColliderShape::Compound {
                parts: [-5., 5.]
                    .into_iter()
                    .enumerate()
                    .map(|(i, x)| ColliderPart {
                        id: format!("{:026}", i + 20),
                        translation: [x, 0., 0.],
                        rotation: [0., 0., 0., 1.],
                        shape: PrimitiveColliderShape::Box {
                            half_extents: [5., 0.5, 10.]
                        },
                    })
                    .collect(),
            },
            ..Default::default()
        }),
    );
    let mut falling = Entity::new("Falling sphere");
    let falling_id = falling.id.clone();
    falling.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [0., 4., 0.],
            ..Default::default()
        }),
    );
    falling.components.insert(
        "Collider".into(),
        json!(Collider {
            shape: ColliderShape::Sphere { radius: 0.5 },
            ..Default::default()
        }),
    );
    falling
        .components
        .insert("RigidBody".into(), json!(RigidBody::default()));
    let mut character = Entity::new("Kinematic character");
    let character_id = character.id.clone();
    let scene_id = scene.id.clone();
    character.components.insert(
        "Transform".into(),
        json!(Transform {
            translation: [-3., 0.91, 0.],
            ..Default::default()
        }),
    );
    character.components.insert(
        "Collider".into(),
        json!(Collider {
            shape: ColliderShape::Capsule {
                half_height: 0.6,
                radius: 0.3
            },
            ..Default::default()
        }),
    );
    character.components.insert(
        "RigidBody".into(),
        json!(RigidBody {
            motion: BodyMotion::Kinematic,
            ..Default::default()
        }),
    );
    scene.entities.insert(character_id.clone(), character);
    scene.entities.insert(floor.id.clone(), floor);
    scene.entities.insert(falling.id.clone(), falling);
    project.scenes.insert(scene.id.clone(), scene);
    let text = project.canonical_text().map_err(|e| e.to_string())?;
    let reloaded = Project::from_text(&text).map_err(|e| e.to_string())?;
    if reloaded != project {
        return Err("document roundtrip failed".into());
    }
    let mut engine = Engine::new(&project).map_err(|e| e.to_string())?;
    engine.run_ticks(120).map_err(|e| e.to_string())?;
    let state = engine.snapshot();
    let x = state.entities[&id].translation[0];
    if (x - 6.).abs() > 1e-9 {
        return Err("ECS fixed-step assertion failed".into());
    }
    let physics_y = state.entities[&falling_id].translation[1];
    if (physics_y - 0.5).abs() > 0.02 {
        return Err("physics contact assertion failed".into());
    }
    let movement = engine.character_mover()(CharacterQuery {
        scene_id,
        entity_id: character_id,
        translation: [0.1, -0.1, 0.],
        options: Default::default(),
    })
    .map_err(|e| e.to_string())?;
    if !movement.grounded
        || movement.sliding_down_slope
        || movement.translation[0] < 0.09
        || movement.translation[1].abs() > 0.02
    {
        return Err("character movement assertion failed".into());
    }
    Ok(json!({"character_movement":movement,"physics_backend":"rapier-0.36-enhanced-determinism","physics_y":physics_y,"incant":"hello-world","ok":true,"ticks":state.tick,"position_x":x,"os":std::env::consts::OS,"arch":std::env::consts::ARCH}).to_string())
}

#[cfg(not(target_arch = "wasm32"))]
#[unsafe(no_mangle)]
pub extern "C" fn incant_smoke_run() -> i32 {
    match std::panic::catch_unwind(run) {
        Ok(Ok(report)) => {
            println!("{report}");
            0
        }
        Ok(Err(error)) => {
            eprintln!("{error}");
            1
        }
        Err(_) => 2,
    }
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_incant_smoke_MainActivity_runSmoke(
    _env: *mut std::ffi::c_void,
    _class: *mut std::ffi::c_void,
) -> i32 {
    incant_smoke_run()
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn smoke() -> Result<String, wasm_bindgen::JsValue> {
    run().map_err(|e| wasm_bindgen::JsValue::from_str(&e))
}

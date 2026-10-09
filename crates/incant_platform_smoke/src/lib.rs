//! Small runnable cross-platform probe of the actual document and Bevy crates.
use incant_core::Engine;
use incant_doc::{Entity, Project, Scene, Transform};
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
    project.scenes.insert(scene.id.clone(), scene);
    let text = project.canonical_text().map_err(|e| e.to_string())?;
    let reloaded = Project::from_text(&text).map_err(|e| e.to_string())?;
    if reloaded != project {
        return Err("document roundtrip failed".into());
    }
    let mut engine = Engine::new(&project).map_err(|e| e.to_string())?;
    engine.run_ticks(120);
    let state = engine.snapshot();
    let x = state.entities[&id].translation[0];
    if (x - 6.).abs() > 1e-9 {
        return Err("ECS fixed-step assertion failed".into());
    }
    Ok(json!({"incant":"hello-world","ok":true,"ticks":state.tick,"position_x":x,"os":std::env::consts::OS,"arch":std::env::consts::ARCH}).to_string())
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

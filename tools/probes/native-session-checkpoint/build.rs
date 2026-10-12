use incant_doc::{Entity, Project, Scene, ScriptSource};
use serde_json::json;
use std::{env, fs, path::PathBuf};
fn main() {
    println!("cargo:rerun-if-changed=src/behavior.ts");
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("a.ts"), include_str!("src/behavior.ts")).unwrap();
    fs::write(root.path().join("b.ts"), include_str!("src/behavior.ts")).unwrap();
    let mut project = Project::empty("Checkpoint authored fixture");
    project.id = "00000000000000000000000099".into();
    let mut scene = Scene::new("Main");
    scene.id = "00000000000000000000000090".into();
    for (number, path) in [(11, "a.ts"), (12, "b.ts")] {
        let id = format!("000000000000000000000000{number}");
        project.scripts.insert(
            id.clone(),
            ScriptSource {
                id,
                name: path.into(),
                path: path.into(),
            },
        );
    }
    for (n, source, speed) in [(1, 11, 2), (2, 11, 5), (3, 12, 9)] {
        let mut entity = Entity::new("Counter");
        entity.id = format!("0000000000000000000000000{n}");
        entity.components.insert(
            "Script".into(),
            json!({"source":format!("000000000000000000000000{source}"),"props":{"speed":speed}}),
        );
        scene.entities.insert(entity.id.clone(), entity);
    }
    let scene_id = scene.id.clone();
    project.scenes.insert(scene_id.clone(), scene);
    let before = project.canonical_text().unwrap();
    let compiled = incant_cook::compile_project_sources(
        incant_cook::prepare_script_scene(&project, &scene_id).unwrap(),
        root.path(),
    )
    .unwrap();
    let (scene, program) = compiled.program.into_parts();
    assert_eq!(project.canonical_text().unwrap(), before);
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    fs::write(out.join("world.scene"), scene.to_bytes()).unwrap();
    fs::write(out.join("behavior.rs"), program.embedded_rust()).unwrap();
    fs::write(
        out.join("compiler.json"),
        serde_json::to_vec_pretty(&compiled.compiler_manifest).unwrap(),
    )
    .unwrap();
    // The actual authored TypeScript inputs are disposable build-time data.
    let path = root.path().to_owned();
    drop(root);
    assert!(!path.exists());
}

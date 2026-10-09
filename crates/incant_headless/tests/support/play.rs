use incant_doc::{Entity, Project, Scene, Transform};
use serde_json::json;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

pub fn fixture(root: &Path) -> String {
    let mut project = Project::empty("Playback test");
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Moving cube");
    let id = entity.id.clone();
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity
        .components
        .insert("Velocity".into(), json!({"linear":[2.,0.,0.]}));
    scene.entities.insert(id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    fs::write(
        root.join("game.incant.json"),
        project.canonical_text().unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("game.incant.journal.jsonl"),
        "author journal must not be opened",
    )
    .unwrap();
    id
}

pub fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_incant_headless"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}

pub fn success(output: Output) -> serde_json::Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

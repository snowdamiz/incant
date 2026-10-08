use incant_cmd::{Actor, Command, CommandBus};
use incant_core::Engine;
use incant_doc::{Entity, Project, Scene, Transform};
use serde_json::json;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut project = Project::empty("CRDT roundtrip");
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Cube");
    let scene_id = scene.id.clone();
    let entity_id = entity.id.clone();
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity
        .components
        .insert("Velocity".into(), json!({"linear":[3.,0.,0.]}));
    scene.entities.insert(entity.id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    let mut bevy = Engine::new(&project)?;
    bevy.run_ticks(120);
    let position = bevy.snapshot().entities[&entity_id].translation;
    let mut author = CommandBus::new(project)?;
    author.execute(
        vec![Command::SetComponent {
            scene_id: scene_id.clone(),
            entity_id: entity_id.clone(),
            component: "Transform".into(),
            value: json!(Transform {
                translation: position,
                ..Default::default()
            }),
        }],
        Actor::user("bevy-import"),
        "Import ECS snapshot",
        None,
    )?;
    let initial = author.project().clone();
    let mut peer = CommandBus::from_crdt(&author.export_crdt()?)?;
    author.execute(
        vec![Command::RenameEntity {
            scene_id: scene_id.clone(),
            entity_id: entity_id.clone(),
            name: "Moved cube".into(),
        }],
        Actor::user("client-a"),
        "Rename",
        None,
    )?;
    peer.execute(
        vec![Command::SetComponent {
            scene_id: scene_id.clone(),
            entity_id: entity_id.clone(),
            component: "Velocity".into(),
            value: json!({"linear":[0.,0.,0.]}),
        }],
        Actor::user("client-b"),
        "Stop",
        None,
    )?;
    let a = author.export_crdt()?;
    let b = peer.export_crdt()?;
    author.merge(&b, Actor::user("client-b"))?;
    peer.merge(&a, Actor::user("client-a"))?;
    assert_eq!(author.project(), peer.project());
    let entity = &author.project().scenes[&scene_id].entities[&entity_id];
    assert_eq!(entity.name, "Moved cube");
    assert_eq!(entity.components["Velocity"]["linear"], json!([0., 0., 0.]));
    let text = author.project().canonical_text()?;
    assert_eq!(Project::from_text(&text)?.canonical_text()?, text);
    assert_eq!(
        initial.scenes[&scene_id].entities[&entity_id].components["Transform"]["translation"],
        json!(position)
    );
    let directory = std::env::args_os()
        .nth(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "artifacts/collaboration".into());
    std::fs::create_dir_all(&directory)?;
    std::fs::write(directory.join("merged.incant.json"), &text)?;
    std::fs::write(
        directory.join("reloaded.incant.json"),
        Project::from_text(&text)?.canonical_text()?,
    )?;
    println!(
        "{}",
        json!({"bevy_ticks":120,"position":position,"two_client_convergence":true,"canonical_text_identical":true,"directory":directory})
    );
    Ok(())
}

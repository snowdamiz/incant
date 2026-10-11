use incant_doc::{Asset, Entity, Project, Scene, Transform};
use serde_json::json;
#[test]
fn a_valid_tilemap_requires_its_checked_atlas_before_authoring_preview() {
    let mut project = Project::empty("Tile preview capability");
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Tiles");
    let texture = incant_doc::new_id();
    project.assets.insert(
        texture.clone(),
        Asset {
            id: texture.clone(),
            name: "Tiles".into(),
            path: "tiles.atlas.json".into(),
            kind: "texture".into(),
            sha256: "0".repeat(64),
            import_settings: None,
        },
    );
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    entity.components.insert("Tilemap".into(),json!({"tileset":texture,"size":[1,1],"cell_size":[1,1],"layers":[{"id":incant_doc::new_id(),"cells":[null]}]}));
    scene.entities.insert(entity.id.clone(), entity);
    project.scenes.insert(scene.id.clone(), scene);
    project.validate().unwrap();
    let error = incant_render_authoring::project_input(&project, None)
        .err()
        .unwrap();
    assert!(
        error
            .to_string()
            .contains("tilemap requires a matching checked color atlas")
    );
}

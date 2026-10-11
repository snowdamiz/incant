use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Asset, AssetImportSettings, Entity, Project, Scene, TextureUsage, Transform};
use serde_json::json;
#[test]
fn tilemaps_use_atomic_typed_transactions_provenance_and_exact_history() {
    let mut project = Project::empty("Tile history");
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Map");
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    let sid = scene.id.clone();
    let eid = entity.id.clone();
    let texture = incant_doc::new_id();
    project.assets.insert(
        texture.clone(),
        Asset {
            id: texture.clone(),
            name: "Tiles".into(),
            path: "tiles.atlas.json".into(),
            kind: "texture".into(),
            sha256: "0".repeat(64),
            import_settings: Some(AssetImportSettings::Texture {
                usage: TextureUsage::Color,
            }),
        },
    );
    scene.entities.insert(eid.clone(), entity);
    project.scenes.insert(sid.clone(), scene);
    let mut bus = CommandBus::new(project).unwrap();
    let before = bus.project().canonical_text().unwrap();
    let map = json!({"tileset":texture,"size":[2,1],"cell_size":[1,1],"layers":[{"id":incant_doc::new_id(),"cells":["00000000000000000000000001",null],"solid":true}]});
    let set = |value| Command::SetComponent {
        scene_id: sid.clone(),
        entity_id: eid.clone(),
        component: "Tilemap".into(),
        value,
    };
    for (key, value) in [
        ("tileset", json!(incant_doc::new_id())),
        ("size", json!([0, 1])),
        ("cell_size", json!([0, 1])),
        ("layers", json!([])),
    ] {
        let mut bad = map.clone();
        bad[key] = value;
        assert!(
            bus.execute(
                vec![
                    Command::RenameEntity {
                        scene_id: sid.clone(),
                        entity_id: eid.clone(),
                        name: "Partial".into()
                    },
                    set(bad)
                ],
                Actor::agent("tile-test", "test"),
                "invalid map",
                None
            )
            .is_err()
        );
        assert_eq!(bus.project().canonical_text().unwrap(), before);
        assert_eq!(bus.revision(), 0);
    }
    bus.execute(
        vec![set(map)],
        Actor::agent("tile-test", "test"),
        "author tilemap",
        None,
    )
    .unwrap();
    let authored = bus.project().canonical_text().unwrap();
    let loaded = Project::from_text(&authored).unwrap();
    let entity = &loaded.scenes[&sid].entities[&eid];
    let value: incant_doc::Tilemap =
        serde_json::from_value(entity.components["Tilemap"].clone()).unwrap();
    assert_eq!(value.tileset, texture);
    assert_eq!(
        entity.provenance.as_ref().unwrap().model.as_deref(),
        Some("tile-test")
    );
    for command in [
        Command::RemoveAsset { asset_id: texture },
        Command::RemoveComponent {
            scene_id: sid,
            entity_id: eid,
            component: "Transform".into(),
        },
    ] {
        assert!(
            bus.execute(
                vec![command],
                Actor::user("test"),
                "invalid dependency removal",
                None
            )
            .is_err()
        );
        assert_eq!(bus.project().canonical_text().unwrap(), authored);
    }
    bus.undo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), before);
    bus.redo().unwrap();
    assert_eq!(bus.project().canonical_text().unwrap(), authored);
}

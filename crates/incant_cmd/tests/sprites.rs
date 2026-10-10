use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{
    Asset, AssetImportSettings, Entity, Project, Scene, SpriteRenderer, TextureUsage, Transform,
};
use serde_json::json;
#[test]
fn sprites_share_atomic_commands_reference_validation_and_exact_history() {
    let mut project = Project::empty("Sprite history");
    let mut scene = Scene::new("Main");
    let mut entity = Entity::new("Sprite");
    entity
        .components
        .insert("Transform".into(), json!(Transform::default()));
    let texture = incant_doc::new_id();
    project.assets.insert(
        texture.clone(),
        Asset {
            id: texture.clone(),
            name: "Sheet".into(),
            path: "sheet.png".into(),
            kind: "texture".into(),
            sha256: "0".repeat(64),
            import_settings: Some(AssetImportSettings::Texture {
                usage: TextureUsage::Color,
            }),
        },
    );
    let sid = scene.id.clone();
    let eid = entity.id.clone();
    scene.entities.insert(eid.clone(), entity);
    project.scenes.insert(sid.clone(), scene);
    let mut bus = CommandBus::new(project).unwrap();
    let before = bus.project().canonical_text().unwrap();
    let sprite = json!({"texture":texture,"region":[0,0,32,16],"size":[2,1],"pivot":[0.5,0.5],"tint":[1,1,1,1]});
    let set = |value| Command::SetComponent {
        scene_id: sid.clone(),
        entity_id: eid.clone(),
        component: "SpriteRenderer".into(),
        value,
    };
    for (key, value) in [
        ("texture", json!(incant_doc::new_id())),
        ("region", json!([4294967295u32, 0, 1, 1])),
        ("region", json!([0, 0, 0, 16])),
        ("size", json!([0, 1])),
        ("pivot", json!([2, 0])),
        ("tint", json!([1, 1, 1, 1.01])),
        ("mode", json!({"kind":"unknown"})),
        (
            "mode",
            json!({"kind":"nine_slice","border":[30,30,0,0],"pixels_per_unit":32}),
        ),
    ] {
        let mut bad = sprite.clone();
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
                Actor::agent("sprite-test", "test"),
                "invalid sprite",
                None
            )
            .is_err()
        );
        assert_eq!(bus.project().canonical_text().unwrap(), before);
        assert_eq!(bus.revision(), 0);
    }
    bus.execute(
        vec![set(sprite)],
        Actor::agent("sprite-test", "test"),
        "author sprite",
        None,
    )
    .unwrap();
    let authored = bus.project().canonical_text().unwrap();
    let loaded = Project::from_text(&authored).unwrap();
    let entity = &loaded.scenes[&sid].entities[&eid];
    let value: SpriteRenderer =
        serde_json::from_value(entity.components["SpriteRenderer"].clone()).unwrap();
    assert_eq!(value.texture, texture);
    assert_eq!(
        entity.provenance.as_ref().unwrap().model.as_deref(),
        Some("sprite-test")
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

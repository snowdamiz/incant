use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Asset, Entity, MeshRenderer, Origin, Project, Scene, new_id};
use serde_json::json;

fn asset() -> Asset {
    Asset {
        id: new_id(),
        name: "Lighthouse".into(),
        path: "assets/lighthouse.glb".into(),
        kind: "model".into(),
        import_settings: None,
        sha256: "ab".repeat(32),
    }
}

#[test]
fn environment_binding_and_reimport_are_atomic_and_reversible() {
    let mut bus = CommandBus::new(Project::empty("Environment transaction")).unwrap();
    let mut source = asset();
    source.kind = "texture".into();
    source.path = "studio.exr".into();
    let mut scene = Scene::new("World");
    let mut entity = Entity::new("Studio");
    entity.components.insert(
        "EnvironmentLight".into(),
        json!(incant_doc::EnvironmentLight {
            texture: source.id.clone(),
            intensity: 2.,
            rotation_degrees: 90.,
        }),
    );
    scene.entities.insert(entity.id.clone(), entity);
    bus.execute(
        vec![
            Command::UpsertAsset {
                asset: source.clone(),
            },
            Command::CreateScene { scene },
        ],
        Actor::user("test"),
        "Add environment",
        Some(0),
    )
    .unwrap();
    let first = bus.project().clone();
    source.sha256 = "cd".repeat(32);
    bus.execute(
        vec![Command::UpsertAsset {
            asset: source.clone(),
        }],
        Actor::import("image"),
        "Reimport environment",
        None,
    )
    .unwrap();
    let second = bus.project().clone();
    bus.undo().unwrap();
    assert_eq!(bus.project(), &first);
    bus.redo().unwrap();
    assert_eq!(bus.project(), &second);
    source.import_settings = Some(incant_doc::AssetImportSettings::Texture {
        usage: incant_doc::TextureUsage::Normal,
    });
    assert!(
        bus.execute(
            vec![Command::UpsertAsset {
                asset: source.clone()
            }],
            Actor::import("image"),
            "Invalid interpretation",
            None
        )
        .is_err()
    );
    assert!(
        bus.execute(
            vec![Command::RemoveAsset {
                asset_id: source.id
            }],
            Actor::user("test"),
            "Remove referenced environment",
            None
        )
        .is_err()
    );
    assert_eq!(bus.project(), &second);
    assert_eq!(bus.history().len(), 2);
}

#[test]
fn import_with_references_is_one_reversible_transaction() {
    let before = Project::empty("import test");
    let mut bus = CommandBus::new(before.clone()).unwrap();
    let asset = asset();
    let mut scene = Scene::new("world");
    let mut entity = Entity::new("Lighthouse");
    entity.components.insert(
        "MeshRenderer".into(),
        json!(MeshRenderer {
            mesh: asset.id.clone(),
            materials: vec![],
            cast_shadows: true,
        }),
    );
    scene.entities.insert(entity.id.clone(), entity);
    bus.execute(
        vec![
            Command::UpsertAsset {
                asset: asset.clone(),
            },
            Command::CreateScene { scene },
        ],
        Actor::import("glTF"),
        "Import lighthouse",
        Some(0),
    )
    .unwrap();
    let imported = bus.project().clone();
    assert_eq!(bus.history().len(), 1);
    assert_eq!(bus.history()[0].actor.origin, Origin::Import);
    bus.undo().unwrap();
    assert_eq!(bus.project(), &before);
    bus.redo().unwrap();
    assert_eq!(bus.project(), &imported);
    assert!(
        bus.execute(
            vec![Command::RemoveAsset { asset_id: asset.id }],
            Actor::user("test"),
            "Remove referenced asset",
            None,
        )
        .is_err()
    );
    assert_eq!(bus.project(), &imported);
    assert_eq!(bus.history().len(), 1);
}

#[test]
fn asset_reimport_preserves_identity_and_undo_restores_previous_hash() {
    let mut bus = CommandBus::new(Project::empty("reimport")).unwrap();
    let mut source = asset();
    bus.execute(
        vec![Command::UpsertAsset {
            asset: source.clone(),
        }],
        Actor::import("glTF"),
        "Import",
        None,
    )
    .unwrap();
    let before = bus.project().clone();
    source.sha256 = "cd".repeat(32);
    bus.execute(
        vec![Command::UpsertAsset {
            asset: source.clone(),
        }],
        Actor::import("glTF"),
        "Reimport",
        None,
    )
    .unwrap();
    assert_eq!(bus.project().assets.len(), 1);
    assert_eq!(bus.project().assets[&source.id].sha256, source.sha256);
    bus.undo().unwrap();
    assert_eq!(bus.project(), &before);
    bus.execute(
        vec![Command::RemoveAsset {
            asset_id: source.id,
        }],
        Actor::user("test"),
        "Remove unused asset",
        None,
    )
    .unwrap();
    assert!(bus.project().assets.is_empty());
    bus.undo().unwrap();
    assert_eq!(bus.project(), &before);
}

#[test]
fn invalid_import_metadata_cannot_partially_modify_a_project() {
    let mut bus = CommandBus::new(Project::empty("rejection")).unwrap();
    let before = bus.project().clone();
    let mut invalid = asset();
    invalid.path = "../outside.glb".into();
    assert!(
        bus.execute(
            vec![Command::UpsertAsset { asset: invalid }],
            Actor::import("glTF"),
            "Invalid import",
            None
        )
        .is_err()
    );
    for field in ["name", "kind", "sha256"] {
        let mut invalid = asset();
        match field {
            "name" => invalid.name.clear(),
            "kind" => invalid.kind.clear(),
            _ => invalid.sha256 = "not-a-digest".into(),
        }
        assert!(
            bus.execute(
                vec![Command::UpsertAsset { asset: invalid }],
                Actor::import("glTF"),
                "Invalid import",
                None
            )
            .is_err()
        );
    }
    assert_eq!(bus.project(), &before);
    assert_eq!(bus.revision(), 0);
    assert!(bus.history().is_empty());
}

#[test]
fn texture_import_options_are_validated_and_undoable() {
    use incant_doc::{AssetImportSettings, TextureUsage};
    let mut source = asset();
    source.import_settings = Some(AssetImportSettings::Texture {
        usage: TextureUsage::Normal,
    });
    let mut bus = CommandBus::new(Project::empty("settings")).unwrap();
    assert!(
        bus.execute(
            vec![Command::UpsertAsset {
                asset: source.clone()
            }],
            Actor::import("texture"),
            "Invalid settings",
            None
        )
        .is_err()
    );
    source.kind = "texture".into();
    bus.execute(
        vec![Command::UpsertAsset {
            asset: source.clone(),
        }],
        Actor::import("texture"),
        "Import normal",
        None,
    )
    .unwrap();
    let before = bus.project().clone();
    source.import_settings = Some(AssetImportSettings::Texture {
        usage: TextureUsage::Color,
    });
    bus.execute(
        vec![Command::UpsertAsset { asset: source }],
        Actor::import("texture"),
        "Change usage",
        None,
    )
    .unwrap();
    bus.undo().unwrap();
    assert_eq!(bus.project(), &before);
}

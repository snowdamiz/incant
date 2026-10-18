use super::support::Fixture;
use incant_assets::{CacheKind, project_cache_directory};
use incant_doc::{SpriteMode, SpriteRegion, Transform};
use incant_import::{AlphaColliderError as E, ImportSnapshot};
use serde_json::json;
use std::fs;

#[test]
fn empty_threshold_complexity_and_selected_pixel_bounds_refuse_without_edits() {
    let empty = Fixture::new(2, &[0; 4], false);
    assert!(matches!(empty.prepare(1), Err(E::EmptyMask)));
    assert!(matches!(empty.prepare(0), Err(E::Threshold)));
    let checker: Vec<_> = (0..17 * 17)
        .map(|i| if (i / 17 + i % 17) % 2 == 0 { 255 } else { 0 })
        .collect();
    let f = Fixture::new(17, &checker, false);
    let before = f.bus.project().clone();
    let history = f.bus.history().to_vec();
    assert!(matches!(f.prepare(128), Err(E::RectangleLimit)));
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.history(), history);
    let checker: Vec<_> = (0..16 * 8)
        .map(|i| if (i / 16 + i % 16) % 2 == 0 { 255 } else { 0 })
        .collect();
    let f = Fixture::new(16, &checker, false);
    assert_eq!(f.prepare(128).unwrap().derivation().rectangles.len(), 64);
    let mut f = Fixture::new(1025, &vec![255; 1025 * 1024], false);
    assert!(matches!(f.prepare(128), Err(E::PixelLimit)));
    let mut sprite = f.sprite();
    sprite.region = SpriteRegion::Pixels([0, 0, 1024, 1024]);
    f.component("SpriteRenderer", json!(sprite));
    assert_eq!(
        f.prepare(128).unwrap().derivation().solid_pixels,
        1024 * 1024
    );
}

#[test]
fn unsupported_transform_and_sprite_semantics_refuse_and_dimensions_are_validated() {
    let mut f = Fixture::new(2, &[255; 4], false);
    for transform in [
        Transform {
            scale: [2., 1., 1.],
            ..Default::default()
        },
        Transform {
            rotation: [0.6, 0., 0., 0.8],
            ..Default::default()
        },
    ] {
        f.component("Transform", json!(transform));
        assert!(matches!(f.prepare(128), Err(E::Transform)));
    }
    f.component("Transform", json!(Transform::default()));
    let mut sprite = f.sprite();
    sprite.tint[3] = 0.5;
    f.component("SpriteRenderer", json!(sprite));
    assert!(matches!(f.prepare(128), Err(E::Sprite)));
    sprite.tint[3] = 1.;
    sprite.mode = SpriteMode::NineSlice {
        border: [0; 4],
        pixels_per_unit: 1.,
    };
    f.component("SpriteRenderer", json!(sprite));
    assert!(matches!(f.prepare(128), Err(E::Sprite)));
    sprite.mode = SpriteMode::Simple;
    sprite.region = SpriteRegion::Pixels([1, 0, 2, 2]);
    f.component("SpriteRenderer", json!(sprite));
    assert!(matches!(f.prepare(128), Err(E::Region)));
    sprite.region = SpriteRegion::Pixels([0, 0, 2, 2]);
    sprite.size = [0.0001, 1.];
    f.component("SpriteRenderer", json!(sprite));
    assert!(matches!(f.prepare(128), Err(E::Document(_))));
}

#[test]
fn extra_components_parented_sources_and_project_swap_are_not_static_snapshots() {
    let f = Fixture::new(2, &[255; 4], false);
    let mut project = f.bus.project().clone();
    project
        .scenes
        .get_mut(&f.scene)
        .unwrap()
        .entities
        .get_mut(&f.entity)
        .unwrap()
        .components
        .insert("Velocity".into(), json!({"linear":[0,0,0]}));
    let bus = incant_cmd::CommandBus::new(project).unwrap();
    assert!(matches!(
        ImportSnapshot::capture(&bus).prepare_alpha_colliders(f.root.path(), None, &f.request(128)),
        Err(E::StaticSprite)
    ));
    let mut project = f.bus.project().clone();
    let mut parent = incant_doc::Entity::new("Parent");
    parent
        .components
        .insert("Transform".into(), json!(Transform::default()));
    let scene = project.scenes.get_mut(&f.scene).unwrap();
    scene.entities.get_mut(&f.entity).unwrap().parent = Some(parent.id.clone());
    scene.entities.insert(parent.id.clone(), parent);
    let bus = incant_cmd::CommandBus::new(project).unwrap();
    assert!(matches!(
        ImportSnapshot::capture(&bus).prepare_alpha_colliders(f.root.path(), None, &f.request(128)),
        Err(E::StaticSprite)
    ));
    let prepared = f.prepare(128).unwrap();
    let mut other = incant_cmd::CommandBus::new(incant_doc::Project::empty("Other")).unwrap();
    assert!(matches!(
        prepared.commit(&mut other, incant_cmd::Actor::user("test")),
        Err(E::StaleProject)
    ));
}

#[test]
fn corrupted_cooked_bytes_refuse_even_with_original_source_present() {
    let f = Fixture::new(2, &[255; 4], false);
    let cache = project_cache_directory(f.root.path(), CacheKind::Textures).unwrap();
    let prepared = f.prepare(128).unwrap();
    let hash = &prepared.derivation().texture_content_sha256;
    fs::write(cache.join(format!("{hash}.ktx2")), b"broken payload").unwrap();
    assert!(matches!(f.prepare(128), Err(E::Cache(_))));
    assert!(f.root.path().join("mask.png").exists());
    fs::remove_file(cache.join(format!("{hash}.ktx2"))).unwrap();
    assert!(matches!(f.prepare(128), Err(E::Cache(_))));
}

#[test]
fn missing_atlas_frame_and_high_precision_alpha_are_explicit_refusals() {
    let mut f = Fixture::new(2, &[255; 4], true);
    let mut sprite = f.sprite();
    sprite.region = SpriteRegion::Atlas {
        frame: "00000000000000000000000003".into(),
    };
    f.component("SpriteRenderer", json!(sprite));
    assert!(matches!(f.prepare(128), Err(E::Region)));
    let mut f = Fixture::new(2, &[255; 4], false);
    let image = image::ImageBuffer::from_pixel(2, 2, image::Rgba([u16::MAX; 4]));
    image.save(f.root.path().join("mask.png")).unwrap();
    ImportSnapshot::capture(&f.bus)
        .prepare(
            f.root.path(),
            None,
            &[incant_import::ImportRequest {
                source: "mask.png".into(),
                texture_usage: None,
            }],
        )
        .unwrap()
        .commit(
            &mut f.bus,
            incant_cmd::Actor::import("test"),
            "Import high precision source",
        )
        .unwrap();
    assert!(matches!(f.prepare(128), Err(E::Texture)));
}

#[test]
fn linked_prefab_sources_refuse_without_detaching_or_editing_the_instance() {
    use incant_cmd::{Actor, Command};
    use incant_doc::new_id;
    use std::collections::BTreeMap;
    let mut f = Fixture::new(2, &[255; 4], false);
    let prefab = new_id();
    let node = new_id();
    let instance = new_id();
    f.bus
        .execute(
            vec![
                Command::CreatePrefab {
                    scene_id: f.scene.clone(),
                    root_entity_id: f.entity.clone(),
                    prefab_id: prefab.clone(),
                    name: "Static sprite".into(),
                    nodes: BTreeMap::from([(f.entity.clone(), node.clone())]),
                },
                Command::InstantiatePrefab {
                    source: prefab,
                    scene_id: f.scene.clone(),
                    parent: None,
                    entities: BTreeMap::from([(node, instance.clone())]),
                },
            ],
            Actor::user("test"),
            "Create linked sprite",
            None,
        )
        .unwrap();
    f.entity = instance;
    let before = f.bus.project().clone();
    let history = f.bus.history().to_vec();
    assert!(matches!(f.prepare(128), Err(E::StaticSprite)));
    assert_eq!(f.bus.project(), &before);
    assert_eq!(f.bus.history(), history);
}

#![cfg(feature = "scene")]
use incant_cook::{CookError, cook_scene, stable_id};
use incant_doc::{Asset, Entity, Project, Scene, new_id};
use incant_runtime::{CookedScene, NativeWorld, VisualAssetKind};
use serde_json::json;
fn fixture() -> (Project, String, String, String, String) {
    let mut project = Project::empty("Typed spatial");
    let mut scene = Scene::new("Main");
    let model = new_id();
    let texture = new_id();
    for (id, kind) in [(&model, "model"), (&texture, "texture")] {
        project.assets.insert(
            id.clone(),
            Asset {
                id: id.clone(),
                name: kind.into(),
                path: format!("absent/{kind}"),
                kind: kind.into(),
                sha256: "Ab".repeat(32),
                import_settings: None,
            },
        );
    }
    let mut a = Entity::new("Camera and mesh");
    let entity = a.id.clone();
    a.components.insert("Camera".into(),json!({"fov_degrees":60,"near":0.1,"far":100,"projection":{"kind":"pixel_perfect","pixels_per_unit":32,"reference_resolution":[320,180]}}));
    a.components.insert(
        "MeshRenderer".into(),
        json!({"mesh":model,"materials":[],"cast_shadows":true}),
    );
    a.components.insert(
        "DirectionalLight".into(),
        json!({"color":[1,0.5,0.25],"intensity":100,"shadows":{"distance":20}}),
    );
    let mut b = Entity::new("Child and environment");
    b.parent = Some(entity.clone());
    b.components.insert(
        "MeshRenderer".into(),
        json!({"mesh":model,"materials":[],"cast_shadows":false}),
    );
    b.components.insert(
        "EnvironmentLight".into(),
        json!({"texture":texture,"intensity":1,"rotation_degrees":20}),
    );
    scene.entities.insert(a.id.clone(), a);
    scene.entities.insert(b.id.clone(), b);
    let sid = scene.id.clone();
    project.scenes.insert(sid.clone(), scene);
    (project, sid, entity, model, texture)
}
#[test]
fn cook_preserves_projection_hierarchy_and_deduplicates_binary_asset_versions_without_source_io() {
    let (project, sid, entity, model, texture) = fixture();
    let before = serde_json::to_vec(&project).unwrap();
    let bytes = cook_scene(&project, &sid).unwrap();
    assert_eq!(serde_json::to_vec(&project).unwrap(), before);
    drop(project);
    let cooked = CookedScene::from_bytes(&bytes).unwrap();
    assert_eq!(cooked.visual_assets().len(), 2);
    for (id, kind) in [
        (model, VisualAssetKind::Model),
        (texture, VisualAssetKind::Texture),
    ] {
        let asset = cooked
            .visual_assets()
            .iter()
            .find(|a| a.id == stable_id(&id).unwrap())
            .unwrap();
        assert_eq!(asset.kind, kind);
        assert_eq!(asset.fingerprint, [0xab; 32]);
    }
    let world = NativeWorld::new(cooked);
    let view = world.spatial_view().unwrap();
    assert_eq!(view.nodes().len(), 2);
    let camera = view
        .nodes()
        .find(|n| n.id == stable_id(&entity).unwrap())
        .unwrap();
    assert!(matches!(
        camera
            .declaration
            .unwrap()
            .camera
            .as_ref()
            .unwrap()
            .projection,
        incant_doc::CameraProjection::PixelPerfect {
            pixels_per_unit: 32.,
            reference_resolution: [320, 180]
        }
    ));
    assert_eq!(view.nodes().filter(|n| n.parent_index.is_some()).count(), 1);
}
#[test]
fn nonempty_material_overrides_and_invalid_asset_recipes_fail_explicitly() {
    let (mut project, sid, entity, _, _) = fixture();
    let material = new_id();
    project.assets.insert(
        material.clone(),
        Asset {
            id: material.clone(),
            name: "Material".into(),
            path: "absent/mat".into(),
            kind: "material".into(),
            sha256: "00".repeat(32),
            import_settings: None,
        },
    );
    project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&entity)
        .unwrap()
        .components
        .get_mut("MeshRenderer")
        .unwrap()["materials"] = json!([material]);
    assert!(
        matches!(cook_scene(&project,&sid),Err(CookError::MaterialOverrides(id)) if id==entity)
    );
    let (mut project, sid, _, model, _) = fixture();
    project.assets.get_mut(&model).unwrap().sha256 = "zz".repeat(32);
    assert!(matches!(
        cook_scene(&project, &sid),
        Err(CookError::Document(_))
    ));
}

#[test]
fn cooked_scene_binds_real_sealed_asset_after_sources_are_removed_and_rejects_stale_bytes() {
    use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData};
    use std::{fs, path::Path};
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    fs::create_dir(&source).unwrap();
    let cache = temp.path().join("cache");
    let vertices: [f32; 9] = [0., 0., 0., 1., 0., 0., 0., 1., 0.];
    fs::write(
        source.join("tri.bin"),
        vertices
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    fs::write(source.join("tri.gltf"),json!({"asset":{"version":"2.0"},"buffers":[{"byteLength":36,"uri":"tri.bin"}],"bufferViews":[{"buffer":0,"byteLength":36}],"accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]}],"meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],"nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0}).to_string()).unwrap();
    let model = incant_assets::cook_gltf(&source, Path::new("tri.gltf"), &cache).unwrap();
    let (mut project, sid, entity, model_id, _) = fixture();
    // Retain one mesh declaration: no environment file is needed for this proof.
    project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .retain(|id, _| id == &entity);
    project.assets.get_mut(&model_id).unwrap().sha256 = model.metadata.fingerprint.clone();
    let bytes = cook_scene(&project, &sid).unwrap();
    drop(project);
    drop(model);
    fs::remove_dir_all(&source).unwrap();
    let world = NativeWorld::from_bytes(&bytes).unwrap();
    let view = world.spatial_view().unwrap();
    let descriptor = view.assets()[0];
    let retained = RuntimeAsset::load_visual(&descriptor, 0, &cache).unwrap();
    let RuntimeAssetData::Model(model) = retained.data() else {
        panic!("model binding")
    };
    assert_eq!(model.meshes[0].indices.len(), 3);
    assert_eq!(
        view.nodes()
            .next()
            .unwrap()
            .declaration
            .unwrap()
            .mesh
            .as_ref()
            .unwrap()
            .mesh,
        descriptor.id
    );
    let mut stale = descriptor;
    stale.fingerprint[0] ^= 1;
    assert!(RuntimeAsset::load_visual(&stale, 1, &cache).is_err());
    let mut wrong = descriptor;
    wrong.kind = VisualAssetKind::Texture;
    assert!(RuntimeAsset::load_visual(&wrong, 1, &cache).is_err());
    let path = cache.join(format!("{}.incmodel", retained.info().fingerprint));
    let mut damaged = fs::read(&path).unwrap();
    damaged[48] ^= 1;
    fs::write(path, damaged).unwrap();
    assert!(RuntimeAsset::load_visual(&descriptor, 1, &cache).is_err());
    // Rejected replacements cannot mutate the retained version or native view.
    let RuntimeAssetData::Model(model) = retained.data() else {
        unreachable!()
    };
    assert_eq!(model.meshes[0].indices.len(), 3);
    assert_eq!(view.binding_generation(), 0);
}

#[test]
fn native_cook_uses_binary_sprite_references_and_rejects_ambiguous_authored_bindings() {
    let (mut project, sid, entity, _, texture) = fixture();
    let frame = "00000000000000000000000001";
    project.scenes.get_mut(&sid).unwrap().entities.get_mut(&entity).unwrap().components.insert("SpriteRenderer".into(),json!({"texture":texture,"region":{"frame":frame},"size":[2,3],"pivot":[0.25,0.75],"tint":[1,0.5,0.25,0.75],"flip_x":true,"layer":-2,"order":17,"mode":{"kind":"nine_slice","border":[1,2,3,4],"pixels_per_unit":16}}));
    assert!(matches!(
        cook_scene(&project, &sid),
        Err(CookError::Document(_))
    ));
    let components = &mut project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&entity)
        .unwrap()
        .components;
    components.remove("MeshRenderer");
    components.insert("Transform".into(), json!(incant_doc::Transform::default()));
    let bytes = cook_scene(&project, &sid).unwrap();
    let scene = CookedScene::from_bytes(&bytes).unwrap();
    let node = scene
        .spatial()
        .iter()
        .find(|n| n.entity == stable_id(&entity).unwrap())
        .unwrap();
    assert!(node.mesh.is_none());
    let sprite = node.sprite.as_ref().unwrap();
    assert_eq!(sprite.texture, stable_id(&texture).unwrap());
    assert_eq!(
        sprite.region,
        incant_runtime::NativeSpriteRegion::Atlas {
            frame: stable_id(frame).unwrap()
        }
    );
    assert_eq!(sprite.layer, -2);
    assert_eq!(sprite.order, 17);
    assert!(sprite.flip_x);
}

#[test]
fn tilemap_cook_preserves_binary_ids_local_cells_and_explicit_collision_refusal() {
    let (mut project, sid, entity, _, texture) = fixture();
    let layer = "00000000000000000000000003";
    let frame = "00000000000000000000000001";
    let components = &mut project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&entity)
        .unwrap()
        .components;
    components.insert("Tilemap".into(), json!({"tileset":texture,"size":[2,1],"cell_size":[0.5,2],"layers":[{"id":layer,"cells":[frame,null],"layer":-4,"order":7,"solid":true}]}));
    assert!(matches!(
        cook_scene(&project, &sid),
        Err(CookError::Document(_))
    ));
    let components = &mut project
        .scenes
        .get_mut(&sid)
        .unwrap()
        .entities
        .get_mut(&entity)
        .unwrap()
        .components;
    components.remove("MeshRenderer");
    components.insert("Transform".into(), json!(incant_doc::Transform::default()));
    let before = project.clone();
    let bytes = cook_scene(&project, &sid).unwrap();
    assert_eq!(project, before);
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 6);
    drop(project);
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    let view = world.spatial_view().unwrap();
    let node = view
        .nodes()
        .find(|node| node.id == stable_id(&entity).unwrap())
        .unwrap();
    let declared = node.declaration.unwrap().tilemap.as_ref().unwrap();
    assert_eq!(declared.tileset, stable_id(&texture).unwrap());
    assert_eq!(declared.layers[0].id, stable_id(layer).unwrap());
    assert_eq!(
        declared.layers[0].cells,
        [Some(stable_id(frame).unwrap()), None]
    );
    assert_eq!(
        (declared.layers[0].layer, declared.layers[0].order),
        (-4, 7)
    );
    let prepared = node.tilemap.unwrap();
    assert_eq!(prepared.cell_bounds([1, 0]), [[0.5, 0.], [1., 2.]]);
    assert_eq!(prepared.collision()[0].size, [1, 1]);
    assert_eq!(
        world.step(),
        Err(incant_runtime::RuntimeError::TilemapPhysicsUnsupported)
    );
    assert_eq!(world.tick(), 0);
}

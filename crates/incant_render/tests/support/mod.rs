use incant_assets::{AssetStore, cook_gltf};
use incant_doc::{Asset, Entity, MeshRenderer, Project, Scene, Transform, new_id};
use serde_json::json;
use std::{fs, path::Path};

pub fn model(root: &Path, offset: f32) -> Asset {
    let positions: [f32; 9] = [offset - 1., -1., 0., offset + 1., -1., 0., offset, 1., 0.];
    fs::write(
        root.join("triangle.bin"),
        positions
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    fs::write(root.join("triangle.gltf"), serde_json::to_vec(&json!({
        "asset":{"version":"2.0"}, "buffers":[{"uri":"triangle.bin","byteLength":36}],
        "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[offset-1.,-1.,0.],"max":[offset+1.,1.,0.]}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
        "nodes":[{"translation":[1,0,0],"children":[1]},{"mesh":0,"translation":[0,2,0]},{"mesh":0,"translation":[-7,0,0]}],
        "scenes":[{"nodes":[2]},{"nodes":[0]}],"scene":1
    })).unwrap()).unwrap();
    let cooked = cook_gltf(
        root,
        Path::new("triangle.gltf"),
        &root.join(".incant/cache/models"),
    )
    .unwrap();
    Asset {
        id: new_id(),
        name: "Triangle".into(),
        path: "triangle.gltf".into(),
        kind: "model".into(),
        sha256: cooked.metadata.fingerprint,
        import_settings: None,
    }
}
pub fn fixture(root: &Path) -> (Project, AssetStore, String) {
    let asset = model(root, 0.);
    let id = asset.id.clone();
    let mut project = Project::empty("GPU scene");
    project.assets.insert(id.clone(), asset);
    let mut scene = Scene::new("Scene");
    for x in [-3., 0.] {
        let mut entity = Entity::new("Model instance");
        entity.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [x, 0., 0.],
                ..Default::default()
            }),
        );
        entity.components.insert(
            "MeshRenderer".into(),
            json!(MeshRenderer {
                mesh: id.clone(),
                materials: vec![],
                cast_shadows: true
            }),
        );
        scene.entities.insert(entity.id.clone(), entity);
    }
    project.scenes.insert(scene.id.clone(), scene);
    let mut assets = AssetStore::default();
    assets.sync(&project, &root.join(".incant/cache")).unwrap();
    (project, assets, id)
}

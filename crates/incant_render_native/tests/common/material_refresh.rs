//! Real importer inputs for complete model-local table replacement.
use super::fixture::{DARK, RED, Variant, model, reference};
use incant_runtime::{VisualAssetKind, VisualAssetRef};
use serde_json::json;
use std::{fs, path::Path};

#[derive(Clone, Copy)]
pub enum Tables {
    Initial,
    Rebound,
    Expanded,
    Reordered,
    Empty,
}

pub fn material_model(root: &Path, tables: Tables) -> VisualAssetRef {
    material_model_with_scale(root, tables, 1.)
}

pub fn material_model_with_scale(root: &Path, tables: Tables, scale: f32) -> VisualAssetRef {
    model(root, scale, RED, [1.; 4], Variant::Static);
    fs::write(root.join("other.png"), DARK).unwrap();
    let path = root.join("tri.gltf");
    let mut source: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    source["materials"] = json!([
        {"name":"first","pbrMetallicRoughness":{"baseColorTexture":{"index":0}}},
        {"name":"second","pbrMetallicRoughness":{"baseColorFactor":[0.2,0.6,0.4,0.5],"baseColorTexture":{"index":1}},"alphaMode":"BLEND"}
    ]);
    source["samplers"] = json!([
        {"magFilter":9728,"minFilter":9984,"wrapS":33071,"wrapT":33648},
        {"magFilter":9729,"minFilter":9987,"wrapS":10497,"wrapT":10497}
    ]);
    source["textures"] = json!([{"source":0,"sampler":0},{"source":1,"sampler":1}]);
    source["images"] = json!([{"uri":"model.png"},{"uri":"other.png"}]);
    match tables {
        Tables::Initial => {}
        Tables::Rebound => source["meshes"][0]["primitives"][0]["material"] = json!(1),
        Tables::Expanded => {
            fs::write(root.join("added.png"), RED).unwrap();
            source["images"]
                .as_array_mut()
                .unwrap()
                .push(json!({"uri":"added.png"}));
            source["textures"]
                .as_array_mut()
                .unwrap()
                .push(json!({"source":2,"sampler":1}));
            source["materials"].as_array_mut().unwrap().push(json!({
                "name":"added","pbrMetallicRoughness":{"baseColorTexture":{"index":2},"metallicRoughnessTexture":{"index":1}},
                "normalTexture":{"index":0},"occlusionTexture":{"index":1},"emissiveTexture":{"index":2},
                "emissiveFactor":[0.1,0.2,0.3],"alphaMode":"MASK","alphaCutoff":0.25,"doubleSided":true
            }));
            source["meshes"][0]["primitives"][0]["material"] = json!(2);
            source["samplers"][1]["wrapS"] = json!(33071);
        }
        Tables::Reordered => {
            for field in ["materials", "textures", "images", "samplers"] {
                source[field].as_array_mut().unwrap().reverse();
            }
        }
        Tables::Empty => {
            source["materials"] = json!([]);
            source["textures"] = json!([]);
            source["images"] = json!([]);
            source["samplers"] = json!([]);
            source["meshes"][0]["primitives"][0]
                .as_object_mut()
                .unwrap()
                .remove("material");
        }
    }
    fs::write(path, serde_json::to_vec(&source).unwrap()).unwrap();
    recook(root)
}

pub fn recook(root: &Path) -> VisualAssetRef {
    let model = incant_assets::cook_gltf(
        root,
        Path::new("tri.gltf"),
        &root.join(".incant/cache/models"),
    )
    .unwrap();
    reference(90, VisualAssetKind::Model, &model.metadata.fingerprint)
}

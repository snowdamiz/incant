use incant_assets_runtime::CookedModel;
use serde_json::{Value, json};
use std::{fs, path::Path};
#[path = "../skeletal/fixture.rs"]
mod skeletal;

/// Real glTF tracks are cooked and reloaded after their source files disappear.
pub fn cooked(edit: impl FnOnce(&mut Value, &mut [u8])) -> CookedModel {
    let dir = tempfile::tempdir().unwrap();
    let mut doc = skeletal::fixture(dir.path());
    let mut bytes = fs::read(dir.path().join("model.bin")).unwrap();
    edit(&mut doc, &mut bytes);
    fs::write(dir.path().join("model.bin"), bytes).unwrap();
    skeletal::write(dir.path(), &doc);
    let cache = dir.path().join("cache");
    let cooked = incant_assets::cook_gltf(dir.path(), Path::new("model.gltf"), &cache).unwrap();
    fs::remove_file(dir.path().join("model.gltf")).unwrap();
    fs::remove_file(dir.path().join("model.bin")).unwrap();
    let loaded = incant_assets_runtime::load_model(&cache, &cooked.metadata.fingerprint).unwrap();
    assert_eq!(cooked.animation, loaded.animation);
    dir.close().unwrap();
    loaded
}
pub fn rotation_keys(doc: &Value, bytes: &mut [u8], values: [f32; 8]) {
    let output = doc["animations"][1]["samplers"][0]["output"]
        .as_u64()
        .unwrap() as usize;
    let view = doc["accessors"][output]["bufferView"].as_u64().unwrap() as usize;
    let offset = doc["bufferViews"][view]["byteOffset"].as_u64().unwrap() as usize;
    for (i, value) in values.into_iter().enumerate() {
        bytes[offset + 4 * i..offset + 4 * i + 4].copy_from_slice(&value.to_le_bytes());
    }
}
pub fn turn(doc: &mut Value, bytes: &mut [u8]) {
    let q = std::f32::consts::FRAC_1_SQRT_2;
    rotation_keys(doc, bytes, [0., 0., 0., 1., 0., 0., q, q]);
    let sampler = doc["animations"][1]["samplers"][0].clone();
    doc["animations"][0]["samplers"]
        .as_array_mut()
        .unwrap()
        .push(sampler);
    doc["animations"][0]["channels"]
        .as_array_mut()
        .unwrap()
        .push(json!({"sampler":1,"target":{"node":1,"path":"rotation"}}));
}
pub fn cubic_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let mut doc = skeletal::fixture(dir.path());
    doc["animations"][0]["samplers"][0]["interpolation"] = json!("CUBICSPLINE");
    skeletal::write(dir.path(), &doc);
    let cache = dir.path().join("cache");
    let error = incant_assets::cook_gltf(dir.path(), Path::new("model.gltf"), &cache).unwrap_err();
    assert!(
        matches!(error, incant_assets::AssetError::Unsupported(ref message) if message == "glTF CUBICSPLINE animation")
    );
    assert!(!cache.exists() || fs::read_dir(cache).unwrap().next().is_none());
}

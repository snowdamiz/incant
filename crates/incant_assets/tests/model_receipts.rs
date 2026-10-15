#[path = "skeletal/fixture.rs"]
mod fixture;
use incant_assets_runtime::RuntimeAsset;
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

#[test]
fn sealed_model_receipt_binds_bytes_independently_of_recipe_and_process_version() {
    let dir = tempfile::tempdir().unwrap();
    fixture::fixture(dir.path());
    let cache = dir.path().join("cache");
    let cooked = incant_assets::cook_gltf(dir.path(), Path::new("model.gltf"), &cache).unwrap();
    let key = cooked.metadata.fingerprint;
    let path = cache.join(format!("{key}.incmodel"));
    let original = fs::read(&path).unwrap();
    let a = RuntimeAsset::load_model("same", 1, &cache, &key).unwrap();
    let b = RuntimeAsset::load_model("same", 1, &cache, &key).unwrap();
    assert_ne!(a.version_id(), b.version_id());
    assert_eq!(a.model_content_sha256(), b.model_content_sha256());
    assert_eq!(
        a.model_content_sha256(),
        Some(Sha256::digest(&original).into())
    );

    // Valid cooked metadata with the same recipe label is different content.
    let length = u32::from_le_bytes(original[8..12].try_into().unwrap()) as usize;
    let mut metadata: serde_json::Value =
        serde_json::from_slice(&original[12..12 + length]).unwrap();
    metadata["nodes"][0]["name"] = serde_json::json!("changed cooked content");
    let encoded = serde_json::to_vec(&metadata).unwrap();
    let mut changed = original[..8].to_vec();
    changed.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
    changed.extend_from_slice(&encoded);
    changed.extend_from_slice(&original[12 + length..original.len() - 32]);
    let checksum = Sha256::digest(&changed);
    changed.extend_from_slice(&checksum);
    fs::write(&path, &changed).unwrap();
    let c = RuntimeAsset::load_model("same", 1, &cache, &key).unwrap();
    assert_eq!(a.info().fingerprint, c.info().fingerprint);
    assert_ne!(a.model_content_sha256(), c.model_content_sha256());
    fs::remove_file(path).unwrap();
    assert_eq!(
        c.model_content_sha256(),
        Some(Sha256::digest(&changed).into())
    );
}

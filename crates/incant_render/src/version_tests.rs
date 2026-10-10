//! Distinct, valid cooked payloads may carry the same source-recipe fingerprint.
//! Rendering must retain and cache the actual loaded immutable versions.
use incant_assets_runtime::RuntimeAsset;
use incant_render_data::{MeshInput, SceneInput, SceneNode};
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Arc};

fn versions(root: &Path) -> [Arc<RuntimeAsset>; 2] {
    let first = root.join("first");
    let second = root.join("second");
    std::fs::create_dir_all(&first).unwrap();
    std::fs::create_dir_all(&second).unwrap();
    let a = crate::test_support::model(&first, 0.);
    crate::test_support::model(&second, 2.);
    let gltf = second.join("triangle.gltf");
    let mut value: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&gltf).unwrap()).unwrap();
    let primitives = value["meshes"][0]["primitives"].as_array_mut().unwrap();
    primitives.push(primitives[0].clone());
    std::fs::write(gltf, serde_json::to_vec(&value).unwrap()).unwrap();
    let cache = second.join(".incant/cache/models");
    let cooked = incant_assets::cook_gltf(&second, Path::new("triangle.gltf"), &cache).unwrap();
    let original =
        std::fs::read(cache.join(format!("{}.incmodel", cooked.metadata.fingerprint))).unwrap();
    let metadata_len = u32::from_le_bytes(original[8..12].try_into().unwrap()) as usize;
    let mut metadata = cooked.metadata;
    metadata.fingerprint = a.sha256.clone();
    let json = serde_json::to_vec(&metadata).unwrap();
    let mut rewritten = original[..8].to_vec();
    rewritten.extend((json.len() as u32).to_le_bytes());
    rewritten.extend(json);
    rewritten.extend(&original[12 + metadata_len..original.len() - 32]);
    let digest = Sha256::digest(&rewritten);
    rewritten.extend(digest);
    std::fs::write(cache.join(format!("{}.incmodel", a.sha256)), rewritten).unwrap();
    [first.join(".incant/cache/models"), cache].map(|cache| {
        Arc::new(RuntimeAsset::load_model("same label", 1, &cache, &a.sha256).unwrap())
    })
}
fn input(sources: &[Arc<RuntimeAsset>]) -> SceneInput {
    SceneInput {
        nodes: sources
            .iter()
            .enumerate()
            .map(|(index, source)| SceneNode {
                id: format!("node{index}"),
                world: glam::DMat4::IDENTITY,
                camera: None,
                light: None,
                sprite: None,
                diagnostic: false,
                mesh: Some(MeshInput {
                    source: Arc::clone(source),
                    cast_shadows: true,
                }),
            })
            .collect(),
        environment: None,
    }
}
#[test]
fn same_recipe_different_payloads_never_alias_resolved_model_plans() {
    let directory = tempfile::tempdir().unwrap();
    let sources = versions(directory.path());
    assert_eq!(sources[0].info().fingerprint, sources[1].info().fingerprint);
    assert_eq!(sources[0].info().id, sources[1].info().id);
    assert_eq!(sources[0].info().generation, sources[1].info().generation);
    assert_ne!(sources[0].version_id(), sources[1].version_id());
    assert_eq!(
        sources[0].version_id(),
        Arc::clone(&sources[0]).version_id()
    );
    directory.close().unwrap();
    let plans = crate::scene::resolve(&input(&sources)).unwrap();
    assert_eq!(plans.models.len(), 2);
    assert_eq!(plans.stats.primitive_instances, 3);
}
#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn live_gpu_versions_with_equal_recipe_keys_preserve_old_and_new_geometry() {
    let directory = tempfile::tempdir().unwrap();
    let sources = versions(directory.path());
    let renderer = crate::Renderer::headless().unwrap();
    let old = renderer.prepare(&input(&sources[..1])).unwrap();
    let before = renderer.screenshot_scene_png(&old, 160, 90).unwrap();
    let new = renderer.prepare(&input(&sources[1..])).unwrap();
    assert_eq!(old.stats().primitive_instances, 1);
    assert_eq!(new.stats().primitive_instances, 2);
    assert_ne!(
        before,
        renderer.screenshot_scene_png(&new, 160, 90).unwrap()
    );
    let combined = renderer.prepare(&input(&sources)).unwrap();
    assert_eq!(combined.stats().primitive_instances, 3);
    assert_eq!(renderer.models.lock().unwrap().len(), 2);
    assert_eq!(
        before,
        renderer.screenshot_scene_png(&old, 160, 90).unwrap()
    );
}

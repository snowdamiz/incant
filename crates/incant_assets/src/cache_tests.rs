use super::*;
use crate::{SourceSet, import_fbx};
use serde_json::json;

#[test]
fn tangent_recipe_change_reimports_both_formats_without_removing_older_caches() {
    let root = tempfile::tempdir().unwrap();
    let cache = root.path().join("cache");
    let positions = [0_f32, 0., 0., 1., 0., 0., 0., 1., 0.];
    fs::write(
        root.path().join("triangle.bin"),
        positions
            .into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    )
    .unwrap();
    fs::write(root.path().join("triangle.gltf"), serde_json::to_vec(&json!({
        "asset":{"version":"2.0"}, "buffers":[{"uri":"triangle.bin","byteLength":36}],
        "bufferViews":[{"buffer":0,"byteLength":36}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0}}]}],
        "nodes":[{"mesh":0}], "scenes":[{"nodes":[0]}], "scene":0
    })).unwrap()).unwrap();
    fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fbx/maya_cube_7500_ascii.fbx"),
        root.path().join("cube.fbx"),
    )
    .unwrap();

    for (name, previous_recipe) in [
        (
            "triangle.gltf",
            b"incant-static-gltf-v2-meshopt-0.6.2-textures-v1\0".as_slice(),
        ),
        (
            "cube.fbx",
            b"incant-static-fbx-v1-ufbx-0.11.5-c0.23.1-meters-yup-meshopt-0.6.2-textures-v1\0"
                .as_slice(),
        ),
    ] {
        let source = Path::new(name);
        let mut imported = if name.ends_with("fbx") {
            import_fbx(root.path(), source).unwrap()
        } else {
            import_gltf(root.path(), source).unwrap()
        };
        let current_key = imported.fingerprint.clone();
        let mut snapshot = SourceSet::new(root.path()).unwrap();
        for dependency in &imported.dependencies {
            snapshot.read(Path::new(&dependency.path)).unwrap();
        }
        let old_key = snapshot.fingerprint_with_salt(previous_recipe);
        // Seed a valid cooked entry under the exact previous recipe. It has
        // unchanged source dependencies, so a missing recipe bump would hit it.
        imported.fingerprint = old_key.clone();
        let previous = cook_imported(imported, &cache).unwrap();
        assert_ne!(
            current_key, old_key,
            "{name} must invalidate the previous recipe"
        );
        let fresh = cook_model(root.path(), source, &cache).unwrap();
        assert!(
            !fresh.cache_hit,
            "{name} must recook despite a valid previous entry"
        );
        assert_eq!(fresh.metadata.fingerprint, current_key);
        assert!(cook_model(root.path(), source, &cache).unwrap().cache_hit);
        fs::remove_file(root.path().join(source)).unwrap();
        assert_eq!(
            load_model(&cache, &old_key).unwrap().metadata,
            previous.metadata
        );
        assert_eq!(
            load_model(&cache, &current_key).unwrap().metadata,
            fresh.metadata
        );
    }
}

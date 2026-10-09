use incant_assets::{SourceSet, cook_gltf, cook_mesh, decode_mesh, import_gltf, load_model};
use incant_cmd::{Actor, Command, CommandBus};
use incant_doc::{Asset, Project, new_id};
use serde_json::{Value, json};
use std::{fs, path::Path};

fn triangle() -> Vec<u8> {
    [0.0_f32, 0., 0., 1., 0., 0., 0., 1., 0.]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .chain([0_u16, 1, 2].into_iter().flat_map(u16::to_le_bytes))
        .collect()
}
fn document(uri: Option<&str>) -> Value {
    let mut buffer = json!({"byteLength":42});
    if let Some(uri) = uri {
        buffer["uri"] = json!(uri);
    }
    json!({
        "asset":{"version":"2.0"},"buffers":[buffer],
        "bufferViews":[{"buffer":0,"byteOffset":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":6}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]},{"bufferView":1,"componentType":5123,"count":3,"type":"SCALAR"}],
        "materials":[{"pbrMetallicRoughness":{"baseColorFactor":[0.5,0.2,0.1,1],"metallicFactor":0.1,"roughnessFactor":0.7}}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0},"indices":1,"material":0}]}],
        "nodes":[{"name":"Triangle","mesh":0,"translation":[2,3,4]}],"scenes":[{"nodes":[0]}],"scene":0
    })
}
fn fixture(root: &Path) {
    fs::create_dir_all(root.join("models")).unwrap();
    fs::create_dir_all(root.join("buffers")).unwrap();
    fs::write(root.join("buffers/triangle.bin"), triangle()).unwrap();
    fs::write(
        root.join("models/triangle.gltf"),
        document(Some("../buffers/triangle.bin")).to_string(),
    )
    .unwrap();
}
#[test]
fn import_cook_runtime_load_and_dependency_reimport_are_reversible() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let cache = root.join("cache");
    let source = Path::new("models/triangle.gltf");
    let first = cook_gltf(root, source, &cache).unwrap();
    assert!(!first.cache_hit);
    assert_eq!(first.metadata.nodes[0].transform[3], [2., 3., 4., 1.]);
    assert_eq!(first.meshes[0].vertices[0][3..6], [0., 0., 1.]);
    assert_eq!(first.metadata.mesh_materials, vec![Some(0)]);
    assert_eq!(first.metadata.dependencies.len(), 2);
    let cached = cook_gltf(root, source, &cache).unwrap();
    assert!(cached.cache_hit);
    assert_eq!(cached.meshes, first.meshes);
    let mut bus = CommandBus::new(Project::empty("assets")).unwrap();
    let mut asset = Asset {
        id: new_id(),
        name: "Triangle".into(),
        path: "models/triangle.gltf".into(),
        kind: "model".into(),
        import_settings: None,
        sha256: first.metadata.fingerprint.clone(),
    };
    bus.execute(
        vec![Command::UpsertAsset {
            asset: asset.clone(),
        }],
        Actor::import("glTF"),
        "Import",
        None,
    )
    .unwrap();
    let before = bus.project().clone();
    let mut data = triangle();
    data[12..16].copy_from_slice(&2.0_f32.to_le_bytes());
    fs::write(root.join("buffers/triangle.bin"), data).unwrap();
    let changed = cook_gltf(root, source, &cache).unwrap();
    assert!(!changed.cache_hit);
    assert_ne!(changed.metadata.fingerprint, first.metadata.fingerprint);
    asset.sha256 = changed.metadata.fingerprint;
    bus.execute(
        vec![Command::UpsertAsset { asset }],
        Actor::import("glTF"),
        "Reimport",
        None,
    )
    .unwrap();
    assert_eq!(bus.project().assets.len(), 1);
    bus.undo().unwrap();
    assert_eq!(&before, bus.project());
    fs::remove_file(root.join("models/triangle.gltf")).unwrap();
    fs::remove_file(root.join("buffers/triangle.bin")).unwrap();
    assert_eq!(
        load_model(&cache, &first.metadata.fingerprint)
            .unwrap()
            .meshes,
        first.meshes
    );
}
#[test]
fn glb_and_embedded_buffers_produce_the_same_geometry() {
    use base64::{Engine, engine::general_purpose::STANDARD};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let data = triangle();
    let uri = format!(
        "data:application/octet-stream;base64,{}",
        STANDARD.encode(&data)
    );
    fs::write(root.join("inline.gltf"), document(Some(&uri)).to_string()).unwrap();
    let mut json = document(None).to_string().into_bytes();
    while !json.len().is_multiple_of(4) {
        json.push(b' ');
    }
    let mut bin = data;
    while !bin.len().is_multiple_of(4) {
        bin.push(0);
    }
    let mut glb = b"glTF".to_vec();
    glb.extend(2_u32.to_le_bytes());
    glb.extend(((12 + 8 + json.len() + 8 + bin.len()) as u32).to_le_bytes());
    glb.extend((json.len() as u32).to_le_bytes());
    glb.extend(b"JSON");
    glb.extend(json);
    glb.extend((bin.len() as u32).to_le_bytes());
    glb.extend(b"BIN\0");
    glb.extend(bin);
    fs::write(root.join("triangle.glb"), glb).unwrap();
    let a = import_gltf(root, Path::new("inline.gltf")).unwrap();
    let b = import_gltf(root, Path::new("triangle.glb")).unwrap();
    assert_eq!(a.meshes, b.meshes);
    assert_eq!(a.nodes, b.nodes);
}
#[test]
fn corrupt_cache_is_rejected_and_rebuilt_from_source() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let cache = root.join("cache");
    let source = Path::new("models/triangle.gltf");
    let first = cook_gltf(root, source, &cache).unwrap();
    let key = &first.metadata.fingerprint;
    let path = cache.join(format!("{key}.incmodel"));
    let mut bytes = fs::read(&path).unwrap();
    bytes[20] ^= 1;
    fs::write(&path, bytes).unwrap();
    assert!(load_model(&cache, key).is_err());
    let rebuilt = cook_gltf(root, source, &cache).unwrap();
    assert!(!rebuilt.cache_hit);
    assert_eq!(rebuilt.meshes, first.meshes);
    let mesh = cook_mesh(&first.meshes[0]).unwrap();
    for n in 0..mesh.len() {
        assert!(decode_mesh(&mesh[..n]).is_err());
    }
    let mut bad = mesh;
    bad[24] ^= 1;
    assert!(decode_mesh(&bad).is_err());
}
#[test]
fn malformed_geometry_and_unsupported_features_fail_before_cooking() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let path = root.join("models/triangle.gltf");
    for (pointer, value) in [
        ("/accessors/0/count", json!(99999999)),
        ("/bufferViews/0/byteOffset", json!(4096)),
        ("/meshes/0/primitives/0/mode", json!(1)),
        ("/nodes/0/children", json!([0])),
        ("/extensionsUsed", json!(["KHR_mesh_quantization"])),
    ] {
        let mut doc = document(Some("../buffers/triangle.bin"));
        if pointer == "/extensionsUsed" {
            doc["extensionsUsed"] = value;
        } else if pointer == "/nodes/0/children" {
            doc["nodes"][0]["children"] = value;
        } else if pointer.ends_with("mode") {
            doc["meshes"][0]["primitives"][0]["mode"] = value;
        } else {
            *doc.pointer_mut(pointer).unwrap() = value;
        }
        fs::write(&path, doc.to_string()).unwrap();
        assert!(
            import_gltf(root, Path::new("models/triangle.gltf")).is_err(),
            "{pointer}"
        );
    }
    fs::write(&path, document(Some("../buffers/triangle.bin")).to_string()).unwrap();
    let mut bytes = triangle();
    bytes[36..38].copy_from_slice(&999_u16.to_le_bytes());
    fs::write(root.join("buffers/triangle.bin"), bytes).unwrap();
    assert!(import_gltf(root, Path::new("models/triangle.gltf")).is_err());
}
#[test]
fn uri_resolution_confines_files_but_allows_project_siblings() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let mut sources = SourceSet::new(root).unwrap();
    let source = Path::new("models/triangle.gltf");
    assert_eq!(
        sources.uri(source, "../buffers/triangle%2ebin").unwrap(),
        triangle()
    );
    for uri in [
        "../../escape.bin",
        "%2e%2e/%2e%2e/escape.bin",
        "https://example.com/file",
        "file:///tmp/file",
        "/tmp/file",
        "..%5cfile",
        "other?query",
    ] {
        assert!(sources.uri(source, uri).is_err(), "{uri}");
    }
    #[cfg(unix)]
    {
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("x"), b"outside").unwrap();
        std::os::unix::fs::symlink(outside.path().join("x"), root.join("models/link")).unwrap();
        assert!(sources.uri(source, "link").is_err());
    }
}

#[test]
fn absent_normals_preserve_hard_edges_instead_of_smoothing_them() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let bytes: Vec<_> = [0_f32, 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1.]
        .into_iter()
        .flat_map(f32::to_le_bytes)
        .chain(
            [0_u16, 1, 2, 0, 3, 1]
                .into_iter()
                .flat_map(u16::to_le_bytes),
        )
        .collect();
    fs::write(root.join("mesh.bin"), bytes).unwrap();
    let mut doc = document(Some("mesh.bin"));
    doc["buffers"][0]["byteLength"] = json!(60);
    doc["bufferViews"][0]["byteLength"] = json!(48);
    doc["bufferViews"][1]["byteOffset"] = json!(48);
    doc["bufferViews"][1]["byteLength"] = json!(12);
    doc["accessors"][0]["count"] = json!(4);
    doc["accessors"][0]["max"] = json!([1, 1, 1]);
    doc["accessors"][1]["count"] = json!(6);
    fs::write(root.join("mesh.gltf"), doc.to_string()).unwrap();
    let model = import_gltf(root, Path::new("mesh.gltf")).unwrap();
    let mesh = &model.meshes[0];
    assert_eq!(mesh.vertices.len(), 6);
    assert_eq!(mesh.vertices[0][..3], mesh.vertices[3][..3]);
    assert_eq!(mesh.vertices[0][3..6], [0., 0., 1.]);
    assert_eq!(mesh.vertices[3][3..6], [0., 1., 0.]);
}

#[test]
fn version_one_caches_remain_loadable_after_texture_support() {
    use sha2::{Digest, Sha256};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    fixture(root);
    let imported = import_gltf(root, Path::new("models/triangle.gltf")).unwrap();
    let mut metadata = serde_json::to_value(incant_assets::ModelMetadata::from(&imported)).unwrap();
    metadata.as_object_mut().unwrap().remove("textures");
    let json = serde_json::to_vec(&metadata).unwrap();
    let mut bytes = b"INCMOD01".to_vec();
    bytes.extend((json.len() as u32).to_le_bytes());
    bytes.extend(json);
    bytes.extend((imported.meshes.len() as u32).to_le_bytes());
    for mesh in &imported.meshes {
        let cooked = cook_mesh(mesh).unwrap();
        bytes.extend((cooked.len() as u32).to_le_bytes());
        bytes.extend(cooked);
    }
    let hash = Sha256::digest(&bytes);
    bytes.extend(hash);
    fs::write(
        root.join(format!("{}.incmodel", imported.fingerprint)),
        bytes,
    )
    .unwrap();
    let loaded = load_model(root, &imported.fingerprint).unwrap();
    assert!(loaded.images.is_empty());
    assert!(loaded.metadata.textures.is_empty());
    assert_eq!(loaded.meshes, imported.meshes);
}

#[test]
fn native_path_separators_become_portable_dependency_names() {
    let temp = tempfile::tempdir().unwrap();
    fixture(temp.path());
    let mut sources = SourceSet::new(temp.path()).unwrap();
    assert_eq!(
        sources
            .read(&Path::new("buffers").join("triangle.bin"))
            .unwrap(),
        triangle()
    );
    assert_eq!(sources.dependencies()[0].path, "buffers/triangle.bin");
}

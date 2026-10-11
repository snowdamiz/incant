//! Cross-crate boundary tests use the real authoring cooks, then remove sources.
use incant_assets as authoring;
use incant_assets_runtime as runtime;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

fn triangle() -> runtime::Mesh {
    runtime::Mesh {
        vertices: vec![
            [0., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 1.],
            [1., 0., 0., 0., 0., 1., 1., 0., 1., 0., 0., 1.],
            [0., 1., 0., 0., 0., 1., 0., 1., 1., 0., 0., 1.],
        ],
        indices: vec![0, 1, 2],
    }
}
fn model_source(root: &Path) {
    let vertices: Vec<u8> = triangle()
        .vertices
        .into_iter()
        .flat_map(|v| {
            v[..3]
                .iter()
                .copied()
                .flat_map(f32::to_le_bytes)
                .collect::<Vec<_>>()
        })
        .collect();
    fs::write(root.join("triangle.bin"), vertices).unwrap();
    fs::write(root.join("triangle.gltf"), json!({
        "asset":{"version":"2.0"},"buffers":[{"byteLength":36,"uri":"triangle.bin"}],
        "bufferViews":[{"buffer":0,"byteLength":36}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]}],
        "materials":[{"pbrMetallicRoughness":{"baseColorFactor":[0.2,0.4,0.6,1]}}],
        "meshes":[{"primitives":[{"attributes":{"POSITION":0},"material":0}]}],
        "nodes":[{"name":"Triangle","mesh":0}],"scenes":[{"nodes":[0]}],"scene":0
    }).to_string()).unwrap();
}
fn checksum(bytes: &mut Vec<u8>) {
    let hash = Sha256::digest(&*bytes);
    bytes.extend_from_slice(&hash);
}
fn edit_model(bytes: &[u8], edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let n = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let mut metadata: Value = serde_json::from_slice(&bytes[12..12 + n]).unwrap();
    edit(&mut metadata);
    let json = serde_json::to_vec(&metadata).unwrap();
    let mut result = bytes[..8].to_vec();
    result.extend((json.len() as u32).to_le_bytes());
    result.extend(json);
    result.extend_from_slice(&bytes[12 + n..bytes.len() - 32]);
    checksum(&mut result);
    result
}
fn same_error(legacy: authoring::AssetError, native: runtime::AssetError) {
    // Compare the public typed categories as well as their observable messages.
    assert_eq!(legacy.to_string(), native.to_string());
    assert!(matches!(
        (legacy, native),
        (
            authoring::AssetError::Atlas(_),
            runtime::AssetError::Atlas(_)
        ) | (authoring::AssetError::Io(_), runtime::AssetError::Io(_))
            | (authoring::AssetError::Json(_), runtime::AssetError::Json(_))
            | (
                authoring::AssetError::Invalid(_),
                runtime::AssetError::Invalid(_)
            )
            | (
                authoring::AssetError::Unsupported(_),
                runtime::AssetError::Unsupported(_)
            )
            | (
                authoring::AssetError::Limit(_),
                runtime::AssetError::Limit(_)
            )
    ));
}

#[test]
fn cooked_model_v1_v2_readers_share_types_and_load_without_sources() {
    let temp = tempfile::tempdir().unwrap();
    model_source(temp.path());
    let cache = temp.path().join("cache");
    let cooked = authoring::cook_gltf(temp.path(), Path::new("triangle.gltf"), &cache).unwrap();
    let key = &cooked.metadata.fingerprint;
    let path = cache.join(format!("{key}.incmodel"));
    let v2 = fs::read(&path).unwrap();
    fs::remove_file(temp.path().join("triangle.gltf")).unwrap();
    fs::remove_file(temp.path().join("triangle.bin")).unwrap();
    // v1 differs only by omitting v2's (empty here) embedded image section.
    let mut v1 = v2[..v2.len() - 36].to_vec();
    v1[..8].copy_from_slice(b"INCMOD01");
    checksum(&mut v1);
    for bytes in [&v1, &v2] {
        fs::write(&path, bytes).unwrap();
        let legacy: runtime::CookedModel = authoring::load_model(&cache, key).unwrap();
        let native = runtime::load_model(&cache, key).unwrap();
        let decoded = runtime::decode_model(bytes, key).unwrap();
        assert_eq!(native.metadata, cooked.metadata);
        assert_eq!(legacy.metadata, native.metadata);
        assert_eq!(legacy.meshes, native.meshes);
        assert_eq!(decoded.meshes, native.meshes);
        assert_eq!(native.materials, cooked.materials);
        assert_eq!(native.images, cooked.images);
        assert!(native.cache_hit && legacy.cache_hit);
    }
}

#[test]
fn model_errors_keep_typed_categories_and_reject_invalid_metadata_or_streams() {
    let temp = tempfile::tempdir().unwrap();
    model_source(temp.path());
    let cache = temp.path().join("cache");
    let cooked = authoring::cook_gltf(temp.path(), Path::new("triangle.gltf"), &cache).unwrap();
    let key = cooked.metadata.fingerprint;
    let path = cache.join(format!("{key}.incmodel"));
    let original = fs::read(&path).unwrap();
    let variants = [
        edit_model(&original, |m| m["nodes"][0]["children"] = json!([0])),
        edit_model(&original, |m| m["nodes"][0]["children"] = json!([999])),
        edit_model(&original, |m| m["mesh_materials"] = json!([99])),
        edit_model(&original, |m| {
            m["materials"][0]["extensions"] = json!({"KHR_materials_unlit":{}})
        }),
        edit_model(
            &original,
            |m| m["textures"] = json!([{"color":0,"linear":null,"normal":null,"mag_filter":null,"min_filter":null,"wrap_s":10497,"wrap_t":10497}]),
        ),
    ];
    for bytes in &variants {
        fs::write(&path, bytes).unwrap();
        same_error(
            authoring::load_model(&cache, &key).unwrap_err(),
            runtime::load_model(&cache, &key).unwrap_err(),
        );
    }
    for end in 0..original.len() {
        assert!(
            runtime::decode_model(&original[..end], &key).is_err(),
            "truncated at {end}"
        );
    }
    let mut bad = original.clone();
    bad[20] ^= 1;
    fs::write(&path, bad).unwrap();
    same_error(
        authoring::load_model(&cache, &key).unwrap_err(),
        runtime::load_model(&cache, &key).unwrap_err(),
    );
    assert!(runtime::decode_model(&original, "../escape").is_err());
    fs::remove_file(path).unwrap();
    same_error(
        authoring::load_model(&cache, &key).unwrap_err(),
        runtime::load_model(&cache, &key).unwrap_err(),
    );
}

#[test]
fn mesh_and_ktx_decoding_match_the_authoring_api_and_keep_resource_limits() {
    let bytes = authoring::cook_mesh(&triangle()).unwrap();
    assert_eq!(
        authoring::decode_mesh(&bytes).unwrap(),
        runtime::decode_mesh(&bytes).unwrap()
    );
    for end in 0..bytes.len() {
        same_error(
            authoring::decode_mesh(&bytes[..end]).unwrap_err(),
            runtime::decode_mesh(&bytes[..end]).unwrap_err(),
        );
    }
    let mut huge = bytes[..bytes.len() - 32].to_vec();
    huge[8..12].copy_from_slice(&u32::MAX.to_le_bytes());
    checksum(&mut huge);
    same_error(
        authoring::decode_mesh(&huge).unwrap_err(),
        runtime::decode_mesh(&huge).unwrap_err(),
    );
    assert!(matches!(
        runtime::decode_mesh(&huge),
        Err(runtime::AssetError::Limit("cooked geometry counts"))
    ));
    for format in [
        runtime::TextureFormat::Rgba8Srgb,
        runtime::TextureFormat::Rgba8Linear,
        runtime::TextureFormat::Rgba32Float,
    ] {
        let level = if format == runtime::TextureFormat::Rgba32Float {
            [4_f32, 2., -0.5, 1.]
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect()
        } else {
            vec![32, 64, 128, 255]
        };
        let texture = runtime::Texture {
            width: 1,
            height: 1,
            format,
            levels: vec![level],
        };
        let bytes = authoring::encode_ktx2(&texture).unwrap();
        assert_eq!(runtime::decode_ktx2(&bytes).unwrap(), texture);
        assert_eq!(authoring::decode_ktx2(&bytes).unwrap(), texture);
        for end in 0..bytes.len() {
            same_error(
                authoring::decode_ktx2(&bytes[..end]).unwrap_err(),
                runtime::decode_ktx2(&bytes[..end]).unwrap_err(),
            );
        }
        let mut bad = bytes.clone();
        bad[20..24].copy_from_slice(&u32::MAX.to_le_bytes());
        same_error(
            authoring::decode_ktx2(&bad).unwrap_err(),
            runtime::decode_ktx2(&bad).unwrap_err(),
        );
    }
}

#[test]
fn ordinary_and_atlas_texture_manifests_load_without_sources_and_preserve_errors() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cache = root.join("cache");
    image::RgbaImage::from_pixel(2, 2, image::Rgba([48_u8, 96, 144, 255]))
        .save(root.join("a.png"))
        .unwrap();
    fs::write(root.join("test.atlas.json"), json!({"version":1,"id":"00000000000000000000000009","maximum_size":[16,16],"padding":1,"frames":{"00000000000000000000000001":{"name":"Frame","path":"a.png"}}}).to_string()).unwrap();
    let ordinary = authoring::cook_texture(
        root,
        Path::new("a.png"),
        &cache,
        runtime::TextureUsage::Color,
    )
    .unwrap();
    let atlas = authoring::cook_texture(
        root,
        Path::new("test.atlas.json"),
        &cache,
        runtime::TextureUsage::Color,
    )
    .unwrap();
    fs::remove_file(root.join("a.png")).unwrap();
    fs::remove_file(root.join("test.atlas.json")).unwrap();
    for cooked in [&ordinary, &atlas] {
        let key = &cooked.metadata.fingerprint;
        let path = cache.join(format!("{key}.json"));
        let mut manifest: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        if cooked.metadata.version == 1 {
            // Original standalone texture metadata predates both optional fields.
            manifest.as_object_mut().unwrap().remove("atlas");
            manifest.as_object_mut().unwrap().remove("dependencies");
            fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        }
        let legacy: runtime::CookedTexture = authoring::load_texture(&cache, key).unwrap();
        let native = runtime::load_texture(&cache, key).unwrap();
        assert_eq!(native.metadata, cooked.metadata);
        assert_eq!(native.texture, cooked.texture);
        assert_eq!(legacy.metadata, native.metadata);
        assert_eq!(legacy.texture, native.texture);
        let mut bad = manifest.clone();
        if cooked.metadata.version == 2 {
            bad["atlas"]["padding"] = json!(0);
        } else {
            bad["dependency"]["sha256"] = json!("invalid");
        }
        fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        same_error(
            authoring::load_texture(&cache, key).unwrap_err(),
            runtime::load_texture(&cache, key).unwrap_err(),
        );
        fs::write(&path, b"{").unwrap();
        same_error(
            authoring::load_texture(&cache, key).unwrap_err(),
            runtime::load_texture(&cache, key).unwrap_err(),
        );
        fs::write(&path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    }
    let key = &atlas.metadata.fingerprint;
    fs::write(
        cache.join(format!("{}.ktx2", atlas.metadata.content_sha256)),
        b"corrupt",
    )
    .unwrap();
    same_error(
        authoring::load_texture(&cache, key).unwrap_err(),
        runtime::load_texture(&cache, key).unwrap_err(),
    );
}

#[cfg(unix)]
#[test]
fn runtime_readers_reject_symlink_cache_entries_without_following_sources() {
    let temp = tempfile::tempdir().unwrap();
    let key = "a".repeat(64);
    let source = temp.path().join("untrusted");
    fs::write(&source, b"not a cache").unwrap();
    for extension in ["incmodel", "json"] {
        std::os::unix::fs::symlink(&source, temp.path().join(format!("{key}.{extension}")))
            .unwrap();
    }
    assert!(matches!(
        runtime::load_model(temp.path(), &key),
        Err(runtime::AssetError::Invalid(_))
    ));
    assert!(matches!(
        runtime::load_texture(temp.path(), &key),
        Err(runtime::AssetError::Invalid(_))
    ));
}

#[test]
fn binary_visual_bindings_load_sealed_versions_and_fail_stale_or_wrong_kind() {
    use incant_types::{StableId, VisualAssetKind, VisualAssetRef};
    let temp = tempfile::tempdir().unwrap();
    model_source(temp.path());
    let cache = temp.path().join("cache");
    let cooked = authoring::cook_gltf(temp.path(), Path::new("triangle.gltf"), &cache).unwrap();
    let key = cooked.metadata.fingerprint;
    let reference = VisualAssetRef {
        id: StableId([42; 16]),
        kind: VisualAssetKind::Model,
        fingerprint: std::array::from_fn(|i| {
            u8::from_str_radix(&key[i * 2..i * 2 + 2], 16).unwrap()
        }),
    };
    fs::remove_file(temp.path().join("triangle.gltf")).unwrap();
    fs::remove_file(temp.path().join("triangle.bin")).unwrap();
    let version = runtime::RuntimeAsset::load_visual(&reference, 7, &cache).unwrap();
    assert_eq!(version.info().fingerprint, key);
    let runtime::RuntimeAssetData::Model(model) = version.data() else {
        panic!("model binding")
    };
    assert_eq!(model.meshes[0].indices.len(), 3);
    let mut wrong = reference;
    wrong.kind = VisualAssetKind::Texture;
    assert!(runtime::RuntimeAsset::load_visual(&wrong, 8, &cache).is_err());
    wrong = reference;
    wrong.fingerprint[0] ^= 1;
    assert!(runtime::RuntimeAsset::load_visual(&wrong, 8, &cache).is_err());
    let path = cache.join(format!("{key}.incmodel"));
    let mut corrupt = fs::read(&path).unwrap();
    corrupt[48] ^= 1;
    fs::write(&path, corrupt).unwrap();
    assert!(runtime::RuntimeAsset::load_visual(&reference, 8, &cache).is_err());
    // A rejected replacement does not mutate the retained, already decoded version.
    let runtime::RuntimeAssetData::Model(model) = version.data() else {
        unreachable!()
    };
    assert_eq!(model.meshes[0].indices.len(), 3);
    assert_eq!(version.info().generation, 7);
}

//! Actual cooks and source-free readers, with independently computed RGB oracles.
use incant_assets::{TextureUsage, cook_gltf, cook_texture, encode_ktx2, load_model, load_texture};
use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, sync::Arc};

fn image(root: &Path) {
    image::RgbaImage::from_raw(2, 1, vec![255, 0, 0, 0, 0, 0, 255, 255])
        .unwrap()
        .save(root.join("map.png"))
        .unwrap();
}
fn gltf(root: &Path) {
    image(root);
    let data: Vec<u8> = [
        0_f32, 0., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1., 0., 0., 1.,
    ]
    .into_iter()
    .flat_map(f32::to_le_bytes)
    .collect();
    fs::write(root.join("mesh.bin"), data).unwrap();
    let materials = vec![
        json!({"pbrMetallicRoughness":{"baseColorTexture":{"index":0}}}),
        json!({"alphaMode":"MASK","pbrMetallicRoughness":{"baseColorTexture":{"index":1}}}),
        json!({"alphaMode":"BLEND","pbrMetallicRoughness":{"baseColorTexture":{"index":1}}}),
        json!({"alphaMode":"BLEND","emissiveTexture":{"index":1},"emissiveFactor":[1,1,1]}),
    ];
    fs::write(root.join("model.gltf"), json!({
        "asset":{"version":"2.0"},"buffers":[{"byteLength":60,"uri":"mesh.bin"}],
        "bufferViews":[{"buffer":0,"byteLength":36},{"buffer":0,"byteOffset":36,"byteLength":24}],
        "accessors":[{"bufferView":0,"componentType":5126,"count":3,"type":"VEC3","min":[0,0,0],"max":[1,1,0]},
                     {"bufferView":1,"componentType":5126,"count":3,"type":"VEC2"}],
        "images":[{"uri":"map.png"}],"samplers":[{}, {"wrapS":33071}],
        "textures":[{"source":0,"sampler":0},{"source":0,"sampler":1}],"materials":materials,
        "meshes":[{"primitives":[{"attributes":{"POSITION":0,"TEXCOORD_0":1},"material":0}]}],
        "nodes":[{"mesh":0}],"scenes":[{"nodes":[0]}],"scene":0
    }).to_string()).unwrap();
}
fn edit_model(bytes: &[u8], version: &[u8; 8], edit: impl FnOnce(&mut Value)) -> Vec<u8> {
    let len = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let mut metadata: Value = serde_json::from_slice(&bytes[12..12 + len]).unwrap();
    edit(&mut metadata);
    let json = serde_json::to_vec(&metadata).unwrap();
    let mut output = version.to_vec();
    output.extend((json.len() as u32).to_le_bytes());
    output.extend(json);
    output.extend_from_slice(&bytes[12 + len..bytes.len() - 32]);
    output.extend_from_slice(&Sha256::digest(&output));
    output
}

fn replace_image(bytes: &[u8], image: usize, replacement: &[u8]) -> Vec<u8> {
    fn size(bytes: &[u8], at: &mut usize) -> usize {
        let n = u32::from_le_bytes(bytes[*at..*at + 4].try_into().unwrap()) as usize;
        *at += 4;
        n
    }
    let mut at = 8;
    let metadata = size(bytes, &mut at);
    at += metadata;
    let meshes = size(bytes, &mut at);
    for _ in 0..meshes {
        let n = size(bytes, &mut at);
        at += n;
    }
    let images = size(bytes, &mut at);
    assert!(image < images);
    for _ in 0..image {
        let n = size(bytes, &mut at);
        at += n;
    }
    let start = at;
    let n = size(bytes, &mut at);
    let mut output = bytes[..start].to_vec();
    output.extend((replacement.len() as u32).to_le_bytes());
    output.extend_from_slice(replacement);
    output.extend_from_slice(&bytes[at + n..bytes.len() - 32]);
    output.extend_from_slice(&Sha256::digest(&output));
    output
}

#[test]
fn checksummed_model_roles_reject_bad_indices_formats_and_mismatched_bases() {
    let temp = tempfile::tempdir().unwrap();
    gltf(temp.path());
    let cache = temp.path().join("cache");
    let cooked = cook_gltf(temp.path(), Path::new("model.gltf"), &cache).unwrap();
    let key = &cooked.metadata.fingerprint;
    let bytes = fs::read(cache.join(format!("{key}.incmodel"))).unwrap();
    for index in [json!(-1), json!(cooked.images.len()), json!(u64::MAX)] {
        let bad = edit_model(&bytes, b"INCMOD03", |m| {
            m["textures"][0]["color_unweighted"] = index
        });
        assert!(incant_assets_runtime::decode_model(&bad, key).is_err());
    }
    let coverage = cooked.materials[1].maps[0].unwrap().image;
    let paired = edit_model(&bytes, b"INCMOD03", |m| {
        m["textures"][0]["color"] = json!(coverage)
    });
    let mut changed = cooked.images[coverage].clone();
    Arc::make_mut(&mut changed.levels[0])[0] ^= 1;
    let bad = replace_image(&paired, coverage, &encode_ktx2(&changed).unwrap());
    assert!(
        incant_assets_runtime::decode_model(&bad, key)
            .unwrap_err()
            .to_string()
            .contains("base/layout mismatch")
    );
    let independent = cooked.materials[0].maps[0].unwrap().image;
    let mut wrong_format = cooked.images[independent].clone();
    wrong_format.format = incant_assets::TextureFormat::Rgba8Linear;
    let bad = replace_image(&bytes, independent, &encode_ktx2(&wrong_format).unwrap());
    assert!(incant_assets_runtime::decode_model(&bad, key).is_err());
    let legacy_with_new_field = edit_model(&bytes, b"INCMOD02", |_| {});
    assert!(incant_assets_runtime::decode_model(&legacy_with_new_field, key).is_err());
}

#[test]
fn one_source_cooks_distinct_material_roles_preserving_sampler_indices_and_old_versions() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cache = root.join("cache");
    gltf(root);
    let model = cook_gltf(root, Path::new("model.gltf"), &cache).unwrap();
    assert_eq!(model.images.len(), 2);
    let independent = model.materials[0].maps[0].unwrap();
    let coverage = model.materials[1].maps[0].unwrap();
    assert_eq!((independent.texture, coverage.texture), (0, 1));
    assert_ne!(independent.image, coverage.image);
    assert_eq!(
        &*model.images[independent.image].levels[1],
        &[188, 0, 188, 128]
    );
    assert_eq!(&*model.images[coverage.image].levels[1], &[0, 0, 255, 128]);
    assert_eq!(model.materials[2].maps[0].unwrap().image, coverage.image);
    assert_eq!(model.materials[3].maps[4].unwrap().image, independent.image);
    assert!(Arc::ptr_eq(
        &model.images[0].levels[0],
        &model.images[1].levels[0]
    ));
    let source_path = root.join("model.gltf");
    let mut reordered: Value = serde_json::from_slice(&fs::read(&source_path).unwrap()).unwrap();
    reordered["materials"].as_array_mut().unwrap().reverse();
    fs::write(&source_path, serde_json::to_vec(&reordered).unwrap()).unwrap();
    let reversed = cook_gltf(root, Path::new("model.gltf"), &cache).unwrap();
    assert_eq!(
        reversed.images, model.images,
        "material order cannot select a different role policy"
    );
    assert_eq!(reversed.metadata.textures, model.metadata.textures);
    assert_eq!(
        reversed.materials,
        model.materials.iter().cloned().rev().collect::<Vec<_>>()
    );
    let key = &model.metadata.fingerprint;
    let path = cache.join(format!("{key}.incmodel"));
    let bytes = fs::read(&path).unwrap();
    assert_eq!(&bytes[..8], b"INCMOD03");
    let invalid = edit_model(&bytes, b"INCMOD03", |metadata| {
        metadata["textures"][0]["color_unweighted"] = Value::Null;
    });
    assert!(incant_assets_runtime::decode_model(&invalid, key).is_err());
    let legacy = edit_model(&bytes, b"INCMOD02", |metadata| {
        for texture in metadata["textures"].as_array_mut().unwrap() {
            texture["color"] = json!(coverage.image);
            texture.as_object_mut().unwrap().remove("color_unweighted");
        }
    });
    for file in ["model.gltf", "mesh.bin", "map.png"] {
        fs::remove_file(root.join(file)).unwrap();
    }
    assert_eq!(load_model(&cache, key).unwrap().images, model.images);
    let old = incant_assets_runtime::decode_model(&legacy, key).unwrap();
    assert_eq!(old.materials[0].maps[0].unwrap().image, coverage.image);
    assert_eq!(&*old.images[coverage.image].levels[1], &[0, 0, 255, 128]);
    // Source-free loading preserves the old semantics instead of repairing a
    // historical immutable model behind its identity.
}

#[test]
fn standalone_color_has_checked_shared_roles_and_separate_logical_resident_and_disk_sizes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cache = root.join("cache");
    image(root);
    let cooked = cook_texture(root, Path::new("map.png"), &cache, TextureUsage::Color).unwrap();
    assert_eq!(cooked.metadata.version, 3);
    let independent = cooked.radiance_texture().unwrap();
    assert_eq!(&*cooked.texture.levels[1], &[0, 0, 255, 128]);
    assert_eq!(&*independent.levels[1], &[188, 0, 188, 128]);
    assert!(Arc::ptr_eq(
        &cooked.texture.levels[0],
        &independent.levels[0]
    ));
    let key = &cooked.metadata.fingerprint;
    let asset = RuntimeAsset::load_texture(
        "shared sprite/environment",
        1,
        &cache,
        key,
        Some(TextureUsage::Color),
    )
    .unwrap();
    assert_eq!(asset.info().payload_bytes, 24); // Two full 8+4 chains.
    assert_eq!(asset.resident_payload_bytes(), 16); // One shared 8-byte base, two 4-byte tails.
    let RuntimeAssetData::Texture(loaded) = asset.data() else {
        panic!()
    };
    assert!(Arc::ptr_eq(
        &loaded.texture.levels[0],
        &loaded.radiance_texture().unwrap().levels[0]
    ));
    let disk = fs::metadata(cache.join(format!("{}.ktx2", cooked.metadata.content_sha256)))
        .unwrap()
        .len()
        + fs::metadata(cache.join(format!(
            "{}.ktx2",
            cooked.metadata.unweighted_content_sha256.as_ref().unwrap()
        )))
        .unwrap()
        .len();
    assert_eq!(
        disk as usize,
        encode_ktx2(&cooked.texture).unwrap().len() + encode_ktx2(independent).unwrap().len()
    );
    assert!(disk > asset.info().payload_bytes as u64);
    fs::remove_file(root.join("map.png")).unwrap();
    assert_eq!(
        load_texture(&cache, key)
            .unwrap()
            .radiance_texture()
            .unwrap(),
        independent
    );
    let mut old = cooked.metadata.clone();
    old.version = 1;
    old.unweighted_content_sha256 = None;
    old.fingerprint = old.expected_key().unwrap();
    fs::write(
        cache.join(format!("{}.json", old.fingerprint)),
        serde_json::to_vec(&old).unwrap(),
    )
    .unwrap();
    let old = load_texture(&cache, &old.fingerprint).unwrap();
    assert!(old.color_unweighted.is_none());
    assert_eq!(old.radiance_texture().unwrap(), &cooked.texture);
}

#[test]
fn independent_manifest_digest_base_mismatch_and_combined_size_refuse_before_publication() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cache = root.join("cache");
    image(root);
    let cooked = cook_texture(root, Path::new("map.png"), &cache, TextureUsage::Color).unwrap();
    let key = &cooked.metadata.fingerprint;
    let manifest = cache.join(format!("{key}.json"));
    let original = fs::read(&manifest).unwrap();
    let mut bad = cooked.metadata.clone();
    bad.unweighted_content_sha256 = None;
    fs::write(&manifest, serde_json::to_vec(&bad).unwrap()).unwrap();
    assert!(
        load_texture(&cache, key)
            .unwrap_err()
            .to_string()
            .contains("independent color digest")
    );
    let independent_path = cache.join(format!(
        "{}.ktx2",
        cooked.metadata.unweighted_content_sha256.as_ref().unwrap()
    ));
    let original_ktx = fs::read(&independent_path).unwrap();
    let mut corrupt = original_ktx.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    fs::write(&manifest, &original).unwrap();
    fs::write(&independent_path, corrupt).unwrap();
    assert!(
        load_texture(&cache, key)
            .unwrap_err()
            .to_string()
            .contains("independent color cache checksum mismatch")
    );
    fs::write(independent_path, original_ktx).unwrap();
    let mut independent = cooked.color_unweighted.as_ref().unwrap().clone();
    Arc::make_mut(&mut independent.levels[0])[0] ^= 1;
    let bytes = encode_ktx2(&independent).unwrap();
    let digest = incant_assets::sha256(&bytes);
    fs::write(cache.join(format!("{digest}.ktx2")), &bytes).unwrap();
    bad.unweighted_content_sha256 = Some(digest);
    fs::write(&manifest, serde_json::to_vec(&bad).unwrap()).unwrap();
    assert!(
        load_texture(&cache, key)
            .unwrap_err()
            .to_string()
            .contains("base/layout mismatch")
    );
    let oversized_hash = "12".repeat(32);
    let oversized = fs::File::create(cache.join(format!("{oversized_hash}.ktx2"))).unwrap();
    oversized
        .set_len(incant_assets::MAX_SOURCE_BYTES as u64)
        .unwrap();
    drop(oversized);
    bad.unweighted_content_sha256 = Some(oversized_hash);
    fs::write(&manifest, serde_json::to_vec(&bad).unwrap()).unwrap();
    assert!(
        load_texture(&cache, key)
            .unwrap_err()
            .to_string()
            .contains("invalid texture cache entry")
    );
    fs::write(&manifest, original).unwrap();
    assert_eq!(load_texture(&cache, key).unwrap().texture, cooked.texture);
}

#[test]
fn old_atlas_manifest_preserves_primary_filtering_and_current_cache_requires_both_roles() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let cache = root.join("cache");
    image(root);
    fs::write(
        root.join("sheet.atlas.json"),
        json!({"version":1,"id":"00000000000000000000000009","maximum_size":[16,16],"padding":1,
        "frames":{"00000000000000000000000001":{"name":"Frame","path":"map.png"}}})
        .to_string(),
    )
    .unwrap();
    let current = cook_texture(
        root,
        Path::new("sheet.atlas.json"),
        &cache,
        TextureUsage::Color,
    )
    .unwrap();
    assert!(current.color_unweighted.is_some());
    let mut old = current.metadata.clone();
    old.version = 2;
    old.unweighted_content_sha256 = None;
    old.fingerprint = old.expected_key().unwrap();
    fs::write(
        cache.join(format!("{}.json", old.fingerprint)),
        serde_json::to_vec(&old).unwrap(),
    )
    .unwrap();
    assert_ne!(old.fingerprint, current.metadata.fingerprint);
    fs::remove_file(root.join("sheet.atlas.json")).unwrap();
    fs::remove_file(root.join("map.png")).unwrap();
    let legacy = load_texture(&cache, &old.fingerprint).unwrap();
    assert_eq!(legacy.texture, current.texture);
    assert_eq!(legacy.radiance_texture().unwrap(), &current.texture);
    let current = load_texture(&cache, &current.metadata.fingerprint).unwrap();
    assert_ne!(
        current.texture.levels.last(),
        current.radiance_texture().unwrap().levels.last()
    );
}

#[test]
fn store_budget_still_charges_both_logical_chains_despite_shared_resident_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    image(root);
    let cache = root.join(".incant/cache/textures");
    let cooked = cook_texture(root, Path::new("map.png"), &cache, TextureUsage::Color).unwrap();
    let mut bus = incant_cmd::CommandBus::new(incant_doc::Project::empty("Role budget")).unwrap();
    let asset = incant_doc::Asset {
        id: incant_doc::new_id(),
        name: "Shared".into(),
        path: "map.png".into(),
        kind: "texture".into(),
        sha256: cooked.metadata.fingerprint,
        import_settings: Some(incant_doc::AssetImportSettings::Texture {
            usage: TextureUsage::Color,
        }),
    };
    bus.execute(
        vec![incant_cmd::Command::UpsertAsset {
            asset: asset.clone(),
        }],
        incant_cmd::Actor::import("test"),
        "Import",
        None,
    )
    .unwrap();
    let mut under = incant_assets::AssetStore::with_budget(20);
    assert!(under.sync_project(bus.project(), root).is_err());
    assert!(under.get(&asset.id).is_none());
    let mut exact = incant_assets::AssetStore::with_budget(24);
    exact.sync_project(bus.project(), root).unwrap();
    assert_eq!(exact.get(&asset.id).unwrap().info().payload_bytes, 24);
    assert_eq!(exact.get(&asset.id).unwrap().resident_payload_bytes(), 16);
}

#[test]
fn role_payload_accounting_reports_actual_square_thin_and_odd_cooks() {
    let mut report = Vec::new();
    for (width, height) in [
        (1, 1),
        (2, 1),
        (4, 4),
        (1, 8192),
        (3, 5),
        (256, 256),
        (1024, 1024),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let cache = root.join("cache");
        image::RgbaImage::from_fn(width, height, |x, y| {
            if (x + y) % 2 == 0 {
                image::Rgba([255, 0, 0, 0])
            } else {
                image::Rgba([0, 0, 255, 255])
            }
        })
        .save(root.join("map.png"))
        .unwrap();
        let cooked = cook_texture(root, Path::new("map.png"), &cache, TextureUsage::Color).unwrap();
        let primary_hash = &cooked.metadata.content_sha256;
        let independent_hash = cooked.metadata.unweighted_content_sha256.as_ref().unwrap();
        let primary_file = fs::metadata(cache.join(format!("{primary_hash}.ktx2")))
            .unwrap()
            .len();
        let independent_file = fs::metadata(cache.join(format!("{independent_hash}.ktx2")))
            .unwrap()
            .len();
        let asset = RuntimeAsset::load_texture(
            "metrics",
            1,
            &cache,
            &cooked.metadata.fingerprint,
            Some(TextureUsage::Color),
        )
        .unwrap();
        let single_payload = cooked.texture.payload_bytes();
        let shared_base = (width * height * 4) as usize;
        assert_eq!(asset.info().payload_bytes, 2 * single_payload);
        assert_eq!(
            asset.resident_payload_bytes(),
            2 * single_payload - shared_base
        );
        report.push(json!({
            "size":[width, height], "single_chain_payload":single_payload,
            "logical_both_chains":asset.info().payload_bytes,
            "resident_both_chains":asset.resident_payload_bytes(),
            "resident_increment":asset.resident_payload_bytes() - single_payload,
            "primary_ktx":primary_file, "independent_ktx":independent_file,
            "serialized_both_chains":primary_file + independent_file,
            "unique_ktx_disk":primary_file + if primary_hash == independent_hash { 0 } else { independent_file },
            "manifest_bytes":fs::metadata(cache.join(format!("{}.json",cooked.metadata.fingerprint))).unwrap().len()
        }));
    }
    // Captured by the documented focused --nocapture invocation. These are exact
    // bytes of the real cook/load results, not a process-memory or GPU estimate.
    println!(
        "color_role_storage={}",
        serde_json::to_string(&report).unwrap()
    );
}

#[test]
fn actual_color_exr_cook_keeps_independent_hdr_rgb_and_shared_base_after_source_removal() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let source = root.join("hdr.exr");
    let pixels = image::ImageBuffer::<image::Rgba<f32>, Vec<f32>>::from_raw(
        3,
        1,
        vec![6., 0., 0., 0., 0., 3., 0., 0.5, 0., 0., 9., 1.],
    )
    .unwrap();
    image::DynamicImage::ImageRgba32F(pixels)
        .save(&source)
        .unwrap();
    let cache = root.join("cache");
    let cooked = cook_texture(root, Path::new("hdr.exr"), &cache, TextureUsage::Color).unwrap();
    let floats = |bytes: &[u8]| {
        bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|word| f32::from_le_bytes(*word))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        floats(&cooked.radiance_texture().unwrap().levels[1]),
        [2., 1., 3., 0.5]
    );
    assert_eq!(floats(&cooked.texture.levels[1]), [0., 1., 6., 0.5]);
    assert!(Arc::ptr_eq(
        &cooked.texture.levels[0],
        &cooked.radiance_texture().unwrap().levels[0]
    ));
    fs::remove_file(source).unwrap();
    let loaded = load_texture(&cache, &cooked.metadata.fingerprint).unwrap();
    assert_eq!(loaded.texture, cooked.texture);
    assert_eq!(
        loaded.radiance_texture().unwrap(),
        cooked.radiance_texture().unwrap()
    );
}

#[test]
fn model_four_role_expansion_preserves_existing_768_image_limit() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    gltf(root);
    let path = root.join("model.gltf");
    let mut doc: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    doc["images"] = json!(
        (0..256)
            .map(|_| json!({"uri":"map.png"}))
            .collect::<Vec<_>>()
    );
    doc["textures"] = json!((0..256).map(|i| json!({"source":i})).collect::<Vec<_>>());
    doc["materials"] = json!((0..256).flat_map(|i| [
        json!({"pbrMetallicRoughness":{"baseColorTexture":{"index":i},"metallicRoughnessTexture":{"index":i}},"normalTexture":{"index":i}}),
        json!({"alphaMode":"MASK","pbrMetallicRoughness":{"baseColorTexture":{"index":i}}}),
    ]).collect::<Vec<_>>());
    fs::write(path, serde_json::to_vec(&doc).unwrap()).unwrap();
    let error = cook_gltf(root, Path::new("model.gltf"), &root.join("cache")).unwrap_err();
    assert!(
        error.to_string().contains("model image variants"),
        "{error}"
    );
    assert!(
        !root.join("cache").exists(),
        "oversized variant set must not publish a model"
    );
}

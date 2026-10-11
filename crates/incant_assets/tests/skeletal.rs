#[path = "skeletal/fixture.rs"]
mod fixture;
use incant_assets::{cook_gltf, import_gltf};
use incant_assets_runtime::{
    animation::{AnimationPlayer, Interpolation, Property},
    decode_model,
};
use std::{fs, path::Path};
fn near(a: [f64; 3], b: [f64; 3]) {
    for (a, b) in a.into_iter().zip(b) {
        assert!((a - b).abs() < 1e-6, "{a} != {b}");
    }
}
#[test]
fn source_deleted_skin_sampling_preserves_hierarchy_weights_and_mesh_instances() {
    let dir = tempfile::tempdir().unwrap();
    fixture::fixture(dir.path());
    let cache = dir.path().join("cache");
    let cooked = cook_gltf(dir.path(), Path::new("model.gltf"), &cache).unwrap();
    let key = cooked.metadata.fingerprint.clone();
    let path = cache.join(format!("{key}.incmodel"));
    assert_eq!(&fs::read(&path).unwrap()[..8], b"INCMOD04");
    assert!(
        cook_gltf(dir.path(), Path::new("model.gltf"), &cache)
            .unwrap()
            .cache_hit
    );
    fs::remove_file(dir.path().join("model.gltf")).unwrap();
    fs::remove_file(dir.path().join("model.bin")).unwrap();
    let cooked = incant_assets_runtime::load_model(&cache, &key).unwrap();
    assert!(incant_assets_runtime::RuntimeAsset::load_model("animated", 1, &cache, &key).is_err());
    assert!(incant_assets::model_navigation_geometry(&cooked).is_err());
    // Missing normals expand in index order 2,0,1. Weights must follow exactly.
    assert_eq!(
        cooked.animation.as_ref().unwrap().influences[0]
            .iter()
            .map(|v| v.weights[0])
            .collect::<Vec<_>>(),
        vec![0.25, 1., 1.]
    );
    assert_eq!(
        cooked.animation.as_ref().unwrap().influences[0][2].joints[0],
        1
    );
    let mut player = AnimationPlayer::new(&cooked).unwrap();
    for time in [-10., 0., 0.5, 1., 1.5, 2., 10.] {
        player.sample(Some(0), time).unwrap();
        for node in [3, 4] {
            for (i, v) in cooked.meshes[0].vertices.iter().enumerate() {
                let p = [v[0], v[1], v[2]];
                near(
                    player.skin_position(node, 0, i, p).unwrap(),
                    [
                        f64::from(p[0]) + 10. + time.clamp(0., 2.),
                        f64::from(p[1]),
                        0.,
                    ],
                );
            }
            let mut local = [glam::DMat4::IDENTITY; 2];
            player.mesh_local_palette(node, &mut local).unwrap();
            for (i, m) in local.iter().enumerate() {
                let restored = player.globals()[node] * *m;
                assert!(
                    (restored - player.joint_palette(0).unwrap()[i])
                        .abs_diff_eq(glam::DMat4::ZERO, 1e-9)
                );
            }
        }
    }
    player.sample(Some(1), 1.).unwrap(); // second joint rotates90 around (11,1)
    near(
        player.skin_position(3, 0, 2, [2., 0., 0.]).unwrap(),
        [12., 2., 0.],
    );
    player.sample(Some(2), 1.999).unwrap();
    near(
        player.skin_position(4, 0, 1, [1., 0., 0.]).unwrap(),
        [11., 0., 0.],
    );
    player.sample(Some(2), 2.).unwrap();
    near(
        player.skin_position(4, 0, 2, [2., 0., 0.]).unwrap(),
        [10., 0., 0.],
    );
    let before = player.globals().to_vec();
    assert!(player.sample(Some(99), 0.).is_err());
    assert!(player.sample(Some(0), f64::NAN).is_err());
    assert_eq!(before, player.globals());
}
#[test]
fn malformed_and_unsupported_sources_refuse_without_cache_publication() {
    let dir = tempfile::tempdir().unwrap();
    let original = fixture::fixture(dir.path());
    for change in 0..7 {
        let mut doc = original.clone();
        match change {
            0 => {
                doc["animations"][0]["samplers"][0]["interpolation"] =
                    serde_json::json!("CUBICSPLINE")
            }
            1 => doc["meshes"][0]["primitives"][0]["attributes"]["JOINTS_1"] = serde_json::json!(2),
            2 => doc["skins"][0]["joints"] = serde_json::json!([1, 1]),
            3 => {
                doc["nodes"][1]["matrix"] =
                    serde_json::json!([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 1])
            }
            4 => doc["nodes"][3]["skin"] = serde_json::json!(7),
            5 => doc["skins"][0]["skeleton"] = serde_json::json!(2),
            _ => doc["animations"][0]["channels"]
                .as_array_mut()
                .unwrap()
                .push(original["animations"][0]["channels"][0].clone()),
        }
        fixture::write(dir.path(), &doc);
        let cache = dir.path().join(format!("reject{change}"));
        assert!(
            cook_gltf(dir.path(), Path::new("model.gltf"), &cache).is_err(),
            "case {change}"
        );
        assert!(!cache.exists());
    }
}
#[test]
fn direct_mutable_data_is_revalidated_and_repeated_sampling_is_stable() {
    let dir = tempfile::tempdir().unwrap();
    fixture::fixture(dir.path());
    let mut cooked = cook_gltf(
        dir.path(),
        Path::new("model.gltf"),
        &dir.path().join("cache"),
    )
    .unwrap();
    let saved = cooked.animation.clone();
    for change in 0..8 {
        cooked.animation = saved.clone();
        let a = cooked.animation.as_mut().unwrap();
        match change {
            0 => a.clips[0].channels[0].times = vec![0., 0.],
            1 => a.clips[0].channels[0].times = vec![2., 0.],
            2 => a.clips[0].channels[0].times[0] = f32::NAN,
            3 => a.influences[0][0].weights = [0.; 4],
            4 => a.influences[0][0].joints = [9; 4],
            5 => {
                a.influences[0].pop();
            }
            6 => a.clips[1].channels[0].values[1] = [0.; 4],
            _ => a.skins[0].inverse_bind[0][0][3] = 1.,
        }
        assert!(AnimationPlayer::new(&cooked).is_err(), "case {change}");
    }
    cooked.animation = saved;
    let a = cooked.animation.as_mut().unwrap();
    a.clips[1].channels[0].values[1] = [0., 0., 0., -1.];
    a.clips[2].channels[0].interpolation = Interpolation::Linear;
    assert_eq!(a.clips[1].channels[0].property, Property::Rotation);
    let mut player = AnimationPlayer::new(&cooked).unwrap();
    player.sample(Some(1), 1.).unwrap();
    let expected = player.globals().to_vec();
    for _ in 0..256 {
        player.sample(Some(2), 0.7).unwrap();
        player.sample(Some(1), 1.).unwrap();
        assert_eq!(player.globals(), expected);
    }
}
#[test]
fn animation_decoder_checks_truncation_unknown_tags_counts_and_checksums() {
    let dir = tempfile::tempdir().unwrap();
    fixture::fixture(dir.path());
    let cache = dir.path().join("cache");
    let model = cook_gltf(dir.path(), Path::new("model.gltf"), &cache).unwrap();
    let key = model.metadata.fingerprint;
    let bytes = fs::read(cache.join(format!("{key}.incmodel"))).unwrap();
    let offset = bytes.windows(8).position(|v| v == b"INCANM01").unwrap();
    let end = bytes.len() - 32;
    for n in 0..end - offset {
        assert!(
            incant_assets_runtime::animation::decode_animation(&bytes[offset..offset + n]).is_err()
        );
    }
    for (at, data) in [
        (offset + 8, vec![255; 4]),
        (offset + 12, vec![2]),
        (offset, vec![0]),
    ] {
        let mut bad = bytes.clone();
        bad[at..at + data.len()].copy_from_slice(&data);
        use sha2::{Digest, Sha256};
        let checksum = Sha256::digest(&bad[..end]);
        bad[end..].copy_from_slice(&checksum);
        assert!(decode_model(&bad, &key).is_err());
    }
    assert!(import_gltf(dir.path(), Path::new("model.gltf")).is_ok());
}
#[path = "skeletal/alloc.rs"]
mod alloc;
#[global_allocator]
static ALLOC: alloc::CountAlloc = alloc::CountAlloc;

#[test]
fn accepted_sampling_has_no_allocations_and_decoder_count_refusal_is_early() {
    let dir = tempfile::tempdir().unwrap();
    fixture::fixture(dir.path());
    let model = cook_gltf(
        dir.path(),
        Path::new("model.gltf"),
        &dir.path().join("cache"),
    )
    .unwrap();
    let mut player = AnimationPlayer::new(&model).unwrap();
    let mut palette = [glam::DMat4::IDENTITY; 2];
    let (_, count) = alloc::measured(|| {
        for i in 0..1024 {
            player.sample(Some(i % 3), i as f64 / 512.).unwrap();
            player.mesh_local_palette(4, &mut palette).unwrap();
            std::hint::black_box(player.skin_position(3, 0, 0, [1., 1., 0.]).unwrap());
        }
    });
    assert_eq!(count, 0);
    let mut huge = b"INCANM01".to_vec();
    huge.extend_from_slice(&4097u32.to_le_bytes());
    let (result, count) =
        alloc::measured(|| incant_assets_runtime::animation::decode_animation(&huge));
    assert!(matches!(
        result,
        Err(incant_assets_runtime::AssetError::Limit(_))
    ));
    assert_eq!(count, 0);
}
#[test]
fn numerical_overflow_preserves_last_complete_pose_and_palette_buffer() {
    use incant_assets_runtime::animation::{Channel, NodeTrs};
    let dir = tempfile::tempdir().unwrap();
    fixture::fixture(dir.path());
    let mut model = cook_gltf(
        dir.path(),
        Path::new("model.gltf"),
        &dir.path().join("cache"),
    )
    .unwrap();
    let data = model.animation.as_mut().unwrap();
    let first = model.metadata.nodes.len();
    model.metadata.nodes[0].children.push(first);
    for i in 0..20 {
        let n = first + i;
        model.metadata.nodes.push(incant_assets::ModelNode {
            name: None,
            children: if i == 19 { vec![] } else { vec![n + 1] },
            meshes: vec![],
            transform: glam::Mat4::IDENTITY.to_cols_array_2d(),
        });
        data.rest.push(Some(NodeTrs {
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        }));
        data.node_skins.push(None);
        data.clips[0].channels.push(Channel {
            node: n,
            property: Property::Scale,
            interpolation: Interpolation::Linear,
            times: vec![0., 1.],
            values: vec![[1., 1., 1., 0.], [1e30, 1e30, 1e30, 0.]],
        });
    }
    let mut player = AnimationPlayer::new(&model).unwrap();
    let before = player.globals().to_vec();
    assert!(player.sample(Some(0), 1.).is_err());
    assert_eq!(player.globals(), before);
    player.sample(Some(0), 0.).unwrap();
    model.metadata.nodes[4].transform[0][0] = 0.;
    model.animation.as_mut().unwrap().rest[4]
        .as_mut()
        .unwrap()
        .scale[0] = 0.;
    let player = AnimationPlayer::new(&model).unwrap();
    let mut palette = [glam::DMat4::IDENTITY; 2];
    assert!(player.mesh_local_palette(4, &mut palette).is_err());
    assert_eq!(palette, [glam::DMat4::IDENTITY; 2]);
}
#[test]
fn generated_tangent_expansion_keeps_influences_and_reimport_versions() {
    let dir = tempfile::tempdir().unwrap();
    let mut doc = fixture::fixture(dir.path());
    let mut bytes = fs::read(dir.path().join("model.bin")).unwrap();
    // Numeric unit UV/normal fields and a 1x1 neutral normal-map fixture.
    for (semantic, values, kind) in [
        (
            "NORMAL",
            vec![0_f32, 0., 1., 0., 0., 1., 0., 0., 1.],
            "VEC3",
        ),
        ("TEXCOORD_0", vec![0., 0., 1., 0., 0., 1.], "VEC2"),
    ] {
        while !bytes.len().is_multiple_of(4) {
            bytes.push(0);
        }
        let view = doc["bufferViews"].as_array().unwrap().len();
        let accessor = doc["accessors"].as_array().unwrap().len();
        let data: Vec<u8> = values.iter().flat_map(|f| f.to_le_bytes()).collect();
        doc["bufferViews"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"buffer":0,"byteOffset":bytes.len(),"byteLength":data.len()}));
        bytes.extend(data);
        doc["accessors"].as_array_mut().unwrap().push(
            serde_json::json!({"bufferView":view,"componentType":5126,"count":3,"type":kind}),
        );
        doc["meshes"][0]["primitives"][0]["attributes"][semantic] = serde_json::json!(accessor);
    }
    doc["buffers"][0]["byteLength"] = serde_json::json!(bytes.len());
    doc["images"] = serde_json::json!([{"uri":"normal.png"}]);
    doc["textures"] = serde_json::json!([{"source":0}]);
    doc["materials"] = serde_json::json!([{"normalTexture":{"index":0}}]);
    doc["meshes"][0]["primitives"][0]["material"] = serde_json::json!(0);
    image::RgbaImage::from_pixel(1, 1, image::Rgba([128, 128, 255, 255]))
        .save(dir.path().join("normal.png"))
        .unwrap();
    fs::write(dir.path().join("model.bin"), bytes).unwrap();
    fixture::write(dir.path(), &doc);
    let cache = dir.path().join("cache");
    let first = cook_gltf(dir.path(), Path::new("model.gltf"), &cache).unwrap();
    let weights = &first.animation.as_ref().unwrap().influences[0];
    assert_eq!(weights[0].weights, [0.25, 0.75, 0., 0.]);
    assert_eq!(weights[2].joints[0], 1);
    let before = fs::read(cache.join(format!("{}.incmodel", first.metadata.fingerprint))).unwrap();
    doc["animations"][0]["name"] = serde_json::json!("changed clip");
    fixture::write(dir.path(), &doc);
    let second = cook_gltf(dir.path(), Path::new("model.gltf"), &cache).unwrap();
    assert!(!second.cache_hit);
    assert_ne!(first.metadata.fingerprint, second.metadata.fingerprint);
    assert_eq!(
        fs::read(cache.join(format!("{}.incmodel", first.metadata.fingerprint))).unwrap(),
        before
    );
}
#[test]
fn fresh_process_loads_only_cooked_animation_data() {
    let dir = tempfile::tempdir().unwrap();
    fixture::fixture(dir.path());
    let cache = dir.path().join("cache");
    let model = cook_gltf(dir.path(), Path::new("model.gltf"), &cache).unwrap();
    fs::remove_file(dir.path().join("model.gltf")).unwrap();
    fs::remove_file(dir.path().join("model.bin")).unwrap();
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "source_deleted_process_child", "--nocapture"])
        .env("INCANT_SKELETAL_CACHE", &cache)
        .env("INCANT_SKELETAL_KEY", &model.metadata.fingerprint)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("SKELETAL_NUMERIC_OK [12.0, 1.0, 0.0]")
    );
}
#[test]
fn source_deleted_process_child() {
    let Some(cache) = std::env::var_os("INCANT_SKELETAL_CACHE") else {
        return;
    };
    let key = std::env::var("INCANT_SKELETAL_KEY").unwrap();
    let root = Path::new(&cache).parent().unwrap();
    assert!(!root.join("model.gltf").exists());
    assert!(!root.join("model.bin").exists());
    let model = incant_assets_runtime::load_model(Path::new(&cache), &key).unwrap();
    let mut player = AnimationPlayer::new(&model).unwrap();
    player.sample(Some(0), 1.).unwrap();
    let p = player.skin_position(3, 0, 0, [1., 1., 0.]).unwrap();
    assert_eq!(p, [12., 1., 0.]);
    println!("SKELETAL_NUMERIC_OK {p:?}");
}
#[test]
fn source_buffers_refuse_bad_key_times_weights_and_aggregate_channels() {
    let dir = tempfile::tempdir().unwrap();
    let original = fixture::fixture(dir.path());
    let bytes = fs::read(dir.path().join("model.bin")).unwrap();
    let offset = |accessor: usize| {
        let view = original["accessors"][accessor]["bufferView"]
            .as_u64()
            .unwrap() as usize;
        original["bufferViews"][view]["byteOffset"]
            .as_u64()
            .unwrap() as usize
    };
    for (at, values) in [
        (offset(5), vec![0_f32, 0.]),
        (offset(5), vec![2., 1.]),
        (offset(5), vec![-1., 2.]),
        (offset(3), vec![0.; 4]),
        (offset(3), vec![f32::INFINITY, 0., 0., 0.]),
    ] {
        let mut bad = bytes.clone();
        for (i, v) in values.iter().enumerate() {
            bad[at + i * 4..at + (i + 1) * 4].copy_from_slice(&v.to_le_bytes());
        }
        fs::write(dir.path().join("model.bin"), bad).unwrap();
        assert!(
            cook_gltf(
                dir.path(),
                Path::new("model.gltf"),
                &dir.path().join("refused")
            )
            .is_err()
        );
        assert!(!dir.path().join("refused").exists());
    }
    fs::write(dir.path().join("model.bin"), bytes).unwrap();
    let mut doc = original.clone();
    let channel = doc["animations"][0]["channels"][0].clone();
    doc["animations"][0]["channels"] = serde_json::json!(vec![channel; 8193]);
    fixture::write(dir.path(), &doc);
    assert!(matches!(
        import_gltf(dir.path(), Path::new("model.gltf")),
        Err(incant_assets::AssetError::Limit("animation channels/keys"))
    ));
}
#[test]
fn skinless_clips_and_missing_bind_matrices_have_explicit_rest_semantics() {
    let dir = tempfile::tempdir().unwrap();
    let mut doc = fixture::fixture(dir.path());
    doc["skins"][0]
        .as_object_mut()
        .unwrap()
        .remove("inverseBindMatrices");
    fixture::write(dir.path(), &doc);
    let model = cook_gltf(
        dir.path(),
        Path::new("model.gltf"),
        &dir.path().join("cache"),
    )
    .unwrap();
    let mut player = AnimationPlayer::new(&model).unwrap();
    near(
        player.skin_position(3, 0, 1, [1., 0., 0.]).unwrap(),
        [12., 0., 0.],
    );
    doc.as_object_mut().unwrap().remove("skins");
    for i in [3, 4] {
        doc["nodes"][i].as_object_mut().unwrap().remove("skin");
    }
    for s in ["JOINTS_0", "WEIGHTS_0"] {
        doc["meshes"][0]["primitives"][0]["attributes"]
            .as_object_mut()
            .unwrap()
            .remove(s);
    }
    fixture::write(dir.path(), &doc);
    let model = cook_gltf(
        dir.path(),
        Path::new("model.gltf"),
        &dir.path().join("clips"),
    )
    .unwrap();
    assert!(model.animation.as_ref().unwrap().skins.is_empty());
    player = AnimationPlayer::new(&model).unwrap();
    player.sample(Some(0), 1.).unwrap();
    near(
        player.globals()[1].w_axis.truncate().to_array(),
        [12., 0., 0.],
    );
    assert!(player.joint_palette(0).is_err());
}
#[test]
fn clip_only_scene_roots_are_revalidated_before_retained_copy() {
    let dir = tempfile::tempdir().unwrap();
    fixture::fixture(dir.path());
    let mut model = cook_gltf(
        dir.path(),
        Path::new("model.gltf"),
        &dir.path().join("cache"),
    )
    .unwrap();
    let data = model.animation.as_mut().unwrap();
    data.skins.clear();
    data.node_skins.fill(None);
    data.influences.iter_mut().for_each(Vec::clear);
    data.validate(&model.metadata.nodes, &model.meshes, &model.metadata.scenes)
        .unwrap();
    AnimationPlayer::new(&model).unwrap();

    // Public cooked values can change after decoding. Scene validation must not
    // depend on a bound skin; both root range and hierarchy still apply to clips.
    for root in [model.metadata.nodes.len(), 1] {
        model.metadata.scenes = vec![vec![root]];
        let shared = model.animation.as_ref().unwrap().validate(
            &model.metadata.nodes,
            &model.meshes,
            &model.metadata.scenes,
        );
        assert!(shared.is_err(), "shared validation accepted root {root}");
        assert!(
            AnimationPlayer::new(&model).is_err(),
            "retained player accepted root {root}"
        );
    }
}
#[test]
fn mutable_model_geometry_aggregate_is_rechecked_before_retained_copy() {
    let dir = tempfile::tempdir().unwrap();
    fixture::fixture(dir.path());
    let mut model = cook_gltf(
        dir.path(),
        Path::new("model.gltf"),
        &dir.path().join("cache"),
    )
    .unwrap();
    // Each mesh is individually within the bound; the model aggregate is not.
    let vertex = model.meshes[0].vertices[0];
    for _ in 0..2 {
        model.meshes.push(incant_assets::Mesh {
            vertices: vec![vertex; 500_000],
            indices: vec![0, 1, 2],
        });
        model.animation.as_mut().unwrap().influences.push(vec![]);
    }
    assert!(matches!(
        AnimationPlayer::new(&model),
        Err(incant_assets_runtime::AssetError::Limit(
            "total cached geometry"
        ))
    ));
}
#[test]
fn semantic_accessor_types_refuse_without_reader_panics() {
    let dir = tempfile::tempdir().unwrap();
    let original = fixture::fixture(dir.path());
    for case in 0..8 {
        let mut doc = original.clone();
        match case {
            0 => doc["accessors"][7]["componentType"] = serde_json::json!(5125), // rotation reader previously unreachable!
            1 => {
                doc["accessors"][2]["componentType"] = serde_json::json!(5125);
                doc["bufferViews"][2]["byteLength"] = serde_json::json!(48);
            }
            2 => doc["accessors"][3]["componentType"] = serde_json::json!(5125),
            3 => doc["accessors"][4]["type"] = serde_json::json!("VEC4"),
            4 => doc["accessors"][5]["componentType"] = serde_json::json!(5125),
            5 => doc["accessors"][6]["componentType"] = serde_json::json!(5125),
            6 => doc["accessors"][2]["normalized"] = serde_json::json!(true),
            _ => {
                doc["accessors"][3]["componentType"] = serde_json::json!(5123);
                doc["accessors"][3]["normalized"] = serde_json::json!(false);
            }
        }
        fixture::write(dir.path(), &doc);
        let result = std::panic::catch_unwind(|| import_gltf(dir.path(), Path::new("model.gltf")));
        assert!(result.is_ok(), "reader panicked in case{case}");
        assert!(
            result.unwrap().is_err(),
            "invalid type accepted in case{case}"
        );
    }
}
#[test]
fn normalized_integer_weights_and_byte_joints_are_imported_numerically() {
    let dir = tempfile::tempdir().unwrap();
    let original = fixture::fixture(dir.path());
    let bytes = fs::read(dir.path().join("model.bin")).unwrap();
    let j = original["bufferViews"][2]["byteOffset"].as_u64().unwrap() as usize;
    let w = original["bufferViews"][3]["byteOffset"].as_u64().unwrap() as usize;
    for (component, maximum, quarter) in [(5121, 255u16, 64u16), (5123, 65535, 16384)] {
        let mut doc = original.clone();
        let mut data = bytes.clone();
        doc["accessors"][2]["componentType"] = serde_json::json!(5121);
        data[j..j + 12].copy_from_slice(&[0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0]);
        doc["accessors"][3]["componentType"] = serde_json::json!(component);
        doc["accessors"][3]["normalized"] = serde_json::json!(true);
        let weights = [
            maximum,
            0,
            0,
            0,
            maximum,
            0,
            0,
            0,
            quarter,
            maximum - quarter,
            0,
            0,
        ];
        let encoded: Vec<u8> = if component == 5121 {
            weights.into_iter().map(|v| v as u8).collect()
        } else {
            weights.into_iter().flat_map(u16::to_le_bytes).collect()
        };
        data[w..w + encoded.len()].copy_from_slice(&encoded);
        fs::write(dir.path().join("model.bin"), data).unwrap();
        fixture::write(dir.path(), &doc);
        let cooked = cook_gltf(
            dir.path(),
            Path::new("model.gltf"),
            &dir.path().join(format!("cache{component}")),
        )
        .unwrap();
        let weights = cooked.animation.as_ref().unwrap().influences[0][0];
        assert!(
            (f64::from(weights.weights[0]) - f64::from(quarter) / f64::from(maximum)).abs() < 1e-7
        );
        let mut player = AnimationPlayer::new(&cooked).unwrap();
        player.sample(Some(0), 1.).unwrap();
        near(
            player.skin_position(3, 0, 0, [1., 1., 0.]).unwrap(),
            [12., 1., 0.],
        );
    }
}

#![cfg(not(target_arch = "wasm32"))]

use incant_assets::{ImportedModel, cook_fbx, import_fbx, load_model};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fbx")
}
#[test]
fn upstream_fixture_bytes_match_recorded_provenance_on_every_host() {
    use sha2::{Digest, Sha256};
    let root = fixtures();
    let provenance: serde_json::Value =
        serde_json::from_slice(&fs::read(root.join("provenance.json")).unwrap()).unwrap();
    let hashes = provenance["files_sha256"].as_object().unwrap();
    assert_eq!(hashes.len(), 10);
    for (name, expected) in hashes {
        let actual = format!("{:x}", Sha256::digest(fs::read(root.join(name)).unwrap()));
        assert_eq!(actual, expected.as_str().unwrap(), "{name}");
    }
}
fn globals(model: &ImportedModel) -> Vec<glam::DMat4> {
    let mut out = vec![glam::DMat4::IDENTITY; model.nodes.len()];
    let mut todo = vec![(model.scenes[0][0], glam::DMat4::IDENTITY)];
    while let Some((id, parent)) = todo.pop() {
        let n = &model.nodes[id];
        let world = parent * glam::Mat4::from_cols_array_2d(&n.transform).as_dmat4();
        out[id] = world;
        for &child in &n.children {
            todo.push((child, world));
        }
    }
    out
}
fn bounds(m: &ImportedModel) -> ([f64; 3], [f64; 3]) {
    let g = globals(m);
    let mut lo = glam::DVec3::splat(f64::INFINITY);
    let mut hi = glam::DVec3::splat(f64::NEG_INFINITY);
    for (i, n) in m.nodes.iter().enumerate() {
        for &mesh in &n.meshes {
            for v in &m.meshes[mesh].vertices {
                let p =
                    g[i].transform_point3(glam::DVec3::new(v[0] as f64, v[1] as f64, v[2] as f64));
                lo = lo.min(p);
                hi = hi.max(p);
            }
        }
    }
    (lo.to_array(), hi.to_array())
}
#[test]
fn real_ascii_and_binary_exporter_geometry_in_meters() {
    let root = fixtures();
    for (name, extent, tolerance) in [
        ("maya_cube_7500_ascii.fbx", 0.005, 1e-9),
        ("blender_272_cube_7400_binary.fbx", 1., 1e-6),
    ] {
        let model = import_fbx(&root, Path::new(name)).unwrap();
        assert_eq!(model.meshes.len(), 1);
        assert_eq!(model.meshes[0].indices.len(), 36);
        let (lo, hi) = bounds(&model);
        for axis in 0..3 {
            assert!((lo[axis] + extent).abs() < tolerance, "{name}: {lo:?}");
            assert!((hi[axis] - extent).abs() < tolerance, "{name}: {hi:?}");
        }
        for tri in model.meshes[0].vertices.as_chunks::<3>().0 {
            let v = |i: usize| glam::Vec3::from_slice(&tri[i][..3]);
            let n = glam::Vec3::from_slice(&tri[0][3..6]);
            assert!((v(1) - v(0)).cross(v(2) - v(0)).dot(n) > 0.);
        }
    }
}
#[test]
fn helper_nodes_preserve_instance_geometry_and_centimeter_hierarchy() {
    let model = import_fbx(
        &fixtures(),
        Path::new("max_geometry_transform_instances_7700_ascii.fbx"),
    )
    .unwrap();
    assert_eq!(model.meshes.len(), 1);
    assert_eq!(
        model.nodes.iter().filter(|n| !n.meshes.is_empty()).count(),
        4
    );
    let parented =
        import_fbx(&fixtures(), Path::new("maya_parented_cubes_7500_ascii.fbx")).unwrap();
    let g = globals(&parented);
    let parent = parented
        .nodes
        .iter()
        .position(|n| n.name.as_deref() == Some("Parent"))
        .unwrap();
    let child = parented
        .nodes
        .iter()
        .position(|n| n.name.as_deref() == Some("Child"))
        .unwrap();
    let origin = g[parent].transform_point3(glam::DVec3::ZERO);
    assert!((origin - glam::DVec3::new(0.001, 0.002, 0.003)).length() < 1e-9);
    let expected = g[parent].transform_point3(glam::DVec3::new(0., -1., 0.));
    assert!((g[child].transform_point3(glam::DVec3::ZERO) - expected).length() < 1e-12);
}
#[test]
fn reflected_nonuniform_units_have_independent_vertex_oracle() {
    let root = tempfile::tempdir().unwrap();
    let source = fs::read_to_string(fixtures().join("maya_cube_7500_ascii.fbx")).unwrap();
    let source = source
        .replace(
            "\"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\",1,1,1",
            "\"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\",-2,3,4",
        )
        .replace(
            "\"Lcl Translation\", \"Lcl Translation\", \"\", \"A\",0,0,0",
            "\"Lcl Translation\", \"Lcl Translation\", \"\", \"A\",100,200,300",
        );
    fs::write(root.path().join("mirror.fbx"), source).unwrap();
    let m = import_fbx(root.path(), Path::new("mirror.fbx")).unwrap();
    let (lo, hi) = bounds(&m);
    for a in 0..3 {
        assert!((lo[a] - [0.99, 1.985, 2.98][a]).abs() < 1e-6, "{lo:?}");
        assert!((hi[a] - [1.01, 2.015, 3.02][a]).abs() < 1e-6, "{hi:?}");
    }
    assert!(globals(&m)[1].determinant() < 0.);
}
#[test]
fn cooked_reader_has_no_fbx_source_requirement() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("cube.fbx");
    fs::copy(fixtures().join("maya_cube_7500_ascii.fbx"), &path).unwrap();
    let cache = root.path().join("cache");
    let a = cook_fbx(root.path(), Path::new("cube.fbx"), &cache).unwrap();
    let b = cook_fbx(root.path(), Path::new("cube.fbx"), &cache).unwrap();
    assert!(!a.cache_hit && b.cache_hit);
    assert_eq!(a.meshes, b.meshes);
    fs::remove_file(path).unwrap();
    let c = load_model(&cache, &a.metadata.fingerprint).unwrap();
    assert_eq!(a.meshes, c.meshes);
    assert_eq!(a.metadata, c.metadata);
}
#[test]
fn unsupported_material_and_malformed_data_fail_explicitly() {
    for name in [
        "maya_textured_cube_7500_ascii.fbx",
        "blender_293_embedded_textures_7400_binary.fbx",
    ] {
        let error = import_fbx(&fixtures(), Path::new(name))
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("unsupported"), "{error}");
    }
    let root = tempfile::tempdir().unwrap();
    for bytes in [
        b"".as_slice(),
        b"not FBX",
        &fs::read(fixtures().join("blender_272_cube_7400_binary.fbx")).unwrap()[..200],
    ] {
        fs::write(root.path().join("bad.fbx"), bytes).unwrap();
        assert!(import_fbx(root.path(), Path::new("bad.fbx")).is_err());
    }
}

fn textured_source(filename: &str, embedded: Option<&[u8]>) -> String {
    use base64::Engine;
    let original = fs::read_to_string(fixtures().join("maya_cube_7500_ascii.fbx")).unwrap();
    let content = embedded
        .map(|b| {
            format!(
                "Content: ,\n\"{}\"",
                base64::engine::general_purpose::STANDARD.encode(b)
            )
        })
        .unwrap_or_default();
    let objects = format!(
        r#"
    Video: 90001, "Video::Image", "Clip" {{
        Type: "Clip"
        Filename: "never/open/absolute.png"
        RelativeFilename: "{filename}"
        {content}
    }}
    Texture: 90002, "Texture::Image", "" {{
        Type: "TextureVideoClip"
        Version: 202
        TextureName: "Texture::Image"
        Media: "Video::Image"
        FileName: "never/open/absolute.png"
        RelativeFilename: "{filename}"
        ModelUVTranslation: 0,0
        ModelUVScaling: 1,1
        Texture_Alpha_Source: "None"
        Cropping: 0,0,0,0
    }}
"#
    );
    original.replace("\n\tAnimationStack:",&format!("{objects}\n\tAnimationStack:")).replace("Connections:  {","Connections:  {\n C: \"OO\",90001,90002\n C: \"OP\",90002,1907674217712,\"DiffuseColor\"")
}
fn image_bytes() -> Vec<u8> {
    let pixels = image::RgbaImage::from_fn(2, 2, |x, y| {
        image::Rgba([(x * 255) as u8, (y * 255) as u8, 64, 255])
    });
    let mut out = std::io::Cursor::new(Vec::new());
    pixels.write_to(&mut out, image::ImageFormat::Png).unwrap();
    out.into_inner()
}
#[test]
fn embedded_and_external_textures_use_same_ktx_material_and_dependency_snapshot() {
    let root = tempfile::tempdir().unwrap();
    let source = Path::new("model.fbx");
    let bytes = image_bytes();
    fs::write(
        root.path().join(source),
        textured_source("image.png", Some(&bytes)),
    )
    .unwrap();
    let embedded = import_fbx(root.path(), source).unwrap();
    assert_eq!(embedded.images.len(), 1);
    assert_eq!(embedded.dependencies.len(), 1);
    fs::write(root.path().join("image.png"), &bytes).unwrap();
    fs::write(root.path().join(source), textured_source("image.png", None)).unwrap();
    let external = import_fbx(root.path(), source).unwrap();
    assert_eq!(external.images, embedded.images);
    assert_eq!(external.materials, embedded.materials);
    assert_eq!(external.dependencies.len(), 2);
    assert_ne!(external.fingerprint, embedded.fingerprint);
    // Texture orientation agrees with the original FBX's UV0: exported (u,0)
    // is stored as (u,1), matching the cooked top-left image convention.
    assert!(
        external.meshes[0]
            .vertices
            .iter()
            .any(|v| v[6] == 0.375 && v[7] == 1.)
    );
    let a = cook_fbx(root.path(), source, &root.path().join("cache")).unwrap();
    assert_eq!(a.images, external.images);
    assert!(a.materials[0].maps[0].is_some());
    fs::remove_file(root.path().join("image.png")).unwrap();
    assert!(import_fbx(root.path(), source).is_err());
    assert_eq!(
        load_model(&root.path().join("cache"), &a.metadata.fingerprint)
            .unwrap()
            .images,
        a.images
    );
}
#[test]
fn image_path_escape_and_unsupported_channels_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("bad.fbx");
    for name in [
        "../escape.png",
        "/absolute.png",
        "C:\\private.png",
        "https://example.invalid/x.png",
    ] {
        fs::write(&path, textured_source(name, None)).unwrap();
        assert!(import_fbx(root.path(), Path::new("bad.fbx")).is_err());
    }
    let source = textured_source("embedded.png", Some(&image_bytes()));
    // Same texture identity attached to a second unsupported channel must not
    // evade the channel check just because DiffuseColor already accepted it.
    let source = source.replace(
        "Connections:  {",
        "Connections:  {\n C: \"OP\",90002,1907674217712,\"SpecularColor\"",
    );
    fs::write(&path, source).unwrap();
    assert!(import_fbx(root.path(), Path::new("bad.fbx")).is_err());
}

#[test]
fn normal_texture_keeps_linear_role_and_generates_orthonormal_corner_tangents() {
    let root = tempfile::tempdir().unwrap();
    let source = textured_source("normal.png", Some(&image_bytes()))
        .replace("\"DiffuseColor\"", "\"NormalMap\"");
    fs::write(root.path().join("normal.fbx"), source).unwrap();
    let cooked = cook_fbx(
        root.path(),
        Path::new("normal.fbx"),
        &root.path().join("cache"),
    )
    .unwrap();
    assert!(cooked.materials[0].maps[0].is_none());
    let normal = cooked.materials[0].maps[2].unwrap();
    assert_eq!(
        cooked.metadata.textures[normal.texture].normal,
        Some(normal.image)
    );
    assert_eq!(
        cooked.images[normal.image].format,
        incant_assets::TextureFormat::Rgba8Linear
    );
    let mut front_vertices = 0;
    for mesh in &cooked.meshes {
        for vertex in &mesh.vertices {
            let n = glam::Vec3::from_slice(&vertex[3..6]);
            let t = glam::Vec3::from_slice(&vertex[8..11]);
            assert!(n.dot(t).abs() < 1e-5);
            assert!((n.length() - 1.).abs() < 1e-5);
            assert!((t.length() - 1.).abs() < 1e-5);
            assert_eq!(vertex[11].abs(), 1.);
            if n == glam::Vec3::Z {
                // Maya's front face has +U along +X. After conversion to glTF
                // top-left UVs, normal-map +Y must still follow world +Y.
                assert!((t - glam::Vec3::X).length() < 1e-5);
                assert!((n.cross(t) * vertex[11] - glam::Vec3::Y).length() < 1e-5);
                front_vertices += 1;
            }
        }
    }
    assert!(front_vertices >= 4);
}

#[test]
fn real_blender_material_part_assignments_are_retained() {
    let imported = import_fbx(
        &fixtures(),
        Path::new("blender_suzanne_multimaterial_7400_binary.fbx"),
    )
    .unwrap();
    assert_eq!(imported.materials.len(), 7);
    assert!(imported.meshes.len() > 1);
    assert!(
        imported
            .mesh_materials
            .iter()
            .all(|m| m.is_some_and(|i| i < imported.materials.len()))
    );
    let cooked = cook_fbx(
        &fixtures(),
        Path::new("blender_suzanne_multimaterial_7400_binary.fbx"),
        tempfile::tempdir().unwrap().path(),
    )
    .unwrap();
    assert_eq!(cooked.metadata.mesh_materials, imported.mesh_materials);
}
#[test]
fn corrupt_utf8_and_invalid_indices_never_enter_safe_rust_strings_or_vertices() {
    let root = tempfile::tempdir().unwrap();
    let original = fs::read(fixtures().join("maya_cube_7500_ascii.fbx")).unwrap();
    let mut bad = original.clone();
    let pos = bad.windows(6).position(|s| s == b"pCube1").unwrap();
    bad[pos] = 0xff;
    fs::write(root.path().join("bad.fbx"), bad).unwrap();
    assert!(import_fbx(root.path(), Path::new("bad.fbx")).is_err());
    let bad = String::from_utf8(original)
        .unwrap()
        .replace("a: 0,1,3,-3,2,3,5,-5", "a: 99999999,1,3,-3,2,3,5,-5");
    fs::write(root.path().join("bad.fbx"), bad).unwrap();
    assert!(import_fbx(root.path(), Path::new("bad.fbx")).is_err());
}

#[test]
fn actual_animated_skinned_and_morphed_files_fail_without_static_fallback() {
    for (name, reason) in [
        ("max2009_cube_anim_6100_ascii.fbx", "animation"),
        ("maya_blend_shape_cube_7700_ascii.fbx", "morph"),
        ("maya_instanced_skin_7700_ascii.fbx", "skinning"),
    ] {
        let error = import_fbx(&fixtures(), Path::new(name)).err().unwrap();
        assert!(
            matches!(error, incant_assets::AssetError::Unsupported(_)),
            "{name}: {error}"
        );
        // Morph/skin fixtures may also carry animation, which is refused first.
        let message = error.to_string();
        assert!(
            message.contains(reason) || message.contains("animation"),
            "{message}"
        );
    }
}
#[test]
fn source_z_up_inches_map_to_y_up_meters_without_geometry_transform_leakage() {
    let m = import_fbx(
        &fixtures(),
        Path::new("max_geometry_transform_instances_7700_ascii.fbx"),
    )
    .unwrap();
    let globals = globals(&m);
    let origins: Vec<_> = m
        .nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| !n.meshes.is_empty())
        .map(|(i, _)| globals[i].transform_point3(glam::DVec3::ZERO))
        .collect();
    // FBX geometric origin of Box001 is (40,0,0) inches; child Box002 is
    // rotated30deg around Z and its geometric origin resolves to (0,40,0).
    // Target axes map source +Z to +Y and +Y to -Z; 1in=0.0254m.
    assert!(
        (origins[0] - glam::DVec3::new(1.016, 0., 0.)).length() < 1e-6,
        "{origins:?}"
    );
    assert!(
        (origins[1] - glam::DVec3::new(0., 0., -1.016)).length() < 1e-6,
        "{origins:?}"
    );
}

#[test]
fn invalid_source_units_cannot_silently_skip_meter_conversion() {
    let root = tempfile::tempdir().unwrap();
    let original = fs::read_to_string(fixtures().join("maya_cube_7500_ascii.fbx")).unwrap();
    for unit in ["0", "-1"] {
        let source = original.replace(
            "\"UnitScaleFactor\", \"double\", \"Number\", \"\",1",
            &format!("\"UnitScaleFactor\", \"double\", \"Number\", \"\",{unit}"),
        );
        assert_ne!(source, original);
        fs::write(root.path().join("bad.fbx"), source).unwrap();
        let error = import_fbx(root.path(), Path::new("bad.fbx")).err().unwrap();
        assert!(error.to_string().contains("source units"), "{error}");
    }
}

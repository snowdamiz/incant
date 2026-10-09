//! Real GPU checks, explicitly run by the desktop renderer workflow.
mod cameras;
mod environments;
mod lights;
mod materials;
mod shadows;
mod support;
use incant_render::Renderer;
use materials::{Fixture, center, emissive, near};
use serde_json::json;

// Derived preview-curve reference values after the sRGB display transfer.
const DISPLAY_RED: [u8; 4] = [243, 30, 30, 255];
const DISPLAY_GREEN: [u8; 4] = [30, 243, 30, 255];

fn capture(fixture: &Fixture, renderer: &Renderer, name: &str) -> Vec<u8> {
    let png = renderer
        .screenshot_scene_png(&fixture.scene(renderer), 320, 180)
        .unwrap();
    if let Some(directory) = std::env::var_os("INCANT_MATERIAL_EVIDENCE") {
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            std::path::Path::new(&directory).join(format!("{name}.png")),
            &png,
        )
        .unwrap();
    }
    png
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn color_maps_use_srgb_and_texture_reimport_keeps_retained_materials() {
    let renderer = Renderer::headless().unwrap();
    let mut fixture = Fixture::new(json!({"pbrMetallicRoughness":{"metallicFactor":0}}));
    fixture.texture(&[128, 64, 32, 255], 1, 1, |g| {
        g["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"] = json!({"index":0});
    });
    let original = fixture.scene(&renderer);
    let first = capture(&fixture, &renderer, "base-color-texture");
    let linear = |byte: f32| ((byte / 255. + 0.055) / 1.055).powf(2.4);
    fixture.edit(|g| {g["materials"][0] = json!({"pbrMetallicRoughness":{"metallicFactor":0,"baseColorFactor":[linear(128.),linear(64.),linear(32.),1]}});});
    near(fixture.pixel(&renderer), center(&first), 1);
    fixture.texture(&[32, 128, 255, 255], 1, 1, |g| {
        g["materials"][0] =
            json!({"pbrMetallicRoughness":{"metallicFactor":0,"baseColorTexture":{"index":0}}});
    });
    let updated = capture(&fixture, &renderer, "reimported-blue-texture");
    assert_ne!(first, updated);
    assert_eq!(
        first,
        renderer.screenshot_scene_png(&original, 320, 180).unwrap()
    );
    // The current cooked replacement remains self-contained after source removal.
    for name in ["quad.gltf", "quad.bin", "map.png"] {
        std::fs::remove_file(fixture.root.path().join(name)).unwrap();
    }
    let mut reload = incant_assets::AssetStore::default();
    reload
        .sync_project(&fixture.project, fixture.root.path())
        .unwrap();
    let scene = renderer.prepare_scene(&fixture.project, &reload).unwrap();
    assert_eq!(
        updated,
        renderer.screenshot_scene_png(&scene, 320, 180).unwrap()
    );
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn alpha_modes_and_back_faces_follow_material_semantics() {
    let renderer = Renderer::headless().unwrap();
    let mut fixture = Fixture::new(emissive([1., 0., 0.], 0., "OPAQUE"));
    near(
        center(&capture(&fixture, &renderer, "opaque-alpha-zero")),
        DISPLAY_RED,
        1,
    );
    let empty = renderer
        .screenshot_png(&incant_doc::Project::empty("Empty"), 320, 180)
        .unwrap();
    fixture.edit(|g| g["materials"][0] = emissive([1., 0., 0.], 0., "MASK"));
    assert_eq!(capture(&fixture, &renderer, "mask-discarded"), empty);
    fixture.edit(|g| g["materials"][0] = emissive([1., 0., 0.], 0.5, "MASK"));
    near(fixture.pixel(&renderer), DISPLAY_RED, 1);
    // A reflected instance keeps its authored front face through winding reversal.
    fixture.edit(|g| g["nodes"][0]["scale"] = json!([-1, 1, 1]));
    near(
        center(&capture(&fixture, &renderer, "reflected-single-sided")),
        DISPLAY_RED,
        1,
    );
    fixture.edit(|g| g["nodes"][0]["scale"] = json!([1, 1, -1]));
    assert_eq!(fixture.pixel(&renderer), center(&empty));
    fixture.edit(|g| g["materials"][0]["doubleSided"] = json!(true));
    near(
        center(&capture(&fixture, &renderer, "double-sided-back")),
        DISPLAY_RED,
        1,
    );
    fixture.edit(|g| {
        g["nodes"][0]["scale"] = json!([1, 1, 1]);
        g["materials"] = json!([
            emissive([1., 0., 0.], 0.5, "BLEND"),
            emissive([0., 0., 1.], 0.5, "BLEND")
        ]);
        let mut far = g["meshes"][0].clone();
        far["primitives"][0]["material"] = json!(1);
        g["meshes"].as_array_mut().unwrap().push(far);
        // Near red is intentionally first. Depth sorting must draw blue first.
        g["nodes"] =
            json!([{"mesh":0,"translation":[0,0,0.1]},{"mesh":1,"translation":[0,0,-0.1]}]);
        g["scenes"][0]["nodes"] = json!([0, 1]);
    });
    let pixel = center(&capture(&fixture, &renderer, "layered-transparency"));
    // With a dark background, half red over half blue is about (.5, 0, .25) linear.
    assert!(
        pixel[0] > pixel[2] + 30 && pixel[2] > 100 && pixel[1] < 70,
        "{pixel:?}"
    );
    assert_eq!(fixture.scene(&renderer).stats().model_draw_calls, 2);
    fixture.edit(|g| g["scenes"][0]["nodes"] = json!([1, 0]));
    near(fixture.pixel(&renderer), pixel, 0);
    fixture.edit(|g| {
        g["materials"]
            .as_array_mut()
            .unwrap()
            .push(emissive([0., 1., 0.], 1., "OPAQUE"));
        let mut opaque = g["meshes"][0].clone();
        opaque["primitives"][0]["material"] = json!(2);
        g["meshes"].as_array_mut().unwrap().push(opaque);
        g["nodes"].as_array_mut().unwrap().push(json!({"mesh":2}));
        g["scenes"][0]["nodes"] = json!([0, 1, 2]);
    });
    // The middle opaque layer hides far blue before near red blends over it.
    near(
        center(&capture(&fixture, &renderer, "opaque-between-transparent")),
        [188, 188, 0, 255],
        2,
    );
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn material_maps_preserve_numeric_channels_normal_scale_and_occlusion_strength() {
    let renderer = Renderer::headless().unwrap();
    let mut fixture = Fixture::new(
        json!({"pbrMetallicRoughness":{"baseColorFactor":[0.7,0.4,0.2,1],"metallicFactor":1,"roughnessFactor":1}}),
    );
    fixture.texture(&[230, 128, 64, 255], 1, 1, |g| {
        g["materials"][0]["pbrMetallicRoughness"]["metallicRoughnessTexture"] = json!({"index":0})
    });
    let mapped = center(&capture(&fixture, &renderer, "metallic-roughness-map"));
    fixture.edit(|g|g["materials"][0]["pbrMetallicRoughness"]=json!({"baseColorFactor":[0.7,0.4,0.2,1],"metallicFactor":64./255.,"roughnessFactor":128./255.}));
    near(fixture.pixel(&renderer), mapped, 1);
    fixture.edit(|g| g["materials"][0] = json!({"pbrMetallicRoughness":{"metallicFactor":0}}));
    let flat = fixture.pixel(&renderer);
    fixture.edit(|g| {
        g["nodes"][0]["scale"] = json!([1, 1, -1]);
        g["materials"][0]["doubleSided"] = json!(true);
    });
    near(
        center(&capture(&fixture, &renderer, "lit-double-sided-back")),
        flat,
        1,
    );
    fixture.edit(|g| {
        g["nodes"][0]["scale"] = json!([1, 1, 1]);
        g["materials"][0]["doubleSided"] = json!(false);
    });
    fixture.texture(&[255, 128, 128, 255], 1, 1, |g| {
        g["materials"][0]["normalTexture"] = json!({"index":0,"scale":1})
    });
    let mapped = capture(&fixture, &renderer, "normal-map");
    assert_ne!(flat, center(&mapped));
    fixture.texture(&[255, 128, 128, 255, 128, 128, 255, 255], 2, 1, |_| {});
    assert_ne!(capture(&fixture, &renderer, "varying-normal-map"), mapped);
    fixture.edit(|g| g["materials"][0]["normalTexture"]["scale"] = json!(0));
    near(fixture.pixel(&renderer), flat, 1);
    fixture.texture(&[0,0,0,255],1,1,|g| {
        g["materials"][0]=json!({"pbrMetallicRoughness":{"metallicFactor":0},"occlusionTexture":{"index":0,"strength":1}});
    });
    let occluded = center(&capture(&fixture, &renderer, "occlusion-map"));
    assert!(occluded[0] + 20 < flat[0]);
    fixture.edit(|g| g["materials"][0]["occlusionTexture"]["strength"] = json!(0));
    near(fixture.pixel(&renderer), flat, 1);
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn emissive_maps_and_samplers_retain_color_and_addressing() {
    let renderer = Renderer::headless().unwrap();
    let mut fixture = Fixture::new(emissive([1.; 3], 1., "OPAQUE"));
    fixture.texture(&[128, 64, 32, 255], 1, 1, |g| {
        g["materials"][0]["emissiveTexture"] = json!({"index":0})
    });
    near(
        center(&capture(&fixture, &renderer, "emissive-map")),
        [128, 64, 32, 255],
        1,
    );
    {
        let path = fixture.root.path().join("map.png");
        let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Sixteen);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(
                &[32768u16, 16384, 8192, 65535]
                    .into_iter()
                    .flat_map(u16::to_be_bytes)
                    .collect::<Vec<_>>(),
            )
            .unwrap();
    }
    fixture.cook();
    near(
        center(&capture(&fixture, &renderer, "sixteen-bit-color")),
        [128, 64, 32, 255],
        1,
    );
    // Constant coordinates isolate sampler behavior from the camera projection.
    let mut bytes = std::fs::read(fixture.root.path().join("quad.bin")).unwrap();
    for vertex in 0..6 {
        bytes[72 + vertex * 8..80 + vertex * 8].copy_from_slice(
            &[1.25f32, 0.5]
                .into_iter()
                .flat_map(f32::to_le_bytes)
                .collect::<Vec<_>>(),
        );
    }
    std::fs::write(fixture.root.path().join("quad.bin"), &bytes).unwrap();
    fixture.texture(&[255, 0, 0, 255, 0, 255, 0, 255], 2, 1, |_| {});
    near(fixture.pixel(&renderer), DISPLAY_GREEN, 1);
    fixture.edit(|g| g["samplers"][0]["wrapS"] = json!(10497));
    near(fixture.pixel(&renderer), DISPLAY_RED, 1);
    fixture.edit(|g| g["samplers"][0]["wrapS"] = json!(33648));
    near(fixture.pixel(&renderer), DISPLAY_GREEN, 1);
    for vertex in 0..6 {
        bytes[72 + vertex * 8..76 + vertex * 8].copy_from_slice(&0.5f32.to_le_bytes());
    }
    std::fs::write(fixture.root.path().join("quad.bin"), bytes).unwrap();
    fixture.edit(|g| g["samplers"][0]["magFilter"] = json!(9729));
    near(
        center(&capture(&fixture, &renderer, "linear-filter")),
        [188, 188, 0, 255],
        2,
    );
    let mut bytes = std::fs::read(fixture.root.path().join("quad.bin")).unwrap();
    let uv: [f32; 12] = [
        0., 1000., 1000., 1000., 1000., 0., 0., 1000., 1000., 0., 0., 0.,
    ];
    bytes[72..].copy_from_slice(
        &uv.into_iter()
            .flat_map(f32::to_le_bytes)
            .collect::<Vec<_>>(),
    );
    std::fs::write(fixture.root.path().join("quad.bin"), bytes).unwrap();
    fixture.edit(|g| {
        g["samplers"][0] = json!({"minFilter":9987,"magFilter":9728,"wrapS":10497,"wrapT":10497})
    });
    near(
        center(&capture(&fixture, &renderer, "minified-color-mips")),
        [188, 188, 0, 255],
        2,
    );
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn imported_geometry_survives_sources_and_retained_versions_survive_reimport() {
    let temp = tempfile::tempdir().unwrap();
    let (mut project, mut assets, id) = support::fixture(temp.path());
    let renderer = Renderer::headless().unwrap();
    let original = renderer.prepare_scene(&project, &assets).unwrap();
    assert_eq!(original.stats().model_draw_calls, 1);
    assert_eq!(original.stats().primitive_instances, 2);
    std::fs::remove_file(temp.path().join("triangle.gltf")).unwrap();
    std::fs::remove_file(temp.path().join("triangle.bin")).unwrap();
    let first = renderer.screenshot_scene_png(&original, 320, 180).unwrap();
    let empty = renderer
        .screenshot_png(&incant_doc::Project::empty("Empty"), 320, 180)
        .unwrap();
    assert_ne!(first, empty, "indexed models must draw actual pixels");
    let mut replacement = support::model(temp.path(), 2.);
    replacement.id = id.clone();
    project.assets.insert(id.clone(), replacement);
    assets
        .sync(&project, &temp.path().join(".incant/cache"))
        .unwrap();
    let updated = renderer.prepare_scene(&project, &assets).unwrap();
    let second = renderer.screenshot_scene_png(&updated, 320, 180).unwrap();
    assert_ne!(
        first, second,
        "changed source vertices must change the GPU output"
    );
    assert_eq!(
        first,
        renderer.screenshot_scene_png(&original, 320, 180).unwrap(),
        "retained scene must keep old vertex/index buffers"
    );
    project.assets.get_mut(&id).unwrap().sha256 = "0".repeat(64);
    assert!(renderer.prepare_scene(&project, &assets).is_err());
    assert_eq!(
        second,
        renderer.screenshot_scene_png(&updated, 320, 180).unwrap(),
        "failed replacement must leave the prior scene usable"
    );
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn highlight_energy_survives_and_transparency_blends_before_tone_mapping() {
    let renderer = Renderer::headless().unwrap();
    let mut fixture =
        Fixture::new(json!({"pbrMetallicRoughness":{"metallicFactor":0,"roughnessFactor":0.3}}));
    let normal = (glam::Vec3::new(1., 2., 3.).normalize()
        + glam::Vec3::new(6., 5., 9.).normalize())
    .normalize();
    fixture.normal(normal);
    let highlight = center(&capture(&fixture, &renderer, "hdr-glossy-highlight"));
    // Radiance > 1 must reach the shoulder, without clipping before the transform.
    // Premature UNORM clipping would yield the tone-mapped 1.0 value (243).
    assert!(highlight[0] > 245 && highlight[0] < 255, "{highlight:?}");
    assert_eq!(highlight[0], highlight[1]);
    assert_eq!(highlight[1], highlight[2]);
    assert_eq!(highlight[3], 255);
    fixture.edit(|g| {
        g["materials"] = json!([
            emissive([1.; 3], 0.5, "BLEND"),
            emissive([0.; 3], 1., "OPAQUE")
        ]);
        let mut back = g["meshes"][0].clone();
        back["primitives"][0]["material"] = json!(1);
        g["meshes"].as_array_mut().unwrap().push(back);
        g["nodes"] =
            json!([{"mesh":0,"translation":[0,0,0.1]},{"mesh":1,"translation":[0,0,-0.1]}]);
        g["scenes"][0]["nodes"] = json!([0, 1]);
    });
    near(
        center(&capture(&fixture, &renderer, "hdr-transparent-over-black")),
        [188, 188, 188, 255],
        1,
    );
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn dark_fill_lit_materials_remain_distinguishable() {
    let renderer = Renderer::headless().unwrap();
    let mut fixture = Fixture::new(json!({"pbrMetallicRoughness":{"metallicFactor":0}}));
    // This surface faces the camera but away from the key. Only the convolved
    // environment contributes, including dielectric specular reflection.
    fixture.normal(glam::Vec3::new(1., 0., -0.4));
    let mut previous = 0;
    for (name, albedo) in [
        ("dark-fill-10", 0.1),
        ("dark-fill-18", 0.18),
        ("dark-fill-50", 0.5),
    ] {
        fixture.edit(|g| {
            g["materials"][0]["pbrMetallicRoughness"]["baseColorFactor"] =
                json!([albedo, albedo, albedo, 1])
        });
        let pixel = center(&capture(&fixture, &renderer, name));
        assert!((20..160).contains(&pixel[0]), "{pixel:?}");
        assert!(pixel[0].abs_diff(pixel[1]) <= 1 && pixel[1].abs_diff(pixel[2]) <= 1);
        assert_eq!(pixel[3], 255);
        assert!(pixel[0] > previous + 12);
        previous = pixel[0];
    }
}

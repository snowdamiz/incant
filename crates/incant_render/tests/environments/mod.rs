use super::{
    capture,
    materials::{Fixture, center, near},
};
use incant_doc::TextureUsage;
use incant_render::Renderer;
use serde_json::json;

// Analytic UV sphere: numerical coverage fixture, not authored game art.
pub(super) fn sphere(fixture: &mut Fixture) {
    let (rings, sectors) = (32, 64);
    let vertex = |y: usize, x: usize| {
        let theta = std::f32::consts::PI * y as f32 / rings as f32;
        let phi = std::f32::consts::TAU * x as f32 / sectors as f32;
        glam::Vec3::new(
            theta.sin() * phi.cos(),
            theta.cos(),
            theta.sin() * phi.sin(),
        )
    };
    let mut normals = Vec::new();
    for y in 0..rings {
        for x in 0..sectors {
            for mut tri in [
                [vertex(y, x), vertex(y + 1, x), vertex(y + 1, x + 1)],
                [vertex(y, x), vertex(y + 1, x + 1), vertex(y, x + 1)],
            ] {
                let cross = (tri[1] - tri[0]).cross(tri[2] - tri[0]);
                if cross.length_squared() < 1e-12 {
                    continue;
                }
                if cross.dot(tri[0]) < 0. {
                    tri.swap(1, 2);
                }
                normals.extend(tri);
            }
        }
    }
    let positions: Vec<f32> = normals.iter().flat_map(|p| (*p * 2.5).to_array()).collect();
    let bytes: Vec<_> = positions
        .into_iter()
        .chain(normals.iter().flat_map(|p| p.to_array()))
        .flat_map(f32::to_le_bytes)
        .collect();
    let count = normals.len();
    std::fs::write(fixture.root.path().join("quad.bin"), &bytes).unwrap();
    fixture.edit(|g| {
        g["buffers"][0]["byteLength"] = json!(bytes.len());
        g["bufferViews"] = json!([{"buffer":0,"byteLength":count*12},
            {"buffer":0,"byteOffset":count*12,"byteLength":count*12}]);
        g["accessors"] = json!([
            {"bufferView":0,"componentType":5126,"count":count,"type":"VEC3","min":[-2.5,-2.5,-2.5],"max":[2.5,2.5,2.5]},
            {"bufferView":1,"componentType":5126,"count":count,"type":"VEC3"}]);
        g["meshes"][0]["primitives"][0]["attributes"] = json!({"POSITION":0,"NORMAL":1});
    });
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn studio_environment_renders_metal_and_dielectric_across_roughness() {
    let renderer = Renderer::headless().unwrap();
    let mut fixture = Fixture::new(json!({"pbrMetallicRoughness":{"metallicFactor":0}}));
    sphere(&mut fixture);
    let mut outputs = Vec::new();
    for (label, metallic, color) in [
        ("metal", 1., [0.8; 3]),
        ("dielectric", 0., [0.5; 3]),
        ("dark", 0., [0.03; 3]),
    ] {
        for (suffix, roughness) in [
            ("smooth", 0.045),
            ("satin", 0.3),
            ("rough", 0.7),
            ("matte", 1.),
        ] {
            fixture.edit(|g| g["materials"][0] = json!({"pbrMetallicRoughness":{
                "metallicFactor":metallic,"roughnessFactor":roughness,"baseColorFactor":[color[0],color[1],color[2],1]
            }}));
            let scene = fixture.scene(&renderer);
            let png = renderer.screenshot_scene_png(&scene, 640, 480).unwrap();
            assert_eq!(scene.stats().diagnostic_entities, 0);
            assert_eq!(scene.stats().model_draw_calls, 1);
            if let Some(directory) = std::env::var_os("INCANT_MATERIAL_EVIDENCE") {
                std::fs::create_dir_all(&directory).unwrap();
                std::fs::write(
                    std::path::Path::new(&directory).join(format!("studio-{label}-{suffix}.png")),
                    &png,
                )
                .unwrap();
            }
            assert!(
                !outputs.contains(&png),
                "roughness/material changes must affect the image"
            );
            outputs.push(png);
        }
    }
}

fn metal() -> Fixture {
    let mut fixture = Fixture::new(json!({"pbrMetallicRoughness":{
        "baseColorFactor":[1,1,1,1],"metallicFactor":1,"roughnessFactor":0.045
    }}));
    // Faces the camera, but has N.L < 0; direct key contributes exactly zero.
    fixture.normal(glam::Vec3::new(1., 0., -0.4));
    fixture
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn authored_environments_preserve_color_space_versions_and_source_free_loading() {
    let renderer = Renderer::headless().unwrap();
    let mut fixture = metal();
    let id = fixture.environment(&[128, 64, 32, 255].repeat(2), 2, 1, TextureUsage::Color);
    let original_document = fixture.project.clone();
    let original = fixture.scene(&renderer);
    let first = capture(&fixture, &renderer, "environment-srgb-metal");
    near(center(&first), [128, 64, 32, 255], 2);
    fixture.environment(&[128, 64, 32, 255].repeat(2), 2, 1, TextureUsage::Linear);
    let linear = capture(&fixture, &renderer, "environment-linear-metal");
    near(center(&linear), [188, 137, 99, 255], 2);
    assert_eq!(
        first,
        renderer.screenshot_scene_png(&original, 320, 180).unwrap()
    );
    // Restoring the prior document revision resolves the old cooked version.
    let linear_document = fixture.project.clone();
    fixture.project = original_document;
    fixture
        .assets
        .sync_project(&fixture.project, fixture.root.path())
        .unwrap();
    assert_eq!(first, capture(&fixture, &renderer, "environment-restored"));
    fixture.project = linear_document;
    fixture
        .assets
        .sync_project(&fixture.project, fixture.root.path())
        .unwrap();
    for name in ["environment.png", "quad.gltf", "quad.bin"] {
        std::fs::remove_file(fixture.root.path().join(name)).unwrap();
    }
    let mut reload = incant_assets::AssetStore::default();
    reload
        .sync_project(&fixture.project, fixture.root.path())
        .unwrap();
    let scene = renderer.prepare_scene(&fixture.project, &reload).unwrap();
    assert_eq!(
        linear,
        renderer.screenshot_scene_png(&scene, 320, 180).unwrap()
    );
    fixture.project.assets.get_mut(&id).unwrap().sha256 = "00".repeat(32);
    assert!(
        renderer
            .prepare_scene(&fixture.project, &reload)
            .err()
            .unwrap()
            .to_string()
            .contains("environment texture")
    );
    assert_eq!(
        linear,
        renderer.screenshot_scene_png(&scene, 320, 180).unwrap()
    );
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn environment_intensity_rotation_and_roughness_change_real_reflections() {
    let renderer = Renderer::headless().unwrap();
    let mut fixture = metal();
    // Source +Z hemisphere is red, -Z blue; no downloaded artwork.
    let pixels: Vec<u8> = (0..32)
        .flat_map(|_| {
            (0..64).flat_map(|x| {
                if x >= 32 {
                    [255, 0, 0, 255]
                } else {
                    [0, 0, 255, 255]
                }
            })
        })
        .collect();
    fixture.environment(&pixels, 64, 32, TextureUsage::Linear);
    let first = center(&capture(&fixture, &renderer, "environment-rotation-zero"));
    assert!(first[2] > first[0] + 100);
    fixture.environment_setting("rotation_degrees", 90.);
    let positive = fixture.pixel(&renderer);
    fixture.environment_setting("rotation_degrees", -90.);
    let negative = fixture.pixel(&renderer);
    assert!(
        positive[2] > positive[0] + 100 && negative[0] > negative[2] + 100,
        "positive/negative world +Y yaw: {positive:?} / {negative:?}"
    );
    fixture.environment_setting("rotation_degrees", 180.);
    let second = center(&capture(
        &fixture,
        &renderer,
        "environment-rotation-half-turn",
    ));
    assert!(
        first[0].abs_diff(second[0]) > 100 && first[2].abs_diff(second[2]) > 100,
        "{first:?} / {second:?}"
    );
    fixture.environment_setting("intensity", 0.);
    near(
        center(&capture(&fixture, &renderer, "environment-disabled")),
        [0, 0, 0, 255],
        1,
    );
    fixture.environment_setting("intensity", 1.);
    fixture.environment(&[128, 128, 128, 255].repeat(2), 2, 1, TextureUsage::Linear);
    let smooth = fixture.pixel(&renderer);
    fixture.edit(|g| g["materials"][0]["pbrMetallicRoughness"]["roughnessFactor"] = json!(1));
    let rough = center(&capture(&fixture, &renderer, "environment-rough-metal"));
    // Single scattering GGX reflects less energy at high roughness.
    assert!(
        rough[0] > 40 && rough[0] + 10 < smooth[0],
        "{smooth:?} / {rough:?}"
    );
    fixture.edit(|g| g["materials"][0]["pbrMetallicRoughness"]["metallicFactor"] = json!(0));
    // A white dielectric in a constant environment returns that radiance:
    // diffuse fills exactly the energy remaining after specular reflection.
    near(
        center(&capture(&fixture, &renderer, "environment-white-furnace")),
        [188, 188, 188, 255],
        1,
    );
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn malformed_environment_cannot_replace_a_retained_scene() {
    let renderer = Renderer::headless().unwrap();
    let mut fixture = metal();
    fixture.environment(&[80, 80, 80, 255].repeat(2), 2, 1, TextureUsage::Color);
    let valid = fixture.scene(&renderer);
    let first = renderer.screenshot_scene_png(&valid, 320, 180).unwrap();
    fixture.environment(&[80, 80, 80, 255], 1, 1, TextureUsage::Color);
    let error = renderer
        .prepare_scene(&fixture.project, &fixture.assets)
        .err()
        .unwrap();
    assert!(error.to_string().contains("2:1"));
    assert_eq!(
        first,
        renderer.screenshot_scene_png(&valid, 320, 180).unwrap()
    );
}

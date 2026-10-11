//! Numeric readback against independently authored all-opaque RGB references.
//! No screenshot artifact or appearance approval is produced here.
use super::materials::{Fixture, emissive};
use incant_render::Renderer;
use serde_json::json;
use std::fs;

fn minify(fixture: &mut Fixture) {
    let path = fixture.root.path().join("quad.bin");
    let mut bytes = fs::read(&path).unwrap();
    // Every covered fragment must select the final mip; no pixel-location tie
    // or runtime alpha policy is involved in this filtering oracle.
    for chunk in bytes[72..120].as_chunks_mut::<4>().0 {
        let uv = f32::from_le_bytes(*chunk) * 1000.;
        chunk.copy_from_slice(&uv.to_le_bytes());
    }
    fs::write(path, bytes).unwrap();
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn opaque_and_emissive_minification_ignore_alpha_without_recoloring_coverage_roles() {
    let renderer = Renderer::headless().unwrap();
    for (emission, mode) in [
        (false, "OPAQUE"),
        (true, "OPAQUE"),
        (true, "MASK"),
        (true, "BLEND"),
    ] {
        for alpha in [[0, 0], [0, 255]] {
            let material = if emission {
                emissive([1.; 3], if mode == "BLEND" { 0.75 } else { 1. }, mode)
            } else {
                json!({"pbrMetallicRoughness":{"metallicFactor":0,"roughnessFactor":1}})
            };
            let mut fixture = Fixture::new(material);
            minify(&mut fixture);
            let set_map = |g: &mut serde_json::Value| {
                g["samplers"][0] =
                    json!({"minFilter":9987,"magFilter":9728,"wrapS":10497,"wrapT":10497});
                if emission {
                    g["materials"][0]["emissiveTexture"] = json!({"index":0});
                } else {
                    g["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"] =
                        json!({"index":0});
                }
            };
            fixture.texture(&[255, 0, 0, alpha[0], 0, 0, 255, alpha[1]], 2, 1, set_map);
            let retained = fixture.scene(&renderer);
            let actual = renderer.screenshot_scene_png(&retained, 160, 90).unwrap();
            fixture.texture(&[255, 0, 0, 255, 0, 0, 255, 255], 2, 1, set_map);
            let expected = renderer
                .screenshot_scene_png(&fixture.scene(&renderer), 160, 90)
                .unwrap();
            assert_eq!(actual, expected, "{emission}/{mode}/{alpha:?}");
            fixture.texture(&[0, 0, 255, 255, 0, 0, 255, 255], 2, 1, set_map);
            let wrong = renderer
                .screenshot_scene_png(&fixture.scene(&renderer), 160, 90)
                .unwrap();
            assert_ne!(actual, wrong, "actual RGB must affect the frame");
            for name in ["quad.bin", "quad.gltf", "map.png"] {
                fs::remove_file(fixture.root.path().join(name)).unwrap();
            }
            fs::remove_dir_all(fixture.root.path().join(".incant/cache")).unwrap();
            assert_eq!(
                actual,
                renderer.screenshot_scene_png(&retained, 160, 90).unwrap()
            );
        }
    }
    // MASK/BLEND base color must retain coverage-aware filtering. An alpha-zero
    // red contributor cannot contaminate the surviving blue texel at low LOD.
    for mode in ["MASK", "BLEND"] {
        let mut fixture = Fixture::new(json!({"alphaMode":mode,"alphaCutoff":0.4,
            "pbrMetallicRoughness":{"metallicFactor":0,"roughnessFactor":1}}));
        minify(&mut fixture);
        let set_map = |g: &mut serde_json::Value| {
            g["samplers"][0] =
                json!({"minFilter":9987,"magFilter":9728,"wrapS":10497,"wrapT":10497});
            g["materials"][0]["pbrMetallicRoughness"]["baseColorTexture"] = json!({"index":0});
        };
        fixture.texture(&[255, 0, 0, 0, 0, 0, 255, 255], 2, 1, set_map);
        let actual = renderer
            .screenshot_scene_png(&fixture.scene(&renderer), 160, 90)
            .unwrap();
        fixture.texture(&[0, 0, 255, 128], 1, 1, set_map);
        let expected = renderer
            .screenshot_scene_png(&fixture.scene(&renderer), 160, 90)
            .unwrap();
        assert_eq!(actual, expected, "coverage role {mode}");
        fixture.texture(&[188, 0, 188, 128], 1, 1, set_map);
        assert_ne!(
            actual,
            renderer
                .screenshot_scene_png(&fixture.scene(&renderer), 160, 90)
                .unwrap()
        );
    }
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn color_environment_minification_matches_alpha_one_and_retains_prior_versions() {
    let renderer = Renderer::headless().unwrap();
    let mut fixture = Fixture::new(
        json!({"pbrMetallicRoughness":{"baseColorFactor":[0.7,0.7,0.7,1],"metallicFactor":0,"roughnessFactor":1}}),
    );
    let pixels = |opaque| {
        (0..512)
            .flat_map(|y| {
                (0..1024).flat_map(move |x| {
                    if (x + y) % 2 == 0 {
                        [255, 0, 0, if opaque { 255 } else { 0 }]
                    } else {
                        [0, 0, 255, 255]
                    }
                })
            })
            .collect::<Vec<u8>>()
    };
    fixture.environment(&pixels(false), 1024, 512, incant_doc::TextureUsage::Color);
    let retained = fixture.scene(&renderer);
    let actual = renderer.screenshot_scene_png(&retained, 160, 90).unwrap();
    fixture.environment(&pixels(true), 1024, 512, incant_doc::TextureUsage::Color);
    let expected = renderer
        .screenshot_scene_png(&fixture.scene(&renderer), 160, 90)
        .unwrap();
    assert_eq!(
        actual, expected,
        "environment RGB ignores source alpha at every mip"
    );
    fixture.environment(
        &[0, 0, 255, 255].repeat(1024 * 512),
        1024,
        512,
        incant_doc::TextureUsage::Color,
    );
    let wrong = renderer
        .screenshot_scene_png(&fixture.scene(&renderer), 160, 90)
        .unwrap();
    assert_ne!(
        actual, wrong,
        "environment readback must respond to the lowpass color"
    );
    for file in ["quad.gltf", "quad.bin", "environment.png"] {
        fs::remove_file(fixture.root.path().join(file)).unwrap();
    }
    fs::remove_dir_all(fixture.root.path().join(".incant/cache")).unwrap();
    assert_eq!(
        actual,
        renderer.screenshot_scene_png(&retained, 160, 90).unwrap()
    );
}

//! Numeric framebuffer equivalence against analytically authored tangent frames.
//! Appearance acceptance remains the separate Claude review.
use super::materials::Fixture;
use incant_render::Renderer;
use serde_json::json;
use std::fs;

fn supply_tangent(fixture: &mut Fixture, tangent: [f32; 4]) {
    let path = fixture.root.path().join("quad.bin");
    let mut bytes = fs::read(&path).unwrap();
    bytes.truncate(120);
    for _ in 0..6 {
        bytes.extend(tangent.into_iter().flat_map(f32::to_le_bytes));
    }
    fs::write(path, bytes).unwrap();
    fixture.edit(|g| {
        g["buffers"][0]["byteLength"] = json!(216);
        if g["bufferViews"].as_array().unwrap().len() == 2 {
            g["bufferViews"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::Value::Null);
            g["accessors"]
                .as_array_mut()
                .unwrap()
                .push(serde_json::Value::Null);
        }
        g["bufferViews"][2] = json!({"buffer":0,"byteOffset":120,"byteLength":96});
        g["accessors"][2] = json!({"bufferView":2,"componentType":5126,"count":6,"type":"VEC4"});
        g["meshes"][0]["primitives"][0]["attributes"]["TANGENT"] = json!(2);
    });
}

#[test]
#[ignore = "requires a native GPU; run by the desktop workflow"]
fn generated_normal_map_basis_matches_explicit_frames_under_uv_and_instance_reflection() {
    let renderer = crate::support::checked(Renderer::headless().unwrap());
    for (mirror_u, mirror_v, x, w) in [
        (false, false, 1_f32, 1_f32),
        (true, false, -1., -1.),
        (false, true, 1., -1.),
        (true, true, -1., 1.),
    ] {
        for (scale, rotation) in [
            ([1., 1., 1.], [0., 0., 0., 1.]),
            ([-1.4, 0.7, 1.], [0., 0., 0., 1.]),
            (
                [0.8, 1.2, 1.],
                [
                    0.,
                    0.,
                    std::f64::consts::FRAC_1_SQRT_2,
                    std::f64::consts::FRAC_1_SQRT_2,
                ],
            ),
        ] {
            let mut fixture = Fixture::new(
                json!({"pbrMetallicRoughness":{"metallicFactor":0,"roughnessFactor":1}}),
            );
            let path = fixture.root.path().join("quad.bin");
            let mut bytes = fs::read(&path).unwrap();
            for vertex in 0..6 {
                for (axis, mirror) in [mirror_u, mirror_v].into_iter().enumerate() {
                    let offset = 72 + vertex * 8 + axis * 4;
                    let value = f32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap());
                    let value = if mirror { 1. - value } else { value };
                    bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
                }
            }
            fs::write(path, bytes).unwrap();
            fixture.texture(&[128, 230, 205, 255], 1, 1, |g| {
                g["materials"][0]["normalTexture"] = json!({"index":0});
                g["nodes"][0]["scale"] = json!(scale);
                g["nodes"][0]["rotation"] = json!(rotation);
            });
            let generated = fixture.scene(&renderer);
            let actual = renderer.screenshot_scene_png(&generated, 160, 90).unwrap();
            supply_tangent(&mut fixture, [x, 0., 0., w]);
            let expected = renderer
                .screenshot_scene_png(&fixture.scene(&renderer), 160, 90)
                .unwrap();
            assert_eq!(
                actual, expected,
                "UV mirror {mirror_u}/{mirror_v}, scale {scale:?}, rotation {rotation:?}"
            );
            // Opposite handedness must visibly affect this numerical control,
            // proving that equal empty/unlit frames cannot make the test pass.
            supply_tangent(&mut fixture, [x, 0., 0., -w]);
            let opposite = renderer
                .screenshot_scene_png(&fixture.scene(&renderer), 160, 90)
                .unwrap();
            assert_ne!(actual, opposite);
            assert_eq!(
                actual,
                renderer.screenshot_scene_png(&generated, 160, 90).unwrap(),
                "retained generated version survives both explicit replacements"
            );
        }
    }
}

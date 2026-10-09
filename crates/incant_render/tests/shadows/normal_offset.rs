//! Normal maps change shading, not the geometric shadow lookup position.
use super::{fixture, set};
use incant_doc::Transform;
use incant_render::Renderer;
use serde_json::json;

fn decode(png: &[u8]) -> Vec<u8> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(png))
        .read_info()
        .unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    reader.next_frame(&mut pixels).unwrap();
    pixels
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn opposing_normal_maps_preserve_the_geometric_shadow_boundary() {
    let renderer = Renderer::headless().unwrap();
    let (mut f, camera, caster, light) = fixture();
    set(
        &mut f,
        &caster,
        "Transform",
        json!(Transform {
            translation: [0.75, 0., 2.],
            scale: [0.25; 3],
            ..Default::default()
        }),
    );
    set(
        &mut f,
        &light,
        "Transform",
        json!(Transform {
            rotation: glam::DQuat::from_rotation_x(std::f64::consts::FRAC_PI_4).to_array(),
            ..Default::default()
        }),
    );
    let mut captures = Vec::new();
    // Mirror the tangent-space X normal. At world x=0 both sun and eye have
    // zero X, so the BRDF is symmetric. The caster's shadow starts at x=0.
    // Offsetting the shadow lookup with a mapped normal would move that edge.
    for (name, x) in [("positive", 218), ("negative", 37)] {
        f.texture(&[x, 128, 218, 255], 1, 1, |g| {
            g["materials"][0]["normalTexture"] = json!({"index":0,"scale":1});
        });
        let scene = f.scene(&renderer).with_camera(&camera).unwrap();
        // An odd width places the central fragment exactly on world x=0.
        let png = renderer.screenshot_scene_png(&scene, 641, 481).unwrap();
        if let Some(dir) = std::env::var_os("INCANT_SHADOW_EVIDENCE") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                std::path::Path::new(&dir).join(format!("shadow-normal-{name}.png")),
                &png,
            )
            .unwrap();
        }
        captures.push(decode(&png));
    }
    assert_ne!(
        captures[0], captures[1],
        "normal maps must affect off-axis shading"
    );
    let differences: Vec<_> = (0..481)
        .filter_map(|y| {
            let i = (y * 641 + 320) * 4;
            let delta = (0..3)
                .map(|c| captures[0][i + c].abs_diff(captures[1][i + c]))
                .max()
                .unwrap();
            (delta > 1).then_some((y, delta))
        })
        .collect();
    assert!(
        differences.is_empty(),
        "mapped normals moved geometric shadow edge: {differences:?}"
    );
}

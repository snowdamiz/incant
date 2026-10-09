use crate::{
    Renderer, Viewport,
    hdr_output::{read, texture},
};
use incant_doc::{Entity, Transform};
use serde_json::json;
fn crop(bytes: &[u8], width: usize, rect: [usize; 4]) -> Vec<u8> {
    (rect[1]..rect[1] + rect[3])
        .flat_map(|y| {
            bytes[(y * width + rect[0]) * 4..(y * width + rect[0] + rect[2]) * 4]
                .iter()
                .copied()
        })
        .collect()
}
#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn clustered_viewport_offsets_and_queued_resize_commands_keep_their_own_light_data() {
    let root = tempfile::tempdir().unwrap();
    let (mut p, assets, _) = crate::test_support::fixture(root.path());
    for x in [-3., 0., 3.] {
        let mut light = Entity::new("Localized light");
        light.components.insert(
            "PointLight".into(),
            json!({"color":[0.8,0.2,0.1],"intensity":5,"range":3}),
        );
        light.components.insert(
            "Transform".into(),
            json!(Transform {
                translation: [x, 1., 1.],
                ..Default::default()
            }),
        );
        p.scenes
            .values_mut()
            .next()
            .unwrap()
            .entities
            .insert(light.id.clone(), light);
    }
    let r = Renderer::headless().unwrap();
    let scene = r.prepare_scene(&p, &assets).unwrap();
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let a = texture(&r, format, 320, 180);
    let b = texture(&r, format, 800, 500);
    let c = texture(&r, format, 512, 320);
    let first = r
        .draw_scene(
            &scene,
            &a.create_view(&Default::default()),
            format,
            320,
            180,
            None,
        )
        .unwrap();
    let second = r
        .draw_scene(
            &scene,
            &b.create_view(&Default::default()),
            format,
            800,
            500,
            Some(Viewport {
                rect: [80., 40., 640., 360.],
                corner_radii: [0.; 4],
                canvas_srgb: [20, 21, 25],
            }),
        )
        .unwrap();
    let third = r
        .draw_scene(
            &scene,
            &c.create_view(&Default::default()),
            format,
            512,
            320,
            Some(Viewport {
                rect: [37., 41., 320., 180.],
                corner_radii: [0.; 4],
                canvas_srgb: [20, 21, 25],
            }),
        )
        .unwrap();
    // Submit in a different order from encoding; retained size-specific buffers
    // and per-command uniforms must be valid until the queue finishes each pass.
    r.queue.submit([second, first, third]);
    let base = read(&r, &a);
    let offset = crop(&read(&r, &c), 512, [37, 41, 320, 180]);
    assert_eq!(base.len(), offset.len());
    assert!(base.iter().zip(offset).all(|(a, b)| a.abs_diff(b) <= 1));
    let small = r.lighting.buffers(&r.device, [5, 3]).unwrap();
    let reused = r.lighting.buffers(&r.device, [5, 3]).unwrap();
    assert!(std::sync::Arc::ptr_eq(&small, &reused));
    assert!(r.lighting.buffers(&r.device, [u32::MAX, 2]).is_err());
}

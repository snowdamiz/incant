//! Actual attachment/raster/ownership controls. Explicit GPU grant required;
//! stored fixture pixels require separate appearance review.
use super::{SceneMsaa, SceneMsaaError, fixture};
use crate::{Renderer, RetainedTarget, SceneInput, Viewport, hdr_output};
use glam::{DMat4, DQuat, DVec3};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
fn renderer() -> Renderer {
    crate::test_support::checked(
        pollster::block_on(Renderer::new_with_scene_msaa(
            &wgpu::Instance::default(),
            None,
            4,
        ))
        .unwrap(),
    )
}
fn count(pixels: &[u8], channel: usize) -> usize {
    pixels
        .as_chunks::<4>()
        .0
        .iter()
        .filter(|p| {
            p[channel] > 150
                && (0..3)
                    .filter(|&c| c != channel)
                    .all(|c| p[channel].saturating_sub(p[c]) > 80)
        })
        .count()
}
#[test]
#[ignore = "requires an explicitly granted native GPU; numerical fixture only"]
fn four_sample_world_draws_resolve_after_models_sprites_and_text_with_depth() {
    let renderer = renderer();
    let mut input = fixture::mixed(1.);
    let all = fixture::draw(&renderer, &input, [160, 96], None);
    fixture::record("world-all", &renderer, &all, [160, 96]);
    hdr_output::save("msaa4-world-all", &all, 160, 96);
    assert!(count(&all, 0) > 50, "opaque red primitive missing");
    assert!(count(&all, 1) > 50, "MASK green primitive missing");
    assert!(count(&all, 2) > 50, "BLEND blue primitive missing");
    assert!(all.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    let no_mask = fixture::draw(&renderer, &fixture::mixed(0.), [160, 96], None);
    assert_eq!(count(&no_mask, 1), 0, "discarded MASK must not draw");
    fixture::record("world-mask-discarded", &renderer, &no_mask, [160, 96]);
    assert_ne!(all, no_mask);
    let text = input.text.take();
    let no_text = fixture::draw(&renderer, &input, [160, 96], None);
    assert_ne!(all, no_text, "world text must reach resolved output");
    input.text = text;
    let sprite_index = input.nodes.iter().position(|n| n.sprite.is_some()).unwrap();
    let sprite = input.nodes.remove(sprite_index);
    let no_sprite = fixture::draw(&renderer, &input, [160, 96], None);
    assert_ne!(all, no_sprite, "PMA sprite must reach resolved output");
    input.nodes.insert(sprite_index, sprite);
    input.nodes.push(fixture::node(
        "occluded-cube",
        DMat4::from_scale_rotation_translation(
            DVec3::splat(0.3),
            DQuat::from_rotation_y(1.3),
            DVec3::new(0., 1., -2.),
        ),
        true,
    ));
    assert_eq!(
        fixture::draw(&renderer, &input, [160, 96], None),
        all,
        "back geometry must fail per-sample depth"
    );
    input.nodes.pop();
    let frame = renderer
        .frames
        .get(&renderer.device, 160, 96, SceneMsaa::Four)
        .unwrap();
    assert_eq!(frame.color.texture().sample_count(), 4);
    assert_eq!(frame.depth.texture().sample_count(), 4);
    assert_eq!(frame.resolved.texture().sample_count(), 1);
    assert!(Arc::ptr_eq(
        &frame,
        &renderer
            .frames
            .get(&renderer.device, 160, 96, SceneMsaa::Four)
            .unwrap()
    ));
    let one = renderer
        .frames
        .get(&renderer.device, 160, 96, SceneMsaa::One)
        .unwrap();
    assert!(
        !Arc::ptr_eq(&frame, &one),
        "cache identity must include sample count"
    );
    assert_eq!(one.color.texture(), one.resolved.texture());
    renderer.check_device().unwrap();
}
#[test]
#[ignore = "requires an explicitly granted native GPU; numerical fixture only"]
fn scene_msaa_queued_resize_clear_refusal_and_explicit_one_sample_routes() {
    let mut renderer = renderer();
    assert_eq!(renderer.scene_sample_count(), 4);
    let ao = pollster::block_on(renderer.enable_ambient_occlusion(crate::AmbientOcclusion {
        radius: 1.,
        bias: 0.01,
        strength: 1.,
    }))
    .unwrap_err();
    assert_eq!(
        ao.downcast_ref::<SceneMsaaError>(),
        Some(&SceneMsaaError::AmbientOcclusion)
    );
    assert!(renderer.ambient_occlusion.is_none());
    let input = fixture::mixed(1.);
    let mut jobs = Vec::new();
    for (index, (w, h)) in [(160, 96), (192, 128), (160, 96)].into_iter().enumerate() {
        let viewport = Viewport {
            rect: [11.25, 13.75, w as f32 - 25.5, h as f32 - 28.5],
            corner_radii: [0.; 4],
            canvas_srgb: [64, 80, 96],
        };
        let expected = fixture::draw(&renderer, &input, [w, h], Some(viewport));
        assert_eq!(&expected[..4], &[64, 80, 96, 255]);
        fixture::record(&format!("viewport-{index}"), &renderer, &expected, [w, h]);
        hdr_output::save(&format!("msaa4-viewport-{w}x{h}"), &expected, w, h);
        let scene = renderer
            .prepare(&input)
            .unwrap()
            .with_camera("camera")
            .unwrap();
        let target = hdr_output::texture(&renderer, FORMAT, w, h);
        let command = renderer
            .draw_scene(
                &scene,
                &target.create_view(&Default::default()),
                FORMAT,
                w,
                h,
                Some(viewport),
            )
            .unwrap();
        let accepted = renderer
            .frames
            .get(&renderer.device, w, h, SceneMsaa::Four)
            .unwrap();
        let error = renderer
            .frames
            .get(&renderer.device, 4096, 4096, SceneMsaa::Four)
            .err()
            .unwrap();
        assert!(error.downcast_ref::<SceneMsaaError>().is_some());
        assert!(
            Arc::ptr_eq(
                &accepted,
                &renderer
                    .frames
                    .get(&renderer.device, w, h, SceneMsaa::Four)
                    .unwrap()
            ),
            "size refusal replaced the admitted frame cache"
        );
        jobs.push((target, command, expected));
    }
    for (target, command, expected) in jobs.into_iter().rev() {
        renderer.queue.submit([command]);
        assert_eq!(
            hdr_output::read(&renderer, &target),
            expected,
            "queued old sample/size owners changed"
        );
    }
    let empty = SceneInput {
        nodes: vec![fixture::camera(false)],
        text: None,
        environment: None,
    };
    let cleared = fixture::draw(&renderer, &empty, [160, 96], None);
    assert!(
        cleared
            .as_chunks::<4>()
            .0
            .iter()
            .all(|p| *p == [20, 21, 25, 255]),
        "4x clear/resolve leaked old contents"
    );
    fixture::record("cleared", &renderer, &cleared, [160, 96]);
    let ordinary = crate::test_support::checked(Renderer::headless().unwrap());
    assert_eq!(ordinary.scene_sample_count(), 1);
    let one_pixels = fixture::draw(&ordinary, &input, [160, 96], None);
    let four_pixels = fixture::draw(&renderer, &input, [160, 96], None);
    fixture::record("world-one", &ordinary, &one_pixels, [160, 96]);
    fixture::record("world-four", &renderer, &four_pixels, [160, 96]);
    assert_ne!(
        one_pixels, four_pixels,
        "geometric diagonal fixture did not exercise sample coverage"
    );
    let mut retained = renderer.prepare_retained(&input, "camera").unwrap();
    let output = hdr_output::texture(&renderer, FORMAT, 160, 96);
    retained
        .submit(
            &renderer,
            &input,
            RetainedTarget {
                view: &output.create_view(&Default::default()),
                format: FORMAT,
                width: 160,
                height: 96,
                viewport: None,
            },
        )
        .unwrap();
    assert_eq!(
        hdr_output::read(&renderer, &output),
        one_pixels,
        "retained submit inherited editor MSAA"
    );
    let mut pixel_input = fixture::mixed(1.);
    let camera = pixel_input
        .nodes
        .iter_mut()
        .find(|n| n.camera.is_some())
        .unwrap();
    *camera = fixture::camera(true);
    assert_eq!(
        fixture::draw(&renderer, &pixel_input, [320, 192], None),
        fixture::draw(&ordinary, &pixel_input, [320, 192], None),
        "pixel grid must remain 1x nearest"
    );
    renderer.check_device().unwrap();
}
#[test]
#[ignore = "requires an explicitly granted native GPU; validates real resolve timing"]
fn scene_resolve_timestamp_is_real_after_text_and_absent_for_one_sample() {
    use crate::timing::GpuPass;
    let instance = wgpu::Instance::default();
    let renderer = crate::test_support::checked(
        pollster::block_on(Renderer::new_with_scene_msaa_and_gpu_timing(
            &instance, None, 4,
        ))
        .unwrap(),
    );
    let mut input = fixture::mixed(1.);
    for (pixel_grid, expected_resolve) in [(false, true), (true, false)] {
        *input.nodes.iter_mut().find(|n| n.camera.is_some()).unwrap() = fixture::camera(pixel_grid);
        fixture::draw(&renderer, &input, [160, 96], None);
        let deadline = Instant::now() + Duration::from_secs(10);
        let sample = loop {
            if let Some(sample) = renderer.take_gpu_timing().unwrap() {
                break sample;
            }
            assert!(Instant::now() < deadline, "real query callback timed out");
            std::thread::sleep(Duration::from_millis(2));
        };
        let resolve = sample.passes[GpuPass::SceneResolve.index()];
        assert_eq!(resolve.is_some(), expected_resolve);
        if let Some(resolve) = resolve {
            let text = sample.passes[GpuPass::Text.index()].unwrap();
            let display = sample.passes[GpuPass::Display.index()].unwrap();
            assert!(
                text.end_ticks <= resolve.start_ticks && resolve.end_ticks <= display.start_ticks,
                "resolve interval must follow final world draw before display"
            );
        }
        eprintln!("MSAA timing {}", serde_json::to_string(&sample).unwrap());
    }
    renderer.check_device().unwrap();
}

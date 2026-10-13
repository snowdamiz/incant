//! Queued for Claude/device execution. These tests do not save images.
use super::*;
use crate::{
    Renderer,
    frame::HDR_FORMAT,
    hdr_output::{read, texture},
};

#[test]
#[ignore = "requires root-granted GPU slot and Claude numerical/appearance review"]
fn actual_pyramid_nonzero_halo_resize_and_zero_alpha_control() {
    let mut r = crate::test_support::checked(Renderer::headless().unwrap());
    let enabled = pollster::block_on(r.enable_bloom(config()));
    if cfg!(all(feature = "cooked-output", not(target_arch = "wasm32"))) {
        assert_eq!(
            enabled.unwrap_err().downcast_ref::<BloomError>(),
            Some(&BloomError::CookedOutput)
        );
        assert!(r.bloom.is_none());
        return;
    }
    enabled.unwrap();
    let validation = r.device.push_error_scope(wgpu::ErrorFilter::Validation);
    let system = r.bloom.as_ref().unwrap();
    let mut jobs = Vec::new();
    for (width, height, alpha) in [(17, 13, 1.), (23, 19, 1.), (17, 13, 0.)] {
        let source = texture(&r, HDR_FORMAT, width, height);
        let mut data = vec![0u8; (width * height * 8) as usize];
        let offset = ((height / 2 * width + width / 2) * 8) as usize;
        for (i, v) in [32., 16., 8., alpha].into_iter().enumerate() {
            data[offset + i * 2..offset + i * 2 + 2]
                .copy_from_slice(&half::f16::from_f32(v).to_le_bytes());
        }
        r.queue.write_texture(
            source.as_image_copy(),
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * 8),
                rows_per_image: Some(height),
            },
            source.size(),
        );
        let view = source.create_view(&Default::default());
        let rect = [0., 0., width as f32, height as f32];
        let frame = crate::resolution::FramePlan::new(
            [width, height],
            rect,
            None,
            crate::resolution::ResolutionScale::FULL,
        )
        .unwrap();
        let draw = system
            .prepare(
                &r.device,
                &view,
                [width, height],
                &frame,
                crate::resolution::ResolutionScale::FULL,
            )
            .unwrap();
        let target = texture(&r, wgpu::TextureFormat::Rgba8UnormSrgb, width, height);
        let baseline = texture(&r, wgpu::TextureFormat::Rgba8UnormSrgb, width, height);
        let mut encoder = r.device.create_command_encoder(&Default::default());
        r.output
            .prepare(
                &r.device,
                &view,
                &baseline.create_view(&Default::default()),
                baseline.format(),
                None,
                None,
            )
            .unwrap()
            .encode(&mut encoder, None);
        draw.encode(&mut encoder, None);
        system
            .output
            .prepare_optional_bloom(
                &r.device,
                &view,
                &target.create_view(&Default::default()),
                target.format(),
                None,
                None,
                Some((&draw.result, system.config.intensity, [width, height])),
            )
            .unwrap()
            .encode(&mut encoder, None);
        jobs.push((encoder.finish(), target, baseline, alpha));
    }
    // Retire the system/cache before submitting any old-size command buffers.
    r.disable_bloom();
    for (command, target, baseline, alpha) in jobs {
        r.queue.submit([command]);
        let actual = read(&r, &target);
        let original = read(&r, &baseline);
        if alpha == 0. {
            assert_eq!(actual, original);
        } else {
            let brighter = actual
                .as_chunks::<4>()
                .0
                .iter()
                .zip(original.as_chunks::<4>().0)
                .filter(|(a, b)| u16::from(a[0]) > u16::from(b[0]) + 1)
                .count();
            assert!(brighter > 1, "bloom did not spread the isolated emitter");
        }
        assert!(actual.as_chunks::<4>().0.iter().all(|p| p[3] == 255));
    }
    assert!(pollster::block_on(validation.pop()).is_none());
}

#[test]
#[ignore = "requires root-granted GPU slot and Claude numerical/appearance review"]
fn canonical_graph_zero_gain_and_pixel_camera_bypass_and_invalid_replace() {
    let mut r = crate::test_support::checked(Renderer::headless().unwrap());
    if cfg!(all(feature = "cooked-output", not(target_arch = "wasm32"))) {
        let error = pollster::block_on(r.enable_bloom(config())).unwrap_err();
        assert_eq!(
            error.downcast_ref::<BloomError>(),
            Some(&BloomError::CookedOutput)
        );
        assert!(r.bloom.is_none());
        return;
    }
    let mut scene = r.prepare(&crate::SceneInput::default()).unwrap();
    scene.diagnostics.push(glam::Mat4::IDENTITY);
    scene.camera.pixel_resolution = Some([32, 24]);
    scene.camera.orthographic_half_height = Some(2.);
    let baseline = texture(&r, wgpu::TextureFormat::Rgba8UnormSrgb, 101, 79);
    let target = texture(&r, baseline.format(), 101, 79);
    let draw = |r: &Renderer, t: &wgpu::Texture, scene: &crate::RenderScene| {
        r.draw_scene(
            scene,
            &t.create_view(&Default::default()),
            t.format(),
            101,
            79,
            None,
        )
        .unwrap()
    };
    r.queue.submit([draw(&r, &baseline, &scene)]);
    pollster::block_on(r.enable_bloom(config())).unwrap();
    assert!(
        pollster::block_on(r.enable_bloom(Bloom {
            intensity: f32::NAN,
            ..config()
        }))
        .is_err()
    );
    assert_eq!(r.bloom.as_ref().unwrap().config, config());
    r.queue.submit([draw(&r, &target, &scene)]);
    assert_eq!(read(&r, &target), read(&r, &baseline));
    assert!(r.bloom.as_ref().unwrap().cache.lock().unwrap().is_none());
    scene.camera.pixel_resolution = None;
    r.disable_bloom();
    r.queue.submit([draw(&r, &baseline, &scene)]);
    pollster::block_on(r.enable_bloom(Bloom {
        intensity: 0.,
        ..config()
    }))
    .unwrap();
    r.queue.submit([draw(&r, &target, &scene)]);
    assert_eq!(read(&r, &target), read(&r, &baseline));
    assert!(r.bloom.as_ref().unwrap().cache.lock().unwrap().is_none());
}

#[test]
#[ignore = "requires coordinated numerical GPU review; three output frames"]
fn bloom_output_keeps_emitter_position_at_reduced_internal_resolution() {
    let mut r = crate::test_support::checked(Renderer::headless().unwrap());
    if cfg!(all(feature = "cooked-output", not(target_arch = "wasm32"))) {
        let error = pollster::block_on(r.enable_bloom(config())).unwrap_err();
        assert_eq!(
            error.downcast_ref::<BloomError>(),
            Some(&BloomError::CookedOutput)
        );
        assert!(r.bloom.is_none());
        return;
    }
    pollster::block_on(r.enable_bloom(config())).unwrap();
    let validation = r.device.push_error_scope(wgpu::ErrorFilter::Validation);
    let output_size = [67, 53];
    let viewport = crate::Viewport {
        rect: [5.5, 4.25, 57., 43.],
        corner_radii: [2.; 4],
        canvas_srgb: [2, 3, 4],
    };
    for numerator in [16, 12, 8] {
        let scale = crate::resolution::ResolutionScale::new(numerator).unwrap();
        let plan =
            crate::resolution::FramePlan::new(output_size, viewport.rect, None, scale).unwrap();
        let source = texture(&r, HDR_FORMAT, plan.size[0], plan.size[1]);
        let bloom_size = output_size.map(|v| v.div_ceil(2));
        let bloom = texture(&r, HDR_FORMAT, bloom_size[0], bloom_size[1]);
        // Isolate output-coordinate correctness from extraction/filtering: one
        // known HDR texel is placed in a prepared bloom field over empty scene.
        let hot = [bloom_size[0] / 3, bloom_size[1] * 2 / 3];
        let mut data = vec![0u8; (bloom_size[0] * bloom_size[1] * 8) as usize];
        let offset = ((hot[1] * bloom_size[0] + hot[0]) * 8) as usize;
        data[offset..offset + 2].copy_from_slice(&half::f16::from_f32(4.).to_le_bytes());
        r.queue.write_texture(
            bloom.as_image_copy(),
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bloom_size[0] * 8),
                rows_per_image: Some(bloom_size[1]),
            },
            bloom.size(),
        );
        let target = texture(&r, wgpu::TextureFormat::Rgba8UnormSrgb, 67, 53);
        let mut encoder = r.device.create_command_encoder(&Default::default());
        r.bloom
            .as_ref()
            .unwrap()
            .output
            .prepare_optional_bloom(
                &r.device,
                &source.create_view(&Default::default()),
                &target.create_view(&Default::default()),
                target.format(),
                Some(viewport),
                plan.output_mapping,
                Some((&bloom.create_view(&Default::default()), 0.2, output_size)),
            )
            .unwrap()
            .encode(&mut encoder, None);
        r.queue.submit([encoder.finish()]);
        let actual = read(&r, &target);
        let (index, pixel) = actual
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .max_by_key(|(_, pixel)| pixel[0])
            .unwrap();
        assert!(pixel[0] > 80, "the isolated emitter must produce light");
        let observed = [(index % 67) as f32 + 0.5, (index / 67) as f32 + 0.5];
        for axis in 0..2 {
            // Texel center -> normalized output-anchored bloom -> physical.
            let expected =
                (hot[axis] as f32 + 0.5) / bloom_size[axis] as f32 * output_size[axis] as f32;
            assert!(
                (observed[axis] - expected).abs() <= 1.,
                "scale {numerator}, axis {axis}: observed {observed:?}, expected {expected}"
            );
        }
    }
    assert!(pollster::block_on(validation.pop()).is_none());
}

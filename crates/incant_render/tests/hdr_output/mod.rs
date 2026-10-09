//! Numerical GPU output checks; screenshot approval is a separate Claude handoff.
use crate::{Renderer, ResourceError, Viewport, frame::HDR_FORMAT, output::OutputPass};
use std::{sync::Arc, time::Duration};

fn texture(
    renderer: &Renderer,
    format: wgpu::TextureFormat,
    width: u32,
    height: u32,
) -> wgpu::Texture {
    renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Output behavior probe"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST
            | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}
fn read(renderer: &Renderer, texture: &wgpu::Texture) -> Vec<u8> {
    let width = texture.width();
    let height = texture.height();
    let stride = (width * 4).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output probe readback"),
        size: u64::from(stride) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
        },
        texture.size(),
    );
    renderer.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        tx.send(r).unwrap();
    });
    renderer
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    let data = buffer.slice(..).get_mapped_range();
    let mut pixels: Vec<_> = data
        .chunks(stride as usize)
        .flat_map(|r| r[..width as usize * 4].iter().copied())
        .collect();
    drop(data);
    buffer.unmap();
    if matches!(
        texture.format(),
        wgpu::TextureFormat::Bgra8Unorm | wgpu::TextureFormat::Bgra8UnormSrgb
    ) {
        for pixel in pixels.chunks_mut(4) {
            pixel.swap(0, 2);
        }
    }
    pixels
}
fn save(name: &str, pixels: &[u8], width: u32, height: u32) {
    if let Some(directory) = std::env::var_os("INCANT_HDR_EVIDENCE") {
        std::fs::create_dir_all(&directory).unwrap();
        let file =
            std::fs::File::create(std::path::Path::new(&directory).join(format!("{name}.png")))
                .unwrap();
        let mut encoder = png::Encoder::new(file, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(pixels)
            .unwrap();
    }
}
fn near(actual: &[u8], expected: &[u8]) {
    assert_eq!(actual.len(), expected.len());
    if let Some((index, (a, b))) = actual
        .iter()
        .zip(expected)
        .enumerate()
        .find(|(_, (a, b))| a.abs_diff(**b) > 1)
    {
        panic!("output byte {index}: {a} != {b}");
    }
}

#[test]
#[ignore = "requires a native GPU; run by all desktop workflows"]
fn hdr_values_survive_until_tone_mapping_and_transfer_runs_once() {
    let renderer = Renderer::headless().unwrap();
    // Expected sRGB code values independently evaluated from the upstream GLSL.
    let signals: [([f32; 3], [u8; 3]); 12] = [
        ([0.; 3], [0; 3]),
        ([0.01; 3], [2; 3]),
        ([0.18; 3], [105; 3]),
        ([0.5; 3], [181; 3]),
        ([0.8; 3], [226; 3]),
        ([1.; 3], [240; 3]),
        ([2.; 3], [250; 3]),
        ([4.; 3], [253; 3]),
        ([16.; 3], [255; 3]),
        ([65504.; 3], [255; 3]),
        ([4., 0.2, 0.2], [253, 156, 156]),
        ([65504., 0., 0.], [255, 255, 255]),
    ];
    let width = 384;
    let height = 128;
    let source = texture(&renderer, HDR_FORMAT, width, height);
    let mut data = Vec::new();
    for _ in 0..height {
        for x in 0..width {
            for v in signals[(x / 32) as usize].0.into_iter().chain([1.]) {
                data.extend_from_slice(&half::f16::from_f32(v).to_le_bytes());
            }
        }
    }
    renderer.queue.write_texture(
        source.as_image_copy(),
        &data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width * 8),
            rows_per_image: Some(height),
        },
        source.size(),
    );
    let mut reference: Option<Vec<u8>> = None;
    for format in [
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Bgra8Unorm,
    ] {
        let target = texture(&renderer, format, width, height);
        let mut encoder = renderer.device.create_command_encoder(&Default::default());
        renderer
            .output
            .encode(
                &renderer.device,
                &mut encoder,
                &source.create_view(&Default::default()),
                &target.create_view(&Default::default()),
                format,
                None,
            )
            .unwrap();
        renderer.queue.submit([encoder.finish()]);
        let pixels = read(&renderer, &target);
        for (i, (_, expected)) in signals.iter().enumerate() {
            let offset = ((height / 2 * width) as usize + i * 32 + 16) * 4;
            near(&pixels[offset..offset + 3], expected);
            assert_eq!(pixels[offset + 3], 255);
        }
        if let Some(reference) = reference.as_ref() {
            near(&pixels, reference);
        } else {
            save("hdr-reference-swatches", &pixels, width, height);
            reference = Some(pixels);
        }
    }
}

#[test]
#[ignore = "requires a native GPU; run by all desktop workflows"]
fn output_composes_display_colors_and_reuses_bounded_resize_targets() {
    let renderer = Renderer::headless().unwrap();
    let first = renderer.frames.get(&renderer.device, 96, 64).unwrap();
    assert!(Arc::ptr_eq(
        &first,
        &renderer.frames.get(&renderer.device, 96, 64).unwrap()
    ));
    let resized = renderer.frames.get(&renderer.device, 80, 60).unwrap();
    assert!(!Arc::ptr_eq(&first, &resized));
    // Both encoded versions remain alive after the cache resizes.
    let empty = renderer
        .diagnostic_scene(&incant_doc::Project::empty("Empty"))
        .unwrap();
    let mut jobs = Vec::new();
    for (width, height) in [(96, 64), (80, 60), (96, 64)] {
        for format in [
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureFormat::Rgba8Unorm,
        ] {
            let target = texture(&renderer, format, width, height);
            let viewport = Viewport {
                rect: [10., 8., 60., 40.],
                corner_radii: [7.; 4],
                canvas_srgb: [8, 85, 204],
            };
            let command = renderer
                .draw_scene(
                    &empty,
                    &target.create_view(&Default::default()),
                    format,
                    width,
                    height,
                    Some(viewport),
                )
                .unwrap();
            jobs.push((target, command));
        }
    }
    let mut previous = None;
    for (target, command) in jobs {
        renderer.queue.submit([command]);
        let pixels = read(&renderer, &target);
        near(&pixels[..4], &[8, 85, 204, 255]);
        let offset = ((20 * target.width() + 30) * 4) as usize;
        near(&pixels[offset..offset + 4], &[20, 21, 25, 255]);
        if target.format().is_srgb() {
            previous = Some(pixels);
        } else {
            near(&pixels, previous.as_ref().unwrap());
        }
    }
    let error = renderer
        .frames
        .get(&renderer.device, 8192, 8192)
        .err()
        .unwrap();
    assert!(matches!(
        error.downcast_ref::<ResourceError>(),
        Some(ResourceError::RenderTargetSize)
    ));
    assert!(renderer.frames.get(&renderer.device, 0, 64).is_err());
    assert!(renderer.frames.get(&renderer.device, 8193, 16).is_err());
    let error = OutputPass::validate(wgpu::TextureFormat::Rgba16Float).unwrap_err();
    assert!(matches!(
        error.downcast_ref::<ResourceError>(),
        Some(ResourceError::OutputFormat(_))
    ));
}

//! Actual GPU text draws compared with an independent CPU composition of the
//! CPU atlas under the documented placement, tap and blending rules. Ignored
//! without a granted native GPU; numeric agreement, not appearance approval.
use super::tests::{MIXED, input};
use super::*;
use crate::{Renderer, frame::HDR_FORMAT, hdr_output::read, models::ModelTarget};
use glam::{DMat4, DQuat, DVec2, DVec3};
use incant_render_data::{SceneInput, SceneNode};
use incant_types::{Camera, CameraProjection};
use std::time::Duration;

pub(crate) const PPU: f64 = 100.;
pub(crate) fn camera(projection: CameraProjection, world: DMat4) -> SceneNode {
    SceneNode {
        id: "camera".into(),
        world,
        camera: Some(Camera {
            fov_degrees: 60.,
            near: 0.1,
            far: 100.,
            projection,
        }),
        light: None,
        mesh: None,
        sprite: None,
        tilemap: None,
        diagnostic: false,
    }
}
/// One layout pixel per frame pixel; frame pixel (W/2, H/2) is world (0, 0).
pub(crate) fn pixel_camera(size: [u32; 2]) -> SceneNode {
    camera(
        CameraProjection::PixelPerfect {
            pixels_per_unit: PPU,
            reference_resolution: size,
        },
        DMat4::from_translation(DVec3::new(0., 0., 8.)),
    )
}
pub(crate) fn scene(text: TextInput, camera: SceneNode) -> SceneInput {
    SceneInput {
        nodes: vec![camera],
        text: Some(text),
        environment: None,
    }
}
fn attachment(renderer: &Renderer, format: wgpu::TextureFormat, size: [u32; 2]) -> wgpu::Texture {
    renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Text HDR probe"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
/// The text pass alone over cleared HDR/depth attachments, read back as f32.
fn hdr_text(renderer: &Renderer, input: &SceneInput, size: [u32; 2]) -> Vec<[f32; 4]> {
    let scene = renderer
        .prepare(input)
        .unwrap()
        .with_camera("camera")
        .unwrap();
    let color = attachment(renderer, HDR_FORMAT, size);
    let depth = attachment(renderer, wgpu::TextureFormat::Depth32Float, size);
    let color_view = color.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Clear text probe"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &color_view,
            resolve_target: None,
            depth_slice: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: &depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        ..Default::default()
    });
    renderer
        .text
        .prepare(
            &renderer.device,
            &scene,
            ModelTarget {
                color: color_view.clone(),
                depth: depth_view.clone(),
                format: HDR_FORMAT,
                rect: [0., 0., size[0] as f32, size[1] as f32],
            },
        )
        .unwrap()
        .expect("visible text")
        .encode(&mut encoder);
    let stride = (size[0] * 8).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Text HDR readback"),
        size: u64::from(stride) * u64::from(size[1]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        color.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(size[1]),
            },
        },
        color.size(),
    );
    renderer.queue.submit([encoder.finish()]);
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, |r| r.unwrap());
    renderer
        .device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    let data = buffer.slice(..).get_mapped_range();
    data.chunks(stride as usize)
        .flat_map(|row| {
            row[..size[0] as usize * 8]
                .as_chunks::<8>()
                .0
                .iter()
                .map(|p| {
                    std::array::from_fn(|i| {
                        half::f16::from_le_bytes([p[i * 2], p[i * 2 + 1]]).to_f32()
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Independent CPU composition for an orthographic pixel-perfect view along -Z,
/// using the documented placement, tap and premultiplied-over rules. A single
/// tap reads only zero padding within half a texel of a quad edge, so edge
/// inclusion cannot matter there. Multi-tap pixels whose centre lies within
/// 0.02 screen pixels of an edge, where GPU fixed-point rasterization may
/// decide either way, are None.
pub(crate) fn reference(text: &TextInput, size: [u32; 2], eye: DVec2) -> Vec<Option<f64>> {
    let [aw, _] = text.atlas.size();
    let texel =
        |x: i64, y: i64| f64::from(text.atlas.pixels()[(y * i64::from(aw) + x) as usize]) / 255.;
    let bilinear = |p: DVec2| {
        let (x, y) = (p.x - 0.5, p.y - 0.5);
        let (i, j) = (x.floor(), y.floor());
        let (fx, fy) = (x - i, y - j);
        let (i, j) = (i as i64, j as i64);
        (texel(i, j) * (1. - fx) + texel(i + 1, j) * fx) * (1. - fy)
            + (texel(i, j + 1) * (1. - fx) + texel(i + 1, j + 1) * fx) * fy
    };
    let mut alpha = vec![Some(0.); (size[0] * size[1]) as usize];
    for node in &text.nodes {
        // Contract, written out: layout (x, y) px is local (x/100, -y/100, 0).
        let m = node.world * DMat4::from_scale(DVec3::new(0.01, -0.01, 1.));
        // Each inner coverage rectangle starts at position + origin, padded by
        // the atlas's one zero texel on every side.
        let quads: Vec<_> = node
            .layout
            .glyphs()
            .iter()
            .zip(node.glyphs.iter())
            .filter_map(|(glyph, handle)| {
                let entry = text.atlas.get(handle).unwrap();
                let [x, y, w, h] = entry.rect?.map(f64::from);
                let left = f64::from(glyph.position[0]) + f64::from(entry.origin[0]);
                let top = f64::from(glyph.position[1]) + f64::from(entry.origin[1]);
                Some((
                    [left - 1., top - 1., left + w + 1., top + h + 1.],
                    [x - 1., y - 1.],
                ))
            })
            .collect();
        // Plane z = const viewed along -Z: layout -> world XY is affine.
        let a = glam::DMat2::from_cols(
            m.x_axis.truncate().truncate(),
            m.y_axis.truncate().truncate(),
        );
        let inverse = a.inverse();
        let origin = m.w_axis.truncate().truncate();
        let dx = inverse * DVec2::new(1. / PPU, 0.);
        let dy = inverse * DVec2::new(0., -1. / PPU);
        let taps = (dx.length().max(dy.length()) - 0.25).ceil().clamp(1., 4.) as usize;
        for py in 0..size[1] {
            for px in 0..size[0] {
                let world = eye
                    + DVec2::new(
                        (f64::from(px) + 0.5 - f64::from(size[0]) / 2.) / PPU,
                        (f64::from(size[1]) / 2. - (f64::from(py) + 0.5)) / PPU,
                    );
                let layout = inverse * (world - origin);
                let pixel = &mut alpha[(py * size[0] + px) as usize];
                // 0.02 screen pixels expressed in layout pixels.
                let band = 0.02 * dx.length().max(dy.length());
                for ([l, t, r, b], [u0, v0]) in &quads {
                    let within = |e: f64| {
                        layout.x > l - e && layout.x < r + e && layout.y > t - e && layout.y < b + e
                    };
                    if taps > 1 && within(band) && !within(-band) {
                        *pixel = None;
                    }
                    if !within(0.) {
                        continue;
                    }
                    let (u1, v1) = (u0 + r - l, v0 + b - t);
                    let center = DVec2::new(u0 + layout.x - l, v0 + layout.y - t);
                    let mut sum = 0.;
                    for j in 0..taps {
                        for i in 0..taps {
                            let o = (DVec2::new(i as f64, j as f64) + 0.5) / taps as f64 - 0.5;
                            let p = (center + dx * o.x + dy * o.y).clamp(
                                DVec2::new(u0 + 0.5, v0 + 0.5),
                                DVec2::new(u1 - 0.5, v1 - 0.5),
                            );
                            sum += bilinear(p);
                        }
                    }
                    let c = sum / (taps * taps) as f64;
                    if let Some(a) = pixel {
                        *a = c + *a * (1. - c);
                    }
                }
            }
        }
    }
    alpha
}
fn compare(gpu: &[[f32; 4]], expected: &[Option<f64>]) -> (f64, usize, f64) {
    let (mut worst, mut skipped, mut ink) = (0f64, 0, 0.);
    for (pixel, expected) in gpu.iter().zip(expected) {
        // White text is premultiplied: every RGB channel equals coverage.
        for c in 0..3 {
            assert!((pixel[c] - pixel[3]).abs() < 1e-3, "{pixel:?}");
        }
        match expected {
            Some(a) => {
                worst = worst.max((f64::from(pixel[3]) - a).abs());
                ink += a;
            }
            None => skipped += 1,
        }
    }
    (worst, skipped, ink)
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn drawn_text_matches_layout_baselines_bearings_bidi_order_and_missing_glyphs() {
    let renderer = Renderer::headless().unwrap();
    let size = [256, 64];
    // Layout top-left at world (-1.2, 0.3): frame pixel (8, 2) + layout pixels.
    let world = DMat4::from_translation(DVec3::new(-1.2, 0.3, 0.));
    let text = input(&[(1, MIXED, 20., world)]);
    let input = scene(text.clone(), pixel_camera(size));
    let gpu = hdr_text(&renderer, &input, size);
    let expected = reference(&text, size, DVec2::ZERO);
    let (worst, skipped, ink) = compare(&gpu, &expected);
    eprintln!("1:1 text: max |alpha error| {worst:.5}, skipped {skipped}, ink {ink:.1}");
    assert!(worst < 0.01, "{worst}");
    assert_eq!(skipped, 0);
    assert!(ink > 200.);
    // Landmarks from layout facts alone: the first line's baseline row bounds
    // the "ffi" ligature's ink; CJK ink lies right of the RTL Arabic run.
    let layout = &text.nodes[0].layout;
    let baseline = 2. + f64::from(layout.lines()[0].baseline);
    let column_ink = |range: std::ops::Range<f32>| {
        let mut rows = Vec::new();
        for y in 0..size[1] as usize {
            for x in 0..size[0] as usize {
                let lx = x as f32 + 0.5 - 8.;
                if range.contains(&lx) && gpu[y * size[0] as usize + x][3] > 0.5 {
                    rows.push((x, y));
                }
            }
        }
        rows
    };
    let glyphs = layout.glyphs();
    let ffi = column_ink(glyphs[0].position[0]..glyphs[0].position[0] + glyphs[0].advance);
    let bottom = ffi.iter().map(|p| p.1).max().unwrap() as f64 + 1.;
    assert!((bottom - baseline).abs() <= 1., "{bottom} vs {baseline}");
    let word = MIXED.find("سلام").unwrap()..MIXED.find(" 中").unwrap();
    let arabic = glyphs.iter().filter(|g| word.contains(&g.source.start));
    let arabic_right = arabic
        .map(|g| g.position[0] + g.advance)
        .fold(0f32, f32::max);
    let cjk = glyphs
        .iter()
        .find(|g| &layout.source()[g.source.clone()] == "中")
        .unwrap();
    assert!(cjk.position[0] >= arabic_right);
    assert!(!column_ink(cjk.position[0]..cjk.position[0] + cjk.advance).is_empty());
    // The same scene through the canonical graph and display transform.
    let scene = renderer
        .prepare(&input)
        .unwrap()
        .with_camera("camera")
        .unwrap();
    assert_eq!(scene.stats().text_entities, 1);
    assert_eq!(scene.stats().text_missing_glyphs, 1);
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let output = crate::hdr_output::texture(&renderer, format, size[0], size[1]);
    renderer.queue.submit([renderer
        .draw_scene(
            &scene,
            &output.create_view(&Default::default()),
            format,
            size[0],
            size[1],
            None,
        )
        .unwrap()]);
    let pixels = read(&renderer, &output);
    crate::hdr_output::save("text-mixed-1to1", &pixels, size[0], size[1]);
    let decode = |v: f64| {
        let s = v / 255.;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    let encode = |v: f64| {
        255. * if v <= 0.0031308 {
            12.92 * v
        } else {
            1.055 * v.powf(1. / 2.4) - 0.055
        }
    };
    let backdrop = [20., 21., 25.].map(decode);
    // tone_map(1.0) = 0.9 exactly (derived curve, see tone_map.wgsl).
    let mut display_worst = 0f64;
    for (pixel, alpha) in pixels.as_chunks::<4>().0.iter().zip(&gpu) {
        for c in 0..3 {
            let want = encode(backdrop[c] + (0.9 - backdrop[c]) * f64::from(alpha[3]));
            display_worst = display_worst.max((f64::from(pixel[c]) - want).abs());
        }
    }
    assert!(display_worst <= 1.0, "{display_worst}");
}

#[test]
#[ignore = "requires a native GPU; run by desktop workflows"]
fn rotated_scaled_minified_and_magnified_text_matches_the_affine_reference() {
    let renderer = Renderer::headless().unwrap();
    let size = [192, 128];
    for (scale, angle, at) in [
        (0.42, 0.4, DVec3::new(-0.8, 0.45, 0.)),
        (1.7, -0.15, DVec3::new(-0.9, 0.55, 0.)),
        (0.24, 0., DVec3::new(-0.9, -0.2, 0.)),
    ] {
        let world = DMat4::from_scale_rotation_translation(
            DVec3::splat(scale),
            DQuat::from_rotation_z(angle),
            at,
        );
        let text = input(&[
            (1, MIXED, 20., world),
            (
                2,
                "Ag",
                48.,
                world * DMat4::from_translation(DVec3::new(0.2, -0.3, 0.)),
            ),
        ]);
        let gpu = hdr_text(&renderer, &scene(text.clone(), pixel_camera(size)), size);
        let expected = reference(&text, size, DVec2::ZERO);
        let (worst, skipped, ink) = compare(&gpu, &expected);
        eprintln!(
            "scale {scale} angle {angle}: max |alpha error| {worst:.5}, skipped {skipped}, ink {ink:.1}"
        );
        assert!(worst < 0.012, "scale {scale}: {worst}");
        assert!(skipped * 200 < (size[0] * size[1]) as usize, "{skipped}");
        assert!(ink > 20.);
    }
}

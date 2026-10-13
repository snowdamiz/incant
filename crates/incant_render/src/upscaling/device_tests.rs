//! Actual fixed shader execution, raw scalar readback only. No image encoding.
use super::*;
fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 != 0 { -1. } else { 1. };
    let exp = ((bits >> 10) & 31) as i32;
    let mant = f32::from(bits & 1023);
    if exp == 0 {
        sign * mant * 2f32.powi(-24)
    } else {
        sign * (1. + mant / 1024.) * 2f32.powi(exp - 15)
    }
}
#[test]
#[ignore = "requires delegated GPU numerical execution; no appearance acceptance"]
fn actual_output_reconstruction_rejects_bad_history_and_never_retains_overlay() {
    let mut r = crate::test_support::checked(Renderer::headless().unwrap());
    if crate::motion::admission().is_err() {
        assert!(pollster::block_on(r.enable_temporal_upscaling()).is_err());
        return;
    }
    pollster::block_on(r.enable_temporal_upscaling()).unwrap();
    println!("upscaling adapter: {:?}", r.adapter.get_info());
    let system = r.upscaling.as_ref().unwrap();
    let d = &r.device;
    let make = |format, size: u32, usage| {
        d.create_texture(&wgpu::TextureDescriptor {
            label: Some("Numeric upscaling fixture"),
            size: wgpu::Extent3d {
                width: size,
                height: size,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let source = |values: &[[f32; 4]], n: u32| {
        let t = make(
            wgpu::TextureFormat::Rgba32Float,
            n,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        r.queue.write_texture(
            t.as_image_copy(),
            bytemuck::cast_slice(values),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(n * 16),
                rows_per_image: Some(n),
            },
            wgpu::Extent3d {
                width: n,
                height: n,
                depth_or_array_layers: 1,
            },
        );
        t.create_view(&Default::default())
    };
    let current_depth = make(
        wgpu::TextureFormat::Depth32Float,
        4,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    )
    .create_view(&Default::default());
    let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
        | wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_SRC;
    let color = make(crate::frame::HDR_FORMAT, 8, usage);
    let color_view = color.create_view(&Default::default());
    let depth = make(wgpu::TextureFormat::R32Float, 8, usage).create_view(&Default::default());
    let composite = make(crate::frame::HDR_FORMAT, 8, usage);
    let composite_view = composite.create_view(&Default::default());
    let read = d.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Output and immutable history scalar rows"),
        size: 4096,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    for mode in 0..19 {
        let mut current = vec![[0.5, 0.5, 0.5, 1.]; 16];
        current[4] = [0., 0., 0., 1.];
        current[1] = [1.; 4];
        if mode == 4 {
            current.fill([100., 100., 100., 0.]);
        }
        if mode == 5 {
            current.fill([0.2, 0.1, 0., 0.5]);
        }
        if mode == 8 {
            current.fill([-1., 0.5, 0.5, 1.]);
        }
        if mode == 9 {
            current.fill([f32::NAN, 0.5, 0.5, 1.]);
        }
        if mode == 13 {
            current.fill([2., 0.5, 100., 1.]);
        }
        if mode >= 16 {
            current.fill([0.5, 0.5, 0.5, 1.]);
        }
        let old_value = if mode == 6 {
            [20., 20., 20., 1.]
        } else if mode == 10 {
            [-1., 0.25, 0.25, 1.]
        } else if mode == 13 {
            [2., 0.5, 100., 1.]
        } else {
            [0.25, 0.25, 0.25, 1.]
        };
        let mut old_values = vec![old_value; 64];
        if mode == 14 {
            old_values[3 * 8 + 4] = [f32::NAN, 0., 0., 1.];
        }
        let old = source(&old_values, 8);
        let current = source(&current, 4);
        let movement = if mode == 3 {
            2.
        } else if mode == 15 {
            0.5 / 6.
        } else if mode == 11 {
            f32::NAN
        } else {
            0.
        };
        let motion = source(&vec![[movement, 0., 0.5, 1.]; 16], 4);
        let z = if mode == 1 {
            50.05
        } else {
            1. / (0.5 / 0.1 + 0.5 / 100.)
        };
        let mut depths = vec![[if mode == 2 { 10. } else { z }, 0., 0., 0.]; 64];
        if mode == 15 {
            depths[3 * 8 + 4][0] = 10.;
        }
        let old_depth = source(&depths, 8);
        let over = if mode == 7 {
            [0.1, 0.05, 0., 0.5]
        } else {
            [0.; 4]
        };
        let mut over_values = vec![over; 16];
        if mode >= 16 {
            for y in 0..4 {
                for x in 0..4 {
                    over_values[y * 4 + x] = [0.02 * x as f32 + 0.03 * y as f32, 0., 0., 0.5];
                }
            }
        }
        let overlay = source(&over_values, 4);
        let frame = Frame {
            source: [0.5, 0.5, 3., 3.],
            output: [1., 1., 6., 6.],
            depth: [
                0.1,
                100.,
                f32::from(u8::from(mode == 1)),
                f32::from(u8::from(mode != 12 && mode < 16)),
            ],
            options: [0.5, 0.001, 0.001, 1.],
            reactive: [f32::from(u8::from(mode == 6)), 0., 0., 0.],
            jitter: if mode == 16 {
                [0.125, -0.125, 0., 0.]
            } else if mode == 17 {
                [-0.125, 0.125, 0., 0.]
            } else {
                [0.; 4]
            },
        };
        let uniform = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&frame),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let draw = Draw {
            resolve: group(
                d,
                &system.resolve_layout,
                &uniform,
                &[&current, &motion, &current_depth, &old, &old_depth],
                None,
            ),
            composite: group(
                d,
                &system.composite_layout,
                &uniform,
                &[&color_view, &overlay],
                None,
            ),
            resolve_pipeline: system.resolve.clone(),
            composite_pipeline: system.composite.clone(),
            record: None,
            color: color_view.clone(),
            depth: depth.clone(),
            target: composite_view.clone(),
            overlay: overlay.clone(),
        };
        let mut e = d.create_command_encoder(&Default::default());
        drop(e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Current native depth fixture"),
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &current_depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.5),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        }));
        draw.resolve(&mut e, None);
        draw.composite(&mut e, None);
        for (i, t) in [&color, &composite].into_iter().enumerate() {
            e.copy_texture_to_buffer(
                t.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &read,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: i as u64 * 2048,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(8),
                    },
                },
                wgpu::Extent3d {
                    width: 8,
                    height: 8,
                    depth_or_array_layers: 1,
                },
            );
        }
        r.queue.submit([e.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        read.slice(..)
            .map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
        d.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        let mapped = read.slice(..).get_mapped_range();
        let history = match mode {
            0 | 1 | 7 | 14 | 15 => [0.375, 0.375, 0.375, 1.],
            4 | 9 => [0.; 4],
            5 => [0.2, 0.1, 0., 0.5],
            8 => [0., 0.5, 0.5, 1.],
            13 => [2., 0.5, 100., 1.],
            _ => [0.5, 0.5, 0.5, 1.],
        };
        let over = if mode >= 16 {
            [0.0625, 0., 0., 0.5]
        } else {
            over
        };
        for (index, expected) in [
            history,
            std::array::from_fn(|i| over[i] + history[i] * (1. - over[3])),
        ]
        .into_iter()
        .enumerate()
        {
            let at = index * 2048 + 3 * 256 + 3 * 8;
            let actual: [f32; 4] = std::array::from_fn(|i| {
                half(u16::from_le_bytes(
                    mapped[at + i * 2..at + i * 2 + 2].try_into().unwrap(),
                ))
            });
            println!("upscaling mode{mode} attachment{index}: {actual:?}");
            for (a, b) in actual.into_iter().zip(expected) {
                assert!(
                    (a - b).abs() < 0.001,
                    "mode{mode} attachment{index}: {actual:?} != {expected:?}"
                );
            }
            assert!(
                mapped[index * 2048..index * 2048 + 8]
                    .iter()
                    .all(|&b| b == 0),
                "outside output viewport must be clear"
            );
        }
        drop(mapped);
        read.unmap();
    }
}

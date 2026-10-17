//! Small execution discriminator for the actual shader before full motion review.
use super::*;
#[test]
#[ignore = "coordinated GPU only: eleven 8x8 resolves, bounded half-float readback"]
fn actual_current_depth_and_history_support_refusals_remain_finite() {
    let mut r = crate::test_support::checked(Renderer::headless().unwrap());
    pollster::block_on(r.enable_temporal_upscale()).unwrap();
    let s = r.temporal_upscale.as_ref().unwrap();
    let d = &r.device;
    let size = wgpu::Extent3d {
        width: 8,
        height: 8,
        depth_or_array_layers: 1,
    };
    let texture = |format, usage| {
        d.create_texture(&wgpu::TextureDescriptor {
            label: Some("TAAU tiny discriminator"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let upload = |v: &[[f32; 4]]| {
        let t = texture(
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        );
        r.queue.write_texture(
            t.as_image_copy(),
            bytemuck::cast_slice(v),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(128),
                rows_per_image: Some(8),
            },
            size,
        );
        t.create_view(&Default::default())
    };
    let depth = texture(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
    )
    .create_view(&Default::default());
    let color = texture(
        crate::frame::HDR_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let output = color.create_view(&Default::default());
    let support = texture(
        wgpu::TextureFormat::Rg32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    )
    .create_view(&Default::default());
    let buffer = d.create_buffer(&wgpu::BufferDescriptor {
        label: Some("TAAU 8x8 readback"),
        size: 256 * 8,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut results = Vec::new();
    for mode in 0..11 {
        let mut current = vec![[0.2, 0.2, 0.2, 1.]; 64];
        for y in 0..8 {
            current[y * 8 + 2] = [0., 0., 0., 1.];
            current[y * 8 + 4] = [1.; 4];
        }
        let mut motion = vec![[0., 0., 0.5, 1.]; 64];
        let mut old = vec![[0.7, 0.7, 0.7, 1.]; 64];
        let mut old_support = vec![[0.5, 1., 0., 0.]; 64];
        if mode == 2 {
            old_support.fill([0.9, 1., 0., 0.]);
        }
        if mode == 3 {
            motion.fill([0., 0., 0.5, 0.]);
        }
        if mode == 4 {
            old_support.fill([0.5, 0., 0., 0.]);
        }
        if mode == 5 {
            old.fill([f32::NAN, 0., 0., 1.]);
        }
        if mode == 6 {
            current.fill([0.; 4]);
        }
        if mode == 7 {
            current.fill([0.1, 0.2, 0.3, 0.5]);
        }
        if mode == 8 {
            motion.fill([2., 0., 0.5, 1.]);
        }
        if mode == 9 {
            motion.fill([0.25 / 8., 0.25 / 8., 0.5, 1.]);
            old_support[2 * 8 + 2] = [0.9, 1., 0., 0.]; // nonzero negative-lobe tap
        }
        if mode == 10 {
            // Current depths tie at 0.5; the first tap (2,2) is the stable
            // current-depth choice. Other taps have SMALLER expected previous
            // depth. Choosing by motion.z would select them and reject history.
            motion.fill([0., 0., 0.1, 1.]);
            motion[2 * 8 + 2] = [0., 0., 0.5, 1.];
        }
        let uniform = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::UNIFORM,
            contents: bytemuck::bytes_of(&Frame {
                source: [0., 0., 8., 8.],
                output: [8., 8., 0., 0.],
                depth: [0.1, 10., 1., if mode == 0 || mode == 7 { 0. } else { 1. }],
                jitter: [0.; 4],
            }),
        });
        let textures = [
            upload(&current),
            upload(&motion),
            depth.clone(),
            upload(&old),
            upload(&old_support),
        ];
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }];
        entries.extend(
            textures
                .iter()
                .enumerate()
                .map(|(i, v)| wgpu::BindGroupEntry {
                    binding: 1 + i as u32,
                    resource: wgpu::BindingResource::TextureView(v),
                }),
        );
        let draw = Draw {
            pipeline: s.resolve.clone(),
            group: d.create_bind_group(&wgpu::BindGroupDescriptor {
                label: None,
                layout: &s.layout,
                entries: &entries,
            }),
            color: output.clone(),
            support: support.clone(),
        };
        let mut encoder = d.create_command_encoder(&Default::default());
        {
            let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(0.5),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
        }
        draw.encode(&mut encoder, None);
        encoder.copy_texture_to_buffer(
            color.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(8),
                },
            },
            size,
        );
        r.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer.slice(..).map_async(wgpu::MapMode::Read, move |v| {
            let _ = tx.send(v);
        });
        d.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        let data = buffer.slice(..).get_mapped_range();
        let pixel: [f32; 4] = std::array::from_fn(|i| {
            half(u16::from_le_bytes(
                data[3 * 256 + 3 * 8 + i * 2..3 * 256 + 3 * 8 + i * 2 + 2]
                    .try_into()
                    .unwrap(),
            ))
        });
        for row in data.as_chunks::<256>().0 {
            for value in row[..64].as_chunks::<2>().0 {
                assert!(half(u16::from_le_bytes(*value)).is_finite());
            }
        }
        drop(data);
        buffer.unmap();
        results.push(pixel);
    }
    let baseline = results[0];
    assert!((baseline[0] - 0.2).abs() < 0.002);
    assert!(
        results[1][0] > baseline[0] + 0.05,
        "valid old color must actually contribute"
    );
    for i in [2, 3, 4, 5, 8, 9] {
        assert_eq!(results[i], baseline, "mode {i} must refuse old history");
    }
    assert_eq!(
        results[10], results[1],
        "selection uses current depth, not expected previous depth"
    );
    assert_eq!(results[6], [0.; 4]);
    for (a, b) in results[7].into_iter().zip([0.1, 0.2, 0.3, 0.5]) {
        assert!((a - b).abs() < 0.002);
    }
    println!(
        "TAAU tiny actual shader: {} modes; {:?}",
        results.len(),
        results
    );
}
fn half(bits: u16) -> f32 {
    let sign = if bits & 0x8000 == 0 { 1. } else { -1. };
    let exponent = (bits >> 10) & 31;
    let mantissa = f32::from(bits & 1023);
    if exponent == 0 {
        sign * mantissa * 2f32.powi(-24)
    } else if exponent == 31 {
        if mantissa == 0. {
            sign * f32::INFINITY
        } else {
            f32::NAN
        }
    } else {
        sign * (1. + mantissa / 1024.) * 2f32.powi(i32::from(exponent) - 15)
    }
}

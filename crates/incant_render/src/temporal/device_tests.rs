//! Direct numeric execution of the actual resolve/copy shaders. No image files.
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
#[ignore = "requires native GPU; Claude owns numerical execution"]
fn actual_temporal_resolve_disocclusion_reactivity_and_alpha_exclusion() {
    let mut renderer = crate::test_support::checked(Renderer::headless().unwrap());
    if crate::motion::admission().is_err() {
        assert!(pollster::block_on(renderer.enable_temporal_accumulation()).is_err());
        return;
    }
    pollster::block_on(renderer.enable_temporal_accumulation()).unwrap();
    let system = renderer.temporal.as_ref().unwrap();
    let d = &renderer.device;
    let size = wgpu::Extent3d {
        width: 8,
        height: 8,
        depth_or_array_layers: 1,
    };
    let texture = |format, usage| {
        d.create_texture(&wgpu::TextureDescriptor {
            label: Some("Temporal numeric fixture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let source = |values: &[[f32; 4]]| {
        let t = texture(
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        renderer.queue.write_texture(
            t.as_image_copy(),
            bytemuck::cast_slice(values),
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
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let depth = depth.create_view(&Default::default());
    let output = texture(
        crate::frame::HDR_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let output_view = output.create_view(&Default::default());
    let copy_target = texture(
        crate::frame::HDR_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let copy_view = copy_target.create_view(&Default::default());
    let buffer = d.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Temporal scalar readback"),
        size: 256 * 8,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    for mode in 0..8 {
        let mut current = vec![[0.5, 0.5, 0.5, 1.]; 64];
        current[3 * 8 + 2] = [0., 0., 0., 1.];
        current[3 * 8 + 4] = [1.; 4];
        if mode == 4 {
            current.fill([0., 0., 0., 0.]);
        }
        if mode == 5 {
            current.fill([0.2, 0.1, 0., 0.5]);
        }
        if mode == 7 {
            current.fill([2., 0.5, 100., 1.]);
        }
        let old = if mode == 7 {
            [2., 0.5, 100., 1.]
        } else if mode == 6 {
            [20., 20., 20., 1.]
        } else {
            [0.25, 0.25, 0.25, 1.]
        };
        let old = source(&vec![old; 64]);
        let current = source(&current);
        let motion = source(&vec![
            [
                if mode == 3 { 2. } else { 0. },
                0.,
                if mode == 2 { 0.8 } else { 0.5 },
                1.
            ];
            64
        ]);
        let uniform = Frame {
            rect: [1., 1., 6., 6.],
            depth: [0.1, 100., f32::from(u8::from(mode == 1)), 1.],
            options: [0.5, 0.001, 0.001, 1.],
            reactive: [f32::from(u8::from(mode == 6)), 0., 0., 0.],
        };
        let uniform = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let resolve = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &system.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&current),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&motion),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&depth),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&old),
                },
            ],
        });
        let copy = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &system.copy_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&output_view),
            }],
        });
        let draw = TemporalDraw {
            resolve,
            copy,
            resolve_pipeline: system.resolve.clone(),
            copy_pipeline: system.copy.clone(),
            history: output_view.clone(),
            current: copy_view.clone(),
        };
        let mut encoder = d.create_command_encoder(&Default::default());
        // Depth clear is the real previous opaque depth input, not a fake sampler.
        drop(encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Numeric prior depth"),
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0.5),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        }));
        draw.encode(&mut encoder);
        encoder.copy_texture_to_buffer(
            copy_target.as_image_copy(),
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
        renderer.queue.submit([encoder.finish()]);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |r| tx.send(r).unwrap());
        d.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(10)),
        })
        .unwrap();
        rx.recv_timeout(std::time::Duration::from_secs(10))
            .unwrap()
            .unwrap();
        let mapped = buffer.slice(..).get_mapped_range();
        let at = 3 * 256 + 3 * 8;
        let actual: [f32; 4] = std::array::from_fn(|i| {
            half(u16::from_le_bytes(
                mapped[at + i * 2..at + i * 2 + 2].try_into().unwrap(),
            ))
        });
        let expected = match mode {
            0 | 1 => [0.375, 0.375, 0.375, 1.],
            4 => [0.; 4],
            5 => [0.2, 0.1, 0., 0.5],
            7 => [2., 0.5, 100., 1.],
            _ => [0.5, 0.5, 0.5, 1.],
        };
        for (a, b) in actual.into_iter().zip(expected) {
            assert!((a - b).abs() < 0.001, "mode{mode}: {actual:?}");
        }
        drop(mapped);
        buffer.unmap();
    }
}

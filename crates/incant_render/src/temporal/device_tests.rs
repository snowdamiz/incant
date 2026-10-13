//! Direct numeric execution of the actual resolve/copy shaders. No image files.
use super::*;
pub(super) fn half(bits: u16) -> f32 {
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
    for mode in 0..14 {
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
        if mode == 12 {
            for y in 0..8 {
                for x in 0..8 {
                    current[y * 8 + x] = [0.2 + 0.03 * x as f32 + 0.02 * y as f32, 0.25, 0.5, 1.];
                }
            }
        }
        let old = if mode == 7 {
            [2., 0.5, 100., 1.]
        } else if mode == 6 {
            [20., 20., 20., 1.]
        } else {
            [0.25, 0.25, 0.25, 1.]
        };
        let mut old_values = vec![old; 64];
        if mode == 8 || mode == 9 {
            // Zero-weight and positive-weight incompatible neighbors respectively.
            old_values[3 * 8 + 4] = [f32::NAN, 0., 0., 1.];
            old_values[4 * 8 + 3] = [0., 0., 0., 2.];
            old_values[4 * 8 + 4] = [0., 0., 0., 2.];
        }
        if mode == 10 {
            old_values.fill([0., 0., 0., 2.]);
        }
        let old = source(&old_values);
        let current = source(&current);
        let motion = source(&vec![
            [
                if mode == 3 {
                    2.
                } else if mode == 9 || mode == 13 {
                    0.5 / 6.
                } else if mode == 11 {
                    -2.25 / 6.
                } else {
                    0.
                },
                0.,
                if mode == 2 { 0.8 } else { 0.5 },
                if mode == 12 { 0. } else { 1. }
            ];
            64
        ]);
        let uniform = Frame {
            rect: [1., 1., 6., 6.],
            depth: [0.1, 100., f32::from(u8::from(mode == 1)), 1.],
            options: [0.5, 0.001, 0.001, 1.],
            reactive: [
                f32::from(u8::from(mode == 6)),
                if mode == 12 { 0.375 } else { 0. },
                if mode == 12 { -0.375 } else { 0. },
                0.,
            ],
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
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&output_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform.as_entire_binding(),
                },
            ],
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
        if mode == 13 {
            // Real nonuniform depth at the positive-weight neighbor. Correct
            // renormalization admits x=3 and rejects x=4 without dropping history.
            let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Depth discontinuity fixture"),
                source: wgpu::ShaderSource::Wgsl(
                    r#"
                    @vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
                        let uv=vec2f(f32((i<<1u)&2u),f32(i&2u));return vec4f(uv*2.-1.,0.,1.);
                    }
                    @fragment fn fragment(@builtin(position) p:vec4f)->@builtin(frag_depth) f32 {
                        return select(0.5,0.8,p.x>=4.);
                    }"#
                    .into(),
                ),
            });
            let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Depth discontinuity fixture"),
                layout: None,
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                multisample: Default::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment"),
                    compilation_options: Default::default(),
                    targets: &[],
                }),
                multiview_mask: None,
                cache: None,
            });
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Depth discontinuity fixture"),
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.draw(0..3, 0..1);
        }
        draw.encode(&mut encoder, None, None);
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
            0 | 1 | 8 | 9 | 11 | 13 => [0.375, 0.375, 0.375, 1.],
            4 => [0.; 4],
            5 => [0.2, 0.1, 0., 0.5],
            7 => [2., 0.5, 100., 1.],
            12 => [0.2 + 0.03 * 3.375 + 0.02 * 2.625, 0.25, 0.5, 1.],
            _ => [0.5, 0.5, 0.5, 1.],
        };
        for (a, b) in actual.into_iter().zip(expected) {
            assert!((a - b).abs() < 0.001, "mode{mode}: {actual:?}");
        }
        drop(mapped);
        buffer.unmap();
    }
}

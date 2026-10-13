//! Bounded numeric shader execution. No images or presentation surfaces.
use super::{super::coverage::*, *};
use wgpu::util::DeviceExt;

#[test]
#[ignore = "requires native GPU; Claude owns numerical execution"]
fn actual_fixed_temporal_coverage_silhouettes_mask_and_trailing_rejection() {
    let mut renderer = crate::test_support::checked(Renderer::headless().unwrap());
    pollster::block_on(renderer.enable_temporal_accumulation()).unwrap();
    let system = renderer.temporal.as_ref().unwrap();
    let d = &renderer.device;
    let extent = wgpu::Extent3d {
        width: SIZE[0] as u32,
        height: SIZE[1] as u32,
        depth_or_array_layers: 1,
    };
    let texture = |format, usage| {
        d.create_texture(&wgpu::TextureDescriptor {
            label: Some("Coverage numeric texture"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let data_usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    let current = texture(wgpu::TextureFormat::Rgba32Float, data_usage);
    let motion = texture(wgpu::TextureFormat::Rgba32Float, data_usage);
    let old = texture(wgpu::TextureFormat::Rgba32Float, data_usage);
    let zdata = texture(wgpu::TextureFormat::Rgba32Float, data_usage);
    let depth = texture(
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let output = texture(
        crate::frame::HDR_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
    );
    let presented = texture(
        crate::frame::HDR_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let views: Vec<_> = [&current, &motion, &old, &zdata, &depth, &output, &presented]
        .into_iter()
        .map(|t| t.create_view(&Default::default()))
        .collect();
    let uniform = d.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
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
                resource: wgpu::BindingResource::TextureView(&views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&views[1]),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(&views[4]),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(&views[2]),
            },
        ],
    });
    let copy = d.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &system.copy_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&views[5]),
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
        history: views[5].clone(),
        current: views[6].clone(),
    };
    let depth_shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Exact supplied prior depth"),
        source: wgpu::ShaderSource::Wgsl(
            r#"
        @group(0) @binding(0) var data:texture_2d<f32>;
        @vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
            let uv=vec2f(f32((i<<1u)&2u),f32(i&2u)); return vec4f(uv*2.-1.,0.,1.);
        }
        @fragment fn fragment(@builtin(position) p:vec4f)->@builtin(frag_depth) f32 {
            return textureLoad(data,vec2i(p.xy),0).x;
        }
    "#
            .into(),
        ),
    });
    let depth_layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        }],
    });
    let depth_pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&depth_layout)],
        immediate_size: 0,
    });
    let depth_pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Coverage prior-depth fixture"),
        layout: Some(&depth_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &depth_shader,
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
            module: &depth_shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    });
    let depth_group = d.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &depth_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&views[3]),
        }],
    });
    let readback = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Coverage scalar readback"),
        contents: &[0; 256 * SIZE[1]],
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
    });
    let upload = |texture: &wgpu::Texture, values: &[[f32; 4]]| {
        renderer.queue.write_texture(
            texture.as_image_copy(),
            bytemuck::cast_slice(values),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE[0] as u32 * 16),
                rows_per_image: Some(SIZE[1] as u32),
            },
            extent,
        )
    };
    for ortho in [false, true] {
        for case in CASES {
            let first = fixture(case, 0, ortho);
            let mut history = first.color;
            if matches!(case, Case::MalformedClear) {
                for c in &mut history {
                    if c[3] == 1. {
                        *c = [0.25, 0.125, 0.0625, 1.];
                    }
                }
            }
            let mut previous_depth = first.depth;
            // Fixed 150 tiny resolves maximum; no open-ended convergence or image IO.
            for tick in 1..16 {
                let input = fixture(case, tick, ortho);
                let expected =
                    super::super::coverage::resolve(&input, &history, &previous_depth, ortho);
                upload(&current, &input.color);
                upload(&motion, &input.motion);
                upload(&old, &history);
                upload(
                    &zdata,
                    &previous_depth
                        .iter()
                        .map(|z| [*z, 0., 0., 0.])
                        .collect::<Vec<_>>(),
                );
                let frame = Frame {
                    rect: RECT.map(|v| v as f32),
                    depth: [NEAR as f32, FAR as f32, f32::from(u8::from(ortho)), 1.],
                    options: [
                        CONFIG.history_weight,
                        CONFIG.relative_depth,
                        CONFIG.absolute_depth,
                        CONFIG.reactive_threshold,
                    ],
                    reactive: [CONFIG.reactive_strength, 0., 0., 0.],
                };
                renderer
                    .queue
                    .write_buffer(&uniform, 0, bytemuck::bytes_of(&frame));
                let mut encoder = d.create_command_encoder(&Default::default());
                {
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Coverage prior depth"),
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &views[4],
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.),
                                store: wgpu::StoreOp::Store,
                            }),
                            stencil_ops: None,
                        }),
                        ..Default::default()
                    });
                    pass.set_pipeline(&depth_pipeline);
                    pass.set_bind_group(0, &depth_group, &[]);
                    pass.draw(0..3, 0..1);
                }
                draw.encode(&mut encoder, None, None);
                encoder.copy_texture_to_buffer(
                    presented.as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &readback,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(256),
                            rows_per_image: Some(SIZE[1] as u32),
                        },
                    },
                    extent,
                );
                renderer.queue.submit([encoder.finish()]);
                let (tx, rx) = std::sync::mpsc::channel();
                readback.slice(..).map_async(wgpu::MapMode::Read, move |r| {
                    let _ = tx.send(r);
                });
                d.poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(std::time::Duration::from_secs(10)),
                })
                .unwrap();
                rx.recv_timeout(std::time::Duration::from_secs(10))
                    .unwrap()
                    .unwrap();
                let mapped = readback.slice(..).get_mapped_range();
                let actual: Vec<[f32; 4]> = (0..SIZE[0] * SIZE[1])
                    .map(|i| {
                        std::array::from_fn(|k| {
                            let at = (i / SIZE[0]) * 256 + (i % SIZE[0]) * 8 + k * 2;
                            super::device_tests::half(u16::from_le_bytes(
                                mapped[at..at + 2].try_into().unwrap(),
                            ))
                        })
                    })
                    .collect();
                drop(mapped);
                readback.unmap();
                for (i, (a, e)) in actual.iter().zip(&expected).enumerate() {
                    for k in 0..4 {
                        assert!(
                            (a[k] - e[k]).abs() < 0.001,
                            "{case:?} ortho={ortho} tick={tick} pixel={i} component={k}: {a:?} vs {e:?}"
                        );
                    }
                }
                let at = 3 * SIZE[0] + 4;
                if tick >= 8 && matches!(case, Case::DepthEdge | Case::ClearEdge | Case::MaskGrille)
                {
                    assert!(actual[at][0] > 0.1 && actual[at][0] < 1.);
                    if !matches!(case, Case::DepthEdge) {
                        assert!(actual[at][3] > 0. && actual[at][3] < 1.);
                    }
                    let contrast = if matches!(case, Case::DepthEdge) {
                        0.9
                    } else {
                        1.
                    };
                    assert!(
                        (actual[at][0] - history[at][0]).abs()
                            <= (1. - CONFIG.history_weight) * contrast + 0.001
                    );
                }
                if matches!(case, Case::MalformedClear) {
                    assert_eq!(actual[at], [0.; 4], "residual RGB at alpha zero");
                    if tick == 1 {
                        assert!((actual[at - 1][0] - 0.325).abs() < 0.001);
                    }
                }
                if matches!(case, Case::Moving) {
                    for x in RECT[0]..=tick.min(RECT[0] + RECT[2] - 1) {
                        let i = 3 * SIZE[0] + x;
                        for (a, expected) in actual[i].iter().zip(input.color[i]) {
                            assert!(
                                (a - expected).abs() < 0.0005,
                                "trailing ghost {x} at {tick}"
                            );
                        }
                    }
                }
                history = actual;
                previous_depth = input.depth;
            }
        }
    }
}

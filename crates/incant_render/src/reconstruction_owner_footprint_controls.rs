//! Actual shader weights against independently integrated Gaussian references.
use super::*;

#[test]
#[ignore = "bounded Metal footprint kernel, one submission/map; not a quality gate"]
fn actual_footprint_kernel_reference() {
    use super::super::recurrence_fixture::Frame;
    let reference: serde_json::Value = serde_json::from_str(include_str!(
        "reconstruction_owner_footprint_reference.json"
    ))
    .unwrap();
    let cases = reference["cases"].as_array().unwrap();
    assert!(!cases.is_empty() && cases.len() <= 2048);
    let absolute = reference["abs_tolerance"].as_f64().unwrap();
    let relative = reference["rel_tolerance"].as_f64().unwrap();
    assert!((0. ..=2e-6).contains(&absolute) && (0. ..=2e-5).contains(&relative));
    let out = PathBuf::from(std::env::var("INCANT_OWNER_FOOTPRINT_CONTROLS_OUT").unwrap());
    std::fs::create_dir(&out).expect("fresh footprint controls output");
    let renderer = owner_renderer();
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Metal);
    let d = &renderer.device;
    let mut source = shader_with_kernel(3, Kernel::Footprint);
    source.push_str(
        "\n@fragment fn footprint_probe()->@location(0) vec4f {\n\
         let v=footprint_weight(frame.options.xy,frame.options.z,frame.options.w);\n\
         return vec4f(v,v,v,1.0);\n}\n",
    );
    std::fs::write(out.join("probe.wgsl"), &source).unwrap();
    let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Integrated footprint numerical probe"),
        source: wgpu::ShaderSource::Wgsl(source.clone().into()),
    });
    let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(size_of::<Frame>() as u64),
            },
            count: None,
        }],
    });
    let pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Actual integrated footprint kernel"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("footprint_probe"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba32Float,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    let target = d
        .create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
        .create_view(&Default::default());
    let read = d.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: cases.len() as u64 * 256,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = d.create_command_encoder(&Default::default());
    for (index, case) in cases.iter().enumerate() {
        let frame = Frame {
            source: [0., 0., 1., 1.],
            output: [0., 0., 1., 1.],
            depth: [1., 17., 1., 1.],
            options: [
                case["d"][0].as_f64().unwrap() as f32,
                case["d"][1].as_f64().unwrap() as f32,
                case["rho"].as_f64().unwrap() as f32,
                case["r"].as_f64().unwrap() as f32,
            ],
            reactive: [0.; 4],
            jitter: [0.; 4],
        };
        assert!(frame.options.iter().all(|v| v.is_finite()));
        assert!(frame.options[2] >= 0.35 && frame.options[3] > 0.);
        let uniform = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&frame),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[attachment(&target)],
                ..Default::default()
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            target.texture().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: index as u64 * 256,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(1),
                },
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
    }
    renderer.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    read.slice(..).map_async(wgpu::MapMode::Read, move |r| {
        let _ = tx.send(r);
    });
    d.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    })
    .unwrap();
    rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    let bytes = read.slice(..).get_mapped_range();
    std::fs::write(out.join("weights.bin"), &bytes).unwrap();
    let mut results = Vec::new();
    for (index, case) in cases.iter().enumerate() {
        let rgba: [f32; 4] = std::array::from_fn(|i| {
            let at = index * 256 + i * 4;
            f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
        });
        let expected = case["expected"].as_f64().unwrap();
        let error = (f64::from(rgba[0]) - expected).abs();
        let passed = rgba[0].is_finite()
            && rgba[0] >= 0.
            && rgba[0] == rgba[1]
            && rgba[0] == rgba[2]
            && rgba[3] == 1.
            && error <= absolute + relative * expected.abs()
            && (expected < f64::from(f32::MIN_POSITIVE) || rgba[0] > 0.);
        results.push(json!({"case":case,"rgba":rgba,"absolute_error":error,"passed":passed}));
    }
    let report = json!({"adapter":describe(&renderer),"submissions":1,"maps":1,"shader_sha256":hex(source.as_bytes()),"reference_sha256":hex(include_bytes!("reconstruction_owner_footprint_reference.json")),"raw_sha256":hex(&bytes),"raw_bytes":bytes.len(),"absolute_tolerance":absolute,"relative_tolerance":relative,"cases":results,"scope":"Actual integrated-kernel numerical controls; no appearance or performance acceptance"});
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    drop(bytes);
    read.unmap();
    assert!(
        results.iter().all(|row| row["passed"] == true),
        "{results:?}"
    );
}

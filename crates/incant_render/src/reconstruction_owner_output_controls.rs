//! Actual final-store precision, with runtime texture inputs and paired float32
//! diagnostics. The CPU half crate supplies the independent ties-to-even oracle;
//! passing scalar controls does not establish scene or appearance acceptance.
use super::*;

const N: u32 = 16;
const STRIDE: u32 = 256;
const PLANE: u64 = STRIDE as u64 * N as u64;

fn cases() -> Vec<(String, f32)> {
    let mut values = vec![
        ("positive_zero".into(), 0.),
        ("negative_zero".into(), -0.),
        ("below_zero_midpoint".into(), 2_f32.powi(-26)),
        ("subhalf_feedback".into(), 2.495_559_8e-8_f32),
        ("representable_feedback".into(), 4.991_119_7e-8_f32),
        ("minimum_subnormal".into(), half::f16::from_bits(1).to_f32()),
        (
            "largest_subnormal".into(),
            half::f16::from_bits(1023).to_f32(),
        ),
        ("minimum_normal".into(), half::f16::from_bits(1024).to_f32()),
        ("one".into(), 1.),
    ];
    // Consecutive half values have exactly representable f32 midpoints. Both
    // parity choices and exponent carries are observed immediately around ties.
    for lo in [
        0_u16, 1, 2, 3, 7, 8, 511, 512, 1022, 1023, 1024, 1025, 2047, 2048, 0x37ff, 0x3800, 0x3bfe,
        0x3bff,
    ] {
        let a = half::f16::from_bits(lo).to_f32();
        let b = half::f16::from_bits(lo + 1).to_f32();
        let midpoint = (a + b) * 0.5;
        for (name, bits) in [
            ("below", midpoint.to_bits() - 1),
            ("tie", midpoint.to_bits()),
            ("above", midpoint.to_bits() + 1),
        ] {
            values.push((format!("half_{lo:04x}_{name}"), f32::from_bits(bits)));
        }
    }
    for exponent in -14..=0 {
        let value = 2_f32.powi(exponent);
        values.push((format!("power_{exponent}"), value));
        values.push((
            format!("power_{exponent}_below"),
            f32::from_bits(value.to_bits() - 1),
        ));
        if value < 1. {
            values.push((
                format!("power_{exponent}_above"),
                f32::from_bits(value.to_bits() + 1),
            ));
        }
    }
    assert!(values.len() <= (N * N) as usize);
    assert!(
        values
            .iter()
            .all(|(_, v)| v.is_finite() && (0. ..=1.).contains(v))
    );
    values
}

#[test]
#[ignore = "bounded final RGBA16 store precision; one submit/map, 256 pixels"]
fn actual_owner_output_precision() {
    let out = PathBuf::from(std::env::var("INCANT_OWNER_OUTPUT_CONTROLS_OUT").unwrap());
    std::fs::create_dir(&out).expect("fresh output precision directory");
    let cases = cases();
    let helper = include_str!("upscaling/owner_output_half_experiment.wgsl");
    let entry = "@fragment fn resolve(";
    assert_eq!(helper.matches(entry).count(), 1);
    let (function, _) = helper.split_once(entry).unwrap();
    assert_eq!(function.matches("fn output_alpha(").count(), 1);
    let source = format!(
        "{function}\n{}",
        r#"
@group(0) @binding(0) var input:texture_2d<f32>;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
    let uv=vec2f(f32((i<<1u)&2u),f32(i&2u));
    return vec4f(uv*2.0-1.0,0.0,1.0);
}
struct Probe { @location(0) color:vec4f, @location(1) observed:vec4f }
@fragment fn probe(@builtin(position) p:vec4f)->Probe {
    let original=textureLoad(input,vec2i(p.xy),0);
    let rounded=output_alpha(original.a);
    var color=original;
    if rounded==0.0 {color=vec4f(0.0);} else {color.a=rounded;}
    return Probe(color,vec4f(original.a,rounded,color.a,1.0));
}
"#
    );
    std::fs::write(out.join("probe.wgsl"), &source).unwrap();
    std::fs::write(out.join("output-helper.wgsl"), helper).unwrap();
    let renderer = owner_renderer();
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Metal);
    let d = &renderer.device;
    let extent = wgpu::Extent3d {
        width: N,
        height: N,
        depth_or_array_layers: 1,
    };
    let texture = |format, usage| {
        d.create_texture(&wgpu::TextureDescriptor {
            label: Some("Output precision probe"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let input = texture(
        wgpu::TextureFormat::Rgba32Float,
        wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
    );
    let input_view = input.create_view(&Default::default());
    let values: Vec<[f32; 4]> = (0..N * N)
        .map(|i| {
            [
                0.5,
                0.25,
                0.125,
                cases.get(i as usize).map_or(0., |(_, v)| *v),
            ]
        })
        .collect();
    let input_bytes: &[u8] = bytemuck::cast_slice(&values);
    std::fs::write(out.join("input-rgba32.bin"), input_bytes).unwrap();
    renderer.queue.write_texture(
        input.as_image_copy(),
        input_bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(N * 16),
            rows_per_image: Some(N),
        },
        extent,
    );
    let formats = [
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureFormat::Rgba32Float,
    ];
    let outputs = formats.map(|format| {
        texture(
            format,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        )
    });
    let views = outputs
        .each_ref()
        .map(|t| t.create_view(&Default::default()));
    let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    let group = d.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&input_view),
        }],
    });
    let pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Final output precision"),
        source: wgpu::ShaderSource::Wgsl(source.clone().into()),
    });
    let targets = formats.map(|format| {
        Some(wgpu::ColorTargetState {
            format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })
    });
    let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
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
            entry_point: Some("probe"),
            compilation_options: Default::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    });
    let read = d.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 2 * PLANE,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = d.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[attachment(&views[0]), attachment(&views[1])],
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
    for (i, output) in outputs.iter().enumerate() {
        encoder.copy_texture_to_buffer(
            output.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: i as u64 * PLANE,
                    bytes_per_row: Some(STRIDE),
                    rows_per_image: Some(N),
                },
            },
            extent,
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
    std::fs::write(out.join("paired-output.bin"), &bytes).unwrap();
    let mut results = Vec::new();
    for (i, (name, value)) in cases.iter().enumerate() {
        let x = i % N as usize;
        let y = i / N as usize;
        let rgba: [u16; 4] = std::array::from_fn(|c| {
            let at = y * STRIDE as usize + x * 8 + c * 2;
            u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
        });
        let diagnostic: [f32; 4] = std::array::from_fn(|c| {
            let at = PLANE as usize + y * STRIDE as usize + x * 16 + c * 4;
            f32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
        });
        let nearest = half::f16::from_f32(*value);
        let expected_bits = if nearest == half::f16::ZERO {
            0
        } else {
            nearest.to_bits()
        };
        let expected = half::f16::from_bits(expected_bits).to_f32();
        let expected_rgba = if expected_bits == 0 {
            [0; 4]
        } else {
            [0x3800, 0x3400, 0x3000, expected_bits]
        };
        let input_matches =
            diagnostic[0].to_bits() == value.to_bits() || (diagnostic[0] == 0. && *value == 0.);
        let helper_matches = diagnostic[1].to_bits() == expected.to_bits();
        let final_matches = diagnostic[2].to_bits() == expected.to_bits();
        let passed = input_matches
            && helper_matches
            && final_matches
            && diagnostic[3] == 1.
            && rgba == expected_rgba;
        results.push(
            json!({"name":name,"pixel":[x,y],"input":value,"input_bits":value.to_bits(),
            "expected_alpha_bits":expected_bits,"expected_rgba16_bits":expected_rgba,
            "actual_rgba16_bits":rgba,"diagnostic":diagnostic,
            "diagnostic_bits":diagnostic.map(f32::to_bits),"input_matches":input_matches,
            "helper_matches":helper_matches,"final_matches":final_matches,"passed":passed}),
        );
    }
    let report = json!({"adapter":describe(&renderer),"submissions":1,"maps":1,"draws":1,
        "upload_writes":1,"extent":[N,N],"formats":["RGBA16Float","RGBA32Float"],
        "diagnostic_channels":["runtime input alpha","helper alpha","final alpha","one"],
        "shader_sha256":hex(source.as_bytes()),"helper_sha256":hex(helper.as_bytes()),
        "input_sha256":hex(input_bytes),"raw_sha256":hex(&bytes),"raw_bytes":bytes.len(),
        "row_stride":STRIDE,"plane_offsets":[0,PLANE],"cases":results,
        "scope":"Exact scalar RNE and final attachment controls; no scene, appearance or performance acceptance"});
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

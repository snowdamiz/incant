//! Test-only hardware checks of the candidate classifier, not scene quality.
use super::*;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Input {
    history: [f32; 4],
    fresh: [f32; 4],
    minimum: [f32; 4],
    maximum: [f32; 4],
    ring: [f32; 4],
}
fn half_round(v: f32) -> f32 {
    half::f16::from_f32(v).to_f32()
}
fn bounds(v: f32) -> [f32; 2] {
    let bits = half::f16::from_f32(v.clamp(0., 65_504.)).to_bits() & 0x7fff;
    [bits.saturating_sub(1), (bits + 1).min(0x7bff)].map(|bits| half::f16::from_bits(bits).to_f32())
}

#[test]
fn recurrence_shader_validates_without_gpu() {
    let source = super::recurrence_fixture::SHADER;
    let module = naga::front::wgsl::parse_str(source).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        // pack/unpack store binary16 precision in f32; no f16 shader type.
        naga::valid::Capabilities::SHADER_FLOAT16_IN_FLOAT32,
    )
    .validate(&module)
    .unwrap();
    assert_eq!(bounds(-0.), [0., 2f32.powi(-24)]);
    assert_eq!(bounds(65_504.), [65_472., 65_504.]);
}

#[test]
#[ignore = "bounded explicit Metal classifier experiment; no production or visual acceptance"]
fn actual_recurrence_classifier_controls() {
    let out = PathBuf::from(std::env::var("INCANT_RECONSTRUCTION_CLASSIFIER_OUT").unwrap());
    std::fs::create_dir(&out).expect("fresh classifier output directory");
    let renderer = owner_renderer();
    let d = &renderer.device;
    let base = Input {
        history: [0.45, 0.45, 0.45, 1.],
        fresh: [0.4, 0.4, 0.4, 1.],
        minimum: [0.4, 0.4, 0.4, 1.],
        maximum: [0.4, 0.4, 0.4, 0.],
        ring: [0.8, 0.3, 0.6, 0.1].map(half_round),
    };
    let mut cases = vec![("reversal_with_matching_chroma", base, true)];
    let mut add = |name, mut value: Input, expected| {
        value.ring = value.ring.map(half_round);
        cases.push((name, value, expected));
    };
    add(
        "constant",
        Input {
            ring: [0.4; 4],
            ..base
        },
        false,
    );
    add(
        "adjacent_half_not_recurrence",
        Input {
            fresh: [0.5, 0.5, 0.5, 1.],
            ring: [0.5, 0.500_488_3, 0.5, 0.500_488_3],
            ..base
        },
        false,
    );
    add(
        "rising_only",
        Input {
            ring: [0.3, 0.2, 0.1, 0.05],
            ..base
        },
        false,
    );
    add(
        "falling_only",
        Input {
            ring: [0.5, 0.6, 0.7, 0.8],
            ..base
        },
        false,
    );
    add(
        "history_outside_observed_envelope",
        Input {
            history: [2., 2., 2., 1.],
            ..base
        },
        false,
    );
    let green = 0.6 * 0.2126 / 0.7152;
    add(
        "equal_luma_red_to_green",
        Input {
            history: [0.6, 0., 0., 1.],
            fresh: [0., green, 0., 1.],
            minimum: [0., green, 0., 1.],
            maximum: [0., green, 0., 0.],
            ..base
        },
        false,
    );
    add(
        "black_has_no_chroma_ratio",
        Input {
            fresh: [0., 0., 0., 1.],
            ..base
        },
        false,
    );
    for (name, invalid) in [
        ("missing_lag", -1.),
        ("nan_lag", f32::NAN),
        ("infinite_lag", f32::INFINITY),
    ] {
        add(
            name,
            Input {
                ring: [0.8, invalid, 0.6, 0.1],
                ..base
            },
            false,
        );
    }
    add(
        "nonfinite_history",
        Input {
            history: [f32::NAN, 0.4, 0.4, 1.],
            ..base
        },
        false,
    );
    // An actual oscillating light is observationally indistinguishable here.
    add("known_uniform_lighting_false_positive", base, true);
    add(
        "negative_zero_bounds",
        Input {
            maximum: [0.4, 0.4, 0.4, -0.],
            ..base
        },
        true,
    );
    add(
        "smallest_half_bounds",
        Input {
            maximum: [0.4, 0.4, 0.4, 2f32.powi(-24)],
            ..base
        },
        true,
    );
    add(
        "largest_half_bounds",
        Input {
            maximum: [0.4, 0.4, 0.4, 65_504.],
            ..base
        },
        true,
    );
    let inputs: Vec<_> = cases.iter().map(|(_, value, _)| *value).collect();
    let input_bytes = bytemuck::cast_slice(&inputs);
    std::fs::write(out.join("inputs.bin"), input_bytes).unwrap();
    let code = format!(
        "{}\n{}",
        include_str!("upscaling/recurrence_experiment.wgsl"),
        r#"
struct Input { history:vec4f, fresh:vec4f, minimum:vec4f, maximum:vec4f, ring:vec4f }
@group(0) @binding(0) var<storage,read> inputs:array<Input>;
@group(0) @binding(1) var<storage,read_write> outputs:array<vec4u>;
@compute @workgroup_size(32) fn controls(@builtin(global_invocation_id) gid:vec3u) {
    if gid.x>=arrayLength(&inputs) {return;}
    let v=inputs[gid.x];let b=recurrence_bounds(v.maximum.w);
    let accepted=recurrence_retains(v.history.rgb,v.fresh.rgb,v.minimum.rgb,v.maximum.rgb,v.ring);
    outputs[gid.x]=vec4u(select(0u,1u,accepted),bitcast<u32>(b.x),bitcast<u32>(b.y),0u);
}
"#
    );
    let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Recurrence classifier hardware controls"),
        source: wgpu::ShaderSource::Wgsl(code.clone().into()),
    });
    let pipeline = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Recurrence classifier hardware controls"),
        layout: None,
        module: &shader,
        entry_point: Some("controls"),
        compilation_options: Default::default(),
        cache: None,
    });
    let input = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: input_bytes,
        usage: wgpu::BufferUsages::STORAGE,
    });
    let size = (cases.len() * 16) as u64;
    let output = d.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = d.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let group = d.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = d.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_compute_pass(&Default::default());
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups((cases.len() as u32).div_ceil(32), 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, size);
    renderer.queue.submit([encoder.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    readback
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |value| {
            tx.send(value).unwrap();
        });
    d.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: Some(Duration::from_secs(10)),
    })
    .unwrap();
    rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    let mapped = readback.slice(..).get_mapped_range();
    std::fs::write(out.join("outputs.bin"), &mapped).unwrap();
    let outputs: &[[u32; 4]] = bytemuck::cast_slice(&mapped);
    let mut rows = Vec::new();
    for ((name, input, expected), actual) in cases.iter().zip(outputs) {
        let expected_bounds = bounds(input.maximum[3]).map(f32::to_bits);
        rows.push(json!({"case":name,"expected":expected,"actual":actual[0],
            "bounds_bits":&actual[1..3],"expected_bounds_bits":expected_bounds}));
    }
    let report = json!({"adapter":describe(&renderer),"shader_sha256":hex(code.as_bytes()),
        "input_sha256":hex(input_bytes),"output_sha256":hex(&mapped),"cases":rows,
        "submissions":1,"maps":1,"scope":"test-only classifier; no history gather, scene or appearance acceptance"});
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    for ((name, input, expected), actual) in cases.iter().zip(outputs) {
        assert_eq!(actual[0], u32::from(*expected), "{name}");
        assert_eq!(
            &actual[1..3],
            &bounds(input.maximum[3]).map(f32::to_bits),
            "{name}"
        );
    }
    drop(mapped);
    readback.unmap();
    println!(
        "RECURRENCE_CLASSIFIER cases={} submissions=1 maps=1",
        cases.len()
    );
}

//! Actual RGBA16 feedback and explicit coverage metadata; no appearance gate.
use super::*;
use controls::{N, PLANE, PRESENT, texture, upload};
use coverage::Coverage;
const PARTIAL: u32 = 1 << 17;
const CASES: [&str; 25] = [
    "static_partial_one",
    "moving_partial_one",
    "static_opaque",
    "moving_opaque",
    "partial_half",
    "unflagged_partial",
    "unflagged_partial_current",
    "invalid_owner",
    "secondary_owner",
    "changed_static",
    "changed_moving",
    "global",
    "age7",
    "age8",
    "reserved_bit",
    "missing_present",
    "invalid_depth",
    "zero_mass",
    "cold",
    "foreground",
    "opaque_current",
    "zero_weight_carry",
    "positive_weight_carry",
    "partial_current",
    "mixed_opaque_current",
];
const TOTAL: usize = CASES.len() * 2 + 18;
const DEPTH: &str = "@group(0) @binding(0) var<uniform> mode:vec4f;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
let uv=vec2f(f32((i<<1u)&2u),f32(i&2u));return vec4f(uv*2.0-1.0,0.0,1.0);}
@fragment fn depth(@builtin(position) p:vec4f)->@builtin(frag_depth) f32 {
if mode.x==1.0 {return 0.5;}
if mode.x==2.0 && u32(p.y)==8u {return 0.5;}
if mode.x==3.0 && all(vec2u(p.xy)==vec2u(8u)) {return 0.25;}
if mode.x==4.0 {return select(0.5,0.25,((u32(p.x)+u32(p.y))&1u)!=0u);}
if mode.x==5.0 && u32(p.y)==6u {return 0.5;}
return 1.0;}";
fn mass(r: f64, jitter: [f64; 2], at: [u32; 2]) -> f64 {
    let p = at.map(|v| f64::from(v) + 0.5);
    let q = [p[0] / r + jitter[0], p[1] / r + jitter[1]];
    let rho = 0.35 + 0.325 * (r - 2.).max(0.);
    let mut sum = 0.;
    for y in -1..=1 {
        for x in -1..=1 {
            let t = [q[0].floor() + f64::from(x), q[1].floor() + f64::from(y)];
            let d = [
                p[0] - r * (t[0] + 0.5 - jitter[0]),
                p[1] - r * (t[1] + 0.5 - jitter[1]),
            ];
            sum += r * r / (2. * std::f64::consts::PI * rho * rho)
                * (-(d[0] * d[0] + d[1] * d[1]) / (2. * rho * rho)).exp();
        }
    }
    sum
}
fn close(a: f32, b: f64) -> bool {
    a.is_finite()
        && a >= 0.
        && if b == 0. {
            a == 0.
        } else {
            half::f16::from_f32(a)
                .to_bits()
                .abs_diff(half::f16::from_f64(b).to_bits())
                <= 1
        }
}
struct Input {
    mode: u32,
    flags: u32,
    changed: u32,
    record: [u32; 2],
    alpha: f32,
    mass: f32,
    age: f32,
    depth: f32,
    cold: bool,
}
fn input(case: &str) -> Input {
    let mut v = Input {
        mode: 0,
        flags: 0,
        changed: 0,
        record: [PRESENT | PARTIAL | 1, 0],
        alpha: 1.,
        mass: 1.,
        age: 0.,
        depth: 9.,
        cold: false,
    };
    match case {
        "static_partial_one" => v.flags = 1,
        "static_opaque" => {
            v.flags = 1;
            v.record[0] = PRESENT | 1;
        }
        "moving_opaque" => v.record[0] = PRESENT | 1,
        "partial_half" => v.alpha = 0.5,
        "unflagged_partial" => {
            v.alpha = 0.5;
            v.record[0] = PRESENT | 1;
        }
        "unflagged_partial_current" => {
            v.alpha = 0.5;
            v.record[0] = PRESENT | 1;
            v.flags = 1;
            v.mode = 1;
        }
        "invalid_owner" => v.record[0] = PRESENT | PARTIAL | 2,
        "secondary_owner" => v.record[1] = 2,
        "changed_static" => {
            v.flags = 1;
            v.changed = 2;
        }
        "changed_moving" => v.changed = 2,
        "global" => v.flags = 2,
        "age7" => v.age = 7.,
        "age8" => v.age = 8.,
        "reserved_bit" => v.record[0] |= 1 << 18,
        "missing_present" => v.record[0] &= !PRESENT,
        "invalid_depth" => v.depth = f32::NAN,
        "zero_mass" => v.mass = 0.,
        "cold" => v.cold = true,
        "foreground" => v.mode = 3,
        "opaque_current" => v.mode = 1,
        "zero_weight_carry" => {
            v.mode = 1;
            v.flags = 1;
            v.mass = 0.;
        }
        "positive_weight_carry" => {
            v.mode = 1;
            v.flags = 1;
        }
        "partial_current" => {
            v.mode = 2;
            v.flags = 1;
            v.record[0] = PRESENT | 1;
        }
        "mixed_opaque_current" => v.mode = 4,
        "moving_partial_one" => {}
        _ => panic!("unknown fixture"),
    }
    v
}
fn expected(case: &str) -> ([f64; 4], [f64; 2], [u32; 2]) {
    let m = mass(1., [0.; 2], [8; 2]);
    let center = 1. / (2. * std::f64::consts::PI * 0.35_f64.powi(2));
    let partial = [PRESENT | PARTIAL | 1, 0];
    let opaque = [PRESENT | 1, 0];
    match case {
        "static_partial_one" | "moving_partial_one" => (
            [0.25 / (1. + m), 0., 0., 1. / (1. + m)],
            [1. + m, 1.],
            partial,
        ),
        "partial_half" => (
            [0.25 / (1. + m), 0., 0., 0.5 / (1. + m)],
            [1. + m, 1.],
            partial,
        ),
        "foreground" => ([0., 0.25 * center / m, 0., center / m], [m, 0.], partial),
        "opaque_current" | "zero_weight_carry" | "unflagged_partial_current" => {
            ([0., 0.25, 0., 1.], [m, 0.], opaque)
        }
        "positive_weight_carry" => (
            [0.25 / (1. + m), 0.25 * m / (1. + m), 0., 1.],
            [1. + m, 0.],
            partial,
        ),
        "partial_current" => {
            let row = center * (1. + 2. * (-0.5 / 0.35_f64.powi(2)).exp());
            (
                [
                    0.25 / (1. + m),
                    0.25 * row / (1. + m),
                    0.,
                    (1. + row) / (1. + m),
                ],
                [1. + m, 0.],
                partial,
            )
        }
        "mixed_opaque_current" => ([0., 0.25, 0., 1.], [0.; 2], [0; 2]),
        _ => ([0.; 4], [0.; 2], [0; 2]),
    }
}
struct DepthPass {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}
impl DepthPass {
    fn new(d: &wgpu::Device) -> Self {
        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(DEPTH.into()),
        });
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(16),
                },
                count: None,
            }],
        });
        let pl = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pl),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("depth"),
                compilation_options: Default::default(),
                targets: &[],
            }),
            multiview_mask: None,
            cache: None,
        });
        Self { layout, pipeline }
    }
    fn encode(
        &self,
        d: &wgpu::Device,
        e: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        mode: u32,
    ) {
        let b = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[mode as f32, 0., 0., 0.]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: b.as_entire_binding(),
            }],
        });
        let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.draw(0..3, 0..1);
    }
}
fn current(
    r: &Renderer,
    encoder: &mut wgpu::CommandEncoder,
    dp: &DepthPass,
    mode: u32,
) -> [wgpu::TextureView; 4] {
    let d = &r.device;
    let q = &r.queue;
    let views = [
        texture(d, wgpu::TextureFormat::Rgba16Float),
        texture(d, wgpu::TextureFormat::Rgba32Float),
        texture(d, wgpu::TextureFormat::Depth32Float),
        texture(d, wgpu::TextureFormat::R32Uint),
    ];
    let color: Vec<[u16; 4]> = (0..N * N)
        .map(|i| {
            let covered = match mode {
                1 | 4 => true,
                2 => i / N == 8,
                3 => i == 8 * N + 8,
                5 => i / N == 6,
                _ => false,
            };
            if covered {
                if mode == 5 {
                    [0x3800, 0x3400, 0, 0x3c00]
                } else {
                    [0, 0x3400, 0, 0x3c00]
                }
            } else {
                [0; 4]
            }
        })
        .collect();
    upload(q, &views[0], &color);
    upload(q, &views[1], &vec![[0f32, 0., 0.5, 1.]; (N * N) as usize]);
    upload(q, &views[3], &vec![1u32; (N * N) as usize]);
    dp.encode(d, encoder, &views[2], mode);
    views
}
fn changes(d: &wgpu::Device, flags: u32, changed: u32) -> wgpu::Buffer {
    let mut words = vec![0u32; 12528 / 4];
    words[0] = changed;
    words[1] = flags;
    words[2] = 1;
    words[4] = changed;
    d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&words),
        usage: wgpu::BufferUsages::UNIFORM,
    })
}
fn frame() -> super::super::recurrence_fixture::Frame {
    super::super::recurrence_fixture::Frame {
        source: [0., 0., 16., 16.],
        output: [0., 0., 16., 16.],
        depth: [1., 17., 1., 1.],
        options: [0.9, 0.05, 0.02, 4.],
        reactive: [0.5, 0., 1., 1.],
        jitter: [0.; 4],
    }
}
fn copy(encoder: &mut wgpu::CommandEncoder, slot: &Slot, read: &wgpu::Buffer, index: usize) {
    for plane in 0..4 {
        encoder.copy_texture_to_buffer(
            slot.views[plane].texture().as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: read,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: PLANE * (index * 4 + plane) as u64,
                    bytes_per_row: Some(256),
                    rows_per_image: Some(N),
                },
            },
            wgpu::Extent3d {
                width: N,
                height: N,
                depth_or_array_layers: 1,
            },
        );
    }
}
fn pixel(bytes: &[u8], index: usize, at: [u32; 2]) -> ([f32; 4], [f32; 2], [u32; 2]) {
    let raw = |plane: usize, bpp: usize| {
        let start =
            (index * 4 + plane) * PLANE as usize + at[1] as usize * 256 + at[0] as usize * bpp;
        &bytes[start..start + bpp]
    };
    let color = std::array::from_fn(|i| {
        half::f16::from_bits(u16::from_le_bytes(
            raw(0, 8)[i * 2..i * 2 + 2].try_into().unwrap(),
        ))
        .to_f32()
    });
    let evidence = std::array::from_fn(|i| {
        half::f16::from_bits(u16::from_le_bytes(
            raw(3, 4)[i * 2..i * 2 + 2].try_into().unwrap(),
        ))
        .to_f32()
    });
    let record = std::array::from_fn(|i| {
        u32::from_le_bytes(raw(2, 8)[i * 4..i * 4 + 4].try_into().unwrap())
    });
    (color, evidence, record)
}
fn depth_value(bytes: &[u8], index: usize, at: [u32; 2]) -> f32 {
    let start = (index * 4 + 1) * PLANE as usize + at[1] as usize * 256 + at[0] as usize * 4;
    f32::from_le_bytes(bytes[start..start + 4].try_into().unwrap())
}
fn fresh_feedback_reference() -> (f64, [f64; 4]) {
    let total = mass(2., [-0.4375, -0.1875], [9, 13]);
    let rho = 0.35_f64;
    let mut covered = 0.;
    for x in 3..=5 {
        let dx = 9.5 - 2. * (f64::from(x) + 0.5 + 0.4375);
        let dy = 13.5 - 2. * (6.5 + 0.1875);
        covered += 4. / (2. * std::f64::consts::PI * rho * rho)
            * (-(dx * dx + dy * dy) / (2. * rho * rho)).exp();
    }
    let partial = covered / total;
    (total, [0.5 * partial, 0.25 * partial, 0., partial])
}
#[test]
fn partial_coverage_reference_survives_half_rounding() {
    let (_, color) = fresh_feedback_reference();
    let partial = color[3];
    assert!(partial > 0.99999 && partial < 1.);
    assert_eq!(half::f16::from_f64(partial), half::f16::from_f32(1.));
    assert!(expected("static_partial_one").0[3] > 0.);
    assert_eq!(expected("static_opaque").0, [0.; 4]);
}
#[test]
#[ignore = "bounded actual partial-coverage provenance and nine-step RGBA16 feedback"]
fn actual_camera_owner_coverage_provenance() {
    let out = PathBuf::from(std::env::var("INCANT_OWNER_CAMERA_COVERAGE_OUT").unwrap());
    std::fs::create_dir(&out).expect("fresh coverage output");
    let renderer = owner_renderer();
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Metal);
    let d = &renderer.device;
    let q = &renderer.queue;
    let dp = DepthPass::new(d);
    let histories = [History::Tent, History::Cubic];
    let experiments = histories
        .map(|h| Experiment::with_coverage(&renderer, 3, Kernel::Point, h, Coverage::Provenance));
    let shaders =
        histories.map(|h| Coverage::Provenance.assemble(shader_with_options(3, Kernel::Point, h)));
    for (i, s) in shaders.iter().enumerate() {
        std::fs::write(out.join(format!("coverage-{i}.wgsl")), s).unwrap();
    }
    let legacy = Experiment::with_options(&renderer, 3, Kernel::Point, History::Tent);
    std::fs::write(
        out.join("legacy.wgsl"),
        shader_with_options(3, Kernel::Point, History::Tent),
    )
    .unwrap();
    std::fs::write(out.join("depth.wgsl"), DEPTH).unwrap();
    let read = d.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: PLANE * (TOTAL * 4) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = d.create_command_encoder(&Default::default());
    let cameras = CameraPair {
        previous_inverse: Mat4::IDENTITY.to_cols_array_2d(),
        current_matrix: Mat4::IDENTITY.to_cols_array_2d(),
    };
    for (index, case) in CASES.iter().enumerate() {
        let v = input(case);
        let previous = Slot::sized(d, [N, N]);
        upload(
            q,
            &previous.views[0],
            &vec![[0x3400, 0, 0, half::f16::from_f32(v.alpha).to_bits()]; (N * N) as usize],
        );
        upload(q, &previous.views[1], &vec![v.depth; (N * N) as usize]);
        upload(q, &previous.views[2], &vec![v.record; (N * N) as usize]);
        upload(
            q,
            &previous.views[3],
            &vec![
                [
                    half::f16::from_f32(v.mass).to_bits(),
                    half::f16::from_f32(v.age).to_bits()
                ];
                (N * N) as usize
            ],
        );
        let views = current(&renderer, &mut encoder, &dp, v.mode);
        let changes = changes(d, v.flags, v.changed);
        let mut f = frame();
        if v.cold {
            f.depth[3] = 0.;
            f.reactive[3] = 0.;
        }
        for (variant, experiment) in experiments.iter().enumerate() {
            let destination = Slot::sized(d, [N, N]);
            experiment.resolve(
                &renderer,
                &mut encoder,
                &previous,
                &destination,
                f,
                cameras,
                views.each_ref(),
                &changes,
            );
            copy(&mut encoder, &destination, &read, index * 2 + variant);
        }
    }
    // Real feedback: a row of opaque source samples plus positive-weight clear
    // neighbors rounds fresh alpha to1. The next eight frames are genuinely clear.
    for (variant, experiment) in [&legacy, &experiments[0]].into_iter().enumerate() {
        let slots: [Slot; 2] = std::array::from_fn(|_| Slot::sized(d, [N, N]));
        slots[0].clear(&mut encoder);
        for step in 0..=8 {
            let mode = if step == 0 { 5 } else { 0 };
            let views = current(&renderer, &mut encoder, &dp, mode);
            let changes = changes(d, 1, 0);
            let mut f = frame();
            f.source = [0., 0., 8., 8.];
            f.jitter = [-0.4375 / 8., -0.1875 / 8., -0.4375 / 8., -0.1875 / 8.];
            f.reactive[3] = if step == 0 { 0. } else { 1. };
            let previous = &slots[step % 2];
            let destination = &slots[(step + 1) % 2];
            experiment.resolve(
                &renderer,
                &mut encoder,
                previous,
                destination,
                f,
                cameras,
                views.each_ref(),
                &changes,
            );
            copy(
                &mut encoder,
                destination,
                &read,
                CASES.len() * 2 + variant * 9 + step,
            );
        }
    }
    q.submit([encoder.finish()]);
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
    std::fs::write(out.join("planes.bin"), &bytes).unwrap();
    let mut results = Vec::new();
    for (index, case) in CASES.iter().enumerate() {
        let (ec, ee, er) = expected(case);
        for (variant, history) in histories.iter().enumerate() {
            let (c, e, r) = pixel(&bytes, index * 2 + variant, [8, 8]);
            let depth = depth_value(&bytes, index * 2 + variant, [8, 8]);
            let expected_depth = if er[0] == 0 {
                0.
            } else if *case == "foreground" {
                5.
            } else {
                9.
            };
            let passed = c.iter().zip(ec).all(|(&a, b)| close(a, b))
                && e.iter().zip(ee).all(|(&a, b)| close(a, b))
                && r == er
                && depth == expected_depth;
            results.push(json!({"case":case,"history":history.name(),"color":c,"evidence":e,"record":r,"expected_color":ec,"expected_evidence":ee,"expected_record":er,"depth":depth,"expected_depth":expected_depth,"passed":passed}));
        }
    }
    let (fresh_mass, fresh_color) = fresh_feedback_reference();
    for variant in 0..2 {
        let mut previous_mass = 0.;
        let mut previous_alpha = 1.;
        let mut previous_color = [0f32; 4];
        for step in 0..=8 {
            let (c, e, r) = pixel(&bytes, CASES.len() * 2 + variant * 9 + step, [9, 13]);
            let depth = depth_value(&bytes, CASES.len() * 2 + variant * 9 + step, [9, 13]);
            let passed = if step == 0 {
                c.iter().zip(fresh_color).all(|(&a, b)| close(a, b))
                    && c[3] == 1.
                    && close(e[0], fresh_mass)
                    && r == [PRESENT | 1 | if variant == 1 { PARTIAL } else { 0 }, 0]
                    && e[0] > 0.
                    && e[1] == 0.
            } else if variant == 0 || step == 8 {
                c == [0.; 4] && e == [0.; 2] && r == [0; 2]
            } else {
                // A conditional one-step oracle: the preceding attachment is
                // quantized input, not ideal radiance. The static cap is about
                // 36 for ratio2/weight.9 and is inactive throughout this probe.
                assert!(previous_mass < 35.);
                let weight = f64::from(previous_mass);
                let total = weight + fresh_mass;
                let expected_color = previous_color.map(|v| f64::from(v) * weight / total);
                c[3] > 0.
                    && c[3] < previous_alpha
                    && e[0] > previous_mass
                    && e[1] == step as f32
                    && r == [PRESENT | PARTIAL | 1, 0]
                    && close(e[0], total)
                    && c.iter().zip(expected_color).all(|(&a, b)| close(a, b))
            };
            results.push(json!({"case":"actual_feedback","coverage":if variant==0 {"legacy-alpha"} else {"explicit-partial-bit17"},"step":step,"pixel":[9,13],"color":c,"evidence":e,"record":r,"depth":depth,"passed":passed && depth==if r[0]==0 {0.} else {9.}}));
            previous_mass = e[0];
            previous_alpha = c[3];
            previous_color = c;
        }
    }
    let report = json!({"adapter":describe(&renderer),"size":[N,N],"submissions":1,"maps":1,"resolves":TOTAL,"depth_passes":CASES.len()+18,"history_initializations":2,"scalar_source":[16,16],"feedback_source":[8,8],"feedback_output":[16,16],"feedback_jitter_pixels":[-0.4375,-0.1875],
        "shader_sha256":shaders.each_ref().map(|s|hex(s.as_bytes())),"depth_shader_sha256":hex(DEPTH.as_bytes()),"raw_sha256":hex(&bytes),"raw_bytes":bytes.len(),"plane_bytes":PLANE,"cases":results,
        "limits":["Explicit partial sample-support provenance is not geometric coverage or a depth-layer proof","Ordinary opaque reuse refuses flagged partial history even when stored alpha1","Existing owner, changed, global, depth, age8 and canonical empty rules remain","One adjacent half allowance for f64 single-step colors/mass; metadata and feedback retirement are exact","Feedback uses independent fresh mass and the actual preceding stored color/mass for each conditional one-step recurrence; not an independent whole-chain radiance oracle","No scene capture, appearance, performance or production adoption"]});
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    drop(bytes);
    read.unmap();
    assert!(results.iter().all(|r| r["passed"] == true), "{results:?}");
}

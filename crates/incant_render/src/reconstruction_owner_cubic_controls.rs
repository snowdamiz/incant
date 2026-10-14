//! Actual paired Tent/Cubic controls. Color reconstruction is a new direct-color
//! estimator; positive bilinear evidence mass remains separate, not confidence.
use super::*;
use controls::{N, PLANE, PRESENT, texture, upload};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Case {
    ConstantUnequalMass,
    Pattern,
    ImpulseClear,
    CanonicalEmpty,
    ForeignNegative,
    ChangedNegative,
    NanMass,
    ZeroMass,
    InvalidAge,
    ExpiringAge,
    NanDepth,
    WrongDepth,
    WrongDisplacement,
    MissingRecord,
    DirtyEmpty,
    Border,
    SignedAge,
    CentralZeroForeign,
    UnusedZeroNan,
    ChangedPrimary,
    Foreground,
    OpaqueMoving,
    OpaqueStatic,
    SubHalfAlpha,
    RepresentableAlpha,
}
const CASES: [Case; 25] = [
    Case::ConstantUnequalMass,
    Case::Pattern,
    Case::ImpulseClear,
    Case::CanonicalEmpty,
    Case::ForeignNegative,
    Case::ChangedNegative,
    Case::NanMass,
    Case::ZeroMass,
    Case::InvalidAge,
    Case::ExpiringAge,
    Case::NanDepth,
    Case::WrongDepth,
    Case::WrongDisplacement,
    Case::MissingRecord,
    Case::DirtyEmpty,
    Case::Border,
    Case::SignedAge,
    Case::CentralZeroForeign,
    Case::UnusedZeroNan,
    Case::ChangedPrimary,
    Case::Foreground,
    Case::OpaqueMoving,
    Case::OpaqueStatic,
    Case::SubHalfAlpha,
    Case::RepresentableAlpha,
];
fn fallback(c: Case) -> bool {
    matches!(
        c,
        Case::ForeignNegative
            | Case::ChangedNegative
            | Case::NanMass
            | Case::ZeroMass
            | Case::InvalidAge
            | Case::ExpiringAge
            | Case::NanDepth
            | Case::WrongDepth
            | Case::WrongDisplacement
            | Case::MissingRecord
            | Case::DirtyEmpty
            | Case::Border
            | Case::CentralZeroForeign
    )
}
struct Input {
    at: [u32; 2],
    delta: [f32; 2],
    shear: f32,
    flags: u32,
    changed: u32,
    color: Vec<[f32; 4]>,
    depth: Vec<f32>,
    record: Vec<[u32; 2]>,
    evidence: Vec<[f32; 2]>,
}
fn input(c: Case) -> Input {
    let len = (N * N) as usize;
    let mut v = Input {
        at: [8, 8],
        delta: [0.25; 2],
        shear: 0.,
        flags: 0,
        changed: 0,
        color: (0..len)
            .map(|i| [[0.25, 0.125, 0.5, 0.0625][i % 4], 0., 0., 0.5])
            .collect(),
        depth: vec![9.; len],
        record: vec![[PRESENT | 1, 0]; len],
        evidence: vec![[1., 0.]; len],
    };
    let n = (7 * N + 6) as usize; // Noncentral negative coefficient at query [8.25,8.25].
    match c {
        Case::ConstantUnequalMass => {
            v.color.fill([0.25, 0., 0., 0.5]);
            for (i, e) in v.evidence.iter_mut().enumerate() {
                e[0] = (1 + i % 2 + 2 * ((i / N as usize) % 2)) as f32;
            }
        }
        Case::ImpulseClear => {
            v.color.fill([0.; 4]);
            v.depth.fill(0.);
            v.record.fill([0; 2]);
            v.evidence.fill([0.; 2]);
            let i = (8 * N + 8) as usize;
            v.color[i] = [0.25, 0., 0., 0.5];
            v.depth[i] = 9.;
            v.record[i] = [PRESENT | 1, 0];
            v.evidence[i] = [1., 0.];
        }
        Case::CanonicalEmpty => {
            v.color[n] = [0.; 4];
            v.depth[n] = 0.;
            v.record[n] = [0; 2];
            v.evidence[n] = [0.; 2];
        }
        Case::ForeignNegative | Case::ChangedNegative => {
            v.record[n] = [PRESENT | 2, 0];
            if c == Case::ChangedNegative {
                v.changed = 4;
            }
        }
        Case::NanMass => v.evidence[n][0] = f32::NAN,
        Case::ZeroMass => v.evidence[n][0] = 0.,
        Case::InvalidAge => v.evidence[n][1] = 8.,
        Case::ExpiringAge => v.evidence[n][1] = 7.,
        Case::NanDepth => v.depth[n] = f32::NAN,
        Case::WrongDepth => v.depth[n] = 15.,
        Case::WrongDisplacement => {
            v.depth[n] = 9.125;
            v.shear = 0.125;
        }
        Case::MissingRecord => v.record[n] = [0; 2],
        Case::DirtyEmpty => {
            v.color[n] = [0.; 4];
            v.record[n] = [0; 2];
        }
        Case::Border => v.at = [1, 8],
        Case::SignedAge => v.evidence[n][1] = 2.,
        Case::CentralZeroForeign => {
            v.delta = [0.; 2];
            v.record[(8 * N + 9) as usize] = [PRESENT | 2, 0];
            // If the invalid zero-weight clamp tap is skipped, this admitted
            // other clamp tap makes the incorrect cubic result's age observable.
            v.evidence[(9 * N + 8) as usize][1] = 2.;
        }
        Case::UnusedZeroNan => {
            v.delta = [0.; 2];
            v.color[(7 * N + 7) as usize][0] = f32::NAN;
            v.evidence[(9 * N + 9) as usize][1] = 2.;
        }
        Case::ChangedPrimary => v.changed = 2,
        Case::OpaqueStatic => {
            v.flags = 1;
            v.delta = [0.; 2];
            v.color.fill([0.25, 0., 0., 1.]);
        }
        Case::SubHalfAlpha | Case::RepresentableAlpha => {
            let unit = half::f16::from_bits(1).to_f32();
            let alpha = if c == Case::SubHalfAlpha {
                unit
            } else {
                2. * unit
            };
            v.color.fill([8. * unit, 0., 0., alpha]);
        }
        Case::Pattern | Case::Foreground | Case::OpaqueMoving => {}
    }
    v
}
fn cameras(v: &Input) -> CameraPair {
    let mut m = Mat4::IDENTITY;
    m.z_axis.x = v.shear;
    m.w_axis.x = v.delta[0] / 8. - 0.5 * v.shear;
    m.w_axis.y = -v.delta[1] / 8.;
    CameraPair {
        previous_inverse: Mat4::IDENTITY.to_cols_array_2d(),
        current_matrix: m.to_cols_array_2d(),
    }
}
fn current_mass() -> f64 {
    let s = 0.35_f64.powi(2);
    (1. + 2. * (-0.5 / s).exp()).powi(2) / (2. * std::f64::consts::PI * s)
}
// Independent Hermite endpoint/tangent derivation, rather than shader's
// piecewise distance polynomial: slopes are central differences of neighbors.
fn weights(t: f64) -> [f64; 4] {
    let h00 = 2. * t.powi(3) - 3. * t.powi(2) + 1.;
    let h10 = t.powi(3) - 2. * t.powi(2) + t;
    let h01 = -2. * t.powi(3) + 3. * t.powi(2);
    let h11 = t.powi(3) - t.powi(2);
    [-0.5 * h10, h00 - 0.5 * h11, h01 + 0.5 * h10, 0.5 * h11]
}
fn expected(c: Case, v: &Input, cubic: bool) -> ([f64; 4], [f64; 2]) {
    let m = current_mass();
    if c == Case::ChangedPrimary {
        return ([0.; 4], [0.; 2]);
    }
    if c == Case::OpaqueMoving {
        return ([0., 0.25, 0., 1.], [0.; 2]);
    }
    if c == Case::OpaqueStatic {
        return ([0.25 / (1. + m), 0.25 * m / (1. + m), 0., 1.], [1. + m, 0.]);
    }
    if c == Case::Foreground {
        let center = 1. / (2. * std::f64::consts::PI * 0.35_f64.powi(2));
        return ([0., 0.25 * center / m, 0., center / m], [m, 0.]);
    }
    let q = [
        f64::from(v.at[0]) - f64::from(v.delta[0]),
        f64::from(v.at[1]) - f64::from(v.delta[1]),
    ];
    let base = q.map(|x| x.floor() as i32);
    let f = q.map(|x| x - x.floor());
    let mut mass = 0.;
    let mut numerator = [0.; 4];
    for y in 0..2 {
        for x in 0..2 {
            let b =
                (if x == 0 { 1. - f[0] } else { f[0] }) * (if y == 0 { 1. - f[1] } else { f[1] });
            let i = ((base[1] + y) * N as i32 + base[0] + x) as usize;
            // Only the impulse has empty positive-tent taps; all refusal mutations
            // are deliberately outside tent support at the observed pixel.
            let w = b * f64::from(v.evidence[i][0]);
            mass += w;
            for (k, n) in numerator.iter_mut().enumerate() {
                *n += w * f64::from(v.color[i][k]);
            }
        }
    }
    let mut color = numerator.map(|n| n / mass);
    let mut age = 1.;
    if cubic && !fallback(c) {
        let wx = weights(f[0]);
        let wy = weights(f[1]);
        let mut sum = [0.; 4];
        let mut low = [f64::INFINITY; 4];
        let mut high = [f64::NEG_INFINITY; 4];
        for (y, &wyv) in wy.iter().enumerate() {
            for (x, &wxv) in wx.iter().enumerate() {
                let i = ((base[1] + y as i32 - 1) * N as i32 + base[0] + x as i32 - 1) as usize;
                if (1..=2).contains(&x) && (1..=2).contains(&y) {
                    for k in 0..4 {
                        low[k] = low[k].min(f64::from(v.color[i][k]));
                        high[k] = high[k].max(f64::from(v.color[i][k]));
                    }
                }
                let w = wxv * wyv;
                if w != 0. {
                    for (k, n) in sum.iter_mut().enumerate() {
                        *n += w * f64::from(v.color[i][k]);
                    }
                }
            }
        }
        color = std::array::from_fn(|k| sum[k].clamp(low[k], high[k]));
        if matches!(c, Case::SignedAge | Case::UnusedZeroNan) {
            age = 3.;
        }
    }
    let blended = color.map(|n| n * mass / (mass + m));
    // The attachment stores half precision. A transparent stored result must
    // contain exact zero RGB even when its f32 precursor was positive.
    let stored = if half::f16::from_f64(blended[3]).to_bits() == 0 {
        [0.; 4]
    } else {
        blended
    };
    (stored, [mass + m, age])
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
const DEPTH: &str = "@group(0) @binding(0) var<uniform> mode:vec4f;
@vertex fn vertex(@builtin(vertex_index) i:u32)->@builtin(position) vec4f {
let uv=vec2f(f32((i<<1u)&2u),f32(i&2u));return vec4f(uv*2.0-1.0,0.0,1.0);}
@fragment fn depth(@builtin(position) p:vec4f)->@builtin(frag_depth) f32 {
if mode.x==1.0 {return select(0.5,0.875,((u32(p.x)+u32(p.y))&1u)!=0u);}
if mode.x==2.0 {return 0.0;}return 1.0;}";

#[test]
fn cubic_color_reference_is_separate_from_mass() {
    for t in [0., 0.125, 0.25, 0.5, 0.75, 1.] {
        let w = weights(t);
        assert!((w.iter().sum::<f64>() - 1.).abs() < 1e-14);
        assert!(
            (w.iter()
                .zip([-1., 0., 1., 2.])
                .map(|(a, b)| a * b)
                .sum::<f64>()
                - t)
                .abs()
                < 1e-14
        );
    }
    let v = input(Case::ConstantUnequalMass);
    assert_eq!(
        expected(Case::ConstantUnequalMass, &v, false),
        expected(Case::ConstantUnequalMass, &v, true)
    );
    for c in [Case::Pattern, Case::ImpulseClear, Case::CanonicalEmpty] {
        let v = input(c);
        assert!((expected(c, &v, true).0[0] - expected(c, &v, false).0[0]).abs() > 0.001);
    }
    let mut v = input(Case::WrongDisplacement);
    let pair = cameras(&v);
    let m = Mat4::from_cols_array_2d(&pair.current_matrix);
    let reference_depth = v.depth[0];
    let perturbed_depth = v.depth[(7 * N + 6) as usize];
    let a = m * glam::Vec4::new(0., 0., (reference_depth - 1.) / 16., 1.);
    let b = m * glam::Vec4::new(0., 0., (perturbed_depth - 1.) / 16., 1.);
    assert_eq!((b.x - a.x) * 8., 1. / 128.);
    assert!(perturbed_depth > reference_depth);
    assert!(perturbed_depth - reference_depth < reference_depth * 0.05);
    v.shear = 0.;
    assert_eq!(cameras(&v).current_matrix[2][0], 0.);
    let underflow = input(Case::SubHalfAlpha);
    assert_eq!(expected(Case::SubHalfAlpha, &underflow, false).0, [0.; 4]);
    let representable = input(Case::RepresentableAlpha);
    let color = expected(Case::RepresentableAlpha, &representable, false).0;
    assert_eq!(half::f16::from_f64(color[3]).to_bits(), 1);
    assert!(half::f16::from_f64(color[0]).to_bits() > 1);
}

#[test]
#[ignore = "bounded paired actual Metal cubic provenance and opaque-depth controls"]
fn actual_camera_owner_cubic_controls() {
    let out = PathBuf::from(std::env::var("INCANT_OWNER_CAMERA_CUBIC_OUT").unwrap());
    std::fs::create_dir(&out).expect("fresh cubic output");
    let renderer = owner_renderer();
    assert_eq!(renderer.adapter.get_info().backend, wgpu::Backend::Metal);
    let d = &renderer.device;
    let q = &renderer.queue;
    let histories = [History::Tent, History::Cubic];
    let experiments = histories.map(|h| Experiment::with_options(&renderer, 3, Kernel::Point, h));
    let shaders = histories.map(|h| shader_with_options(3, Kernel::Point, h));
    for (i, s) in shaders.iter().enumerate() {
        std::fs::write(out.join(format!("history-{i}.wgsl")), s).unwrap();
    }
    std::fs::write(out.join("depth.wgsl"), DEPTH).unwrap();
    let dm = d.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(DEPTH.into()),
    });
    let dl = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        bind_group_layouts: &[Some(&dl)],
        immediate_size: 0,
    });
    let dp = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: None,
        layout: Some(&pl),
        vertex: wgpu::VertexState {
            module: &dm,
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
            module: &dm,
            entry_point: Some("depth"),
            compilation_options: Default::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    });
    let read = d.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: PLANE * (CASES.len() * 8) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = d.create_command_encoder(&Default::default());
    for (index, case) in CASES.iter().copied().enumerate() {
        let v = input(case);
        let previous = Slot::sized(d, [N, N]);
        upload(
            q,
            &previous.views[0],
            &v.color
                .iter()
                .map(|c| c.map(|n| half::f16::from_f32(n).to_bits()))
                .collect::<Vec<_>>(),
        );
        upload(q, &previous.views[1], &v.depth);
        upload(q, &previous.views[2], &v.record);
        upload(
            q,
            &previous.views[3],
            &v.evidence
                .iter()
                .map(|c| c.map(|n| half::f16::from_f32(n).to_bits()))
                .collect::<Vec<_>>(),
        );
        let current = texture(d, wgpu::TextureFormat::Rgba16Float);
        let motion = texture(d, wgpu::TextureFormat::Rgba32Float);
        let depth = texture(d, wgpu::TextureFormat::Depth32Float);
        let owner = texture(d, wgpu::TextureFormat::R32Uint);
        let mut pixels = vec![[0u16; 4]; (N * N) as usize];
        if matches!(case, Case::OpaqueMoving | Case::OpaqueStatic) {
            pixels.fill([0, 0x3400, 0, 0x3c00]);
        }
        if case == Case::Foreground {
            pixels[(8 * N + 8) as usize] = [0, 0x3400, 0, 0x3c00];
        }
        upload(q, &current, &pixels);
        upload(q, &motion, &vec![[0f32, 0., 0.5, 1.]; (N * N) as usize]);
        upload(q, &owner, &vec![1u32; (N * N) as usize]);
        let mode = if matches!(case, Case::OpaqueMoving | Case::OpaqueStatic) {
            1.
        } else if case == Case::Foreground {
            2.
        } else {
            0.
        };
        let uniform = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&[mode, 0f32, 0., 0.]),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &dl,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_pipeline(&dp);
            pass.set_bind_group(0, &group, &[]);
            pass.draw(0..3, 0..1);
        }
        let mut changes = vec![0u32; 12528 / 4];
        changes[0] = v.changed;
        changes[1] = v.flags;
        changes[2] = 2;
        changes[4] = v.changed;
        let changes = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&changes),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        for (variant, experiment) in experiments.iter().enumerate() {
            let destination = Slot::sized(d, [N, N]);
            let frame = super::super::recurrence_fixture::Frame {
                source: [0., 0., N as f32, N as f32],
                output: [0., 0., N as f32, N as f32],
                depth: [1., 17., 1., 1.],
                options: [0.9, 0.05, 0.02, 4.],
                reactive: [0.5, 0., 1., 1.],
                jitter: [0.; 4],
            };
            experiment.resolve(
                &renderer,
                &mut encoder,
                &previous,
                &destination,
                frame,
                cameras(&v),
                [&current, &motion, &depth, &owner],
                &changes,
            );
            for plane in 0..4 {
                encoder.copy_texture_to_buffer(
                    destination.views[plane].texture().as_image_copy(),
                    wgpu::TexelCopyBufferInfo {
                        buffer: &read,
                        layout: wgpu::TexelCopyBufferLayout {
                            offset: PLANE * ((index * 2 + variant) * 4 + plane) as u64,
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
    for (index, case) in CASES.iter().copied().enumerate() {
        let v = input(case);
        let raw_pixel = |variant: usize, plane: usize, bpp: usize| {
            let at = ((index * 2 + variant) * 4 + plane) * PLANE as usize
                + v.at[1] as usize * 256
                + v.at[0] as usize * bpp;
            &bytes[at..at + bpp]
        };
        let half_pixel = |variant: usize, plane: usize, channels: usize| {
            raw_pixel(variant, plane, channels * 2)
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| half::f16::from_bits(u16::from_le_bytes(*b)).to_f32())
                .collect::<Vec<_>>()
        };
        for (variant, history) in histories.iter().enumerate() {
            let color = half_pixel(variant, 0, 4);
            let evidence = half_pixel(variant, 3, 2);
            let (ec, ee) = expected(case, &v, variant == 1);
            let exact_fallback = !fallback(case)
                || [8, 4, 8, 4]
                    .into_iter()
                    .enumerate()
                    .all(|(p, b)| raw_pixel(0, p, b) == raw_pixel(1, p, b));
            let opaque = !matches!(case, Case::OpaqueMoving | Case::OpaqueStatic) || color[3] == 1.;
            let canonical = color[3] != 0. || color[..3].iter().all(|c| *c == 0.);
            let tiny_alpha =
                case != Case::RepresentableAlpha || color[3] == half::f16::from_bits(1).to_f32();
            let passed = color.iter().zip(ec).all(|(&a, b)| close(a, b))
                && close(evidence[0], ee[0])
                && f64::from(evidence[1]) == ee[1]
                && exact_fallback
                && opaque
                && canonical
                && tiny_alpha;
            results.push(json!({"case":format!("{case:?}"),"history":history.name(),"pixel":v.at,"delta":v.delta,
                "expected_color":ec,"color":color,"expected_evidence":ee,"evidence":evidence,"fallback_byte_equal":exact_fallback,"passed":passed}));
        }
    }
    let report = json!({"adapter":describe(&renderer),"size":[N,N],"submissions":1,"maps":1,"resolves":CASES.len()*2,"depth_passes":CASES.len(),
        "shader_sha256":shaders.each_ref().map(|s| hex(s.as_bytes())),"depth_shader_sha256":hex(DEPTH.as_bytes()),"raw_sha256":hex(&bytes),"raw_bytes":bytes.len(),
        "plane_bytes":PLANE,"planes_per_resolve":["RGBA16F","R32F","RG32Uint","RG16F"],"current_mass":current_mass(),"cases":results,
        "limits":["Direct clamped cubic color and separate positive tent mass: different estimator, not confidence","Independent Hermite/f64 oracle; one adjacent half allowance, exact zero/age/opacity and fallback bytes","Foreign+changed negative case tests joint refusal, not an independently variable per-tap changed bit","No appearance, arbitrary camera, performance or production acceptance"]});
    std::fs::write(
        out.join("report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    drop(bytes);
    read.unmap();
    assert!(results.iter().all(|r| r["passed"] == true), "{results:?}");
}

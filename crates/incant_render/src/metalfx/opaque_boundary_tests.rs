//! Fixed boundary shaders, not vendor quality. One submit/map; uploaded bits
//! include nonfinite native samples, never WGSL nonfinite constant expressions.
use super::*;
use half::f16;
use sha2::{Digest, Sha256};
use std::{io::Write as _, time::Duration};
#[derive(Clone)]
struct Case {
    name: &'static str,
    current: [[f32; 4]; 4],
    native: [f32; 4],
    motion: [[f32; 4]; 4],
    previous: bool,
    stable: bool,
    depth_mode: f32,
    expected: [f32; 4],
    packed: [f32; 4],
    packed_depth: f32,
    reactive: f32,
}
fn cases() -> Vec<Case> {
    let foreground = [0.5, 0.25, 0.125, 1.];
    let background = [0.02, 0.03, 0.05, 1.];
    let base = Case {
        name: "foreground",
        current: [foreground; 4],
        native: [4., 2., 1., 0.],
        motion: [[0., 0., 0.25, 1.]; 4],
        previous: true,
        stable: true,
        depth_mode: 0.,
        expected: [4., 2., 1., 1.],
        packed: foreground,
        packed_depth: 0.25,
        reactive: 0.,
    };
    let clear = Case {
        name: "stable-background",
        current: [[0.; 4]; 4],
        motion: [[0.; 4]; 4],
        depth_mode: 2.,
        packed: background,
        packed_depth: 1.,
        ..base.clone()
    };
    let mut out = vec![base.clone(), clear.clone()];
    out.push(Case {
        name: "legacy-background-reactive1",
        stable: false,
        reactive: 1.,
        ..clear.clone()
    });
    out.push(Case {
        name: "first-or-cut-background",
        previous: false,
        reactive: 1.,
        ..clear.clone()
    });
    out.push(Case {
        name: "first-or-cut-foreground",
        previous: false,
        reactive: 1.,
        ..base.clone()
    });
    out.push(Case {
        name: "mixed-depth-opaque",
        depth_mode: 1.,
        ..base.clone()
    });
    // Constant background legitimately fills a MASK hole / missed thin sample;
    // it does not veto vendor reconstruction of the foreground edge.
    let mut hole = clear.clone();
    hole.name = "mask-hole-mixed-footprint";
    hole.current[0] = foreground;
    hole.motion[0] = base.motion[0];
    hole.depth_mode = 3.;
    hole.packed = foreground;
    hole.packed_depth = 0.25;
    out.push(hole);
    let mut miss = clear.clone();
    miss.name = "thin-foreground-missed-current";
    miss.native = [0.5, 0.25, 0.125, 1.];
    miss.expected = miss.native;
    out.push(miss);
    for (name, value) in [
        ("vendor-nan", f32::NAN),
        ("vendor-inf", f32::INFINITY),
        ("vendor-negative", -1.),
    ] {
        let mut c = base.clone();
        c.name = name;
        c.native[0] = value;
        c.expected = foreground;
        out.push(c);
    }
    let mut c = clear.clone();
    c.name = "invalid-vendor-background-fallback";
    c.native[0] = f32::INFINITY;
    c.expected = background;
    out.push(c);
    let mut c = base.clone();
    c.name = "partial-not-sky";
    c.current = [[0.25, 0.125, 0.0625, 0.5]; 4];
    c.expected = c.current[0];
    c.packed = [0.; 4];
    c.packed_depth = 1.;
    c.reactive = 1.;
    out.push(c);
    for (name, value) in [("current-nan", f32::NAN), ("current-inf", f32::INFINITY)] {
        let mut c = base.clone();
        c.name = name;
        c.current = [[value, 0., 0., 1.]; 4];
        c.expected = [0.; 4];
        c.packed = [0.; 4];
        c.packed_depth = 1.;
        c.reactive = 1.;
        out.push(c);
    }
    for (name, value) in [
        ("geometry-motion-nan", f32::NAN),
        ("geometry-motion-inf", f32::INFINITY),
        ("geometry-motion-overflow", f32::MAX),
    ] {
        let mut c = base.clone();
        c.name = name;
        c.motion[0][0] = value;
        c.reactive = 1.;
        out.push(c);
    }
    let mut c = base.clone();
    c.name = "geometry-motion-invalid";
    c.motion = [[0.; 4]; 4];
    c.reactive = 1.;
    out.push(c);
    for (name, motion) in [
        ("clear-nonzero-motion", [0.1, 0., 0., 0.]),
        ("clear-nan-motion", [f32::NAN, 0., 0., 0.]),
        ("clear-negative-zero", [-0., 0., 0., 0.]),
    ] {
        let mut c = clear.clone();
        c.name = name;
        c.motion = [motion; 4];
        c.expected = [0.; 4];
        c.packed = [0.; 4];
        c.reactive = 1.;
        out.push(c);
    }
    let mut c = clear.clone();
    c.name = "clear-with-geometry-depth";
    c.depth_mode = 0.;
    c.expected = [0.; 4];
    c.packed = [0.; 4];
    c.reactive = 1.;
    out.push(c);
    let mut c = clear.clone();
    c.name = "zero-alpha-residual-rgb-not-sky";
    c.current = [[0.25, 0., 0., 0.]; 4];
    c.expected = [0.; 4];
    c.packed = [0.; 4];
    c.reactive = 1.;
    out.push(c);
    out
}
fn upload(r: &crate::Renderer, texture: &wgpu::Texture, data: &[u8], pixel_bytes: u32) {
    r.queue.write_texture(
        texture.as_image_copy(),
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(texture.width() * pixel_bytes),
            rows_per_image: Some(texture.height()),
        },
        texture.size(),
    );
}
fn rgba(values: impl IntoIterator<Item = [f32; 4]>) -> Vec<u8> {
    values
        .into_iter()
        .flatten()
        .flat_map(|v| f16::from_f32(v).to_le_bytes())
        .collect()
}
fn group(
    d: &wgpu::Device,
    s: &System,
    frame: &wgpu::Buffer,
    views: [&wgpu::TextureView; 3],
    motion: &wgpu::TextureView,
) -> wgpu::BindGroup {
    d.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Boundary control"),
        layout: &s.layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 4,
                resource: wgpu::BindingResource::TextureView(motion),
            },
            wgpu::BindGroupEntry {
                binding: 0,
                resource: frame.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(views[1]),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(views[2]),
            },
        ],
    })
}
fn depth_pipeline(d: &wgpu::Device) -> wgpu::RenderPipeline {
    let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Actual boundary depth input"),
        source: wgpu::ShaderSource::Wgsl(
            (include_str!("common.wgsl").to_string()
                + r#"
@fragment fn input_depth(@builtin(position) p:vec4f)->@builtin(frag_depth) f32 {
    if frame.options.w==2.0 {return 1.0;}
    if frame.options.w==3.0 {return select(1.0,0.25,p.x<1.0 && p.y<1.0);}
    return select(0.25,0.75,frame.options.w==1.0 && p.x>1.0);
}"#)
            .into(),
        ),
    });
    d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Boundary depth"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("input_depth"),
            compilation_options: Default::default(),
            targets: &[],
        }),
        primitive: Default::default(),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: Default::default(),
            bias: Default::default(),
        }),
    })
}
#[test]
#[ignore = "coordinated GPU boundary control: one submission and one map, no vendor appearance claim"]
fn actual_opaque_domain_boundary_controls() {
    let r = crate::test_support::checked(crate::Renderer::headless().unwrap());
    let s = System::with_source(&r.device, super::super::opaque_experiment::source, true);
    let depth_pipeline = depth_pipeline(&r.device);
    let cases = cases();
    let formats = [
        wgpu::TextureFormat::Rgba16Float,
        wgpu::TextureFormat::R32Float,
        wgpu::TextureFormat::Rg16Float,
        wgpu::TextureFormat::R8Unorm,
        wgpu::TextureFormat::Rgba16Float,
    ];
    let buffer = r.device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Bounded boundary readback"),
        size: (cases.len() * 5 * 1024) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut e = r.device.create_command_encoder(&Default::default());
    for (index, c) in cases.iter().enumerate() {
        let current = crate::hdr_output::texture(&r, wgpu::TextureFormat::Rgba16Float, 2, 2);
        let motion = crate::hdr_output::texture(&r, wgpu::TextureFormat::Rgba32Float, 2, 2);
        let depth = crate::hdr_output::texture(&r, wgpu::TextureFormat::Depth32Float, 2, 2);
        let native = crate::hdr_output::texture(&r, wgpu::TextureFormat::Rgba16Float, 4, 4);
        upload(&r, &current, &rgba(c.current), 8);
        upload(&r, &native, &rgba([c.native; 16]), 8);
        upload(&r, &motion, bytemuck::cast_slice(&c.motion), 16);
        let frame = Frame {
            source: [0., 0., 2., 2.],
            output: [0., 0., 4., 4.],
            depth: [0.1, 10., 1., f32::from(u8::from(c.previous))],
            options: [0., 0.01, 0.001, c.depth_mode],
            reactive: [0.02, 0.03, 0.05, f32::from(u8::from(c.stable))],
            jitter: [0.; 4],
        };
        let frame = r
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(c.name),
                contents: bytemuck::bytes_of(&frame),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let depth_group = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &depth_pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame.as_entire_binding(),
            }],
        });
        let cv = current.create_view(&Default::default());
        let mv = motion.create_view(&Default::default());
        let dv = depth.create_view(&Default::default());
        let nv = native.create_view(&Default::default());
        {
            let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Write declared depth"),
                color_attachments: &[],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &dv,
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
        let textures = formats.map(|f| crate::hdr_output::texture(&r, f, 4, 4));
        let views = textures
            .each_ref()
            .map(|t| t.create_view(&Default::default()));
        let packed = group(&r.device, &s, &frame, [&cv, &mv, &dv], &mv);
        let normalized = group(&r.device, &s, &frame, [&cv, &nv, &dv], &mv);
        {
            let attachments = views[..4]
                .iter()
                .map(|v| attachment(v, wgpu::Color::TRANSPARENT))
                .collect::<Vec<_>>();
            let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Actual pack"),
                color_attachments: &attachments,
                ..Default::default()
            });
            pass.set_pipeline(&s.pack);
            pass.set_bind_group(0, &packed, &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Actual normalize"),
                color_attachments: &[attachment(&views[4], wgpu::Color::TRANSPARENT)],
                ..Default::default()
            });
            pass.set_pipeline(&s.normalize);
            pass.set_bind_group(0, &normalized, &[]);
            pass.draw(0..3, 0..1);
        }
        for (plane, t) in textures.iter().enumerate() {
            e.copy_texture_to_buffer(
                t.as_image_copy(),
                wgpu::TexelCopyBufferInfo {
                    buffer: &buffer,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: ((index * 5 + plane) * 1024) as u64,
                        bytes_per_row: Some(256),
                        rows_per_image: Some(4),
                    },
                },
                t.size(),
            );
        }
    }
    r.queue.submit([e.finish()]);
    let (tx, rx) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |v| tx.send(v).unwrap());
    r.device
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap();
    let bytes = buffer.slice(..).get_mapped_range().to_vec();
    buffer.unmap();
    // Retain raw bytes before assertions, including failed controls.
    if let Some(dir) = std::env::var_os("INCANT_METALFX_OPAQUE_BOUNDARY_EVIDENCE") {
        let dir = std::path::PathBuf::from(dir);
        std::fs::create_dir_all(&dir).unwrap();
        let adapter = r.adapter.get_info();
        let report = serde_json::json!({"cases":cases.iter().map(|c|c.name).collect::<Vec<_>>(),"submissions":1,"maps":1,"adapter":format!("{adapter:?}"),"raw_sha256":format!("{:x}",Sha256::digest(&bytes)),"pack_sha256":format!("{:x}",Sha256::digest(super::super::opaque_experiment::source(false))),"normalize_sha256":format!("{:x}",Sha256::digest(super::super::opaque_experiment::source(true))),"row_stride":256,"plane_bytes":1024,"planes":["RGBA16-pack","R32-depth","RG16-motion","R8-reactive","RGBA16-normalized"],"quality_acceptance":false});
        for (name, data) in [
            ("raw.bin", bytes.clone()),
            ("report.json", serde_json::to_vec_pretty(&report).unwrap()),
            (
                "pack.wgsl",
                super::super::opaque_experiment::source(false).into_bytes(),
            ),
            (
                "normalize.wgsl",
                super::super::opaque_experiment::source(true).into_bytes(),
            ),
        ] {
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(dir.join(name))
                .unwrap()
                .write_all(&data)
                .unwrap();
        }
    }
    let h = |offset| f16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap()).to_f32();
    for (index, c) in cases.iter().enumerate() {
        let base = index * 5 * 1024;
        let actual = std::array::from_fn::<_, 4, _>(|i| h(base + 4 * 1024 + 256 + 8 + i * 2));
        assert_eq!(actual[3], c.expected[3], "{} alpha", c.name);
        for i in 0..3 {
            half_close(actual[i], c.expected[i], c.name);
        }
        let packed = std::array::from_fn::<_, 4, _>(|i| h(base + i * 2));
        assert_eq!(packed[3], c.packed[3], "{} packed alpha", c.name);
        for i in 0..3 {
            half_close(packed[i], c.packed[i], c.name);
        }
        let z = f32::from_le_bytes(bytes[base + 1024..base + 1024 + 4].try_into().unwrap());
        assert_eq!(z, c.packed_depth, "{} depth", c.name);
        assert_eq!(
            [h(base + 2048), h(base + 2048 + 2)],
            [0.; 2],
            "{} motion",
            c.name
        );
        assert_eq!(
            bytes[base + 3072],
            (c.reactive * 255.) as u8,
            "{} reactive",
            c.name
        );
        // Allocated maximum extent outside the actual 2x2 content is canonical.
        assert_eq!(h(base + 3 * 256 + 3 * 8), 0., "{} padding", c.name);
    }
    println!(
        "{} exact boundary cases; one submit/map; vendor quality untested",
        cases.len()
    );
}

// Positive RGB conversion permits one adjacent half (existing boundary contract);
// alpha/depth/reactivity/motion and zero are exact. No visual tolerance involved.
fn half_close(actual: f32, expected: f32, name: &str) {
    assert!(actual.is_finite() && actual >= 0., "{name}");
    let a = f16::from_f32(actual).to_bits();
    let e = f16::from_f32(expected).to_bits();
    if expected == 0. {
        assert_eq!(a, 0, "{name}");
    } else {
        assert!(a.abs_diff(e) <= 1, "{name}: {actual} vs {expected}");
    }
}

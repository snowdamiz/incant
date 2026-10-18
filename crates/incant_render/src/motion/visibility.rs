//! Test-only 64² four-sample visibility. No color reconstruction or history owner.
use super::{MotionDraw, system};
use crate::Renderer;
use glam::{Mat4, Vec3};
use wgpu::util::DeviceExt;
#[path = "visibility_owner.rs"]
mod diagnostic;
#[path = "visibility_msl.rs"]
mod msl;
#[path = "visibility_readback.rs"]
mod readback;
#[path = "visibility_storage.rs"]
mod storage;
pub(crate) use msl::Session as MslSession;
pub(crate) use readback::Sample;
pub(crate) use storage::{Record as StorageRecord, Storage};
pub(crate) const SIZE: u32 = 64;
// Guaranteed multisample integer attachments. Split each f32/u32 into exact
// low/high u16 lanes; no adapter-specific format feature or f16 narrowing.
const FORMATS: [wgpu::TextureFormat; 4] = [
    wgpu::TextureFormat::Rgba16Uint,
    wgpu::TextureFormat::Rgba16Uint,
    wgpu::TextureFormat::Rg16Uint,
    wgpu::TextureFormat::Rgba16Uint,
];
const PACK: &str = "fn pack(v:vec2f)->vec4u {let b=bitcast<vec2u>(v);return vec4u(b.x&65535u,b.x>>16u,b.y&65535u,b.y>>16u);}";
pub(crate) fn remove_jitter(matrix: Mat4, phase: Option<u32>, rect: [f32; 4]) -> Mat4 {
    let j = phase.map_or([0.; 2], |p| crate::temporal::jitter::ndc(p, rect));
    Mat4::from_translation(Vec3::new(-j[0], -j[1], 0.)) * matrix
}
pub(crate) fn source() -> String {
    // Derive from the real owner shader; no alternate geometry/identity path.
    let mut s = include_str!("owner.wgsl").to_string();
    for (from, to) in [
        (
            "@location(0) previous:vec4f,",
            "@location(0) @interpolate(perspective, sample) previous:vec4f,",
        ),
        (
            "    @location(1) uv:vec2f,",
            "    @location(1) @interpolate(perspective, sample) uv:vec2f,",
        ),
        (
            "@location(3) @interpolate(flat, first) owner:u32,",
            "@location(3) @interpolate(flat, first) owner:u32,\n    @location(4) @interpolate(linear, sample) sample_position:vec2f,",
        ),
        (
            "out.uv=uv;out.valid=valid;out.owner=owner;return out;",
            "out.sample_position=(out.position.xy/out.position.w*vec2f(0.5,-0.5)+vec2f(0.5))*frame.rect.zw+frame.rect.xy;\n    out.uv=uv;out.valid=valid;out.owner=owner;return out;",
        ),
        (
            "(in.position.xy-frame.rect.xy)/frame.rect.zw",
            "(in.sample_position-frame.rect.xy)/frame.rect.zw",
        ),
        (
            "struct Surface { @location(0) motion:vec4f, @location(1) owner:u32 }",
            "struct Surface { @location(0) motion_xy:vec4u, @location(1) motion_zw:vec4u, @location(2) owner:vec2u, @location(3) sample_position:vec4u }",
        ),
        (
            "return Surface(motion(in),in.owner);",
            "let m=motion(in);return Surface(pack(m.xy),pack(m.zw),vec2u(in.owner&65535u,in.owner>>16u),pack(in.sample_position));",
        ),
    ] {
        assert_eq!(s.matches(from).count(), 1, "owner shader anchor changed");
        s = s.replace(from, to);
    }
    s.push_str(PACK);
    s
}
const CALIBRATION: &str = r#"
struct V { @builtin(position) p:vec4f, @location(0) @interpolate(linear,sample) xy:vec2f }
@vertex fn vertex(@builtin(vertex_index) i:u32)->V {
    let uv=vec2f(f32((i<<1u)&2u),f32(i&2u));
    var v:V; v.p=vec4f(uv*2.0-1.0,0.5,1.0);v.xy=vec2f(uv.x,1.0-uv.y)*64.0;return v;
}
struct F { @location(0) motion_xy:vec4u, @location(1) motion_zw:vec4u, @location(2) sample_id:vec2u, @location(3) xy:vec4u }
@fragment fn fragment(v:V,@builtin(sample_index) i:u32)->F {
    return F(vec4u(0),vec4u(0),vec2u(i+1u,0u),pack(v.xy));
}
"#;
pub(crate) struct Probe {
    pipelines: [wgpu::RenderPipeline; 4],
    calibration: wgpu::RenderPipeline,
    pub(crate) colors: [wgpu::TextureView; 4],
    pub(crate) depth: wgpu::TextureView,
    reader: Option<readback::Reader>,
    pub(crate) size: [u32; 2],
    globals: wgpu::BindGroupLayout,
    diagnostic: Option<diagnostic::Arm>,
}
impl Probe {
    pub(crate) fn new(r: &Renderer) -> Self {
        Self::build(r, None)
    }
    pub(crate) fn owner_diagnostic(
        r: &Renderer,
        root: &std::path::Path,
        explicit: bool,
        reverse: bool,
    ) -> Self {
        Self::build(r, Some(diagnostic::Arm::new(root, explicit, reverse)))
    }
    pub(crate) fn storage_diagnostic(
        r: &Renderer,
        root: &std::path::Path,
        storage: Storage,
    ) -> Self {
        let mut arm = diagnostic::Arm::new(root, false, false);
        arm.storage = storage;
        Self::build(r, Some(arm))
    }
    fn build(r: &Renderer, diagnostic: Option<diagnostic::Arm>) -> Self {
        let storage = diagnostic.as_ref().map_or(Storage::Original, |a| a.storage);
        Self::build_extent(r, diagnostic, storage, [SIZE; 2], true)
    }
    pub(crate) fn reconstruction(r: &Renderer, size: [u32; 2]) -> Self {
        Self::build_extent(r, None, Storage::SampleX, size, false)
    }
    fn build_extent(
        r: &Renderer,
        diagnostic: Option<diagnostic::Arm>,
        storage: Storage,
        size: [u32; 2],
        readback: bool,
    ) -> Self {
        let d = &r.device;
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let mut formats = FORMATS;
        if matches!(storage, Storage::Rgba | Storage::Slots | Storage::SampleX) {
            formats[2] = wgpu::TextureFormat::Rgba16Uint;
        }
        for f in formats
            .into_iter()
            .chain([wgpu::TextureFormat::Depth32Float])
        {
            let features = r.adapter.get_texture_format_features(f);
            assert!(
                features.allowed_usages.contains(usage),
                "visibility format {f:?}"
            );
            assert!(
                features
                    .flags
                    .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4),
                "visibility 4x format {f:?}"
            );
        }
        let make = |format| {
            d.create_texture(&wgpu::TextureDescriptor {
                label: Some("Test-only output4x visibility"),
                size: wgpu::Extent3d {
                    width: size[0],
                    height: size[1],
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 4,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats: &[],
            })
            .create_view(&Default::default())
        };
        let globals = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[system::frame_entry()],
        });
        let layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&globals), Some(&r.materials.layout)],
            immediate_size: 0,
        });
        let wgsl = if storage == Storage::Original {
            diagnostic::source(diagnostic.as_ref().is_some_and(|arm| arm.explicit))
        } else {
            storage::source(storage)
        };
        let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Output4x sampled owner"),
            source: wgpu::ShaderSource::Wgsl(wgsl.clone().into()),
        });
        let targets = formats.map(|format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        });
        let depth = wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        };
        let buffers = system::owner_buffers();
        let calibration_wgsl = storage::calibration(storage);
        let pipeline = |shader: &wgpu::ShaderModule,
                        layout: Option<&wgpu::PipelineLayout>,
                        buffers: &[wgpu::VertexBufferLayout<'_>],
                        i: usize| {
            let descriptor = wgpu::RenderPipelineDescriptor {
                label: Some("Visibility-only diagnostic"),
                layout,
                vertex: wgpu::VertexState {
                    module: shader,
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers,
                },
                primitive: wgpu::PrimitiveState {
                    front_face: if i & 1 != 0 {
                        wgpu::FrontFace::Cw
                    } else {
                        wgpu::FrontFace::Ccw
                    },
                    cull_mode: if i & 2 != 0 {
                        None
                    } else {
                        Some(wgpu::Face::Back)
                    },
                    ..Default::default()
                },
                depth_stencil: Some(depth.clone()),
                multisample: wgpu::MultisampleState {
                    count: 4,
                    ..Default::default()
                },
                fragment: Some(wgpu::FragmentState {
                    module: shader,
                    entry_point: Some("fragment"),
                    compilation_options: Default::default(),
                    targets: &targets,
                }),
                multiview_mask: None,
                cache: None,
            };
            diagnostic::pipeline(
                d,
                &descriptor,
                diagnostic.as_ref(),
                i,
                if buffers.is_empty() {
                    "calibration"
                } else {
                    "scene"
                },
                if buffers.is_empty() {
                    &calibration_wgsl
                } else {
                    &wgsl
                },
            )
        };
        let pipelines = std::array::from_fn(|i| pipeline(&shader, Some(&layout), &buffers, i));
        let calibration = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Actual sample-location calibration"),
            source: wgpu::ShaderSource::Wgsl(calibration_wgsl.clone().into()),
        });
        let colors = formats.map(make);
        let depth_view = make(wgpu::TextureFormat::Depth32Float);
        if let Some(arm) = &diagnostic {
            diagnostic::write(
                &arm.root,
                "attachment-routing.json",
                &serde_json::json!({
                    "storage_arm":storage.name(),
                    "physical_color_attachments":colors.iter().zip(formats).enumerate().map(|(slot,(view,format))|
                        serde_json::json!({"slot":slot,"format":format!("{format:?}"),"view":format!("{view:?}")})).collect::<Vec<_>>(),
                    "depth_view":format!("{depth_view:?}"),
                    "semantic_readback_slots":storage.readback_slots(),
                    "receipt_kind":"actual wgpu views used by this probe; not native driver state introspection"
                }),
            );
        }
        Self {
            pipelines,
            calibration: pipeline(&calibration, None, &[], 2),
            colors,
            depth: depth_view,
            reader: readback.then(|| readback::Reader::new(d, diagnostic.as_ref())),
            size,
            globals,
            diagnostic,
        }
    }
    fn pass<'a>(&'a self, encoder: &'a mut wgpu::CommandEncoder) -> wgpu::RenderPass<'a> {
        let colors = self.colors.each_ref().map(|view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })
        });
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Visibility-only output4x"),
            color_attachments: &colors,
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        })
    }
    pub(crate) fn calibrate(&self, r: &Renderer) -> Vec<Sample> {
        let mut e = r.device.create_command_encoder(&Default::default());
        {
            let mut p = self.pass(&mut e);
            p.set_pipeline(&self.calibration);
            p.draw(0..3, 0..1);
        }
        r.queue.submit([e.finish()]);
        if self
            .diagnostic
            .as_ref()
            .is_some_and(|a| a.storage != Storage::Original)
        {
            self.read_storage(r).into_iter().map(|r| r.sample).collect()
        } else {
            self.read(r)
        }
    }
    pub(crate) fn encode(&self, d: &wgpu::Device, e: &mut wgpu::CommandEncoder, draw: &MotionDraw) {
        if self.reader.is_some() {
            assert_eq!(draw.rect, [0., 0., 48., 48.]);
        }
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct Frame {
            current: [[f32; 4]; 4],
            previous: [[f32; 4]; 4],
            rect: [f32; 4],
        }
        let f = Frame {
            current: draw.visibility_matrices[0].to_cols_array_2d(),
            previous: draw.visibility_matrices[1].to_cols_array_2d(),
            rect: [0., 0., self.size[0] as f32, self.size[1] as f32],
        };
        if let Some(arm) = &self.diagnostic {
            arm.record(draw, bytemuck::bytes_of(&f));
        }
        let uniform = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::bytes_of(&f),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.globals,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let mut p = self.pass(e);
        p.set_viewport(0., 0., self.size[0] as f32, self.size[1] as f32, 0., 1.);
        p.set_bind_group(0, &group, &[]);
        for ordinal in 0..draw.draws.len() {
            let item = &draw.draws[if self.diagnostic.as_ref().is_some_and(|arm| arm.reverse) {
                draw.draws.len() - 1 - ordinal
            } else {
                ordinal
            }];
            let b = &item.batch;
            let primitive = &b.model.primitives[b.primitive];
            let material = &b.model.materials.materials[primitive.material];
            assert_eq!(
                material.alpha,
                incant_assets_runtime::AlphaMode::Opaque,
                "visibility diagnostic excludes MASK and BLEND"
            );
            p.set_pipeline(
                &self.pipelines[usize::from(b.mirrored) + 2 * usize::from(material.double_sided)],
            );
            p.set_bind_group(1, &material.bind_group, &[]);
            p.set_vertex_buffer(
                0,
                b.vertices.as_ref().unwrap_or(&primitive.vertices).slice(..),
            );
            p.set_vertex_buffer(1, item.previous_vertices.slice(..));
            p.set_vertex_buffer(2, b.instances.slice(..));
            p.set_vertex_buffer(3, item.previous_instances.slice(..));
            p.set_index_buffer(primitive.indices.slice(..), wgpu::IndexFormat::Uint32);
            p.draw_indexed(0..primitive.index_count, 0, 0..b.count);
        }
    }
    pub(crate) fn read(&self, r: &Renderer) -> Vec<Sample> {
        let samples = self
            .reader
            .as_ref()
            .expect("diagnostic reader")
            .read(r, self);
        if let Some(arm) = &self.diagnostic {
            arm.flush_uploads();
        }
        samples
    }
    // Explicit diagnostic readback, called only after the submitted prototype.
    // No map/wait or CPU admission exists in the reconstruction encode path.
    pub(crate) fn read_fixture(&self, r: &Renderer) -> Vec<Sample> {
        assert_eq!(self.size, [SIZE; 2]);
        readback::Reader::new(&r.device, None).read(r, self)
    }
    pub(crate) fn read_storage(&self, r: &Renderer) -> Vec<StorageRecord> {
        let samples = self
            .reader
            .as_ref()
            .expect("diagnostic reader")
            .read_storage(r, self);
        if let Some(arm) = &self.diagnostic {
            arm.flush_uploads();
        }
        samples
    }
}
#[test]
fn sampled_visibility_shaders_validate() {
    for s in [
        source(),
        format!("{CALIBRATION}\n{PACK}"),
        readback::COPY.into(),
    ] {
        let m = naga::front::wgsl::parse_str(&s).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&m)
        .unwrap();
    }
}
#[test]
fn visibility_previous_projection_removes_exactly_one_accepted_jitter() {
    for ortho in [false, true] {
        let mut camera = crate::camera::CameraView::preview();
        if ortho {
            camera.orthographic_half_height = Some(4.);
        }
        for phase in 0..8 {
            let rect = [0., 0., 48., 48.];
            camera.jitter = crate::temporal::jitter::ndc(phase, rect);
            let corrected = remove_jitter(camera.matrix(1.).unwrap(), Some(phase), rect);
            let expected = camera.unjittered().matrix(1.).unwrap();
            for p in [Vec3::ZERO, Vec3::new(0.1, 0.2, -0.3)] {
                assert!((corrected.project_point3(p) - expected.project_point3(p)).length() < 2e-6);
            }
        }
    }
}

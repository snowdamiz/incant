//! Test-only adapter for an external, pinned WebGPU reconstruction reference.
//! No production dependency or shader import. Reference sources stay outside Git.
use super::*;
use wgpu::util::DeviceExt;

const SOURCE: [u32; 2] = [480, 270];
const OUTPUT: [u32; 2] = [960, 540];
const SHADERS: [(&str, &str); 6] = [
    (
        "RECONSTRUCT_SHADER",
        "a88e522c5a6ebf19c1e8f3e98a71d835c8be3740e93bc4b167cf860c17e0483a",
    ),
    (
        "DEPTH_CLIP_SHADER",
        "99937ef6f480d439b9ba74a49394903171caf075861ac3d930431a501dd5c9d5",
    ),
    (
        "LUMINANCE_PYRAMID_SHADER",
        "450c74cf5202f147537830a384a2394f8fccf522bd2011101e272634472c1c27",
    ),
    (
        "SHADING_CHANGE_SHADER",
        "8fda85c40ba87e4d84ed69746f6e1e708fcb9efdcc01c73ceb66da25679bb751",
    ),
    (
        "ACCUMULATE_SHADER",
        "ba15b53489882e55fdeabe7718594146d725ddd2ad93dffbc6b2c9706e343595",
    ),
    (
        "BLIT_SHADER",
        "0b13bfa5c56e60184873beab2242ac4e66fa7ac9095ed1244d785b9631c1d347",
    ),
];

#[derive(Clone, Copy)]
enum Kind {
    Uniform,
    Float(bool),
    Uint,
    Depth,
    Sampler,
    Store(wgpu::TextureFormat),
    Buffer(bool),
}
struct Pass {
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
}
impl Pass {
    fn new(d: &wgpu::Device, name: &str, source: &str, kinds: &[Kind]) -> Self {
        let entries: Vec<_> = kinds
            .iter()
            .enumerate()
            .map(|(i, kind)| {
                let ty = match *kind {
                    Kind::Uniform => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(96),
                    },
                    Kind::Float(filterable) => wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    Kind::Uint => wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Uint,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    Kind::Depth => wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    Kind::Sampler => {
                        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
                    }
                    Kind::Store(format) => wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    },
                    Kind::Buffer(read_only) => wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                };
                wgpu::BindGroupLayoutEntry {
                    binding: i as u32,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty,
                    count: None,
                }
            })
            .collect();
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(name),
            entries: &entries,
        });
        let pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(name),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(name),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let pipeline = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(name),
            layout: Some(&pipeline_layout),
            module: &module,
            entry_point: Some("main"),
            compilation_options: Default::default(),
            cache: None,
        });
        Self { layout, pipeline }
    }
    fn dispatch(
        &self,
        d: &wgpu::Device,
        e: &mut wgpu::CommandEncoder,
        resources: &[wgpu::BindingResource<'_>],
        size: [u32; 2],
    ) {
        let entries: Vec<_> = resources
            .iter()
            .enumerate()
            .map(|(i, r)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: r.clone(),
            })
            .collect();
        let group = d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("External reference diagnostic"),
            layout: &self.layout,
            entries: &entries,
        });
        let mut pass = e.begin_compute_pass(&Default::default());
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &group, &[]);
        pass.dispatch_workgroups(size[0].div_ceil(8), size[1].div_ceil(8), 1);
    }
}
fn texture(d: &wgpu::Device, size: [u32; 2], format: wgpu::TextureFormat) -> wgpu::TextureView {
    d.create_texture(&wgpu::TextureDescriptor {
        label: Some("External reference diagnostic allocation"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::STORAGE_BINDING
            | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
    .create_view(&Default::default())
}
pub(super) struct Reference {
    pub(super) accumulation_sha256: String,
    passes: [Pass; 6],
    bridge: Pass,
    uniform: wgpu::Buffer,
    sampler: wgpu::Sampler,
    converted_motion: wgpu::TextureView,
    depth: wgpu::TextureView,
    motion: wgpu::TextureView,
    masks: wgpu::TextureView,
    history: [wgpu::TextureView; 2],
    locks: [wgpu::TextureView; 2],
    exposure: [wgpu::TextureView; 2],
    luma: [wgpu::TextureView; 2],
    memory: [wgpu::TextureView; 2],
    signal: wgpu::TextureView,
    dummy: wgpu::TextureView,
    scatter: [wgpu::Buffer; 2],
    pub(super) output: wgpu::TextureView,
}
impl Reference {
    pub(super) fn diagnostic_state(&self, frame: u32) -> [&wgpu::TextureView; 3] {
        let now = frame as usize % 2;
        [&self.masks, &self.history[now], &self.locks[now]]
    }
    pub(super) fn new(r: &Renderer, locked_coverage: bool) -> Self {
        use Kind::*;
        use wgpu::TextureFormat::{R32Float, Rgba8Unorm, Rgba16Float, Rgba32Float, Rgba32Uint};
        let d = &r.device;
        let root = PathBuf::from(
            std::env::var("INCANT_REFERENCE_SHADER_DIR")
                .expect("explicit external reference directory"),
        );
        let kinds: [Vec<Kind>; 6] = [
            vec![
                Uniform,
                Depth,
                Float(false),
                Store(R32Float),
                Store(Rgba16Float),
                Buffer(false),
            ],
            vec![
                Uniform,
                Float(false),
                Float(false),
                Buffer(true),
                Store(Rgba8Unorm),
                Buffer(false),
            ],
            vec![
                Uniform,
                Float(true),
                Sampler,
                Float(false),
                Store(Rgba16Float),
                Float(false),
                Float(false),
            ],
            vec![
                Uniform,
                Float(false),
                Float(false),
                Float(false),
                Float(false),
                Float(false),
                Store(R32Float),
                Store(R32Float),
                Float(false),
                Uint,
                Store(Rgba32Uint),
            ],
            vec![
                Uniform,
                Float(false),
                Float(false),
                Float(false),
                Float(true),
                Sampler,
                Store(Rgba16Float),
                Float(true),
                Store(Rgba16Float),
                Float(false),
                Float(false),
                Float(false),
                Float(false),
            ],
            vec![
                Uniform,
                Float(true),
                Sampler,
                Float(false),
                Store(Rgba16Float),
                Float(true),
            ],
        ];
        let mut accumulation_sha256 = String::new();
        let passes = std::array::from_fn(|i| {
            let (name, digest) = SHADERS[i];
            let source = std::fs::read_to_string(root.join(format!("{name}.wgsl"))).unwrap();
            assert_eq!(
                hex(source.as_bytes()),
                digest,
                "external reference source identity"
            );
            // Deliberately separate experimental derivative: verify the original
            // first, then replace exactly one expression. The immutable external
            // files and the default comparison remain byte-for-byte unchanged.
            let source =
                if locked_coverage && name == "ACCUMULATE_SHADER" {
                    let original = "let alphaRelax = stillRelax / max(STILL_CLAMP_RELAX, 1.0e-6);";
                    assert_eq!(source.matches(original).count(), 1);
                    source.replace(original,
                    "let alphaRelax = max(stillRelax / max(STILL_CLAMP_RELAX, 1.0e-6), lockLife);")
                } else {
                    source
                };
            if name == "ACCUMULATE_SHADER" {
                accumulation_sha256 = hex(source.as_bytes());
            }
            Pass::new(d, name, &source, &kinds[i])
        });
        let bridge = Pass::new(
            d,
            "Incant motion convention adapter",
            BRIDGE,
            &[Uniform, Float(false), Store(Rgba32Float)],
        );
        let pair = |size, format| std::array::from_fn(|_| texture(d, size, format));
        let uniform = d.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reference constants"),
            size: 256,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let scatter = std::array::from_fn(|_| {
            d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Reference empty depth scatter"),
                contents: bytemuck::cast_slice(&vec![
                    0x7f800000_u32;
                    (SOURCE[0] * SOURCE[1]) as usize
                ]),
                usage: wgpu::BufferUsages::STORAGE,
            })
        });
        Self {
            accumulation_sha256,
            passes,
            bridge,
            uniform,
            scatter,
            sampler: d.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("Reference linear clamp sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            converted_motion: texture(d, SOURCE, Rgba32Float),
            depth: texture(d, SOURCE, R32Float),
            motion: texture(d, SOURCE, Rgba16Float),
            masks: texture(d, SOURCE, Rgba8Unorm),
            history: pair(OUTPUT, Rgba16Float),
            locks: pair(OUTPUT, Rgba16Float),
            exposure: pair([1, 1], Rgba16Float),
            luma: pair(SOURCE, R32Float),
            memory: pair([120, 102], Rgba32Uint),
            signal: texture(d, [240, 135], R32Float),
            dummy: texture(d, [1, 1], Rgba16Float),
            output: texture(d, OUTPUT, Rgba16Float),
        }
    }
    pub(super) fn dispatch(
        &self,
        r: &Renderer,
        e: &mut wgpu::CommandEncoder,
        frame: u32,
        jitter: [f32; 2],
        previous: [f32; 2],
        inputs: [&wgpu::TextureView; 3],
    ) {
        use wgpu::BindingResource::{Sampler as S, TextureView as T};
        let [color, motion, depth] = inputs;
        // Incant samples at t-jitter and stores jittered previous-current UV.
        // Reference samples at t+jitter and expects unjittered current-previous UV.
        let mut words = [0_u32; 64];
        let values = [
            480.,
            270.,
            960.,
            540.,
            1. / 480.,
            1. / 270.,
            1. / 960.,
            1. / 540.,
            -jitter[0],
            -jitter[1],
            -previous[0],
            -previous[1],
            1.,
            1.,
            NEAR as f32,
            FAR as f32,
            0.,
            24.,
            1.,
            1. / 60.,
        ];
        for (i, value) in values.into_iter().enumerate() {
            words[i] = value.to_bits();
        }
        words[20] = 4 | 8 | 32 | 128 | u32::from(frame == 1);
        words[21] = frame - 1;
        words[23] = 32;
        r.queue
            .write_buffer(&self.uniform, 0, bytemuck::cast_slice(&words));
        let b = || self.uniform.as_entire_binding();
        let d = &r.device;
        let now = frame as usize % 2;
        let old = 1 - now;
        self.bridge
            .dispatch(d, e, &[b(), T(motion), T(&self.converted_motion)], SOURCE);
        self.passes[0].dispatch(
            d,
            e,
            &[
                b(),
                T(depth),
                T(&self.converted_motion),
                T(&self.depth),
                T(&self.motion),
                self.scatter[now].as_entire_binding(),
            ],
            SOURCE,
        );
        self.passes[1].dispatch(
            d,
            e,
            &[
                b(),
                T(&self.depth),
                T(&self.motion),
                self.scatter[now].as_entire_binding(),
                T(&self.masks),
                self.scatter[old].as_entire_binding(),
            ],
            SOURCE,
        );
        self.passes[2].dispatch(
            d,
            e,
            &[
                b(),
                T(color),
                S(&self.sampler),
                T(&self.exposure[old]),
                T(&self.exposure[now]),
                T(&self.dummy),
                T(&self.dummy),
            ],
            [8, 8],
        );
        self.passes[3].dispatch(
            d,
            e,
            &[
                b(),
                T(color),
                T(&self.luma[old]),
                T(&self.motion),
                T(&self.exposure[now]),
                T(&self.exposure[old]),
                T(&self.luma[now]),
                T(&self.signal),
                T(&self.masks),
                T(&self.memory[old]),
                T(&self.memory[now]),
            ],
            [240, 135],
        );
        self.passes[4].dispatch(
            d,
            e,
            &[
                b(),
                T(color),
                T(&self.motion),
                T(&self.masks),
                T(&self.history[old]),
                S(&self.sampler),
                T(&self.history[now]),
                T(&self.locks[old]),
                T(&self.locks[now]),
                T(&self.exposure[now]),
                T(&self.dummy),
                T(&self.exposure[old]),
                T(&self.signal),
            ],
            OUTPUT,
        );
        self.passes[5].dispatch(
            d,
            e,
            &[
                b(),
                T(&self.history[now]),
                S(&self.sampler),
                T(&self.exposure[now]),
                T(&self.output),
                T(&self.locks[now]),
            ],
            OUTPUT,
        );
    }
}
// Motion conversion only. Invalid vectors are zeroed for this reference fixture;
// this does not establish a production rejection/reactivity policy for them.
const BRIDGE: &str = r#"
struct Constants { render_size:vec2f,display_size:vec2f,render_inv:vec2f,display_inv:vec2f,jitter:vec2f,previous:vec2f,rest:array<vec4u,3> }
@group(0) @binding(0) var<uniform> c:Constants;
@group(0) @binding(1) var input:texture_2d<f32>;
@group(0) @binding(2) var output:texture_storage_2d<rgba32float,write>;
@compute @workgroup_size(8,8)
fn main(@builtin(global_invocation_id) gid:vec3u) {
    if any(vec2f(gid.xy)>=c.render_size){return;}
    let m=textureLoad(input,vec2i(gid.xy),0);
    let delta=(c.jitter-c.previous)*c.render_inv;
    textureStore(output,gid.xy,vec4f(select(vec2f(0.0),-m.xy+delta,m.w>=0.5),0.0,0.0));
}
"#;

use super::MotionError;
use crate::{Renderer, Result};

pub(crate) struct MotionSystem {
    pub(super) globals: wgpu::BindGroupLayout,
    pipelines: [wgpu::RenderPipeline; 4],
}
const CURRENT: [wgpu::VertexAttribute; 2] = [
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x3,
        offset: 0,
        shader_location: 0,
    },
    wgpu::VertexAttribute {
        format: wgpu::VertexFormat::Float32x2,
        offset: 24,
        shader_location: 1,
    },
];
const PREVIOUS: [wgpu::VertexAttribute; 1] = [wgpu::VertexAttribute {
    format: wgpu::VertexFormat::Float32x3,
    offset: 0,
    shader_location: 2,
}];
const WORLD: [wgpu::VertexAttribute; 4] =
    wgpu::vertex_attr_array![3=>Float32x4,4=>Float32x4,5=>Float32x4,6=>Float32x4];
const OLD_WORLD: [wgpu::VertexAttribute; 5] =
    wgpu::vertex_attr_array![7=>Float32x4,8=>Float32x4,9=>Float32x4,10=>Float32x4,11=>Float32x4];
impl MotionSystem {
    fn new(renderer: &Renderer) -> Self {
        let device = &renderer.device;
        let globals = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Motion frame"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(144),
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Motion opaque surface"),
            bind_group_layouts: &[Some(&globals), Some(&renderer.materials.layout)],
            immediate_size: 0,
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Motion opaque surface"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });
        let buffers = [
            wgpu::VertexBufferLayout {
                array_stride: 48,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &CURRENT,
            },
            wgpu::VertexBufferLayout {
                array_stride: 48,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &PREVIOUS,
            },
            wgpu::VertexBufferLayout {
                array_stride: 112,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &WORLD,
            },
            wgpu::VertexBufferLayout {
                array_stride: 80,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &OLD_WORLD,
            },
        ];
        let pipelines = std::array::from_fn(|i| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Motion opaque surface"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex"),
                    compilation_options: Default::default(),
                    buffers: &buffers,
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
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba32Float,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        });
        Self { globals, pipelines }
    }
    pub(super) fn pipeline(&self, double_sided: bool, mirrored: bool) -> wgpu::RenderPipeline {
        self.pipelines[usize::from(mirrored) + 2 * usize::from(double_sided)].clone()
    }
}
impl Renderer {
    /// Startup-only opt-in. Creates four fixed pipelines; ordinary draw_scene
    /// remains stateless. Enable history separately on a retained owner/host.
    pub async fn enable_motion_vectors(&mut self) -> Result<()> {
        super::admission()?;
        if self.motion.is_some() {
            return Ok(());
        }
        self.check_device()?;
        for format in [
            wgpu::TextureFormat::Rgba32Float,
            wgpu::TextureFormat::Depth32Float,
        ] {
            let needed = wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC;
            if !self
                .adapter
                .get_texture_format_features(format)
                .allowed_usages
                .contains(needed)
            {
                return Err(MotionError::Format.into());
            }
        }
        let validation = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let candidate = MotionSystem::new(self);
        if let Some(error) = validation.pop().await {
            #[cfg(target_arch = "wasm32")]
            let error = error.into();
            return Err(crate::ResourceError::Preparation(error).into());
        }
        self.motion = Some(candidate);
        Ok(())
    }
}

use super::{
    math::{AmbientOcclusion, Uniform},
    shader::{self, Kind},
    targets::Cache,
};
use crate::{Renderer, Result, materials::PipelineKey};
use incant_assets_runtime::AlphaMode;
use std::collections::HashMap;

pub(crate) struct System {
    pub settings: AmbientOcclusion,
    pub globals: wgpu::BindGroupLayout,
    pub sample_layout: wgpu::BindGroupLayout,
    pub sample: wgpu::RenderPipeline,
    pub filter_layout: wgpu::BindGroupLayout,
    pub filters: [wgpu::RenderPipeline; 2],
    pub prepass: HashMap<PipelineKey, wgpu::RenderPipeline>,
    pub shading: HashMap<PipelineKey, wgpu::RenderPipeline>,
    pub targets: Cache,
    #[cfg(all(feature = "ssao-stage-readback", not(target_arch = "wasm32")))]
    pub(super) capture: std::sync::Arc<super::readback::Capture>,
}
#[cfg(any(test, not(all(feature = "cooked-ssao", not(target_arch = "wasm32")))))]
pub(crate) fn material_source(prepass: bool) -> Result<String> {
    super::source::material(prepass)
        .ok_or_else(|| super::AmbientOcclusionError::ShaderContract.into())
}
pub(super) fn texture(binding: u32, depth: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: if depth {
                wgpu::TextureSampleType::Depth
            } else {
                wgpu::TextureSampleType::Float { filterable: false }
            },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}
impl System {
    pub fn new(renderer: &Renderer, settings: AmbientOcclusion) -> Result<Self> {
        shader::validate(renderer)?;
        let device = &renderer.device;
        let cache = renderer.pipeline_cache.cache.as_ref();
        let globals = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SSAO material frame"),
            entries: &[crate::materials::frame_entry(), texture(1, false)],
        });
        let sample_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SSAO sampling"),
            entries: &[uniform_entry(), texture(1, true), texture(2, false)],
        });
        let pre = shader::stages(renderer, Kind::Prepass)?;
        let forward = shader::stages(renderer, Kind::Forward)?;
        let mut prepass = HashMap::new();
        let mut shading = HashMap::new();
        for alpha in [AlphaMode::Opaque, AlphaMode::Mask] {
            for double_sided in [false, true] {
                for mirrored in [false, true] {
                    let key = PipelineKey {
                        format: crate::frame::HDR_FORMAT,
                        alpha,
                        double_sided,
                        mirrored,
                        samples: crate::SceneMsaa::One,
                    };
                    prepass.insert(
                        key,
                        crate::material_pipeline::build(
                            device,
                            &pre,
                            PipelineKey {
                                format: wgpu::TextureFormat::Rgba16Float,
                                ..key
                            },
                            &[&renderer.materials.globals, &renderer.materials.layout],
                            cache,
                        ),
                    );
                    shading.insert(
                        key,
                        crate::material_pipeline::build(
                            device,
                            &forward,
                            key,
                            &[
                                &globals,
                                &renderer.materials.layout,
                                &renderer.environments.layout,
                                &renderer.lighting.layout,
                            ],
                            cache,
                        ),
                    );
                }
            }
        }
        let shader = shader::stages(renderer, Kind::Sample)?;
        let sample = fullscreen(
            device,
            &sample_layout,
            &shader,
            "SSAO bounded sampling",
            cache,
        );
        let filter_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SSAO plane-aware filter"),
            entries: &[
                uniform_entry(),
                texture(1, true),
                texture(2, false),
                texture(3, false),
            ],
        });
        let filters = shader::filters(renderer)?.map(|shader| {
            fullscreen(
                device,
                &filter_layout,
                &shader,
                "SSAO plane-aware filter",
                cache,
            )
        });
        Ok(Self {
            settings,
            globals,
            sample_layout,
            sample,
            filter_layout,
            filters,
            prepass,
            shading,
            targets: Cache::default(),
            #[cfg(all(feature = "ssao-stage-readback", not(target_arch = "wasm32")))]
            capture: Default::default(),
        })
    }
}

pub(super) fn uniform_entry() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(size_of::<Uniform>() as u64),
        },
        count: None,
    }
}

fn fullscreen(
    device: &wgpu::Device,
    bindings: &wgpu::BindGroupLayout,
    shader: &crate::shader::Stages,
    label: &str,
    cache: Option<&wgpu::PipelineCache>,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: &[Some(bindings)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader.vertex,
            entry_point: Some(shader.vertex_entry),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader.fragment,
            entry_point: Some(shader.fragment_entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::R8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::RED,
            })],
        }),
        multiview_mask: None,
        cache,
    })
}

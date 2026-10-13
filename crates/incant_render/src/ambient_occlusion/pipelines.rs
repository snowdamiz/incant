use super::{
    math::{AmbientOcclusion, AmbientOcclusionError, Uniform},
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
    pub prepass: HashMap<PipelineKey, wgpu::RenderPipeline>,
    pub shading: HashMap<PipelineKey, wgpu::RenderPipeline>,
    pub targets: Cache,
}
pub(crate) fn material_source(prepass: bool) -> Result<String> {
    let base = include_str!("../model_material.wgsl");
    if prepass {
        let (vertex, _) = base
            .split_once("@fragment fn fragment")
            .ok_or(AmbientOcclusionError::ShaderContract)?;
        return Ok(format!("{vertex}\n{}", include_str!("prepass.wgsl")));
    }
    let old = "(1.0-reflectance)*(1.0-metallic)*base.rgb*irradiance";
    if base.matches(old).count() != 1 {
        return Err(AmbientOcclusionError::ShaderContract.into());
    }
    // Only diffuse irradiance is occluded: specular visibility needs a different
    // angular integral. Material-authored AO keeps its existing independent role.
    Ok(format!(
        "{}\n{}\n@group(0) @binding(1) var diffuse_accessibility:texture_2d<f32>;\n{}",
        include_str!("../shadows/shade.wgsl"),
        include_str!("../lighting/shade.wgsl"),
        base.replace(
            old,
            &format!("({old})*textureLoad(diffuse_accessibility,vec2i(in.position.xy),0).r")
        )
    ))
}
fn texture(binding: u32, depth: bool) -> wgpu::BindGroupLayoutEntry {
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
        let device = &renderer.device;
        let globals = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SSAO material frame"),
            entries: &[crate::materials::frame_entry(), texture(1, false)],
        });
        let sample_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SSAO sampling"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(size_of::<Uniform>() as u64),
                    },
                    count: None,
                },
                texture(1, true),
                texture(2, false),
            ],
        });
        let module = |label, source: String| {
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(label),
                source: wgpu::ShaderSource::Wgsl(source.into()),
            })
        };
        let pre = module("SSAO opaque normals", material_source(true)?);
        let forward = module("SSAO diffuse-only materials", material_source(false)?);
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
                    };
                    let stages = |module: &wgpu::ShaderModule| crate::shader::Stages {
                        vertex: module.clone(),
                        fragment: module.clone(),
                        vertex_entry: "vertex",
                        fragment_entry: "fragment",
                    };
                    prepass.insert(
                        key,
                        crate::material_pipeline::build(
                            device,
                            &stages(&pre),
                            PipelineKey {
                                format: wgpu::TextureFormat::Rgba16Float,
                                ..key
                            },
                            &[&renderer.materials.globals, &renderer.materials.layout],
                            None,
                        ),
                    );
                    shading.insert(
                        key,
                        crate::material_pipeline::build(
                            device,
                            &stages(&forward),
                            key,
                            &[
                                &globals,
                                &renderer.materials.layout,
                                &renderer.environments.layout,
                                &renderer.lighting.layout,
                            ],
                            None,
                        ),
                    );
                }
            }
        }
        let shader = module(
            "SSAO bounded hemisphere visibility",
            include_str!("ao.wgsl").into(),
        );
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SSAO sampling"),
            bind_group_layouts: &[Some(&sample_layout)],
            immediate_size: 0,
        });
        let sample = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SSAO bounded sampling"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertex"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragment"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::R8Unorm,
                    blend: None,
                    write_mask: wgpu::ColorWrites::RED,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        Ok(Self {
            settings,
            globals,
            sample_layout,
            sample,
            prepass,
            shading,
            targets: Cache::default(),
        })
    }
}

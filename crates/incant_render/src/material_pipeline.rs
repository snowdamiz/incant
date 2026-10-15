use crate::materials::PipelineKey;
use incant_assets_runtime::AlphaMode;

pub(crate) fn build(
    device: &wgpu::Device,
    shader: &crate::shader::Stages,
    key: PipelineKey,
    layouts: &[&wgpu::BindGroupLayout],
    cache: Option<&wgpu::PipelineCache>,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Material pipeline"),
        bind_group_layouts: &layouts
            .iter()
            .map(|layout| Some(*layout))
            .collect::<Vec<_>>(),
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Imported metallic roughness"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader.vertex,
            entry_point: Some(shader.vertex_entry),
            compilation_options: Default::default(),
            buffers: &crate::geometry_shader::shadow_buffers(),
        },
        primitive: wgpu::PrimitiveState {
            front_face: if key.mirrored {
                wgpu::FrontFace::Cw
            } else {
                wgpu::FrontFace::Ccw
            },
            cull_mode: if key.double_sided {
                None
            } else {
                Some(wgpu::Face::Back)
            },
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(key.alpha != AlphaMode::Blend),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: key.samples.state(),
        fragment: Some(wgpu::FragmentState {
            module: &shader.fragment,
            entry_point: Some(shader.fragment_entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: key.format,
                blend: if key.alpha == AlphaMode::Blend {
                    Some(wgpu::BlendState::ALPHA_BLENDING)
                } else {
                    None
                },
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache,
    })
}

#[cfg(all(feature = "cooked-materials", not(target_arch = "wasm32")))]
#[path = "material_shader.rs"]
mod cooked;
#[cfg(all(feature = "cooked-materials", not(target_arch = "wasm32")))]
pub(crate) use cooked::validate_host as validate_cooked_host;

pub(crate) fn shader(
    device: &wgpu::Device,
    _backend: wgpu::Backend,
) -> crate::Result<crate::shader::Stages> {
    #[cfg(all(feature = "cooked-materials", not(target_arch = "wasm32")))]
    return cooked::stages(device, _backend);
    #[cfg(not(all(feature = "cooked-materials", not(target_arch = "wasm32"))))]
    {
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("glTF metallic roughness"),
            source: wgpu::ShaderSource::Wgsl(
                format!(
                    "{}\n{}\n{}",
                    include_str!("shadows/shade.wgsl"),
                    include_str!("lighting/shade.wgsl"),
                    include_str!("model_material.wgsl")
                )
                .into(),
            ),
        });
        Ok(crate::shader::Stages {
            vertex: module.clone(),
            fragment: module,
            vertex_entry: "vertex",
            fragment_entry: "fragment",
        })
    }
}

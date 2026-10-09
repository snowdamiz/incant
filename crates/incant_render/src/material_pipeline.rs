use crate::materials::PipelineKey;
use incant_assets::AlphaMode;

pub(crate) fn build(
    device: &wgpu::Device,
    key: PipelineKey,
    globals: &wgpu::BindGroupLayout,
    materials: &wgpu::BindGroupLayout,
    environment: &wgpu::BindGroupLayout,
    lighting: &wgpu::BindGroupLayout,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("glTF metallic roughness"),
        source: wgpu::ShaderSource::Wgsl(
            format!(
                "{}\n{}",
                include_str!("lighting/shade.wgsl"),
                include_str!("model_material.wgsl")
            )
            .into(),
        ),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Material pipeline"),
        bind_group_layouts: &[
            Some(globals),
            Some(materials),
            Some(environment),
            Some(lighting),
        ],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Imported metallic roughness"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[
                wgpu::VertexBufferLayout {
                    array_stride: size_of::<incant_assets::Vertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,9=>Float32x2,10=>Float32x4],
                },
                wgpu::VertexBufferLayout {
                    array_stride: size_of::<crate::scene::Instance>() as u64,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![2=>Float32x4,3=>Float32x4,4=>Float32x4,5=>Float32x4,6=>Float32x4,7=>Float32x4,8=>Float32x4],
                },
            ],
        },
        primitive: wgpu::PrimitiveState {
            front_face: if key.mirrored { wgpu::FrontFace::Cw } else { wgpu::FrontFace::Ccw },
            cull_mode: if key.double_sided { None } else { Some(wgpu::Face::Back) },
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(key.alpha != AlphaMode::Blend),
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
                format: key.format,
                blend: if key.alpha == AlphaMode::Blend { Some(wgpu::BlendState::ALPHA_BLENDING) } else { None },
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

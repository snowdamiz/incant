use incant_assets_runtime::AlphaMode;
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) struct PipelineKey {
    pub alpha: AlphaMode,
    pub double_sided: bool,
    pub mirrored: bool,
}
pub(super) fn build(
    device: &wgpu::Device,
    shader: &crate::shader::Stages,
    camera: &wgpu::BindGroupLayout,
    materials: &wgpu::BindGroupLayout,
    key: PipelineKey,
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Directional depth caster"),
        bind_group_layouts: &[Some(camera), Some(materials)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Directional depth caster"),
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
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: Default::default(),
            bias: wgpu::DepthBiasState {
                constant: 2,
                slope_scale: 2.,
                clamp: 0.,
            },
        }),
        multisample: Default::default(),
        fragment: (key.alpha == AlphaMode::Mask).then_some(wgpu::FragmentState {
            module: &shader.fragment,
            entry_point: Some(shader.fragment_entry),
            compilation_options: Default::default(),
            targets: &[],
        }),
        multiview_mask: None,
        cache: None,
    })
}

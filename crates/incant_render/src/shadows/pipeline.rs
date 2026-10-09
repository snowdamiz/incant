use incant_assets::AlphaMode;
#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub(crate) struct PipelineKey {
    pub alpha: AlphaMode,
    pub double_sided: bool,
    pub mirrored: bool,
}
pub(super) fn build(
    device: &wgpu::Device,
    camera: &wgpu::BindGroupLayout,
    materials: &wgpu::BindGroupLayout,
    key: PipelineKey,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Directional depth caster"),
        source: wgpu::ShaderSource::Wgsl(include_str!("caster.wgsl").into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Directional depth caster"),
        bind_group_layouts: &[Some(camera), Some(materials)],
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label:Some("Directional depth caster"),layout:Some(&layout),
        vertex:wgpu::VertexState {module:&shader,entry_point:Some("vertex"),compilation_options:Default::default(),buffers:&[
            wgpu::VertexBufferLayout {array_stride:size_of::<incant_assets::Vertex>() as u64,step_mode:wgpu::VertexStepMode::Vertex,attributes:&wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3,9=>Float32x2,10=>Float32x4]},
            wgpu::VertexBufferLayout {array_stride:size_of::<crate::scene::Instance>() as u64,step_mode:wgpu::VertexStepMode::Instance,attributes:&wgpu::vertex_attr_array![2=>Float32x4,3=>Float32x4,4=>Float32x4,5=>Float32x4,6=>Float32x4,7=>Float32x4,8=>Float32x4]},
        ]},
        primitive:wgpu::PrimitiveState {
            front_face:if key.mirrored {wgpu::FrontFace::Cw} else {wgpu::FrontFace::Ccw},
            cull_mode:if key.double_sided {None} else {Some(wgpu::Face::Back)},..Default::default()
        },
        depth_stencil:Some(wgpu::DepthStencilState {
            format:wgpu::TextureFormat::Depth32Float,depth_write_enabled:Some(true),depth_compare:Some(wgpu::CompareFunction::Less),
            stencil:Default::default(),bias:wgpu::DepthBiasState {constant:2,slope_scale:2.,clamp:0.},
        }),
        multisample:Default::default(),
        fragment:(key.alpha==AlphaMode::Mask).then_some(wgpu::FragmentState {module:&shader,entry_point:Some("fragment"),compilation_options:Default::default(),targets:&[]}),
        multiview_mask:None,cache:None,
    })
}

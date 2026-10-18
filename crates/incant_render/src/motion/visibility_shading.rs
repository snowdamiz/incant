//! Test-only current-frame PBR for GPU-requested output samples. No host admission.
use super::MotionDraw;
use crate::{RenderScene, Renderer, Result, SceneError};
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub(crate) struct System {
    pub mask: wgpu::TextureView,
    pub color: wgpu::TextureView,
    pub depth: wgpu::TextureView,
    pub size: [u32; 2],
    pipelines: [wgpu::RenderPipeline; 4],
    globals: wgpu::BindGroupLayout,
    all_lights: bool,
}
pub(crate) struct Prepared {
    system: Arc<System>,
    camera: wgpu::BindGroup,
    environment: wgpu::BindGroup,
    pub lighting: crate::lighting::PreparedLighting,
}

pub(crate) fn source() -> String {
    // Import the actual ordinary material implementation. Only the diagnostic
    // input frequency, exact visibility vertex expression and late gate differ.
    let mut material = include_str!("../model_material.wgsl").replace(
        "@interpolate(perspective, centroid)",
        "@interpolate(perspective, sample)",
    );
    for (from, to) in [
        (
            "uv:vec2f }",
            "uv:vec2f, @location(4) @interpolate(flat, first) owner:u32 }",
        ),
        (
            "@location(10) tangent:vec4f)",
            "@location(10) tangent:vec4f,@location(11) owner:u32)",
        ),
        (
            "out.position=frame.view_projection*p;",
            "out.position=frame.view_projection*world*vec4f(position,1);out.owner=owner;",
        ),
        (
            "@builtin(front_facing) front:bool)",
            "@builtin(front_facing) front:bool,@builtin(sample_index) lane:u32)",
        ),
        (
            "    if material.flags.x==1u && base.a<material.factors.w { discard; }",
            "    // No nonuniform gate precedes the five implicit-derivative samples.\n    let pixel=vec2i(in.position.xy);\n    let mask=textureLoad(missing_mask,pixel,0).x;\n    let packed=textureLoad(current_owner,pixel,i32(lane));\n    let owner=packed.x|(packed.y<<16u);\n    if (mask&(1u<<lane))==0u || owner!=in.owner {discard;}\n    if material.flags.x==1u && base.a<material.factors.w { discard; }",
        ),
    ] {
        assert_eq!(
            material.matches(from).count(),
            1,
            "PBR fallback source anchor changed"
        );
        material = material.replace(from, to);
    }
    format!(
        "{}\n{}\n@group(0) @binding(1) var missing_mask:texture_2d<u32>;\n@group(0) @binding(2) var current_owner:texture_multisampled_2d<u32>;\n{material}",
        include_str!("../shadows/shade.wgsl"),
        include_str!("../lighting/shade.wgsl")
    )
}
impl System {
    pub(crate) fn new(r: &Renderer, size: [u32; 2], all_lights: bool) -> Self {
        for (format, count) in [
            (wgpu::TextureFormat::R32Uint, 1),
            (crate::frame::HDR_FORMAT, 4),
            (wgpu::TextureFormat::Depth32Float, 4),
        ] {
            let f = r.adapter.get_texture_format_features(format);
            assert!(f.allowed_usages.contains(
                wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING
            ));
            assert!(
                count == 1
                    || f.flags
                        .contains(wgpu::TextureFormatFeatureFlags::MULTISAMPLE_X4)
            );
        }
        let make = |format, count| {
            r.device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some("Test-only selective current radiance"),
                    size: wgpu::Extent3d {
                        width: size[0],
                        height: size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: count,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let globals = r
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Selective current PBR frame"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: wgpu::BufferSize::new(96),
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Uint,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Uint,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: true,
                        },
                        count: None,
                    },
                ],
            });
        let layout = r
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[
                    Some(&globals),
                    Some(&r.materials.layout),
                    Some(&r.environments.layout),
                    Some(&r.lighting.layout),
                ],
                immediate_size: 0,
            });
        let shader = r.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Ordinary PBR sample-frequency late mask"),
            source: wgpu::ShaderSource::Wgsl(source().into()),
        });
        let ordinary = crate::geometry_shader::shadow_buffers();
        let owner = [wgpu::VertexAttribute {
            format: wgpu::VertexFormat::Uint32,
            offset: 68,
            shader_location: 11,
        }];
        let buffers = [
            ordinary[0].clone(),
            ordinary[1].clone(),
            wgpu::VertexBufferLayout {
                array_stride: 80,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &owner,
            },
        ];
        let pipelines = std::array::from_fn(|i| {
            r.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("Selective current opaque PBR"),
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
                    multisample: wgpu::MultisampleState {
                        count: 4,
                        ..Default::default()
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &shader,
                        entry_point: Some("fragment"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: crate::frame::HDR_FORMAT,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    multiview_mask: None,
                    cache: None,
                })
        });
        Self {
            mask: make(wgpu::TextureFormat::R32Uint, 1),
            color: make(crate::frame::HDR_FORMAT, 4),
            depth: make(wgpu::TextureFormat::Depth32Float, 4),
            size,
            pipelines,
            globals,
            all_lights,
        }
    }
    pub(crate) fn prepare(
        self: &Arc<Self>,
        r: &Renderer,
        scene: &RenderScene,
        motion: &MotionDraw,
        owners: &wgpu::TextureView,
        shadows: &crate::shadows::PreparedShadows,
    ) -> Result<Prepared> {
        // Recheck every staged scene; startup admission cannot cover live material changes.
        if motion.draws.iter().any(|d| {
            let p = &d.batch.model.primitives[d.batch.primitive];
            d.batch.model.materials.materials[p.material].alpha
                != incant_assets_runtime::AlphaMode::Opaque
        }) {
            return Err(
                SceneError::Input("current radiance fallback requires opaque models").into(),
            );
        }
        let environment = scene.environment.as_ref().expect("model environment");
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct Frame {
            matrix: [[f32; 4]; 4],
            eye: [f32; 4],
            environment: [f32; 4],
        }
        let uniform = r
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsages::UNIFORM,
                contents: bytemuck::bytes_of(&Frame {
                    matrix: motion.visibility_matrices[0].to_cols_array_2d(),
                    eye: scene.camera.shading_eye(),
                    environment: [
                        environment.intensity,
                        environment.rotation.cos(),
                        environment.rotation.sin(),
                        crate::environment::MAX_SPECULAR_LOD,
                    ],
                }),
            });
        let camera = r.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.globals,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.mask),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(owners),
                },
            ],
        });
        let rect = [0., 0., self.size[0] as f32, self.size[1] as f32];
        // The unconditional reference bypasses clustering; exact sample RGB
        // then also tests the candidate's output-coordinate light selection.
        let selection = if self.all_lights {
            crate::LocalLightSelection::All
        } else {
            scene.light_selection
        };
        let lighting = r.lighting.prepare(
            &r.device,
            &scene.lights,
            rect,
            selection,
            scene.camera.unjittered(),
            shadows,
        )?;
        Ok(Prepared {
            system: Arc::clone(self),
            camera,
            environment: environment.resource.group.clone(),
            lighting,
        })
    }
}
impl Prepared {
    pub(crate) fn encode(&self, e: &mut wgpu::CommandEncoder, motion: &MotionDraw) {
        let mut p = e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Current missing-sample radiance; first depth winner"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.system.color,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.system.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        p.set_viewport(
            0.,
            0.,
            self.system.size[0] as f32,
            self.system.size[1] as f32,
            0.,
            1.,
        );
        p.set_bind_group(0, &self.camera, &[]);
        p.set_bind_group(2, &self.environment, &[]);
        p.set_bind_group(3, &self.lighting.shading, &[]);
        for draw in &motion.draws {
            let b = &draw.batch;
            let primitive = &b.model.primitives[b.primitive];
            let material = &b.model.materials.materials[primitive.material];
            p.set_pipeline(
                &self.system.pipelines
                    [usize::from(b.mirrored) + 2 * usize::from(material.double_sided)],
            );
            p.set_bind_group(1, &material.bind_group, &[]);
            p.set_vertex_buffer(
                0,
                b.vertices.as_ref().unwrap_or(&primitive.vertices).slice(..),
            );
            p.set_vertex_buffer(1, b.instances.slice(..));
            p.set_vertex_buffer(2, draw.previous_instances.slice(..));
            p.set_index_buffer(primitive.indices.slice(..), wgpu::IndexFormat::Uint32);
            p.draw_indexed(0..primitive.index_count, 0, 0..b.count);
        }
    }
}

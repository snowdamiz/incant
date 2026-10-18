use super::Pending;
use crate::{Renderer, Result, materials::PipelineKey};
use std::collections::HashMap;
use wgpu::util::DeviceExt;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct Frame {
    pub(super) source: [f32; 4],
    pub(super) output: [f32; 4],
    pub(super) depth: [f32; 4],
    pub(super) jitter: [f32; 4],
}
const _: () = assert!(size_of::<Frame>() == 64);
pub(crate) struct System {
    materials: HashMap<PipelineKey, wgpu::RenderPipeline>,
    layout: wgpu::BindGroupLayout,
    resolve: wgpu::RenderPipeline,
}
pub(crate) struct Draw {
    pub(super) pipeline: wgpu::RenderPipeline,
    pub(super) group: wgpu::BindGroup,
    pub color: wgpu::TextureView,
    pub(super) support: wgpu::TextureView,
    #[cfg(test)]
    pub(super) identity: Option<wgpu::TextureView>,
    #[cfg(test)]
    pub(crate) visibility_before: bool,
    #[cfg(test)]
    pub(crate) classification: Option<super::visibility_fallback::Classification>,
}
pub(super) fn center_source() -> String {
    let material = include_str!("../model_material.wgsl").replace(
        "@interpolate(perspective, centroid)",
        "@interpolate(perspective, center)",
    );
    format!(
        "{}\n{}\n{}",
        include_str!("../shadows/shade.wgsl"),
        include_str!("../lighting/shade.wgsl"),
        material
    )
}
fn entries() -> Vec<wgpu::BindGroupLayoutEntry> {
    (0..6)
        .map(|binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            count: None,
            ty: if binding == 0 {
                wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(64),
                }
            } else {
                wgpu::BindingType::Texture {
                    sample_type: if binding == 3 {
                        wgpu::TextureSampleType::Depth
                    } else {
                        wgpu::TextureSampleType::Float { filterable: false }
                    },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                }
            },
        })
        .collect()
}
impl System {
    fn new(r: &Renderer) -> Self {
        let d = &r.device;
        let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Experimental center material"),
            source: wgpu::ShaderSource::Wgsl(center_source().into()),
        });
        let stages = crate::shader::Stages {
            vertex: module.clone(),
            fragment: module,
            vertex_entry: "vertex",
            fragment_entry: "fragment",
        };
        let mut materials = HashMap::new();
        for alpha in [
            incant_assets_runtime::AlphaMode::Opaque,
            incant_assets_runtime::AlphaMode::Mask,
        ] {
            for double_sided in [false, true] {
                for mirrored in [false, true] {
                    let key = PipelineKey {
                        format: crate::frame::HDR_FORMAT,
                        alpha,
                        double_sided,
                        mirrored,
                        samples: crate::SceneMsaa::One,
                    };
                    materials.insert(
                        key,
                        crate::material_pipeline::build(
                            d,
                            &stages,
                            key,
                            &[
                                &r.materials.globals,
                                &r.materials.layout,
                                &r.environments.layout,
                                &r.lighting.layout,
                            ],
                            r.pipeline_cache.cache.as_ref(),
                        ),
                    );
                }
            }
        }
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("TAAU"),
            entries: &entries(),
        });
        let pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("TAAU"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let shader = d.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Experimental output color TAAU"),
            source: wgpu::ShaderSource::Wgsl(include_str!("resolve.wgsl").into()),
        });
        let targets = [crate::frame::HDR_FORMAT, wgpu::TextureFormat::Rg32Float].map(|format| {
            Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        });
        let resolve = d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Experimental output color TAAU"),
            layout: Some(&pipeline_layout),
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
                entry_point: Some("resolve"),
                compilation_options: Default::default(),
                targets: &targets,
            }),
            multiview_mask: None,
            cache: r.pipeline_cache.cache.as_ref(),
        });
        Self {
            materials,
            layout,
            resolve,
        }
    }
    pub fn material(&self, key: PipelineKey) -> Result<wgpu::RenderPipeline> {
        self.materials
            .get(&key)
            .cloned()
            .ok_or(crate::ResourceError::UnpreparedPipeline("center material TAAU").into())
    }
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        p: &Pending,
        current: &wgpu::TextureView,
        camera: crate::camera::CameraView,
    ) -> Draw {
        #[cfg(test)]
        if let Some(r) = &p.reconstruction {
            return r.prepare(device, p, current, camera);
        }
        let surfaces = p.input.surfaces.as_ref().expect("TAAU admitted motion");
        let jitter = crate::temporal::jitter::SAMPLES
            [p.input.frame.jitter_phase().expect("TAAU phase") as usize];
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("TAAU frame"),
            usage: wgpu::BufferUsages::UNIFORM,
            contents: bytemuck::bytes_of(&Frame {
                source: p.input.rect,
                output: [
                    p.targets.size[0] as f32,
                    p.targets.size[1] as f32,
                    f32::from(u8::from(p.stationary)),
                    0.,
                ],
                depth: [
                    camera.near,
                    camera.far,
                    f32::from(u8::from(camera.orthographic_half_height.is_some())),
                    f32::from(u8::from(p.input.frame.has_previous())),
                ],
                jitter: [
                    jitter[0],
                    jitter[1],
                    p.previous_jitter[0],
                    p.previous_jitter[1],
                ],
            }),
        });
        let views = [
            current,
            &surfaces.motion,
            &surfaces.current_depth,
            &p.targets.color[1 - surfaces.slot],
            &p.targets.support[1 - surfaces.slot],
        ];
        let mut entries = vec![wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }];
        entries.extend(
            views
                .into_iter()
                .enumerate()
                .map(|(i, view)| wgpu::BindGroupEntry {
                    binding: 1 + i as u32,
                    resource: wgpu::BindingResource::TextureView(view),
                }),
        );
        Draw {
            pipeline: self.resolve.clone(),
            group: device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("TAAU frame"),
                layout: &self.layout,
                entries: &entries,
            }),
            color: p.targets.color[surfaces.slot].clone(),
            support: p.targets.support[surfaces.slot].clone(),
            #[cfg(test)]
            identity: None,
            #[cfg(test)]
            visibility_before: false,
            #[cfg(test)]
            classification: None,
        }
    }
}
impl Draw {
    pub fn encode(
        &self,
        e: &mut wgpu::CommandEncoder,
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let attachment = |view| {
            Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })
        };
        let attachments = [&self.color, &self.support].map(attachment);
        #[cfg(test)]
        let attachments = {
            let mut attachments = Vec::from(attachments);
            if let Some(identity) = &self.identity {
                attachments.push(attachment(identity));
            }
            attachments
        };
        let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Reconstruct output HDR and current-depth support"),
            color_attachments: &attachments,
            timestamp_writes: timestamps,
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        pass.draw(0..3, 0..1);
    }
}
impl Renderer {
    /// Experimental startup opt-in. Cooked families are deliberately unsupported;
    /// this cannot silently fall back to parsing WGSL in a cooked shipping route.
    pub async fn enable_temporal_upscale(&mut self) -> Result<()> {
        if cfg!(any(
            feature = "cooked-materials",
            feature = "cooked-temporal"
        )) {
            return Err(
                crate::SceneError::Input("experimental TAAU has no cooked shader family").into(),
            );
        }
        if self.temporal_upscale.is_some() {
            return Ok(());
        }
        self.materials.require_eager()?;
        self.prepare_motion_vectors().await?;
        for format in [crate::frame::HDR_FORMAT, wgpu::TextureFormat::Rg32Float] {
            if !self
                .adapter
                .get_texture_format_features(format)
                .allowed_usages
                .contains(
                    wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
                )
            {
                return Err(crate::temporal::TemporalError::Format.into());
            }
        }
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let candidate = System::new(self);
        let grid = self.lighting.temporal_pipeline(
            &self.device,
            self.adapter.get_info().backend,
            self.pipeline_cache.cache.as_ref(),
        );
        if let Some(error) = scope.pop().await {
            #[cfg(target_arch = "wasm32")]
            let error = error.into();
            return Err(crate::ResourceError::Preparation(error).into());
        }
        let (candidate, grid) =
            self.complete_pipeline_preparation(grid.map(|grid| (candidate, grid)))?;
        self.lighting.accept_temporal(grid);
        self.temporal_upscale = Some(candidate);
        Ok(())
    }
}

#[cfg(test)]
#[path = "gpu_tests.rs"]
mod gpu_tests;

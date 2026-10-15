use super::shader::{self, Kind};
use super::{Fallback, MetalFxError, Pending, targets::Targets};
use std::sync::Arc;
use wgpu::util::DeviceExt;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Frame {
    source: [f32; 4],
    output: [f32; 4],
    depth: [f32; 4],
    options: [f32; 4],
    reactive: [f32; 4],
    jitter: [f32; 4],
}
const _: () = assert!(size_of::<Frame>() == 96);
const _: () = assert!(
    std::mem::offset_of!(Frame, source) == 0
        && std::mem::offset_of!(Frame, output) == 16
        && std::mem::offset_of!(Frame, depth) == 32
        && std::mem::offset_of!(Frame, options) == 48
        && std::mem::offset_of!(Frame, reactive) == 64
        && std::mem::offset_of!(Frame, jitter) == 80
);
pub(crate) struct System {
    layout: wgpu::BindGroupLayout,
    pack: wgpu::RenderPipeline,
    normalize: wgpu::RenderPipeline,
}
pub(crate) struct Draw {
    targets: Arc<Targets>,
    pack: wgpu::BindGroup,
    normalize: wgpu::BindGroup,
    pack_pipeline: wgpu::RenderPipeline,
    normalize_pipeline: wgpu::RenderPipeline,
}
#[cfg(test)]
use super::source::source;
impl crate::Renderer {
    /// Explicit startup-only compilation of fixed engine shaders. Native scaler
    /// compilation is synchronous and bounded to owner/extent preparation.
    pub async fn enable_metalfx(&mut self) -> crate::Result<()> {
        if self.metalfx_admission.is_some() {
            return Ok(());
        }
        let capability = match super::shader_admission() {
            Err(reason) => Err(reason),
            Ok(()) => {
                #[cfg(target_os = "macos")]
                {
                    super::native::range(&self.device)
                }
                #[cfg(not(target_os = "macos"))]
                {
                    Err(super::Fallback::Backend)
                }
            }
        };
        if super::shader_admission().is_ok() {
            self.enable_motion_vectors().await?;
        }
        if let Err(reason) = capability {
            self.metalfx_admission = Some(Err(reason));
            return Ok(());
        }
        for f in [
            crate::frame::HDR_FORMAT,
            wgpu::TextureFormat::R32Float,
            wgpu::TextureFormat::Rg16Float,
            wgpu::TextureFormat::R8Unorm,
            wgpu::TextureFormat::R16Float,
        ] {
            let usage = wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | if f == crate::frame::HDR_FORMAT {
                    wgpu::TextureUsages::STORAGE_BINDING
                } else {
                    wgpu::TextureUsages::empty()
                };
            if !self
                .adapter
                .get_texture_format_features(f)
                .allowed_usages
                .contains(usage)
            {
                self.metalfx_admission = Some(Err(Fallback::Format));
                return Ok(());
            }
        }
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let system = System::new(
            &self.device,
            self.adapter.get_info().backend,
            self.pipeline_cache.cache.as_ref(),
        );
        if let Some(error) = scope.pop().await {
            #[cfg(target_arch = "wasm32")]
            let error = error.into();
            return Err(crate::ResourceError::Preparation(error).into());
        }
        self.metalfx = Some(self.complete_pipeline_preparation(system)?);
        self.metalfx_admission = Some(capability);
        Ok(())
    }
}
pub(super) fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 4] {
    std::array::from_fn(|binding| wgpu::BindGroupLayoutEntry {
        binding: binding as u32,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: if binding == 0 {
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(size_of::<Frame>() as u64),
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
        count: None,
    })
}
pub(super) fn formats(kind: Kind) -> Vec<wgpu::TextureFormat> {
    match kind {
        Kind::Pack => vec![
            crate::frame::HDR_FORMAT,
            wgpu::TextureFormat::R32Float,
            wgpu::TextureFormat::Rg16Float,
            wgpu::TextureFormat::R8Unorm,
        ],
        Kind::Normalize => vec![crate::frame::HDR_FORMAT],
    }
}
impl System {
    fn new(
        d: &wgpu::Device,
        backend: wgpu::Backend,
        cache: Option<&wgpu::PipelineCache>,
    ) -> crate::Result<Self> {
        let pack = shader::stages(d, backend, Kind::Pack)?;
        let normalize = shader::stages(d, backend, Kind::Normalize)?;
        let entries = layout_entries();
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MetalFX boundary"),
            entries: &entries,
        });
        let pipeline = |kind, stages: crate::shader::Stages| {
            let pipeline_layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("MetalFX boundary"),
                bind_group_layouts: &[Some(&layout)],
                immediate_size: 0,
            });
            let targets = formats(kind)
                .into_iter()
                .map(|format| {
                    Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })
                })
                .collect::<Vec<_>>();
            d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("MetalFX boundary"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &stages.vertex,
                    entry_point: Some(stages.vertex_entry),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: Default::default(),
                depth_stencil: None,
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &stages.fragment,
                    entry_point: Some(stages.fragment_entry),
                    compilation_options: Default::default(),
                    targets: &targets,
                }),
                multiview_mask: None,
                cache,
            })
        };
        Ok(Self {
            pack: pipeline(Kind::Pack, pack),
            normalize: pipeline(Kind::Normalize, normalize),
            layout,
        })
    }
    pub fn prepare(
        &self,
        d: &wgpu::Device,
        p: &Pending,
        current: &wgpu::TextureView,
        motion: &crate::motion::MotionDraw,
        camera: crate::camera::CameraView,
    ) -> Result<Draw, MetalFxError> {
        let plan = p.plan.ok_or(MetalFxError::Interop)?;
        let targets = Arc::clone(p.targets.as_ref().ok_or(MetalFxError::Interop)?);
        let (motion, depth) = motion.native_views();
        let f = Frame {
            source: plan.source.map(|x| x as f32),
            output: [0., 0., plan.output[2] as f32, plan.output[3] as f32],
            depth: [
                camera.near,
                camera.far,
                f32::from(u8::from(camera.orthographic_half_height.is_some())),
                f32::from(u8::from(p.has_previous)),
            ],
            options: [0., p.config.relative_depth, p.config.absolute_depth, 0.],
            reactive: [0.; 4],
            jitter: [
                p.jitter[0] / plan.source[2] as f32,
                p.jitter[1] / plan.source[3] as f32,
                p.previous_jitter[0] / plan.source[2] as f32,
                p.previous_jitter[1] / plan.source[3] as f32,
            ],
        };
        let frame = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MetalFX exact coordinates"),
            contents: bytemuck::bytes_of(&f),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = |views: [&wgpu::TextureView; 3]| {
            d.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("MetalFX frame"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: frame.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(views[0]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(views[1]),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(views[2]),
                    },
                ],
            })
        };
        Ok(Draw {
            pack: group([current, motion, depth]),
            normalize: group([current, &targets.views[4], depth]),
            targets,
            pack_pipeline: self.pack.clone(),
            normalize_pipeline: self.normalize.clone(),
        })
    }
}
fn attachment(
    view: &wgpu::TextureView,
    color: wgpu::Color,
) -> Option<wgpu::RenderPassColorAttachment<'_>> {
    Some(wgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        depth_slice: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(color),
            store: wgpu::StoreOp::Store,
        },
    })
}
impl Draw {
    pub fn pack(
        &self,
        e: &mut wgpu::CommandEncoder,
        t: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let attachments = self.targets.views[..4]
            .iter()
            .map(|v| attachment(v, wgpu::Color::TRANSPARENT))
            .collect::<Vec<_>>();
        let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Pack MetalFX inputs"),
            color_attachments: &attachments,
            timestamp_writes: t,
            ..Default::default()
        });
        pass.set_pipeline(&self.pack_pipeline);
        pass.set_bind_group(0, &self.pack, &[]);
        pass.draw(0..3, 0..1);
    }
    pub fn clear(
        &self,
        e: &mut wgpu::CommandEncoder,
        i: usize,
        t: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        drop(e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Initialize native output/exposure"),
            color_attachments: &[attachment(
                &self.targets.views[i],
                if i == 5 {
                    wgpu::Color::WHITE
                } else {
                    wgpu::Color::TRANSPARENT
                },
            )],
            timestamp_writes: t,
            ..Default::default()
        }));
    }
    pub fn transition_before(&self, e: &mut wgpu::CommandEncoder) {
        e.transition_resources(
            std::iter::empty(),
            self.targets
                .textures
                .iter()
                .enumerate()
                .map(|(i, texture)| wgpu::TextureTransition {
                    texture,
                    selector: None,
                    state: if i == 4 {
                        wgpu::TextureUses::STORAGE_WRITE_ONLY
                    } else {
                        wgpu::TextureUses::RESOURCE
                    },
                }),
        );
    }
    pub fn normalize(
        &self,
        e: &mut wgpu::CommandEncoder,
        t: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        e.transition_resources(
            std::iter::empty(),
            std::iter::once(wgpu::TextureTransition {
                texture: &self.targets.textures[4],
                selector: None,
                state: wgpu::TextureUses::RESOURCE,
            }),
        );
        let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Normalize native current coverage"),
            color_attachments: &[attachment(
                &self.targets.normalized,
                wgpu::Color::TRANSPARENT,
            )],
            timestamp_writes: t,
            ..Default::default()
        });
        pass.set_pipeline(&self.normalize_pipeline);
        pass.set_bind_group(0, &self.normalize, &[]);
        pass.draw(0..3, 0..1);
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn actual_fixed_boundary_shaders_validate() {
        for normalize in [false, true] {
            let module = naga::front::wgsl::parse_str(&super::source(normalize)).unwrap();
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&module)
            .unwrap();
        }
    }
}

#[cfg(test)]
#[path = "boundary_tests.rs"]
mod boundary_tests;

use super::{MetalFxError, Pending, targets::Targets};
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
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
}
#[cfg(test)]
const COMMON: &str = include_str!("../upscaling/common.wgsl");
impl crate::Renderer {
    /// Explicit startup preparation. Native scaler creation remains bound to a
    /// retained owner/output extent; unsupported devices use the selected policy.
    pub async fn enable_metalfx(&mut self) -> crate::Result<()> {
        self.enable_temporal_upscaling().await?;
        if self.metalfx_admission.is_some() {
            return Ok(());
        }
        #[cfg(any(target_os = "macos", target_os = "ios"))]
        let capability = super::native::range(&self.device).map(|_| ());
        #[cfg(not(any(target_os = "macos", target_os = "ios")))]
        let capability = Err(super::Fallback::Backend);
        if capability.is_err() {
            self.metalfx_admission = Some(capability);
            return Ok(());
        }
        for f in [
            crate::frame::HDR_FORMAT,
            wgpu::TextureFormat::R32Float,
            wgpu::TextureFormat::Rg16Float,
            wgpu::TextureFormat::R8Unorm,
            wgpu::TextureFormat::R16Float,
        ] {
            let use_ = wgpu::TextureUsages::TEXTURE_BINDING
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
                .contains(use_)
            {
                self.metalfx_admission = Some(Err(super::Fallback::Format));
                return Ok(());
            }
        }
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let candidate = System::new(&self.device, self.adapter.get_info().backend);
        if let Some(e) = scope.pop().await {
            #[cfg(target_arch = "wasm32")]
            let e = e.into();
            return Err(crate::ResourceError::Preparation(e).into());
        }
        self.metalfx = Some(candidate?);
        self.metalfx_admission = Some(Ok(()));
        Ok(())
    }
}
fn shader(
    d: &wgpu::Device,
    l: &wgpu::BindGroupLayout,
    backend: wgpu::Backend,
    kind: crate::temporal_shader::Kind,
    formats: &[wgpu::TextureFormat],
) -> crate::Result<wgpu::RenderPipeline> {
    let stages = crate::temporal_shader::stages(d, backend, kind)?;
    let layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("MetalFX packing"),
        bind_group_layouts: &[Some(l)],
        immediate_size: 0,
    });
    let targets = formats
        .iter()
        .map(|f| {
            Some(wgpu::ColorTargetState {
                format: *f,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })
        })
        .collect::<Vec<_>>();
    Ok(d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("MetalFX packing/coverage"),
        layout: Some(&layout),
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
        cache: None,
    }))
}
fn group(
    d: &wgpu::Device,
    l: &wgpu::BindGroupLayout,
    frame: &wgpu::Buffer,
    views: [&wgpu::TextureView; 3],
) -> wgpu::BindGroup {
    d.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("MetalFX frame"),
        layout: l,
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
}
pub(crate) fn layout_entries() -> [wgpu::BindGroupLayoutEntry; 4] {
    std::array::from_fn::<_, 4, _>(|i| wgpu::BindGroupLayoutEntry {
        binding: i as u32,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: if i == 0 {
            wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(96),
            }
        } else {
            wgpu::BindingType::Texture {
                sample_type: if i == 3 {
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
impl System {
    pub fn new(d: &wgpu::Device, backend: wgpu::Backend) -> crate::Result<Self> {
        let entries = layout_entries();
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("MetalFX input declarations"),
            entries: &entries,
        });
        let pack = shader(
            d,
            &layout,
            backend,
            crate::temporal_shader::Kind::MetalFxPack,
            &[
                crate::frame::HDR_FORMAT,
                wgpu::TextureFormat::R32Float,
                wgpu::TextureFormat::Rg16Float,
                wgpu::TextureFormat::R8Unorm,
            ],
        )?;
        let normalize = shader(
            d,
            &layout,
            backend,
            crate::temporal_shader::Kind::MetalFxNormalize,
            &[crate::frame::HDR_FORMAT, wgpu::TextureFormat::R32Float],
        )?;
        Ok(Self {
            layout,
            pack,
            normalize,
        })
    }
    pub fn prepare(
        &self,
        d: &wgpu::Device,
        n: &Pending,
        p: &crate::upscaling::Pending,
        current: &wgpu::TextureView,
        camera: crate::camera::CameraView,
    ) -> Result<Draw, MetalFxError> {
        let (i, plan, color, outdepth) = p.native_input().ok_or(MetalFxError::Interop)?;
        let (motion, depth) = i.surfaces.as_ref().ok_or(MetalFxError::Interop)?;
        let targets = Arc::clone(n.targets.as_ref().ok_or(MetalFxError::Interop)?);
        let c = p.native_config();
        let f = Frame {
            source: plan.source_rect,
            output: plan.rect,
            depth: [
                camera.near,
                camera.far,
                f32::from(u8::from(camera.orthographic_half_height.is_some())),
                f32::from(u8::from(i.frame.has_previous())),
            ],
            options: [
                c.history_weight,
                c.relative_depth,
                c.absolute_depth,
                c.reactive_threshold,
            ],
            reactive: [c.reactive_strength, 0., 0., 0.],
            jitter: [
                i.jitter[0],
                i.jitter[1],
                i.previous_jitter[0],
                i.previous_jitter[1],
            ],
        };
        let frame = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("MetalFX exact frame coordinates"),
            contents: bytemuck::bytes_of(&f),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        Ok(Draw {
            pack: group(d, &self.layout, &frame, [current, motion, depth]),
            normalize: group(d, &self.layout, &frame, [current, &targets.views[4], depth]),
            targets,
            pack_pipeline: self.pack.clone(),
            normalize_pipeline: self.normalize.clone(),
            color: color.clone(),
            depth: outdepth.clone(),
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
        let mut p = e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Pack MetalFX inputs"),
            color_attachments: &self.targets.views[..4]
                .iter()
                .map(|v| attachment(v, wgpu::Color::TRANSPARENT))
                .collect::<Vec<_>>(),
            timestamp_writes: t,
            ..Default::default()
        });
        p.set_pipeline(&self.pack_pipeline);
        p.set_bind_group(0, &self.pack, &[]);
        p.draw(0..3, 0..1);
    }
    pub fn clear(
        &self,
        e: &mut wgpu::CommandEncoder,
        i: usize,
        t: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        drop(e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Initialize native-only output/exposure"),
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
                .map(|(i, t)| wgpu::TextureTransition {
                    texture: t,
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
        let mut p = e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Normalize native HDR current coverage"),
            color_attachments: &[
                attachment(&self.color, wgpu::Color::TRANSPARENT),
                attachment(&self.depth, wgpu::Color::TRANSPARENT),
            ],
            timestamp_writes: t,
            ..Default::default()
        });
        p.set_pipeline(&self.normalize_pipeline);
        p.set_bind_group(0, &self.normalize, &[]);
        p.draw(0..3, 0..1);
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn native_boundary_wgsl_validates_without_gpu() {
        for body in [include_str!("prepare.wgsl"), include_str!("composite.wgsl")] {
            let module =
                naga::front::wgsl::parse_str(&format!("{}\n{body}", super::COMMON)).unwrap();
            naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::empty(),
            )
            .validate(&module)
            .unwrap();
        }
    }
}

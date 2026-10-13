use super::{Pending, TemporalError};
use crate::{Renderer, Result};
use wgpu::util::DeviceExt;
pub(crate) struct TemporalSystem {
    layout: wgpu::BindGroupLayout,
    copy_layout: wgpu::BindGroupLayout,
    resolve: wgpu::RenderPipeline,
    copy: wgpu::RenderPipeline,
}
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Frame {
    rect: [f32; 4],
    depth: [f32; 4],
    options: [f32; 4],
    reactive: [f32; 4],
}
const _: () = assert!(size_of::<Frame>() == 64);
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
pub(crate) fn layout_entries(copy: bool) -> Vec<wgpu::BindGroupLayoutEntry> {
    if copy {
        return vec![
            texture(0, false),
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(64),
                },
                count: None,
            },
        ];
    }
    vec![
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(64),
            },
            count: None,
        },
        texture(1, false),
        texture(2, false),
        texture(3, true),
        texture(4, false),
    ]
}
fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    backend: wgpu::Backend,
    kind: crate::temporal_shader::Kind,
) -> Result<wgpu::RenderPipeline> {
    let shader = crate::temporal_shader::stages(device, backend, kind)?;
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Opaque temporal HDR"),
        bind_group_layouts: &[Some(layout)],
        immediate_size: 0,
    });
    Ok(
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Opaque temporal HDR"),
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
                    format: crate::frame::HDR_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        }),
    )
}
impl TemporalSystem {
    fn new(device: &wgpu::Device, backend: wgpu::Backend) -> Result<Self> {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Temporal resolve"),
            entries: &layout_entries(false),
        });
        let copy_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Temporal copy"),
            entries: &layout_entries(true),
        });
        Ok(Self {
            resolve: pipeline(
                device,
                &layout,
                backend,
                crate::temporal_shader::Kind::Resolve,
            )?,
            copy: pipeline(
                device,
                &copy_layout,
                backend,
                crate::temporal_shader::Kind::Copy,
            )?,
            layout,
            copy_layout,
        })
    }
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        pending: &Pending,
        current: &wgpu::TextureView,
        camera: crate::camera::CameraView,
    ) -> Option<TemporalDraw> {
        let targets = pending.targets.as_ref()?;
        let surfaces = pending
            .input
            .surfaces
            .as_ref()
            .expect("admitted temporal motion surfaces");
        let c = pending.config;
        let jitter = pending
            .input
            .frame
            .jitter_phase()
            .map_or([0.; 2], |phase| super::jitter::SAMPLES[phase as usize]);
        let uniform = Frame {
            rect: pending.input.rect,
            depth: [
                camera.near,
                camera.far,
                f32::from(u8::from(camera.orthographic_half_height.is_some())),
                f32::from(u8::from(pending.input.frame.has_previous())),
            ],
            options: [
                c.history_weight,
                c.relative_depth,
                c.absolute_depth,
                c.reactive_threshold,
            ],
            reactive: [c.reactive_strength, jitter[0], jitter[1], 0.],
        };
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Temporal resolve frame"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let history = &targets.color[surfaces.slot];
        let previous = &targets.color[1 - surfaces.slot];
        let resolve = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporal resolve frame"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(current),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&surfaces.motion),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&surfaces.previous_depth),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(previous),
                },
            ],
        });
        let copy = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Temporal copy frame"),
            layout: &self.copy_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(history),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform.as_entire_binding(),
                },
            ],
        });
        Some(TemporalDraw {
            resolve,
            copy,
            resolve_pipeline: self.resolve.clone(),
            copy_pipeline: self.copy.clone(),
            history: history.clone(),
            current: current.clone(),
        })
    }
}
pub(crate) struct TemporalDraw {
    resolve: wgpu::BindGroup,
    copy: wgpu::BindGroup,
    resolve_pipeline: wgpu::RenderPipeline,
    copy_pipeline: wgpu::RenderPipeline,
    history: wgpu::TextureView,
    current: wgpu::TextureView,
}
impl TemporalDraw {
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        resolve_time: Option<wgpu::RenderPassTimestampWrites<'_>>,
        copy_time: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        for (label, target, pipeline, group, timestamps) in [
            (
                "Accumulate opaque HDR",
                &self.history,
                &self.resolve_pipeline,
                &self.resolve,
                resolve_time,
            ),
            (
                "Restore resolved HDR before excluded surfaces",
                &self.current,
                &self.copy_pipeline,
                &self.copy,
                copy_time,
            ),
        ] {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                timestamp_writes: timestamps,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            pass.draw(0..3, 0..1);
        }
    }
}
impl Renderer {
    /// Startup-only portable pipelines; no temporal owner or visual policy is
    /// enabled until the caller supplies a validated TemporalConfig separately.
    pub async fn enable_temporal_accumulation(&mut self) -> Result<()> {
        crate::motion::admission()?;
        if self.temporal.is_some() {
            return Ok(());
        }
        self.enable_motion_vectors().await?;
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC;
        if !self
            .adapter
            .get_texture_format_features(crate::frame::HDR_FORMAT)
            .allowed_usages
            .contains(usage)
        {
            return Err(TemporalError::Format.into());
        }
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let candidate = TemporalSystem::new(&self.device, self.adapter.get_info().backend);
        let grid = self
            .lighting
            .temporal_pipeline(&self.device, self.adapter.get_info().backend);
        if let Some(error) = scope.pop().await {
            #[cfg(target_arch = "wasm32")]
            let error = error.into();
            return Err(crate::ResourceError::Preparation(error).into());
        }
        let candidate = candidate?;
        self.lighting.accept_temporal(grid?);
        self.temporal = Some(candidate);
        Ok(())
    }
}

#[cfg(test)]
#[path = "device_tests.rs"]
mod device_tests;

#[cfg(test)]
#[path = "coverage_device.rs"]
mod coverage_device;

use super::{Pending, UpscaleError};
use crate::{Renderer, Result};
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
    resolve_layout: wgpu::BindGroupLayout,
    composite_layout: wgpu::BindGroupLayout,
    resolve: wgpu::RenderPipeline,
    composite: wgpu::RenderPipeline,
}
fn layout(d: &wgpu::Device, count: u32) -> wgpu::BindGroupLayout {
    let mut entries = vec![wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(96),
        },
        count: None,
    }];
    entries.extend((1..=count).map(|binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: if binding == 3 {
                wgpu::TextureSampleType::Depth
            } else {
                wgpu::TextureSampleType::Float { filterable: false }
            },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }));
    d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Temporal upscaling inputs"),
        entries: &entries,
    })
}
fn pipeline(
    d: &wgpu::Device,
    l: &wgpu::BindGroupLayout,
    body: &str,
    entry: &str,
    depth: bool,
) -> wgpu::RenderPipeline {
    let module = d.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Output temporal reconstruction"),
        source: wgpu::ShaderSource::Wgsl(format!("{}\n{body}", include_str!("common.wgsl")).into()),
    });
    let layout = d.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Output temporal reconstruction"),
        bind_group_layouts: &[Some(l)],
        immediate_size: 0,
    });
    let color = |format| {
        Some(wgpu::ColorTargetState {
            format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })
    };
    let mut targets = vec![color(crate::frame::HDR_FORMAT)];
    if depth {
        targets.push(color(wgpu::TextureFormat::R32Float));
    }
    d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Output temporal reconstruction"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        primitive: Default::default(),
        depth_stencil: None,
        multisample: Default::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some(entry),
            compilation_options: Default::default(),
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    })
}
fn group(
    d: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    frame: &wgpu::Buffer,
    textures: &[&wgpu::TextureView],
) -> wgpu::BindGroup {
    let mut entries = vec![wgpu::BindGroupEntry {
        binding: 0,
        resource: frame.as_entire_binding(),
    }];
    entries.extend(
        textures
            .iter()
            .enumerate()
            .map(|(i, t)| wgpu::BindGroupEntry {
                binding: i as u32 + 1,
                resource: wgpu::BindingResource::TextureView(t),
            }),
    );
    d.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Output reconstruction frame"),
        layout,
        entries: &entries,
    })
}
impl System {
    fn new(d: &wgpu::Device) -> Self {
        let resolve_layout = layout(d, 5);
        let composite_layout = layout(d, 2);
        Self {
            resolve: pipeline(
                d,
                &resolve_layout,
                include_str!("resolve.wgsl"),
                "resolve",
                true,
            ),
            composite: pipeline(
                d,
                &composite_layout,
                include_str!("composite.wgsl"),
                "composite",
                false,
            ),
            resolve_layout,
            composite_layout,
        }
    }
    pub fn prepare(
        &self,
        d: &wgpu::Device,
        p: &Pending,
        current: &wgpu::TextureView,
        camera: crate::camera::CameraView,
    ) -> Option<Draw> {
        let plan = p.plan?;
        let output = p.output.as_ref()?;
        let overlay = p.overlay.as_ref()?;
        let (motion, depth) = p.input.surfaces.as_ref()?;
        let c = p.config;
        let f = Frame {
            source: plan.source_rect,
            output: plan.rect,
            depth: [
                camera.near,
                camera.far,
                f32::from(u8::from(camera.orthographic_half_height.is_some())),
                f32::from(u8::from(p.input.frame.has_previous())),
            ],
            options: [
                c.history_weight,
                c.relative_depth,
                c.absolute_depth,
                c.reactive_threshold,
            ],
            reactive: [c.reactive_strength, 0., 0., 0.],
            jitter: [
                p.input.jitter[0],
                p.input.jitter[1],
                p.input.previous_jitter[0],
                p.input.previous_jitter[1],
            ],
        };
        let uniform = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Output reconstruction frame"),
            contents: bytemuck::bytes_of(&f),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let slot = p.slot();
        Some(Draw {
            resolve: group(
                d,
                &self.resolve_layout,
                &uniform,
                &[
                    current,
                    motion,
                    depth,
                    &output.color[1 - slot],
                    &output.depth[1 - slot],
                ],
            ),
            composite: group(
                d,
                &self.composite_layout,
                &uniform,
                &[&output.color[slot], &overlay.view],
            ),
            resolve_pipeline: self.resolve.clone(),
            composite_pipeline: self.composite.clone(),
            color: output.color[slot].clone(),
            depth: output.depth[slot].clone(),
            target: output.composite.clone(),
            overlay: overlay.view.clone(),
        })
    }
}
pub(crate) struct Draw {
    resolve: wgpu::BindGroup,
    composite: wgpu::BindGroup,
    resolve_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    target: wgpu::TextureView,
    overlay: wgpu::TextureView,
}
fn attachment(view: &wgpu::TextureView) -> Option<wgpu::RenderPassColorAttachment<'_>> {
    Some(wgpu::RenderPassColorAttachment {
        view,
        resolve_target: None,
        depth_slice: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
            store: wgpu::StoreOp::Store,
        },
    })
}
impl Draw {
    pub fn resolve(
        &self,
        e: &mut wgpu::CommandEncoder,
        t: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Reconstruct output HDR and depth"),
            color_attachments: &[attachment(&self.color), attachment(&self.depth)],
            timestamp_writes: t,
            ..Default::default()
        });
        pass.set_pipeline(&self.resolve_pipeline);
        pass.set_bind_group(0, &self.resolve, &[]);
        pass.draw(0..3, 0..1);
    }
    pub fn clear_overlay(
        &self,
        e: &mut wgpu::CommandEncoder,
        t: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        // Clear color only: later overlays load the original opaque depth.
        drop(e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear current-only overlay"),
            color_attachments: &[attachment(&self.overlay)],
            timestamp_writes: t,
            ..Default::default()
        }));
    }
    pub fn composite(
        &self,
        e: &mut wgpu::CommandEncoder,
        t: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Composite current-only overlay at output resolution"),
            color_attachments: &[attachment(&self.target)],
            timestamp_writes: t,
            ..Default::default()
        });
        pass.set_pipeline(&self.composite_pipeline);
        pass.set_bind_group(0, &self.composite, &[]);
        pass.draw(0..3, 0..1);
    }
}
impl Renderer {
    /// Startup-only opt-in. No owner, scale or visual policy is selected here.
    pub async fn enable_temporal_upscaling(&mut self) -> Result<()> {
        crate::motion::admission()?;
        if self.upscaling.is_some() {
            return Ok(());
        }
        self.enable_temporal_accumulation().await?;
        let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::TEXTURE_BINDING
            | wgpu::TextureUsages::COPY_SRC;
        if !self
            .adapter
            .get_texture_format_features(wgpu::TextureFormat::R32Float)
            .allowed_usages
            .contains(usage)
        {
            return Err(UpscaleError::Format.into());
        }
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let candidate = System::new(&self.device);
        if let Some(error) = scope.pop().await {
            #[cfg(target_arch = "wasm32")]
            let error = error.into();
            return Err(crate::ResourceError::Preparation(error).into());
        }
        self.upscaling = Some(candidate);
        Ok(())
    }
}
const _: () = assert!(
    size_of::<super::State>()
        + size_of::<Pending>()
        + size_of::<Draw>()
        + 2 * size_of::<super::targets::Output>()
        + 2 * size_of::<super::targets::Overlay>()
        + 256
        < 8192
);

#[cfg(test)]
#[path = "device_tests.rs"]
mod device_tests;

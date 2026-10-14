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
    pub(crate) owner: Option<OwnerSystem>,
}
pub(crate) struct OwnerSystem {
    layout: wgpu::BindGroupLayout,
    resolve: wgpu::RenderPipeline,
}
pub(crate) fn layout_entries(count: u32) -> Vec<wgpu::BindGroupLayoutEntry> {
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
    entries
}
pub(crate) fn owner_layout_entries() -> Vec<wgpu::BindGroupLayoutEntry> {
    // 1-6 as the ordinary resolve (6 = previous RG16F evidence), 7 owner IDs,
    // 8 previous records, 9 the owner-change uniform.
    let mut entries = layout_entries(6);
    entries.extend((7..=8).map(|binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Uint,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }));
    entries.push(wgpu::BindGroupLayoutEntry {
        binding: 9,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: wgpu::BufferSize::new(12_528),
        },
        count: None,
    });
    entries
}
impl OwnerSystem {
    pub(crate) fn new(d: &wgpu::Device, backend: wgpu::Backend) -> Result<Self> {
        let layout = d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Owner-aware output history"),
            entries: &owner_layout_entries(),
        });
        let resolve = pipeline(
            d,
            &layout,
            backend,
            crate::temporal_shader::Kind::UpscaleOwner,
            true,
        )?;
        Ok(Self { layout, resolve })
    }
}
pub(crate) fn check_owner_format(adapter: &wgpu::Adapter) -> Result<()> {
    let usage = wgpu::TextureUsages::RENDER_ATTACHMENT
        | wgpu::TextureUsages::TEXTURE_BINDING
        | wgpu::TextureUsages::COPY_SRC;
    if !adapter
        .get_texture_format_features(wgpu::TextureFormat::Rg32Uint)
        .allowed_usages
        .contains(usage)
    {
        return Err(UpscaleError::Format.into());
    }
    Ok(())
}
fn layout(d: &wgpu::Device, count: u32) -> wgpu::BindGroupLayout {
    d.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Temporal upscaling inputs"),
        entries: &layout_entries(count),
    })
}
fn pipeline(
    d: &wgpu::Device,
    l: &wgpu::BindGroupLayout,
    backend: wgpu::Backend,
    kind: crate::temporal_shader::Kind,
    depth: bool,
) -> Result<wgpu::RenderPipeline> {
    let shader = crate::temporal_shader::stages(d, backend, kind)?;
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
    if kind == crate::temporal_shader::Kind::UpscaleOwner {
        targets.push(color(wgpu::TextureFormat::Rg32Uint));
    }
    // RG16Float evidence (mass, age) last: MRT 8+4+4 = 16 bytes ordinary,
    // 8+4+8+4 = 24 bytes owner.
    if matches!(
        kind,
        crate::temporal_shader::Kind::Upscale | crate::temporal_shader::Kind::UpscaleOwner
    ) {
        targets.push(color(wgpu::TextureFormat::Rg16Float));
    }
    Ok(d.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Output temporal reconstruction"),
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
            targets: &targets,
        }),
        multiview_mask: None,
        cache: None,
    }))
}
fn group(
    d: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    frame: &wgpu::Buffer,
    textures: &[&wgpu::TextureView],
    changes: Option<&wgpu::Buffer>,
) -> wgpu::BindGroup {
    let count = 1 + textures.len() + usize::from(changes.is_some());
    assert!(count <= 10);
    let entries: [wgpu::BindGroupEntry<'_>; 10] = std::array::from_fn(|i| {
        if i == 0 || i >= count {
            wgpu::BindGroupEntry {
                binding: 0,
                resource: frame.as_entire_binding(),
            }
        } else if i <= textures.len() {
            wgpu::BindGroupEntry {
                binding: i as u32,
                resource: wgpu::BindingResource::TextureView(textures[i - 1]),
            }
        } else {
            wgpu::BindGroupEntry {
                binding: textures.len() as u32 + 1,
                resource: changes.expect("owner upload").as_entire_binding(),
            }
        }
    });
    d.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Output reconstruction frame"),
        layout,
        entries: &entries[..count],
    })
}
impl System {
    fn new(d: &wgpu::Device, backend: wgpu::Backend, owner: bool) -> Result<Self> {
        let resolve_layout = layout(d, 6);
        let composite_layout = layout(d, 2);
        Ok(Self {
            owner: owner.then(|| OwnerSystem::new(d, backend)).transpose()?,
            resolve: pipeline(
                d,
                &resolve_layout,
                backend,
                crate::temporal_shader::Kind::Upscale,
                true,
            )?,
            composite: pipeline(
                d,
                &composite_layout,
                backend,
                crate::temporal_shader::Kind::Composite,
                false,
            )?,
            resolve_layout,
            composite_layout,
        })
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
            // y: global mass (owner program only); z: source-rect area density for
            // stored evidence; w: previous evidence written by this program for the
            // accepted slot of this output allocation.
            reactive: [
                c.reactive_strength,
                p.history_mass,
                p.density,
                f32::from(u8::from(p.evidence_valid)),
            ],
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
        // The overlay now has the output extent: with source == output the unchanged
        // composite.wgsl reads it 1:1 (base = p - 0.5, f = 0) and keeps clean() on the
        // overlay and on the result.
        let composite_uniform = d.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Output-resolution overlay composite frame"),
            contents: bytemuck::bytes_of(&Frame {
                source: plan.rect,
                ..f
            }),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let slot = p.slot();
        let mut textures = [
            current,
            motion,
            depth,
            &output.color[1 - slot],
            &output.depth[1 - slot],
            &output.evidence[1 - slot],
            current,
            current,
        ];
        let (layout, pipeline, changes, count) = if let Some((owner, changes)) = &p.input.owner {
            let system = self
                .owner
                .as_ref()
                .expect("owner pipelines admitted before retained opt-in");
            let records = output.record.as_ref().expect("owner output admitted");
            textures[6] = owner;
            textures[7] = &records[1 - slot];
            (&system.layout, &system.resolve, Some(changes), 8)
        } else {
            (&self.resolve_layout, &self.resolve, None, 6)
        };
        Some(Draw {
            resolve: group(d, layout, &uniform, &textures[..count], changes),
            composite: group(
                d,
                &self.composite_layout,
                &composite_uniform,
                &[&output.color[slot], &overlay.view],
                None,
            ),
            resolve_pipeline: pipeline.clone(),
            composite_pipeline: self.composite.clone(),
            record: output.record.as_ref().map(|r| r[slot].clone()),
            evidence: output.evidence[slot].clone(),
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
    record: Option<wgpu::TextureView>,
    /// RG16Float (mass, age) written by both resolve programs.
    evidence: wgpu::TextureView,
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
        let attachments = match &self.record {
            Some(record) => [
                attachment(&self.color),
                attachment(&self.depth),
                attachment(record),
                attachment(&self.evidence),
            ],
            None => [
                attachment(&self.color),
                attachment(&self.depth),
                attachment(&self.evidence),
                None,
            ],
        };
        let count = if self.record.is_some() { 4 } else { 3 };
        let mut pass = e.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Reconstruct output HDR and depth"),
            color_attachments: &attachments[..count],
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
        for format in [
            wgpu::TextureFormat::R32Float,
            wgpu::TextureFormat::Rg16Float,
        ] {
            if !self
                .adapter
                .get_texture_format_features(format)
                .allowed_usages
                .contains(usage)
            {
                return Err(UpscaleError::Format.into());
            }
        }
        let owner = self.motion.as_ref().is_some_and(|m| m.owner_enabled());
        if owner {
            check_owner_format(&self.adapter)?;
        }
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let candidate = System::new(&self.device, self.adapter.get_info().backend, owner);
        if let Some(error) = scope.pop().await {
            #[cfg(target_arch = "wasm32")]
            let error = error.into();
            return Err(crate::ResourceError::Preparation(error).into());
        }
        self.upscaling = Some(candidate?);
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
#[path = "evidence_reference.rs"]
mod evidence_reference;

#[cfg(test)]
#[path = "device_tests.rs"]
mod device_tests;

#[cfg(test)]
#[path = "owner_device_tests.rs"]
mod owner_device_tests;

#[cfg(test)]
#[path = "opaque_feedback_tests.rs"]
mod opaque_feedback_tests;

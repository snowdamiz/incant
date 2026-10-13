//! Scene-linear HDR to display-linear output, then exactly one sRGB transfer.
use crate::{ResourceError, Result, Viewport};
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
use wgpu::util::DeviceExt;
#[cfg(all(feature = "cooked-output", not(target_arch = "wasm32")))]
mod cooked;
#[cfg(all(feature = "cooked-output", not(target_arch = "wasm32")))]
mod layout;

pub(crate) fn required_features(_adapter: &wgpu::Adapter) -> Result<wgpu::Features> {
    #[cfg(all(feature = "cooked-output", not(target_arch = "wasm32")))]
    return cooked::features(_adapter.get_info().backend, _adapter.features());
    #[cfg(not(all(feature = "cooked-output", not(target_arch = "wasm32"))))]
    Ok(wgpu::Features::empty())
}
use crate::shader::Stages;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OutputUniform {
    backdrop: [f32; 4],
    rect: [f32; 4],
    radii: [f32; 4],
    canvas: [f32; 4],
    pixel_grid: [f32; 4],
}
fn decode(byte: u8) -> f32 {
    let s = f32::from(byte) / 255.;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}
pub(crate) struct OutputPass(HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>);
impl OutputPass {
    pub fn new(
        device: &wgpu::Device,
        backend: wgpu::Backend,
        cache: Option<&wgpu::PipelineCache>,
    ) -> Result<Self> {
        let shader = shader(device, backend)?;
        #[cfg(all(feature = "cooked-output", not(target_arch = "wasm32")))]
        let layout = Some(layout::create(device));
        #[cfg(not(all(feature = "cooked-output", not(target_arch = "wasm32"))))]
        let layout: Option<wgpu::PipelineLayout> = None;
        Ok(Self(
            [
                wgpu::TextureFormat::Rgba8Unorm,
                wgpu::TextureFormat::Rgba8UnormSrgb,
                wgpu::TextureFormat::Bgra8Unorm,
                wgpu::TextureFormat::Bgra8UnormSrgb,
            ]
            .into_iter()
            .map(|format| {
                (
                    format,
                    build(device, &shader, layout.as_ref(), format, cache),
                )
            })
            .collect(),
        ))
    }
    pub fn validate(format: wgpu::TextureFormat) -> Result<()> {
        match format {
            wgpu::TextureFormat::Rgba8Unorm
            | wgpu::TextureFormat::Rgba8UnormSrgb
            | wgpu::TextureFormat::Bgra8Unorm
            | wgpu::TextureFormat::Bgra8UnormSrgb => Ok(()),
            _ => Err(ResourceError::OutputFormat(format).into()),
        }
    }
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        source: &wgpu::TextureView,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        viewport: Option<Viewport>,
        pixel_grid: Option<[f32; 4]>,
    ) -> Result<OutputDraw> {
        let pipeline = self
            .0
            .get(&format)
            .cloned()
            .ok_or(ResourceError::OutputFormat(format))?;
        // The backdrop is display-referred and is not exposed or tone mapped.
        // These values retain Claude's neutral #141519 viewport surround.
        let backdrop = [20, 21, 25].map(decode);
        let mut uniform = OutputUniform {
            backdrop: [
                backdrop[0],
                backdrop[1],
                backdrop[2],
                if format.is_srgb() { 0.0 } else { 1.0 },
            ],
            rect: [0.; 4],
            radii: [0.; 4],
            canvas: [0.; 4],
            pixel_grid: pixel_grid.unwrap_or([0.; 4]),
        };
        if let Some(viewport) = viewport {
            let color = viewport.canvas_srgb.map(decode);
            uniform.rect = viewport.rect;
            uniform.radii = viewport.corner_radii;
            uniform.canvas = [color[0], color[1], color[2], 1.];
        }
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Display output settings"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("HDR output"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: buffer.as_entire_binding(),
                },
            ],
        });
        Ok(OutputDraw {
            pipeline,
            bindings,
            target: target.clone(),
        })
    }
}
pub(crate) struct OutputDraw {
    pipeline: wgpu::RenderPipeline,
    bindings: wgpu::BindGroup,
    target: wgpu::TextureView,
}
impl OutputDraw {
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queries: Option<&crate::timing::PassQueries>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            timestamp_writes: queries.map(|q| q.render()),
            label: Some("Tone map scene and compose display backdrop"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bindings, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn shader(device: &wgpu::Device, _backend: wgpu::Backend) -> Result<Stages> {
    #[cfg(all(feature = "cooked-output", not(target_arch = "wasm32")))]
    return cooked::stages(device, _backend);
    #[cfg(not(all(feature = "cooked-output", not(target_arch = "wasm32"))))]
    {
        let source = format!(
            "{}\n{}",
            include_str!("output.wgsl"),
            include_str!("tone_map.wgsl")
        );
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("HDR display transform"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        Ok(Stages {
            vertex: module.clone(),
            fragment: module,
            vertex_entry: "vertex",
            fragment_entry: "fragment",
        })
    }
}

fn build(
    device: &wgpu::Device,
    shader: &Stages,
    layout: Option<&wgpu::PipelineLayout>,
    format: wgpu::TextureFormat,
    cache: Option<&wgpu::PipelineCache>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("HDR display transform"),
        layout,
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
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache,
    })
}

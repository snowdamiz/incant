//! Scene-linear HDR to display-linear output, then exactly one sRGB transfer.
use crate::{ResourceError, Result, Viewport};
use bytemuck::{Pod, Zeroable};
use std::{collections::HashMap, sync::Mutex};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct OutputUniform {
    backdrop: [f32; 4],
    rect: [f32; 4],
    radii: [f32; 4],
    canvas: [f32; 4],
}
fn decode(byte: u8) -> f32 {
    let s = f32::from(byte) / 255.;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}
#[derive(Default)]
pub(crate) struct OutputPass(Mutex<HashMap<wgpu::TextureFormat, wgpu::RenderPipeline>>);
impl OutputPass {
    pub fn validate(format: wgpu::TextureFormat) -> Result<()> {
        match format {
            wgpu::TextureFormat::Rgba8Unorm
            | wgpu::TextureFormat::Rgba8UnormSrgb
            | wgpu::TextureFormat::Bgra8Unorm
            | wgpu::TextureFormat::Bgra8UnormSrgb => Ok(()),
            _ => Err(ResourceError::OutputFormat(format).into()),
        }
    }
    pub fn encode(
        &self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        source: &wgpu::TextureView,
        target: &wgpu::TextureView,
        format: wgpu::TextureFormat,
        viewport: Option<Viewport>,
    ) -> Result<()> {
        let pipeline = {
            let mut pipelines = self
                .0
                .lock()
                .map_err(|_| ResourceError::CacheLock("output"))?;
            pipelines
                .entry(format)
                .or_insert_with(|| build(device, format))
                .clone()
        };
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
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Tone map scene and compose display backdrop"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bindings, &[]);
        pass.draw(0..3, 0..1);
        Ok(())
    }
}
fn build(device: &wgpu::Device, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
    let source = format!(
        "{}\n{}",
        include_str!("output.wgsl"),
        include_str!("tone_map.wgsl")
    );
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("HDR display transform"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("HDR display transform"),
        layout: None,
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
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

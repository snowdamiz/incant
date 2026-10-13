use super::{System, math::Uniform, targets::Targets};
use crate::{Result, models::Draw};
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub(crate) struct Prepared {
    targets: Arc<Targets>,
    group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    pub camera: wgpu::BindGroup,
}
impl System {
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        camera: crate::camera::CameraView,
        rect: [f32; 4],
        size: [u32; 2],
        frame: &wgpu::Buffer,
    ) -> Result<Prepared> {
        let uniform = Uniform::new(camera, rect, size, self.settings)?;
        let targets = self.targets.get(device, size)?;
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("SSAO immutable frame"),
            contents: bytemuck::bytes_of(&uniform),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSAO depth and geometric normals"),
            layout: &self.sample_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&targets.depth),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&targets.normals),
                },
            ],
        });
        let camera = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SSAO forward frame"),
            layout: &self.globals,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&targets.accessibility),
                },
            ],
        });
        Ok(Prepared {
            targets,
            group,
            pipeline: self.sample.clone(),
            camera,
        })
    }
}
impl Prepared {
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        camera: &wgpu::BindGroup,
        rect: [f32; 4],
        draws: &[Draw],
    ) {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO opaque and MASK prepass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.targets.normals,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.targets.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                ..Default::default()
            });
            pass.set_bind_group(0, camera, &[]);
            let [x, y, w, h] = rect;
            pass.set_viewport(x, y, w, h, 0., 1.);
            for draw in draws {
                if let Some(pipeline) = &draw.prepass {
                    draw.encode(&mut pass, pipeline);
                }
            }
        }
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("SSAO diffuse accessibility"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.targets.accessibility,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.group, &[]);
        let [x, y, w, h] = rect;
        pass.set_viewport(x, y, w, h, 0., 1.);
        pass.draw(0..3, 0..1);
    }
}

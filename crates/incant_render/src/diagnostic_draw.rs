//! Clear scene attachments and draw the diagnostic fallback before model geometry.
use crate::{Renderer, Result, frame::FrameTargets, models::RenderScene};
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub(crate) struct DiagnosticDraw {
    targets: Arc<FrameTargets>,
    rect: [f32; 4],
    pipeline: wgpu::RenderPipeline,
    vertices: wgpu::Buffer,
    count: u32,
}
impl DiagnosticDraw {
    pub fn prepare(
        renderer: &Renderer,
        scene: &RenderScene,
        targets: Arc<FrameTargets>,
        rect: [f32; 4],
    ) -> Result<Self> {
        let vertices = crate::vertices(&scene.diagnostics, scene.camera.matrix(rect[2] / rect[3])?);
        Ok(Self {
            targets,
            rect,
            pipeline: renderer.cached_pipeline(crate::frame::HDR_FORMAT)?,
            vertices: renderer
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("ECS diagnostic geometry"),
                    contents: if vertices.is_empty() {
                        &[0u8; 28]
                    } else {
                        bytemuck::cast_slice(&vertices)
                    },
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            count: vertices.len() as u32,
        })
    }
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Clear scene and draw diagnostic geometry"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.targets.color,
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
        pass.set_pipeline(&self.pipeline);
        let [x, y, w, h] = self.rect;
        pass.set_viewport(x, y, w, h, 0., 1.);
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.draw(0..self.count, 0..1);
    }
}

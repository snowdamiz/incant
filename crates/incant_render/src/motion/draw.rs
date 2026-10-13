use super::targets::Targets;
use crate::models::Batch;
use std::sync::Arc;

pub(super) struct Draw {
    pub batch: Arc<Batch>,
    pub previous_vertices: wgpu::Buffer,
    pub previous_instances: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
}
pub(crate) struct MotionDraw {
    pub(super) targets: Arc<Targets>,
    pub(super) slot: usize,
    pub(super) rect: [f32; 4],
    pub(super) frame: wgpu::BindGroup,
    pub(super) draws: Vec<Draw>,
}
impl MotionDraw {
    pub fn encode(&self, encoder: &mut wgpu::CommandEncoder) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Opaque surface motion/depth"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.targets.motion,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.targets.depth[self.slot],
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        let [x, y, w, h] = self.rect;
        pass.set_viewport(x, y, w, h, 0., 1.);
        pass.set_bind_group(0, &self.frame, &[]);
        for draw in &self.draws {
            let batch = &draw.batch;
            let primitive = &batch.model.primitives[batch.primitive];
            let material = &batch.model.materials.materials[primitive.material];
            pass.set_pipeline(&draw.pipeline);
            pass.set_bind_group(1, &material.bind_group, &[]);
            pass.set_vertex_buffer(
                0,
                batch
                    .vertices
                    .as_ref()
                    .unwrap_or(&primitive.vertices)
                    .slice(..),
            );
            pass.set_vertex_buffer(1, draw.previous_vertices.slice(..));
            pass.set_vertex_buffer(2, batch.instances.slice(..));
            pass.set_vertex_buffer(3, draw.previous_instances.slice(..));
            pass.set_index_buffer(primitive.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..primitive.index_count, 0, 0..batch.count);
        }
    }
}

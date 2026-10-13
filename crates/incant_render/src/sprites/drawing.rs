use super::*;
pub(crate) struct SpriteDraw {
    scene: Arc<SpriteScene>,
    pipeline: wgpu::RenderPipeline,
    globals: wgpu::BindGroup,
    _uniform: wgpu::Buffer,
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    rect: [f32; 4],
}
impl SpriteSystem {
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        scene: &crate::models::RenderScene,
        target: crate::models::ModelTarget,
    ) -> Result<Option<SpriteDraw>> {
        let Some(sprites) = &scene.sprites else {
            return Ok(None);
        };
        let view = scene.camera.matrix(target.rect[2] / target.rect[3])?;
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Sprite frame camera"),
            contents: bytemuck::cast_slice(&view.to_cols_array()),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let globals = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sprite frame camera"),
            layout: &self.globals,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        Ok(Some(SpriteDraw {
            scene: Arc::clone(sprites),
            pipeline: self.pipeline.clone(),
            globals,
            _uniform: uniform,
            color: target.color,
            depth: target.depth,
            rect: target.rect,
        }))
    }
}
impl SpriteDraw {
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queries: Option<&crate::timing::PassQueries>,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            timestamp_writes: queries.map(|q| q.render()),
            label: Some("Sorted sprites"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.color,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        });
        pass.set_pipeline(&self.pipeline);
        let [x, y, w, h] = self.rect;
        pass.set_viewport(x, y, w, h, 0., 1.);
        pass.set_bind_group(0, &self.globals, &[]);
        pass.set_vertex_buffer(0, self.scene.positions.slice(..));
        pass.set_vertex_buffer(1, self.scene.geometry.attributes.slice(..));
        for batch in &self.scene.geometry.batches {
            if batch
                .selection
                .is_some_and(|(node, frame)| self.scene.active_frames[node] != frame)
            {
                continue;
            }
            pass.set_bind_group(1, &batch.texture.groups[batch.sampling], &[]);
            pass.draw(batch.start..batch.start + batch.count, 0..1);
        }
    }
}

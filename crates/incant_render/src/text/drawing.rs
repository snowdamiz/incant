//! Text pipeline and per-frame draw packet. The packet owns its scene version,
//! so queued commands outlive any later retained staging or replacement.
use super::{NODE_STRIDE, QUAD_BYTES, TEXT_COLOR, TextScene, TextSystem};
use crate::Result;
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct FrameUniform {
    matrix: [[f32; 4]; 4],
    color: [f32; 4],
}
pub(super) fn pipeline(
    device: &wgpu::Device,
    layouts: [&wgpu::BindGroupLayout; 3],
) -> wgpu::RenderPipeline {
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Text coverage"),
        bind_group_layouts: &layouts.map(Some),
        immediate_size: 0,
    });
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Text placement and coverage math"),
        source: wgpu::ShaderSource::Wgsl(include_str!("text.wgsl").into()),
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("World-space text in shared HDR graph"),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vertex"),
            compilation_options: Default::default(),
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: QUAD_BYTES,
                step_mode: wgpu::VertexStepMode::Instance,
                attributes: &wgpu::vertex_attr_array![0 => Float32x4, 1 => Float32x4],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fragment"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: crate::frame::HDR_FORMAT,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            cull_mode: None,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: Default::default(),
            bias: Default::default(),
        }),
        multisample: Default::default(),
        multiview_mask: None,
        cache: None,
    })
}
pub(crate) struct TextDraw {
    scene: Arc<TextScene>,
    pipeline: wgpu::RenderPipeline,
    frame: wgpu::BindGroup,
    _uniform: wgpu::Buffer,
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    rect: [f32; 4],
}
impl TextSystem {
    pub fn prepare(
        &self,
        device: &wgpu::Device,
        scene: &crate::models::RenderScene,
        target: crate::models::ModelTarget,
    ) -> Result<Option<TextDraw>> {
        let Some(text) = scene.text.as_ref().filter(|t| t.geometry.quads.is_some()) else {
            return Ok(None);
        };
        let matrix = scene.camera.matrix(target.rect[2] / target.rect[3])?;
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Text frame camera and color"),
            contents: bytemuck::bytes_of(&FrameUniform {
                matrix: matrix.to_cols_array_2d(),
                color: [TEXT_COLOR[0], TEXT_COLOR[1], TEXT_COLOR[2], 1.],
            }),
            usage: wgpu::BufferUsages::UNIFORM,
        });
        let frame = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text frame camera and color"),
            layout: &self.frame,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        Ok(Some(TextDraw {
            scene: Arc::clone(text),
            pipeline: self.pipeline.clone(),
            frame,
            _uniform: uniform,
            color: target.color,
            depth: target.depth,
            rect: target.rect,
        }))
    }
}
impl TextDraw {
    pub fn has_passes(&self) -> bool {
        self.scene.geometry.quads.is_some() && self.scene.geometry.atlas.is_some()
    }
    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        queries: Option<&crate::timing::PassQueries>,
    ) {
        let geometry = &self.scene.geometry;
        let (Some(quads), Some(atlas)) = (&geometry.quads, &geometry.atlas) else {
            return;
        };
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            timestamp_writes: queries.map(|q| q.render()),
            label: Some("World-space text"),
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
        pass.set_bind_group(0, &self.frame, &[]);
        pass.set_bind_group(2, &atlas.group, &[]);
        for (index, node) in geometry.nodes.iter().enumerate() {
            if node.count == 0 {
                continue;
            }
            // Bind this node's instance range: no first-instance feature needed.
            let start = u64::from(node.start) * QUAD_BYTES;
            let end = start + u64::from(node.count) * QUAD_BYTES;
            pass.set_bind_group(1, &self.scene.group, &[(index as u64 * NODE_STRIDE) as u32]);
            pass.set_vertex_buffer(0, quads.slice(start..end));
            pass.draw(0..6, 0..node.count);
        }
    }
}

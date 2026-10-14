use super::{System, math::Uniform, raster::PostprocessRaster, targets::Targets};
use crate::{Result, models::Draw};
use std::sync::Arc;
use wgpu::util::DeviceExt;

pub(crate) struct Prepared {
    targets: Arc<Targets>,
    raster: PostprocessRaster,
    group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    filter_groups: [wgpu::BindGroup; 2],
    filter_pipelines: [wgpu::RenderPipeline; 2],
    pub camera: wgpu::BindGroup,
    #[cfg(all(feature = "ssao-stage-readback", not(target_arch = "wasm32")))]
    capture: Arc<super::readback::Capture>,
    #[cfg(all(feature = "ssao-stage-readback", not(target_arch = "wasm32")))]
    device: wgpu::Device,
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
            label: Some("SSAO depth and authored shading normals"),
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
        let filter_groups = [&targets.accessibility, &targets.scratch].map(|source| {
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("SSAO immutable filter inputs"),
                layout: &self.filter_layout,
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
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(source),
                    },
                ],
            })
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
            raster: PostprocessRaster::new(rect, size),
            group,
            pipeline: self.sample.clone(),
            filter_groups,
            filter_pipelines: self.filters.clone(),
            camera,
            #[cfg(all(feature = "ssao-stage-readback", not(target_arch = "wasm32")))]
            capture: Arc::clone(&self.capture),
            #[cfg(all(feature = "ssao-stage-readback", not(target_arch = "wasm32")))]
            device: device.clone(),
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
        timestamps: [Option<wgpu::RenderPassTimestampWrites<'_>>; 4],
    ) {
        let [prepass, sample, horizontal, vertical] = timestamps;
        #[cfg(all(feature = "ssao-stage-readback", not(target_arch = "wasm32")))]
        let capture = self.capture.begin(
            &self.device,
            [
                self.targets.accessibility.texture().width(),
                self.targets.accessibility.texture().height(),
            ],
        );
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SSAO opaque and MASK prepass"),
                timestamp_writes: prepass,
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
        // Every pass ends before its output becomes the next pass's input.
        // The vertical pass restores T0, the unchanged forward-material source.
        for (stage, ((target, pipeline, group, label), timestamps)) in [
            (
                &self.targets.accessibility,
                &self.pipeline,
                &self.group,
                "SSAO diffuse accessibility",
            ),
            (
                &self.targets.scratch,
                &self.filter_pipelines[0],
                &self.filter_groups[0],
                "SSAO horizontal filter",
            ),
            (
                &self.targets.accessibility,
                &self.filter_pipelines[1],
                &self.filter_groups[1],
                "SSAO vertical filter",
            ),
        ]
        .into_iter()
        .zip([sample, horizontal, vertical])
        .enumerate()
        {
            #[cfg(not(all(feature = "ssao-stage-readback", not(target_arch = "wasm32"))))]
            let _ = stage;
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(label),
                timestamp_writes: timestamps,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: target,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, group, &[]);
            if self.raster.apply(&mut pass) {
                pass.draw(0..3, 0..1);
            }
            drop(pass);
            #[cfg(all(feature = "ssao-stage-readback", not(target_arch = "wasm32")))]
            if let Some(capture) = &capture {
                capture.copy(encoder, target.texture(), stage);
            }
        }
    }
}

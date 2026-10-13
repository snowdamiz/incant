//! Owned draw packets retain exactly the scene versions prepared for a frame.
use super::{Batch, RenderScene};
use crate::{Renderer, Result, materials::PipelineKey};
use std::sync::Arc;
use wgpu::util::DeviceExt;
impl Renderer {
    pub(crate) fn prepare_models(
        &self,
        scene: &RenderScene,
        target: ModelTarget,
    ) -> Result<Option<ModelDraw>> {
        if scene.batches.is_empty() {
            return Ok(None);
        }
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct Frame {
            matrix: [[f32; 4]; 4],
            eye: [f32; 4],
            environment: [f32; 4],
        }
        const _: () = {
            assert!(size_of::<Frame>() == 96);
            assert!(std::mem::offset_of!(Frame, matrix) == 0);
            assert!(std::mem::offset_of!(Frame, eye) == 64);
            assert!(std::mem::offset_of!(Frame, environment) == 80);
        };
        let environment = scene
            .environment
            .as_ref()
            .expect("model scene has environment");
        let frame = Frame {
            matrix: scene
                .camera
                .matrix(target.rect[2] / target.rect[3])?
                .to_cols_array_2d(),
            eye: scene.camera.shading_eye(),
            environment: [
                environment.intensity,
                environment.rotation.cos(),
                environment.rotation.sin(),
                crate::environment::MAX_SPECULAR_LOD,
            ],
        };
        let camera_group = |frame: Frame| {
            let camera = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Material preview frame"),
                    contents: bytemuck::bytes_of(&frame),
                    usage: wgpu::BufferUsages::UNIFORM,
                });
            let group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Material preview frame"),
                layout: &self.materials.globals,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera.as_entire_binding(),
                }],
            });
            (camera, group)
        };
        let (camera, bind_group) = camera_group(frame);
        let transparent_camera = if scene.camera.jitter != [0.; 2] {
            Some(
                camera_group(Frame {
                    matrix: scene
                        .camera
                        .unjittered()
                        .matrix(target.rect[2] / target.rect[3])?
                        .to_cols_array_2d(),
                    ..frame
                })
                .1,
            )
        } else {
            None
        };
        let ambient_occlusion = self
            .ambient_occlusion
            .as_ref()
            .map(|system| {
                let size = target.color.texture().size();
                system.prepare(
                    &self.device,
                    scene.camera,
                    target.rect,
                    [size.width, size.height],
                    &camera,
                )
            })
            .transpose()?;
        let mut opaque = Vec::new();
        let shadows = self.prepare_shadow_draw(scene, target.rect[2] / target.rect[3])?;
        let lighting = self.lighting.prepare(
            &self.device,
            &scene.lights,
            target.rect,
            scene.light_selection,
            scene.camera,
            &shadows.maps,
        )?;
        let mut transparent = Vec::new();
        let eye = scene.camera.eye.as_dvec3();
        let forward = scene.camera.forward.as_dvec3();
        for batch in scene.batches.iter().filter(|batch| batch.count > 0) {
            let material =
                &batch.model.materials.materials[batch.model.primitives[batch.primitive].material];
            let key = PipelineKey {
                format: target.format,
                alpha: material.alpha,
                double_sided: material.double_sided,
                mirrored: batch.mirrored,
            };
            let (pipeline, prepass) = if let Some(system) = self
                .ambient_occlusion
                .as_ref()
                .filter(|_| material.alpha != incant_assets_runtime::AlphaMode::Blend)
            {
                let missing = || crate::ResourceError::UnpreparedPipeline("SSAO material");
                (
                    system.shading.get(&key).cloned().ok_or_else(missing)?,
                    Some(system.prepass.get(&key).cloned().ok_or_else(missing)?),
                )
            } else {
                (self.materials.pipeline(key)?, None)
            };
            if material.alpha == incant_assets_runtime::AlphaMode::Blend {
                for (index, center) in batch.centers.iter().enumerate() {
                    transparent.push(Draw {
                        batch: Arc::clone(batch),
                        instances: index as u32..index as u32 + 1,
                        pipeline: pipeline.clone(),
                        depth: (center.as_dvec3() - eye).dot(forward),
                        prepass: None,
                    });
                }
            } else {
                opaque.push(Draw {
                    batch: Arc::clone(batch),
                    instances: 0..batch.count,
                    pipeline,
                    depth: 0.,
                    prepass,
                });
            }
        }
        // Primitive-center sorting handles ordinary layered transparency. Crossing
        // triangles and intersecting transparent surfaces remain order-dependent.
        transparent.sort_by(|a, b| b.depth.total_cmp(&a.depth));
        let opaque_count = opaque.len();
        opaque.extend(transparent);
        Ok(Some(ModelDraw {
            target,
            camera: bind_group,
            transparent_camera,
            environment: environment.resource.group.clone(),
            lighting,
            shadows,
            draws: opaque,
            opaque_count,
            ambient_occlusion,
        }))
    }
}
pub(crate) struct Draw {
    batch: Arc<Batch>,
    instances: std::ops::Range<u32>,
    pipeline: wgpu::RenderPipeline,
    depth: f64,
    pub prepass: Option<wgpu::RenderPipeline>,
}
impl Draw {
    pub(crate) fn encode(&self, pass: &mut wgpu::RenderPass<'_>, pipeline: &wgpu::RenderPipeline) {
        let batch = &self.batch;
        let primitive = &batch.model.primitives[batch.primitive];
        let material = &batch.model.materials.materials[primitive.material];
        pass.set_pipeline(pipeline);
        pass.set_bind_group(1, &material.bind_group, &[]);
        pass.set_vertex_buffer(
            0,
            batch
                .vertices
                .as_ref()
                .unwrap_or(&primitive.vertices)
                .slice(..),
        );
        pass.set_vertex_buffer(1, batch.instances.slice(..));
        pass.set_index_buffer(primitive.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..primitive.index_count, 0, self.instances.clone());
    }
}
pub(crate) struct ModelDraw {
    target: ModelTarget,
    camera: wgpu::BindGroup,
    transparent_camera: Option<wgpu::BindGroup>,
    environment: wgpu::BindGroup,
    pub lighting: crate::lighting::PreparedLighting,
    pub shadows: super::ShadowDraw,
    draws: Vec<Draw>,
    opaque_count: usize,
    ambient_occlusion: Option<crate::ambient_occlusion::Prepared>,
}
impl ModelDraw {
    pub fn has_ambient_occlusion(&self) -> bool {
        self.ambient_occlusion.is_some()
    }
    pub fn encode_ambient_occlusion(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamps: [Option<wgpu::RenderPassTimestampWrites<'_>>; 4],
    ) {
        if let Some(ao) = &self.ambient_occlusion {
            ao.encode(
                encoder,
                &self.camera,
                self.target.rect,
                &self.draws,
                timestamps,
            );
        }
    }

    pub fn encode(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        self.encode_range(
            encoder,
            0..self.draws.len(),
            timestamps,
            &self.target.color,
            &self.camera,
        );
    }
    pub fn encode_opaque(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        self.encode_range(
            encoder,
            0..self.opaque_count,
            timestamps,
            &self.target.color,
            &self.camera,
        );
    }
    pub fn encode_transparent(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
    ) {
        self.encode_range(
            encoder,
            self.opaque_count..self.draws.len(),
            timestamps,
            &self.target.color,
            self.transparent_camera.as_ref().unwrap_or(&self.camera),
        );
    }
    pub fn has_opaque(&self) -> bool {
        self.opaque_count > 0
    }
    pub fn has_transparent(&self) -> bool {
        self.opaque_count < self.draws.len()
    }
    fn encode_range(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        range: std::ops::Range<usize>,
        timestamps: Option<wgpu::RenderPassTimestampWrites<'_>>,
        color: &wgpu::TextureView,
        camera: &wgpu::BindGroup,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Imported model geometry"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: color,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.target.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: timestamps,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, camera, &[]);
        pass.set_bind_group(2, &self.environment, &[]);
        pass.set_bind_group(3, &self.lighting.shading, &[]);
        let [x, y, w, h] = self.target.rect;
        pass.set_viewport(x, y, w, h, 0., 1.);
        for draw in &self.draws[range] {
            if let Some(ao) = &self.ambient_occlusion {
                let frame = if draw.prepass.is_some() {
                    &ao.camera
                } else {
                    camera
                };
                pass.set_bind_group(0, frame, &[]);
            }
            draw.encode(&mut pass, &draw.pipeline);
        }
    }
}
pub(crate) struct ModelTarget {
    pub color: wgpu::TextureView,
    pub depth: wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub rect: [f32; 4],
}

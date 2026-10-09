//! Indexed model versions with immutable geometry and material resources.
use crate::{
    Renderer, ResourceError, Result,
    materials::{MaterialResources, PipelineKey},
    scene::{ResolvedScene, SceneStats},
};
use incant_assets::{AssetStore, RuntimeAsset, RuntimeAssetData};
use incant_doc::Project;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};
use wgpu::util::DeviceExt;

struct Primitive {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
    center: glam::Vec3,
    minimum: glam::Vec3,
    maximum: glam::Vec3,
    material: usize,
}
pub(crate) struct GpuModel {
    // Keep the exact decoded version alive for as long as any published scene uses it.
    _source: Arc<RuntimeAsset>,
    primitives: Vec<Primitive>,
    materials: MaterialResources,
}
struct Batch {
    model: Arc<GpuModel>,
    primitive: usize,
    instances: wgpu::Buffer,
    count: u32,
    mirrored: bool,
    centers: Vec<glam::Vec3>,
}
/// Immutable scene ready for rendering. A failed replacement never changes it.
/// Source bytes are not needed after AssetStore has loaded the cooked model.
pub struct RenderScene {
    pub(crate) diagnostics: Vec<glam::Mat4>,
    batches: Vec<Batch>,
    stats: SceneStats,
    environment: Option<crate::environment::Binding>,
    lights: crate::lighting::GpuLights,
}
impl RenderScene {
    /// Identifies the available appearance without claiming production lighting.
    pub fn shading(&self) -> &'static str {
        if self.batches.is_empty() {
            "diagnostic"
        } else {
            "material_preview"
        }
    }
    pub fn stats(&self) -> &SceneStats {
        &self.stats
    }
}
pub(crate) type ModelCache = HashMap<String, Weak<GpuModel>>;

impl Renderer {
    pub fn prepare_scene(&self, project: &Project, assets: &AssetStore) -> Result<RenderScene> {
        self.upload_scene(crate::scene::resolve(project, Some(assets))?)
    }
    pub(crate) fn diagnostic_scene(&self, project: &Project) -> Result<RenderScene> {
        self.upload_scene(crate::scene::resolve(project, None)?)
    }
    fn upload_scene(&self, mut resolved: ResolvedScene) -> Result<RenderScene> {
        let environment = if resolved.environment.is_some() || !resolved.models.is_empty() {
            Some(self.environments.prepare(
                &self.device,
                &self.queue,
                resolved.environment.take(),
            )?)
        } else {
            None
        };
        let mut cache = self
            .models
            .lock()
            .map_err(|_| ResourceError::CacheLock("GPU model"))?;
        cache.retain(|_, model| model.strong_count() != 0);
        let mut batches = Vec::new();
        for (key, plan) in resolved.models {
            let model = match cache.get(&key).and_then(Weak::upgrade) {
                Some(model) => model,
                None => {
                    let RuntimeAssetData::Model(source) = plan.source.data() else {
                        unreachable!("scene binding validated");
                    };
                    let mut primitives = Vec::new();
                    let materials = self.materials.upload(&self.device, &self.queue, source)?;
                    for (index, mesh) in source.meshes.iter().enumerate() {
                        let vertices = bytemuck::cast_slice(&mesh.vertices);
                        let indices = bytemuck::cast_slice(&mesh.indices);
                        if vertices.len() as u64 > self.device.limits().max_buffer_size
                            || indices.len() as u64 > self.device.limits().max_buffer_size
                        {
                            return Err(ResourceError::BufferSize("cooked geometry").into());
                        }
                        let mut minimum = glam::DVec3::splat(f64::INFINITY);
                        let mut maximum = glam::DVec3::splat(f64::NEG_INFINITY);
                        for v in &mesh.vertices {
                            let position = glam::DVec3::new(v[0] as f64, v[1] as f64, v[2] as f64);
                            minimum = minimum.min(position);
                            maximum = maximum.max(position);
                        }
                        primitives.push(Primitive {
                            center: ((minimum + maximum) * 0.5).as_vec3(),
                            minimum: minimum.as_vec3(),
                            maximum: maximum.as_vec3(),
                            material: source.metadata.mesh_materials[index]
                                .unwrap_or(source.materials.len()),
                            vertices: self.device.create_buffer_init(
                                &wgpu::util::BufferInitDescriptor {
                                    label: Some("Cooked model vertices"),
                                    contents: vertices,
                                    usage: wgpu::BufferUsages::VERTEX,
                                },
                            ),
                            indices: self.device.create_buffer_init(
                                &wgpu::util::BufferInitDescriptor {
                                    label: Some("Cooked model indices"),
                                    contents: indices,
                                    usage: wgpu::BufferUsages::INDEX,
                                },
                            ),
                            index_count: mesh.indices.len() as u32,
                        });
                    }
                    let model = Arc::new(GpuModel {
                        _source: plan.source,
                        primitives,
                        materials,
                    });
                    cache.insert(key, Arc::downgrade(&model));
                    model
                }
            };
            for (primitive, transforms) in plan.primitives {
                // A reflected instance reverses winding. Split it from ordinary
                // instances so back-face culling remains correct for both.
                for mirrored in [false, true] {
                    let transforms: Vec<_> = transforms
                        .iter()
                        .copied()
                        .filter(|t| (t.normal[0][3] < 0.) == mirrored)
                        .collect();
                    if transforms.is_empty() {
                        continue;
                    }
                    let mesh = &model.primitives[primitive];
                    let mut centers = Vec::new();
                    for instance in &transforms {
                        let world = glam::Mat4::from_cols_array_2d(&instance.world);
                        for x in [mesh.minimum.x, mesh.maximum.x] {
                            for y in [mesh.minimum.y, mesh.maximum.y] {
                                for z in [mesh.minimum.z, mesh.maximum.z] {
                                    if !world.transform_point3(glam::Vec3::new(x, y, z)).is_finite()
                                    {
                                        return Err(ResourceError::GeometryRange.into());
                                    }
                                }
                            }
                        }
                        centers.push(world.transform_point3(mesh.center));
                    }
                    let bytes = bytemuck::cast_slice(&transforms);
                    if bytes.len() as u64 > self.device.limits().max_buffer_size {
                        return Err(ResourceError::BufferSize("scene instances").into());
                    }
                    batches.push(Batch {
                        model: Arc::clone(&model),
                        primitive,
                        instances: self.device.create_buffer_init(
                            &wgpu::util::BufferInitDescriptor {
                                label: Some("Model scene instances"),
                                contents: bytes,
                                usage: wgpu::BufferUsages::VERTEX,
                            },
                        ),
                        count: transforms.len() as u32,
                        mirrored,
                        centers,
                    });
                }
            }
        }
        resolved.stats.model_draw_calls = batches
            .iter()
            .map(|b| {
                if b.model.materials.materials[b.model.primitives[b.primitive].material].alpha
                    == incant_assets::AlphaMode::Blend
                {
                    b.count as usize
                } else {
                    1
                }
            })
            .sum();
        Ok(RenderScene {
            lights: crate::lighting::GpuLights::upload(&self.device, resolved.lights),
            diagnostics: resolved.diagnostics,
            batches,
            stats: resolved.stats,
            environment,
        })
    }

    pub(crate) fn render_models(
        &self,
        scene: &RenderScene,
        target: ModelTarget<'_>,
        encoder: &mut wgpu::CommandEncoder,
    ) -> Result<()> {
        if scene.batches.is_empty() {
            return Ok(());
        }
        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct Frame {
            matrix: [[f32; 4]; 4],
            eye: [f32; 4],
            environment: [f32; 4],
        }
        let environment = scene
            .environment
            .as_ref()
            .expect("model scene has environment");
        let frame = Frame {
            matrix: crate::camera(target.rect[2] / target.rect[3]).to_cols_array_2d(),
            eye: glam::Vec3::from_array(crate::studio::EYE)
                .extend(1.)
                .to_array(),
            environment: [
                environment.intensity,
                environment.rotation.cos(),
                environment.rotation.sin(),
                crate::environment::MAX_SPECULAR_LOD,
            ],
        };
        let camera = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Material preview frame"),
                contents: bytemuck::bytes_of(&frame),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Material preview frame"),
            layout: &self.materials.globals,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        struct Draw<'a> {
            batch: &'a Batch,
            instances: std::ops::Range<u32>,
            pipeline: wgpu::RenderPipeline,
            depth: f64,
        }
        let mut opaque = Vec::new();
        let light_group =
            self.lighting
                .prepare(&self.device, encoder, &scene.lights, target.rect)?;
        let mut transparent = Vec::new();
        let eye = glam::Vec3::from_array(crate::studio::EYE).as_dvec3();
        let forward = (-eye).normalize();
        for batch in &scene.batches {
            let material =
                &batch.model.materials.materials[batch.model.primitives[batch.primitive].material];
            let pipeline = self.materials.pipeline(
                &self.device,
                PipelineKey {
                    format: target.format,
                    alpha: material.alpha,
                    double_sided: material.double_sided,
                    mirrored: batch.mirrored,
                },
            )?;
            if material.alpha == incant_assets::AlphaMode::Blend {
                for (index, center) in batch.centers.iter().enumerate() {
                    transparent.push(Draw {
                        batch,
                        instances: index as u32..index as u32 + 1,
                        pipeline: pipeline.clone(),
                        depth: (center.as_dvec3() - eye).dot(forward),
                    });
                }
            } else {
                opaque.push(Draw {
                    batch,
                    instances: 0..batch.count,
                    pipeline,
                    depth: 0.,
                });
            }
        }
        // Primitive-center sorting handles ordinary layered transparency. Crossing
        // triangles and intersecting transparent surfaces remain order-dependent.
        transparent.sort_by(|a, b| b.depth.total_cmp(&a.depth));
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Imported model geometry"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target.color,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: target.depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_bind_group(0, &bind_group, &[]);
        pass.set_bind_group(2, &environment.resource.group, &[]);
        pass.set_bind_group(3, &light_group, &[]);
        let [x, y, w, h] = target.rect;
        pass.set_viewport(x, y, w, h, 0., 1.);
        for draw in opaque.iter().chain(&transparent) {
            let batch = draw.batch;
            let primitive = &batch.model.primitives[batch.primitive];
            let material = &batch.model.materials.materials[primitive.material];
            pass.set_pipeline(&draw.pipeline);
            pass.set_bind_group(1, &material.bind_group, &[]);
            pass.set_vertex_buffer(0, primitive.vertices.slice(..));
            pass.set_vertex_buffer(1, batch.instances.slice(..));
            pass.set_index_buffer(primitive.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..primitive.index_count, 0, draw.instances.clone());
        }
        Ok(())
    }
}
pub(crate) struct ModelTarget<'a> {
    pub color: &'a wgpu::TextureView,
    pub depth: &'a wgpu::TextureView,
    pub format: wgpu::TextureFormat,
    pub rect: [f32; 4],
}

//! Indexed, instanced geometry. Appearance stays on the existing diagnostic shader;
//! this module owns versioned GPU buffers and transform/data correctness.
use crate::{
    Renderer, Result,
    scene::{Instance, ResolvedScene, SceneStats},
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
}
pub(crate) struct GpuModel {
    // Keep the exact decoded version alive for as long as any published scene uses it.
    _source: Arc<RuntimeAsset>,
    primitives: Vec<Primitive>,
}
struct Batch {
    model: Arc<GpuModel>,
    primitive: usize,
    instances: wgpu::Buffer,
    count: u32,
}
/// Immutable scene ready for rendering. A failed replacement never changes it.
/// Source bytes are not needed after AssetStore has loaded the cooked model.
pub struct RenderScene {
    pub(crate) diagnostics: Vec<glam::Mat4>,
    batches: Vec<Batch>,
    stats: SceneStats,
}
impl RenderScene {
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
    fn upload_scene(&self, resolved: ResolvedScene) -> Result<RenderScene> {
        let mut cache = self
            .models
            .lock()
            .map_err(|_| "GPU model cache lock failed")?;
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
                    for mesh in &source.meshes {
                        let vertices = bytemuck::cast_slice(&mesh.vertices);
                        let indices = bytemuck::cast_slice(&mesh.indices);
                        if vertices.len() as u64 > self.device.limits().max_buffer_size
                            || indices.len() as u64 > self.device.limits().max_buffer_size
                        {
                            return Err("cooked geometry exceeds GPU buffer limit".into());
                        }
                        primitives.push(Primitive {
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
                    });
                    cache.insert(key, Arc::downgrade(&model));
                    model
                }
            };
            for (primitive, transforms) in plan.primitives {
                let bytes = bytemuck::cast_slice(&transforms);
                if bytes.len() as u64 > self.device.limits().max_buffer_size {
                    return Err("scene instances exceed GPU buffer limit".into());
                }
                batches.push(Batch {
                    model: Arc::clone(&model),
                    primitive,
                    instances: self
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Model scene instances"),
                            contents: bytes,
                            usage: wgpu::BufferUsages::VERTEX,
                        }),
                    count: transforms.len() as u32,
                });
            }
        }
        Ok(RenderScene {
            diagnostics: resolved.diagnostics,
            batches,
            stats: resolved.stats,
        })
    }

    fn model_pipeline(&self, format: wgpu::TextureFormat) -> Result<wgpu::RenderPipeline> {
        let mut cache = self
            .model_pipelines
            .lock()
            .map_err(|_| "model pipeline cache lock failed")?;
        Ok(cache.entry(format).or_insert_with(|| {
            let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Instanced diagnostic model shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("model_geometry.wgsl").into()),
            });
            self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Indexed model geometry"), layout: None,
                vertex: wgpu::VertexState {
                    module: &shader, entry_point: Some("vertex"), compilation_options: Default::default(),
                    buffers: &[
                        wgpu::VertexBufferLayout {
                            array_stride: size_of::<incant_assets::Vertex>() as u64, step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![0=>Float32x3,1=>Float32x3],
                        },
                        wgpu::VertexBufferLayout {
                            array_stride: size_of::<Instance>() as u64, step_mode: wgpu::VertexStepMode::Instance,
                            attributes: &wgpu::vertex_attr_array![2=>Float32x4,3=>Float32x4,4=>Float32x4,5=>Float32x4,6=>Float32x4,7=>Float32x4,8=>Float32x4],
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState { cull_mode: None, ..Default::default() },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float, depth_write_enabled: Some(true),
                    depth_compare: Some(wgpu::CompareFunction::Less), stencil: Default::default(), bias: Default::default(),
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader, entry_point: Some("fragment"), compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {format,blend:None,write_mask:wgpu::ColorWrites::ALL})],
                }), multiview_mask: None, cache: None,
            })
        }).clone())
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
        let pipeline = self.model_pipeline(target.format)?;
        let matrix = crate::camera(target.rect[2] / target.rect[3]).to_cols_array();
        let camera = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Viewport camera"),
                contents: bytemuck::cast_slice(&matrix),
                usage: wgpu::BufferUsages::UNIFORM,
            });
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Model camera"),
            layout: &pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
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
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        let [x, y, w, h] = target.rect;
        pass.set_viewport(x, y, w, h, 0., 1.);
        for batch in &scene.batches {
            let primitive = &batch.model.primitives[batch.primitive];
            pass.set_vertex_buffer(0, primitive.vertices.slice(..));
            pass.set_vertex_buffer(1, batch.instances.slice(..));
            pass.set_index_buffer(primitive.indices.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..primitive.index_count, 0, 0..batch.count);
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

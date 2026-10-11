//! Indexed model versions with immutable geometry and material resources.
use crate::{
    Renderer, ResourceError, Result,
    materials::MaterialResources,
    scene::{ResolvedScene, SceneStats},
};
use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData};
use incant_render_data::SceneInput;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Weak},
};
use wgpu::util::DeviceExt;
#[path = "model_draw.rs"]
mod drawing;
pub(crate) use drawing::{ModelDraw, ModelTarget};
#[path = "shadow_draw.rs"]
mod shadow_drawing;
pub(crate) use shadow_drawing::ShadowDraw;

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
#[path = "retained.rs"]
mod retained;
pub use retained::{RetainedScene, RetainedTarget};

struct Batch {
    model: Arc<GpuModel>,
    primitive: usize,
    instances: wgpu::Buffer,
    count: u32,
    mirrored: bool,
    centers: Vec<glam::Vec3>,
    casts_shadows: bool,
    bounds: crate::shadows::cascade::Bounds,
    placements: Arc<[crate::scene::Placement]>,
    updates: Vec<crate::scene::Instance>,
}
/// Immutable scene ready for rendering. A failed replacement never changes it.
/// Source bytes are not needed after AssetStore has loaded the cooked model.
pub struct RenderScene {
    pub(crate) diagnostics: Vec<glam::Mat4>,
    pub(crate) sprites: Option<Arc<crate::sprites::SpriteScene>>,
    batches: Vec<Arc<Batch>>,
    stats: SceneStats,
    environment: Option<crate::environment::Binding>,
    lights: crate::lighting::GpuLights,
    light_selection: crate::LocalLightSelection,
    cameras: BTreeMap<String, crate::camera::CameraView>,
    pub(crate) camera: crate::camera::CameraView,
}
impl RenderScene {
    /// Select an authored Camera entity by stable ID for this prepared frame.
    /// The default remains the editor preview; no project state is mutated.
    pub fn with_camera(mut self, id: &str) -> std::result::Result<Self, crate::SceneError> {
        self.camera = *self
            .cameras
            .get(id)
            .ok_or_else(|| crate::SceneError::MissingCamera(id.into()))?;
        self.lights.validate_view(self.camera.view)?;
        Ok(self)
    }

    /// Select an unculled diagnostic or normal clustered light path for this
    /// prepared scene. Authored documents and GPU light data remain unchanged.
    pub fn with_local_light_selection(mut self, selection: crate::LocalLightSelection) -> Self {
        self.light_selection = selection;
        self
    }
    /// Identifies the available appearance without claiming production lighting.
    pub fn shading(&self) -> &'static str {
        if self.sprites.is_some() {
            "sprites_and_materials"
        } else if self.batches.is_empty() {
            "diagnostic"
        } else {
            "material_preview"
        }
    }
    pub fn stats(&self) -> &SceneStats {
        &self.stats
    }
}
pub(crate) type ModelCache = HashMap<incant_assets_runtime::AssetVersionId, Weak<GpuModel>>;

impl Renderer {
    pub fn prepare(&self, input: &SceneInput) -> Result<RenderScene> {
        self.upload_scene(crate::scene::resolve(input)?, false)
    }
    fn upload_scene(&self, mut resolved: ResolvedScene, retained: bool) -> Result<RenderScene> {
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
        for ((_, key), plan) in resolved.models {
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
            for ((primitive, casts_shadows), transforms) in plan.primitives {
                let capacity = transforms.len();
                let placements: Arc<[crate::scene::Placement]> = if retained {
                    plan.placements[&(primitive, casts_shadows)].clone().into()
                } else {
                    Arc::from([])
                };
                // A reflected instance reverses winding. Split it from ordinary
                // instances so back-face culling remains correct for both.
                for mirrored in [false, true] {
                    let transforms: Vec<_> = transforms
                        .iter()
                        .copied()
                        .filter(|t| (t.normal[0][3] < 0.) == mirrored)
                        .collect();
                    if transforms.is_empty() && !retained {
                        continue;
                    }
                    let mesh = &model.primitives[primitive];
                    let mut centers = Vec::new();
                    let mut minimum = glam::Vec3::splat(f32::INFINITY);
                    let mut maximum = glam::Vec3::splat(f32::NEG_INFINITY);
                    for instance in &transforms {
                        let world = glam::Mat4::from_cols_array_2d(&instance.world);
                        for x in [mesh.minimum.x, mesh.maximum.x] {
                            for y in [mesh.minimum.y, mesh.maximum.y] {
                                for z in [mesh.minimum.z, mesh.maximum.z] {
                                    let point = world.transform_point3(glam::Vec3::new(x, y, z));
                                    if !point.is_finite() {
                                        return Err(ResourceError::GeometryRange.into());
                                    }
                                    minimum = minimum.min(point);
                                    maximum = maximum.max(point);
                                }
                            }
                        }
                        centers.push(world.transform_point3(mesh.center));
                    }
                    let bytes: &[u8] = bytemuck::cast_slice(&transforms);
                    let size = if retained {
                        capacity * std::mem::size_of::<crate::scene::Instance>()
                    } else {
                        bytes.len()
                    };
                    if size as u64 > self.device.limits().max_buffer_size {
                        return Err(ResourceError::BufferSize("scene instances").into());
                    }
                    let instances = self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Model scene instances"),
                        size: size as u64,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    if !bytes.is_empty() {
                        self.queue.write_buffer(&instances, 0, bytes);
                    }
                    batches.push(Arc::new(Batch {
                        model: Arc::clone(&model),
                        primitive,
                        instances,
                        placements: Arc::clone(&placements),
                        updates: if retained {
                            Vec::with_capacity(capacity)
                        } else {
                            Vec::new()
                        },
                        count: transforms.len() as u32,
                        mirrored,
                        centers: if retained {
                            let mut reserved = Vec::with_capacity(capacity);
                            reserved.extend(centers);
                            reserved
                        } else {
                            centers
                        },
                        casts_shadows,
                        bounds: crate::shadows::cascade::Bounds { minimum, maximum },
                    }));
                }
            }
        }
        resolved.stats.model_draw_calls = batches
            .iter()
            .map(|b| {
                if b.count == 0 {
                    0
                } else if b.model.materials.materials[b.model.primitives[b.primitive].material]
                    .alpha
                    == incant_assets_runtime::AlphaMode::Blend
                {
                    b.count as usize
                } else {
                    1
                }
            })
            .sum();
        let lights = crate::lighting::GpuLights::upload(&self.device, resolved.lights);
        if !lights.shadows.is_empty()
            && batches.iter().any(|b| {
                b.casts_shadows
                    && b.model.materials.materials[b.model.primitives[b.primitive].material].alpha
                        == incant_assets_runtime::AlphaMode::Blend
            })
        {
            return Err(ResourceError::TransparentShadowCaster.into());
        }
        let sprites = self.sprites.upload(
            &self.device,
            &self.queue,
            resolved.sprites,
            resolved.tilemaps,
            retained,
        )?;
        Ok(RenderScene {
            sprites,
            lights,
            light_selection: crate::LocalLightSelection::Clustered,
            cameras: resolved.cameras,
            camera: crate::camera::CameraView::preview(),
            diagnostics: resolved.diagnostics,
            batches,
            stats: resolved.stats,
            environment,
        })
    }
}

//! Indexed model versions with immutable geometry and material resources.
use crate::{
    Renderer, ResourceError, Result,
    materials::MaterialResources,
    scene::{ResolvedScene, SceneStats},
};
use incant_assets::{AssetStore, RuntimeAsset, RuntimeAssetData};
use incant_doc::Project;
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
struct Batch {
    model: Arc<GpuModel>,
    primitive: usize,
    instances: wgpu::Buffer,
    count: u32,
    mirrored: bool,
    centers: Vec<glam::Vec3>,
    casts_shadows: bool,
    bounds: crate::shadows::cascade::Bounds,
}
/// Immutable scene ready for rendering. A failed replacement never changes it.
/// Source bytes are not needed after AssetStore has loaded the cooked model.
pub struct RenderScene {
    pub(crate) diagnostics: Vec<glam::Mat4>,
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
            for ((primitive, casts_shadows), transforms) in plan.primitives {
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
                    let bytes = bytemuck::cast_slice(&transforms);
                    if bytes.len() as u64 > self.device.limits().max_buffer_size {
                        return Err(ResourceError::BufferSize("scene instances").into());
                    }
                    batches.push(Arc::new(Batch {
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
                        casts_shadows,
                        bounds: crate::shadows::cascade::Bounds { minimum, maximum },
                    }));
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
        let lights = crate::lighting::GpuLights::upload(&self.device, resolved.lights);
        if !lights.shadows.is_empty()
            && batches.iter().any(|b| {
                b.casts_shadows
                    && b.model.materials.materials[b.model.primitives[b.primitive].material].alpha
                        == incant_assets::AlphaMode::Blend
            })
        {
            return Err(ResourceError::TransparentShadowCaster.into());
        }
        Ok(RenderScene {
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

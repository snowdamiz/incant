//! Retained CPU skinning into the existing vertex/material/shadow path. No WGSL
//! variant or second clock: every pose comes from the caller's committed sample.
use super::*;
use crate::{SceneError, scene::Instance as Transform};
use glam::{DMat4, Vec3};
use incant_assets_runtime::{AssetVersionId, CookedModel, Vertex, animation::AnimationPlayer};
use incant_render_data::{MAX_INSTANCES, MeshInput};

/// Both retained pose copies, instance/geometry rows and four vertex payloads
/// (CPU/GPU × live/staging). Arc/Vec headers within rows count; allocator/Arc
/// allocation headers do not.
/// Immutable model/source payload, driver overhead, draw packets and allocator
/// metadata have separate bounds; this is not a total process-memory bound.
pub const MAX_RETAINED_ANIMATION_BYTES: usize = 128 * 1024 * 1024;
pub(crate) struct Plan {
    input_node: usize,
    version: AssetVersionId,
    world: DMat4,
    cast_shadows: bool,
    player: AnimationPlayer,
    placements: Vec<(usize, usize)>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Binding {
    instance: usize,
    model_node: usize,
    geometry: Option<usize>,
}
pub(super) struct Instance {
    input_node: usize,
    model: Arc<GpuModel>,
    player: AnimationPlayer,
    geometry: Vec<Geometry>,
}
struct Geometry {
    model_node: usize,
    primitive: usize,
    vertices: Vec<Vertex>,
    buffer: wgpu::Buffer,
    minimum: Vec3,
    maximum: Vec3,
    mirrored: bool,
}
fn model(source: &RuntimeAsset) -> &CookedModel {
    let RuntimeAssetData::Model(model) = source.data() else {
        unreachable!("resolved model binding");
    };
    model
}
fn charge(total: &mut usize, count: usize, size: usize) -> std::result::Result<(), SceneError> {
    *total = count
        .checked_mul(size)
        .and_then(|n| total.checked_add(n))
        .filter(|&n| n <= MAX_RETAINED_ANIMATION_BYTES)
        .ok_or(SceneError::Input("retained animation byte limit"))?;
    Ok(())
}
use incant_render_data::animation_sample::sample_committed as sample;
pub(crate) use incant_render_data::animation_sample::validate_ik_capacity;

pub(crate) fn resolve(
    input_node: usize,
    world: DMat4,
    binding: &MeshInput,
    stats: &mut SceneStats,
    bytes: &mut usize,
) -> std::result::Result<Plan, SceneError> {
    let model = model(&binding.source);
    let data = model.animation.as_ref().expect("animated branch");
    let roots = model
        .metadata
        .scenes
        .get(model.metadata.default_scene.unwrap_or(0))
        .ok_or_else(|| SceneError::NoScene(binding.source.info().id.clone()))?;
    // Charge both copies before either player duplicates retained data/poses.
    charge(bytes, 2, AnimationPlayer::retained_bytes(model)?)?;
    charge(bytes, 2, size_of::<Instance>())?;
    let mut visited = vec![false; model.metadata.nodes.len()];
    let mut pending: Vec<_> = roots.iter().rev().copied().collect();
    let mut placements = Vec::new();
    while let Some(node) = pending.pop() {
        if std::mem::replace(&mut visited[node], true) {
            return Err(SceneError::RepeatedNode(binding.source.info().id.clone()));
        }
        for &primitive in &model.metadata.nodes[node].meshes {
            stats.primitive_instances += 1;
            if stats.primitive_instances > MAX_INSTANCES {
                return Err(SceneError::InstanceLimit);
            }
            stats.model_triangles += (model.meshes[primitive].indices.len() / 3) as u64;
            if data.node_skins[node].is_some() {
                charge(
                    bytes,
                    model.meshes[primitive].vertices.len(),
                    4 * size_of::<Vertex>(),
                )?;
            }
            // Both live/staging placements/batches reserve both mirrored slots.
            charge(
                bytes,
                4,
                size_of::<Batch>() + 2 * size_of::<Transform>() + size_of::<Vec3>(),
            )?;
            // Geometry rows reserve all placements (including rigid ones) exactly.
            charge(bytes, 2, size_of::<Geometry>())?;
            placements.push((node, primitive));
        }
        pending.extend(model.metadata.nodes[node].children.iter().rev().copied());
    }
    stats.model_entities += 1;
    let mut player = AnimationPlayer::new(model)?;
    sample(&mut player, binding)?;
    Ok(Plan {
        input_node,
        version: binding.source.version_id(),
        world,
        cast_shadows: binding.cast_shadows,
        player,
        placements,
    })
}
impl Geometry {
    fn fill(
        player: &AnimationPlayer,
        node: usize,
        primitive: usize,
        source: &[Vertex],
        out: &mut [Vertex],
    ) -> Result<(Vec3, Vec3, bool)> {
        Ok(incant_render_data::animation_sample::skin_primitive(
            player, node, primitive, source, out,
        )?)
    }
    fn new(
        renderer: &Renderer,
        player: &AnimationPlayer,
        node: usize,
        primitive: usize,
        source: &[Vertex],
    ) -> Result<Self> {
        let bytes = source
            .len()
            .checked_mul(size_of::<Vertex>())
            .ok_or(ResourceError::BufferSize("animated vertices"))?;
        if bytes as u64 > renderer.device.limits().max_buffer_size {
            return Err(ResourceError::BufferSize("animated vertices").into());
        }
        let mut vertices = vec![[0.; 12]; source.len()];
        let (minimum, maximum, mirrored) =
            Self::fill(player, node, primitive, source, &mut vertices)?;
        let buffer = renderer
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Private animated model vertices"),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        Ok(Self {
            model_node: node,
            primitive,
            vertices,
            buffer,
            minimum,
            maximum,
            mirrored,
        })
    }
}
fn transformed_bounds(
    world: glam::Mat4,
    minimum: Vec3,
    maximum: Vec3,
) -> Result<crate::shadows::cascade::Bounds> {
    let mut bounds = crate::shadows::cascade::Bounds {
        minimum: Vec3::splat(f32::INFINITY),
        maximum: Vec3::splat(f32::NEG_INFINITY),
    };
    for x in [minimum.x, maximum.x] {
        for y in [minimum.y, maximum.y] {
            for z in [minimum.z, maximum.z] {
                let p = world.transform_point3(Vec3::new(x, y, z));
                if !p.is_finite() {
                    return Err(ResourceError::GeometryRange.into());
                }
                bounds.minimum = bounds.minimum.min(p);
                bounds.maximum = bounds.maximum.max(p);
            }
        }
    }
    Ok(bounds)
}
pub(super) fn upload(
    renderer: &Renderer,
    plans: Vec<Plan>,
    models: &BTreeMap<AssetVersionId, Arc<GpuModel>>,
    batches: &mut Vec<Arc<Batch>>,
) -> Result<Vec<Instance>> {
    let mut instances = Vec::with_capacity(plans.len());
    for plan in plans {
        let gpu = Arc::clone(&models[&plan.version]);
        let source = model(&gpu._source);
        let mut geometry = Vec::with_capacity(plan.placements.len());
        for (node, primitive) in plan.placements {
            let mesh = &gpu.primitives[primitive];
            let skinned = source.animation.as_ref().unwrap().node_skins[node].is_some();
            let geometry_index = if skinned {
                let index = geometry.len();
                geometry.push(Geometry::new(
                    renderer,
                    &plan.player,
                    node,
                    primitive,
                    &source.meshes[primitive].vertices,
                )?);
                Some(index)
            } else {
                None
            };
            let world = if skinned {
                plan.world
            } else {
                plan.world * plan.player.globals()[node]
            }
            .as_mat4();
            let transform = Transform::new(world)?;
            let (minimum, maximum) = geometry_index.map_or((mesh.minimum, mesh.maximum), |i| {
                (geometry[i].minimum, geometry[i].maximum)
            });
            let bounds = transformed_bounds(world, minimum, maximum)?;
            let center = world.transform_point3(minimum * 0.5 + maximum * 0.5);
            for mirrored in [false, true] {
                let skin_mirrored = geometry_index.is_some_and(|i| geometry[i].mirrored);
                let visible = ((transform.normal[0][3] < 0.) ^ skin_mirrored) == mirrored;
                let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Private animated model instance"),
                    size: size_of::<Transform>() as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                if visible {
                    renderer
                        .queue
                        .write_buffer(&buffer, 0, bytemuck::bytes_of(&transform));
                }
                let mut updates = Vec::with_capacity(1);
                let mut centers = Vec::with_capacity(1);
                if visible {
                    updates.push(transform);
                    centers.push(center);
                }
                batches.push(Arc::new(Batch {
                    model: Arc::clone(&gpu),
                    primitive,
                    instances: buffer,
                    count: u32::from(visible),
                    mirrored,
                    centers,
                    casts_shadows: plan.cast_shadows,
                    bounds: crate::shadows::cascade::Bounds {
                        minimum: bounds.minimum,
                        maximum: bounds.maximum,
                    },
                    placements: Arc::from([]),
                    updates,
                    animated: Some(Binding {
                        instance: instances.len(),
                        model_node: node,
                        geometry: geometry_index,
                    }),
                    vertices: geometry_index.map(|i| geometry[i].buffer.clone()),
                }));
            }
        }
        instances.push(Instance {
            input_node: plan.input_node,
            model: gpu,
            player: plan.player,
            geometry,
        });
    }
    Ok(instances)
}
pub(super) fn stage(instances: &mut [Instance], input: &SceneInput) -> Result<()> {
    for instance in instances {
        let binding = input.nodes[instance.input_node]
            .mesh
            .as_ref()
            .expect("frozen mesh binding");
        sample(&mut instance.player, binding)?;
        let source = model(&instance.model._source);
        for geometry in &mut instance.geometry {
            let (minimum, maximum, mirrored) = Geometry::fill(
                &instance.player,
                geometry.model_node,
                geometry.primitive,
                &source.meshes[geometry.primitive].vertices,
                &mut geometry.vertices,
            )?;
            geometry.minimum = minimum;
            geometry.maximum = maximum;
            geometry.mirrored = mirrored;
        }
    }
    Ok(())
}
pub(super) fn stage_batch(
    batch: &mut Batch,
    instances: &[Instance],
    input: &SceneInput,
) -> Result<bool> {
    let Some(binding) = batch.animated else {
        return Ok(false);
    };
    let instance = &instances[binding.instance];
    let entity = &input.nodes[instance.input_node];
    let world = if binding.geometry.is_some() {
        entity.world
    } else {
        entity.world * instance.player.globals()[binding.model_node]
    }
    .as_mat4();
    let transform = Transform::new(world)?;
    let skin_mirrored = binding
        .geometry
        .is_some_and(|i| instance.geometry[i].mirrored);
    batch.count = u32::from(((transform.normal[0][3] < 0.) ^ skin_mirrored) == batch.mirrored);
    if batch.count != 0 {
        let mesh = &batch.model.primitives[batch.primitive];
        let (minimum, maximum) = binding.geometry.map_or((mesh.minimum, mesh.maximum), |i| {
            (instance.geometry[i].minimum, instance.geometry[i].maximum)
        });
        batch.bounds = transformed_bounds(world, minimum, maximum)?;
        batch
            .centers
            .push(world.transform_point3(minimum * 0.5 + maximum * 0.5));
        batch.updates.push(transform);
    }
    Ok(true)
}
/// Exact previous sampled geometry uses the accepted live pose, not a resample.
#[cfg(test)]
pub(super) fn motion_test_vertex(binding: Binding, instances: &[Instance]) -> Option<[f32; 12]> {
    binding
        .geometry
        .map(|i| instances[binding.instance].geometry[i].vertices[0])
}
/// Stable logical animation placement key, shared by the handedness twins.
pub(super) fn motion_owner_key(binding: Binding) -> [usize; 3] {
    [
        binding.instance,
        binding.model_node,
        binding.geometry.unwrap_or(usize::MAX),
    ]
}
/// Compare already accepted/staged palettes, not the clip label, time, a hash or
/// a resample. Skinned vertices are model-space, so their exact joint palette is
/// the relevant deformation identity. Unknown palette access is conservative.
pub(super) fn motion_pose_unchanged(
    binding: Binding,
    live: &[Instance],
    staged: &[Instance],
) -> bool {
    let (Some(a), Some(b)) = (live.get(binding.instance), staged.get(binding.instance)) else {
        return false;
    };
    if !Arc::ptr_eq(&a.model, &b.model) {
        return false;
    }
    let equal = |a: DMat4, b: DMat4| {
        a.to_cols_array().map(f64::to_bits) == b.to_cols_array().map(f64::to_bits)
    };
    if binding.geometry.is_none() {
        return equal(
            a.player.globals()[binding.model_node],
            b.player.globals()[binding.model_node],
        );
    }
    let Some(skin) = model(&a.model._source)
        .animation
        .as_ref()
        .and_then(|data| data.node_skins[binding.model_node])
    else {
        return false;
    };
    same_joint_palette(&a.player, &b.player, skin)
}
fn same_joint_palette(a: &AnimationPlayer, b: &AnimationPlayer, skin: usize) -> bool {
    let (Ok(a), Ok(b)) = (a.joint_palette(skin), b.joint_palette(skin)) else {
        return false;
    };
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.to_cols_array().map(f64::to_bits) == b.to_cols_array().map(f64::to_bits)
        })
}
pub(super) fn motion_node(binding: Binding, instances: &[Instance]) -> usize {
    instances[binding.instance].input_node
}
pub(super) fn motion_world(
    binding: Binding,
    instances: &[Instance],
    entity_world: DMat4,
) -> glam::Mat4 {
    if binding.geometry.is_some() {
        entity_world
    } else {
        entity_world * instances[binding.instance].player.globals()[binding.model_node]
    }
    .as_mat4()
}
pub(super) fn write(instances: &[Instance], queue: &wgpu::Queue) {
    for instance in instances {
        for geometry in &instance.geometry {
            queue.write_buffer(
                &geometry.buffer,
                0,
                bytemuck::cast_slice(&geometry.vertices),
            );
        }
    }
}

#[cfg(test)]
#[path = "../animation_tests.rs"]
mod tests;

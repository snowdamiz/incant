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
#[derive(Clone, Copy)]
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
fn checked_sample(
    player: &AnimationPlayer,
    sample: incant_types::AnimationSample,
) -> std::result::Result<incant_assets_runtime::animation::ClipSample, SceneError> {
    if !sample.seconds.is_finite()
        || sample.seconds < 0.
        || (sample.clip.is_none() && sample.seconds != 0.)
    {
        return Err(SceneError::Input("invalid committed animation time"));
    }
    if let Some(clip) = sample.clip {
        let duration = player
            .clip_duration(usize::from(clip))
            .ok_or(SceneError::Input("animation clip index"))?;
        if sample.seconds > f64::from(duration) {
            return Err(SceneError::Input("animation sample exceeds clip duration"));
        }
    }
    Ok(incant_assets_runtime::animation::ClipSample {
        clip: sample.clip.map(usize::from),
        seconds: sample.seconds,
    })
}
fn sample(player: &mut AnimationPlayer, input: &MeshInput) -> std::result::Result<(), SceneError> {
    use incant_assets_runtime::animation::{AnimationPoseError, PoseSource, TwoBoneIk};
    let mut constraints = [TwoBoneIk {
        root: 0,
        middle: 1,
        tip: 2,
        target: [0.; 3],
        pole: [0.; 3],
        weight: 0.,
    }; incant_types::MAX_ANIMATION_IK];
    if let Some(ik) = &input.animation_ik {
        if ik.is_empty() || ik.len() > incant_types::MAX_ANIMATION_IK {
            return Err(SceneError::Input("IK constraint count"));
        }
        for (i, (out, c)) in constraints.iter_mut().zip(ik.iter()).enumerate() {
            c.validate().map_err(SceneError::Input)?;
            if ik[..i].iter().any(|old| old.id == c.id) {
                return Err(SceneError::Input("duplicate IK constraint"));
            }
            *out = TwoBoneIk {
                root: usize::from(c.chain[0]),
                middle: usize::from(c.chain[1]),
                tip: usize::from(c.chain[2]),
                target: c.target,
                pole: c.pole,
                weight: c.weight,
            };
        }
    }
    let constraints = &constraints[..input.animation_ik.as_ref().map_or(0, |ik| ik.len())];
    let pose_error = |error| match error {
        AnimationPoseError::Sample(error) => SceneError::from(error),
        AnimationPoseError::Root => SceneError::Input("unsupported sampled root-removal binding"),
        AnimationPoseError::Ik(_) => SceneError::Input("unsupported or degenerate sampled IK pose"),
    };
    let primary = checked_sample(
        player,
        input
            .animation
            .unwrap_or(incant_types::AnimationSample::REST),
    )?;
    if let Some(pose) = input.animation_pose {
        if input.animation_blend.is_some() || input.animation != Some(pose.representative()) {
            return Err(SceneError::Input("inconsistent grouped animation binding"));
        }
        use incant_assets_runtime::animation::{ClipSample, WeightedClipSample};
        let mut samples = [WeightedClipSample {
            sample: ClipSample {
                clip: None,
                seconds: 0.0,
            },
            weight: 0.0,
        }; incant_types::MAX_ANIMATION_POSE_SAMPLES];
        for (out, entry) in samples.iter_mut().zip(pose.samples()) {
            *out = WeightedClipSample {
                sample: checked_sample(player, entry.sample)?,
                weight: entry.weight,
            };
        }
        use incant_types::PoseInterpolation;
        let first = pose.group(0).expect("retained pose group");
        fn source(kind: PoseInterpolation, samples: &[WeightedClipSample]) -> PoseSource<'_> {
            match kind {
                PoseInterpolation::Clip => PoseSource::Clip(samples[0].sample),
                PoseInterpolation::SphericalPair => PoseSource::SphericalPair {
                    from: samples[0].sample,
                    to: samples[1].sample,
                    weight: samples[1].weight,
                },
                PoseInterpolation::Weighted => PoseSource::Weighted(samples),
            }
        }
        let from = source(first.interpolation, &samples[..first.samples.len()]);
        if let Some(second) = pose.group(1) {
            let to = source(
                second.interpolation,
                &samples[first.samples.len()..pose.samples().len()],
            );
            player
                .sample_pose_pipeline(
                    from,
                    to,
                    pose.outer_weight().expect("two pose groups"),
                    input.animation_root,
                    constraints,
                )
                .map_err(pose_error)?;
        } else {
            player
                .sample_pose_pipeline(from, from, 0.0, input.animation_root, constraints)
                .map_err(pose_error)?;
        }
    } else if let Some(blend) = input.animation_blend {
        // Do not skip invalid outgoing samples at weight one or invalid primary
        // samples at zero. These are complete committed bindings, not hints.
        let outgoing = checked_sample(player, blend.sample)?;
        if !blend.weight.is_finite() || !(0. ..=1.).contains(&blend.weight) {
            return Err(SceneError::Input(
                "invalid committed animation blend weight",
            ));
        }
        player
            .sample_pose_pipeline(
                PoseSource::Clip(outgoing),
                PoseSource::Clip(primary),
                blend.weight,
                input.animation_root,
                constraints,
            )
            .map_err(pose_error)?;
    } else {
        player
            .sample_pose_pipeline(
                PoseSource::Clip(primary),
                PoseSource::Clip(primary),
                0.0,
                input.animation_root,
                constraints,
            )
            .map_err(pose_error)?;
    }

    Ok(())
}
pub(crate) fn validate_ik_capacity(input: &SceneInput) -> std::result::Result<(), SceneError> {
    let mut count = 0usize;
    for node in &input.nodes {
        if let Some(ik) = node.mesh.as_ref().and_then(|m| m.animation_ik.as_ref()) {
            if ik.is_empty() || ik.len() > incant_types::MAX_ANIMATION_IK {
                return Err(SceneError::Input("IK constraint count"));
            }
            count = count
                .checked_add(ik.len())
                .ok_or(SceneError::Input("IK aggregate capacity"))?;
            if count > incant_types::MAX_SCENE_ANIMATION_IK {
                return Err(SceneError::Input("IK aggregate capacity"));
            }
        }
    }
    Ok(())
}
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
        let pose = player.skin_pose(node, primitive)?;
        if pose.vertex_count() != source.len() || out.len() != source.len() {
            return Err(SceneError::Input("skin geometry length changed").into());
        }
        let mut minimum = Vec3::splat(f32::INFINITY);
        let mut maximum = Vec3::splat(f32::NEG_INFINITY);
        let mut mirrored = None;
        for (i, (source, out)) in source.iter().zip(out).enumerate() {
            let (vertex, orientation) = pose.deform_vertex_oriented(i, *source)?;
            if mirrored.is_some_and(|prior| prior != orientation) {
                return Err(SceneError::Input(
                    "mixed skin orientation within primitive is unsupported",
                )
                .into());
            }
            mirrored = Some(orientation);
            *out = vertex;
            let position = Vec3::new(out[0], out[1], out[2]);
            minimum = minimum.min(position);
            maximum = maximum.max(position);
        }
        Ok((minimum, maximum, mirrored.unwrap_or(false)))
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

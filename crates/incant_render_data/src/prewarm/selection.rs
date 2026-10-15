//! Retained CPU selection of the same finite keys used by model_draw. Startup
//! expands hierarchy once; moving ticks resample preallocated animation/skin
//! payloads and placement transforms, without source reads or content hashing.
use crate::{
    MAX_INSTANCES, MeshInput, SceneError, SceneInput,
    animation_sample::{sample_committed, skin_primitive, validate_ik_capacity},
    material_keys::{MaterialKey, MaterialSamples, PlacementTransform},
};
use incant_assets_runtime::{
    AlphaMode, AssetVersionId, RuntimeAsset, RuntimeAssetData, Vertex, animation::AnimationPlayer,
};
use std::{collections::BTreeSet, sync::Arc};

// Logical owned pose/vertex/placement payload, excluding shared immutable cooked
// assets and allocator headers. It is not a total process RSS bound.
const MAX_RETAINED_BYTES: usize = 128 * 1024 * 1024;
struct NodeBinding {
    id: String,
    version: Option<AssetVersionId>,
}
enum Placement {
    Static(glam::DMat4),
    Rigid(usize),
    Skin { node: usize, vertices: Vec<Vertex> },
}
struct Primitive {
    index: usize,
    alpha: AlphaMode,
    double_sided: bool,
    placement: Placement,
}
struct Model {
    node: usize,
    source: Arc<RuntimeAsset>,
    player: Option<AnimationPlayer>,
    primitives: Vec<Primitive>,
}
pub(super) struct Selection {
    bindings: Vec<NodeBinding>,
    camera: usize,
    models: Vec<Model>,
    receipts: Vec<[u8; 32]>,
}
fn charge(bytes: &mut usize, count: usize, size: usize) -> Result<(), SceneError> {
    *bytes = count
        .checked_mul(size)
        .and_then(|value| bytes.checked_add(value))
        .filter(|&value| value <= MAX_RETAINED_BYTES)
        .ok_or(SceneError::Input("material recording retained byte limit"))?;
    Ok(())
}
fn static_binding(binding: &MeshInput) -> Result<(), SceneError> {
    if binding.animation.is_some()
        || binding.animation_blend.is_some()
        || binding.animation_pose.is_some()
        || binding.animation_ik.is_some()
        || binding.animation_root.is_some()
    {
        return Err(SceneError::Input(
            "animation sample requires animated model",
        ));
    }
    Ok(())
}
impl Selection {
    pub(super) fn new(input: &SceneInput, camera: &str) -> Result<Self, SceneError> {
        validate_ik_capacity(input)?;
        if input.nodes.len() > 1_000_000 {
            return Err(SceneError::Input("node limit"));
        }
        let camera = input
            .nodes
            .iter()
            .position(|node| node.id == camera && node.camera.is_some())
            .ok_or_else(|| SceneError::MissingCamera(camera.into()))?;
        let mut bytes = 0;
        charge(&mut bytes, input.nodes.len(), size_of::<NodeBinding>())?;
        let mut ids = BTreeSet::new();
        let mut bindings = Vec::with_capacity(input.nodes.len());
        let mut models = Vec::new();
        let mut receipts = BTreeSet::new();
        let mut primitive_count = 0;
        for (index, node) in input.nodes.iter().enumerate() {
            if node.id.is_empty() || node.id.len() > 256 || !ids.insert(&node.id) {
                return Err(SceneError::Input("empty, oversized or duplicate node ID"));
            }
            charge(&mut bytes, node.id.len(), 1)?;
            bindings.push(NodeBinding {
                id: node.id.clone(),
                version: node.mesh.as_ref().map(|mesh| mesh.source.version_id()),
            });
            let Some(binding) = &node.mesh else { continue };
            if node.sprite.is_some() || node.tilemap.is_some() {
                return Err(SceneError::Input(
                    "ambiguous mesh, sprite or tilemap binding",
                ));
            }
            let RuntimeAssetData::Model(model) = binding.source.data() else {
                return Err(SceneError::MissingModel(binding.source.info().id.clone()));
            };
            let receipt = binding
                .source
                .model_content_sha256()
                .ok_or(SceneError::Input(
                    "model recording requires a sealed cooked-content receipt",
                ))?;
            receipts.insert(receipt);
            if receipts.len() > super::MAX_MODEL_RECEIPTS {
                return Err(SceneError::Input("material recording model receipt limit"));
            }
            let roots = model
                .metadata
                .scenes
                .get(model.metadata.default_scene.unwrap_or(0))
                .ok_or_else(|| SceneError::NoScene(binding.source.info().id.clone()))?;
            let player = if model.animation.is_some() {
                charge(&mut bytes, 1, AnimationPlayer::retained_bytes(model)?)?;
                Some(AnimationPlayer::new(model)?)
            } else {
                static_binding(binding)?;
                None
            };
            let mut pending: Vec<_> = roots
                .iter()
                .rev()
                .map(|&node| (node, glam::DMat4::IDENTITY))
                .collect();
            let mut visited = vec![false; model.metadata.nodes.len()];
            let mut primitives = Vec::new();
            while let Some((node_index, parent)) = pending.pop() {
                if std::mem::replace(&mut visited[node_index], true) {
                    return Err(SceneError::RepeatedNode(binding.source.info().id.clone()));
                }
                let node = &model.metadata.nodes[node_index];
                let local = parent
                    * glam::DMat4::from_cols_array_2d(
                        &node.transform.map(|column| column.map(f64::from)),
                    );
                for &primitive in &node.meshes {
                    primitive_count += 1;
                    if primitive_count > MAX_INSTANCES {
                        return Err(SceneError::InstanceLimit);
                    }
                    charge(&mut bytes, 1, size_of::<Primitive>())?;
                    let material = model.metadata.mesh_materials[primitive]
                        .map(|index| &model.materials[index]);
                    let placement = if let Some(animation) = &model.animation {
                        if animation.node_skins[node_index].is_some() {
                            let count = model.meshes[primitive].vertices.len();
                            charge(&mut bytes, count, size_of::<Vertex>())?;
                            Placement::Skin {
                                node: node_index,
                                vertices: vec![[0.; 12]; count],
                            }
                        } else {
                            Placement::Rigid(node_index)
                        }
                    } else {
                        Placement::Static(local)
                    };
                    primitives.push(Primitive {
                        index: primitive,
                        alpha: material.map_or(AlphaMode::Opaque, |m| m.alpha_mode),
                        double_sided: material.is_some_and(|m| m.double_sided),
                        placement,
                    });
                }
                pending.extend(node.children.iter().rev().map(|&child| (child, local)));
            }
            charge(&mut bytes, 1, size_of::<Model>())?;
            models.push(Model {
                node: index,
                source: Arc::clone(&binding.source),
                player,
                primitives,
            });
        }
        Ok(Self {
            bindings,
            camera,
            models,
            receipts: receipts.into_iter().collect(),
        })
    }
    pub(super) fn receipts(&self) -> &[[u8; 32]] {
        &self.receipts
    }
    pub(super) fn matches(&self, input: &SceneInput, camera: &str) -> bool {
        self.bindings.len() == input.nodes.len()
            && input.nodes[self.camera].id == camera
            && input.nodes[self.camera].camera.is_some()
            && self.bindings.iter().zip(&input.nodes).all(|(old, node)| {
                old.id == node.id
                    && old.version == node.mesh.as_ref().map(|mesh| mesh.source.version_id())
            })
    }
    pub(super) fn keys(
        &mut self,
        input: &SceneInput,
        samples: MaterialSamples,
    ) -> Result<u32, SceneError> {
        validate_ik_capacity(input)?;
        let samples = if matches!(
            input.nodes[self.camera].camera.as_ref().unwrap().projection,
            incant_types::CameraProjection::PixelPerfect { .. }
        ) {
            MaterialSamples::One
        } else {
            samples
        };
        let mut keys = 0;
        for model in &mut self.models {
            let node = &input.nodes[model.node];
            if !node.world.is_finite() || node.world.row(3) != glam::DVec4::W {
                return Err(SceneError::TransformRange);
            }
            let binding = node.mesh.as_ref().expect("matched immutable model binding");
            if node.sprite.is_some() || node.tilemap.is_some() {
                return Err(SceneError::Input(
                    "ambiguous mesh, sprite or tilemap binding",
                ));
            }
            let RuntimeAssetData::Model(source) = model.source.data() else {
                unreachable!("checked model binding");
            };
            if let Some(player) = &mut model.player {
                sample_committed(player, binding)?;
            } else {
                static_binding(binding)?;
            }
            for primitive in &mut model.primitives {
                let (world, skin_mirrored) = match &mut primitive.placement {
                    Placement::Static(local) => (node.world * *local, false),
                    Placement::Rigid(index) => (
                        node.world * model.player.as_ref().unwrap().globals()[*index],
                        false,
                    ),
                    Placement::Skin {
                        node: skin,
                        vertices,
                    } => {
                        let (_, _, mirrored) = skin_primitive(
                            model.player.as_ref().unwrap(),
                            *skin,
                            primitive.index,
                            &source.meshes[primitive.index].vertices,
                            vertices,
                        )?;
                        (node.world, mirrored)
                    }
                };
                let mirrored = PlacementTransform::new(world.as_mat4())?.mirrored() ^ skin_mirrored;
                let key =
                    MaterialKey::new(primitive.alpha, primitive.double_sided, mirrored, samples);
                keys |= 1 << key.index();
            }
        }
        Ok(keys)
    }
}

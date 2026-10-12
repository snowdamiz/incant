use crate::{AssetError, Mesh, ModelNode, Result, invalid};
use gltf::{
    animation::{Interpolation as GltfInterpolation, Property as GltfProperty, util::ReadOutputs},
    scene::Transform,
};
use incant_assets_runtime::animation::*;

pub(crate) fn read(
    doc: &gltf::Document,
    buffers: &[Vec<u8>],
    nodes: &[ModelNode],
    meshes: &[Mesh],
    influences: Vec<Vec<Influences>>,
) -> Result<Option<AnimationData>> {
    if doc.skins().len() == 0 && doc.animations().len() == 0 {
        if influences.iter().any(|s| !s.is_empty()) {
            return Err(invalid("skin attributes without a skin"));
        }
        return Ok(None);
    }
    preflight(doc)?;
    let rest = doc
        .nodes()
        .map(|n| match n.transform() {
            Transform::Matrix { .. } => None,
            Transform::Decomposed {
                translation,
                rotation,
                scale,
            } => Some(NodeTrs {
                translation,
                rotation,
                scale,
            }),
        })
        .collect();
    let node_skins = doc.nodes().map(|n| n.skin().map(|s| s.index())).collect();
    let mut skins = Vec::with_capacity(doc.skins().len());
    for skin in doc.skins() {
        let joints: Vec<_> = skin.joints().map(|j| j.index()).collect();
        let inverse_bind = match skin
            .reader(|b| Some(buffers[b.index()].as_slice()))
            .read_inverse_bind_matrices()
        {
            None => vec![glam::Mat4::IDENTITY.to_cols_array_2d(); joints.len()],
            Some(it) => {
                if it.len() < joints.len() {
                    return Err(invalid("inverse bind matrix count"));
                }
                it.take(joints.len()).collect()
            }
        };
        skins.push(Skin {
            name: skin.name().map(str::to_owned),
            skeleton: skin.skeleton().map(|n| n.index()),
            joints,
            inverse_bind,
        });
    }
    let mut clips = Vec::with_capacity(doc.animations().len());
    for clip in doc.animations() {
        let mut channels = Vec::new();
        for channel in clip.channels() {
            let property = match channel.target().property() {
                GltfProperty::Translation => Property::Translation,
                GltfProperty::Rotation => Property::Rotation,
                GltfProperty::Scale => Property::Scale,
                GltfProperty::MorphTargetWeights => {
                    return Err(AssetError::Unsupported("glTF morph animation".into()));
                }
            };
            let interpolation = match channel.sampler().interpolation() {
                GltfInterpolation::Linear => Interpolation::Linear,
                GltfInterpolation::Step => Interpolation::Step,
                GltfInterpolation::CubicSpline => {
                    return Err(AssetError::Unsupported("glTF CUBICSPLINE animation".into()));
                }
            };
            let reader = channel.reader(|b| Some(buffers[b.index()].as_slice()));
            let times = reader
                .read_inputs()
                .ok_or_else(|| invalid("invalid animation inputs"))?
                .collect();
            let values = match reader
                .read_outputs()
                .ok_or_else(|| invalid("invalid animation outputs"))?
            {
                ReadOutputs::Translations(it) if property == Property::Translation => {
                    it.map(|v| [v[0], v[1], v[2], 0.]).collect()
                }
                ReadOutputs::Scales(it) if property == Property::Scale => {
                    it.map(|v| [v[0], v[1], v[2], 0.]).collect()
                }
                ReadOutputs::Rotations(it) if property == Property::Rotation => {
                    it.into_f32().collect()
                }
                _ => return Err(invalid("animation output type mismatch")),
            };
            channels.push(Channel {
                node: channel.target().node().index(),
                property,
                interpolation,
                times,
                values,
            });
        }
        clips.push(Clip {
            name: clip.name().map(str::to_owned),
            channels,
        });
    }
    let data = AnimationData {
        rest,
        skins,
        node_skins,
        influences,
        clips,
    };
    let scenes: Vec<_> = doc
        .scenes()
        .map(|s| s.nodes().map(|n| n.index()).collect())
        .collect();
    data.validate(nodes, meshes, &scenes)?;
    Ok(Some(data))
}
fn checked_name(name: Option<&str>) -> Result<Option<&str>> {
    if name.is_some_and(|s| s.len() > MAX_NAME_BYTES) {
        return Err(AssetError::Limit("animation name"));
    }
    Ok(name)
}
fn charge(budget: &mut usize, n: usize) -> Result<()> {
    *budget = budget
        .checked_add(n)
        .filter(|&n| n <= MAX_ANIMATION_BYTES)
        .ok_or(AssetError::Limit("animation bytes"))?;
    Ok(())
}
/// glTF supports four influences in this initial route. Integer normalized
/// weights are converted then normalized once; zero/negative/nonfinite refuse.
pub(crate) fn weights(
    primitive: &gltf::Primitive<'_>,
    buffers: &[Vec<u8>],
    n: usize,
) -> Result<Vec<Influences>> {
    let reader = primitive.reader(|b| Some(buffers[b.index()].as_slice()));
    match (reader.read_joints(0), reader.read_weights(0)) {
        (None, None) => Ok(vec![]),
        (Some(joints), Some(weights)) => {
            let joints = joints.into_u16();
            let weights = weights.into_f32();
            if joints.len() != n || weights.len() != n {
                return Err(invalid("skin attribute counts differ"));
            }
            joints
                .zip(weights)
                .map(|(joints, mut weights)| {
                    let sum: f64 = weights.iter().map(|&w| f64::from(w)).sum();
                    if weights.iter().any(|w| !w.is_finite() || *w < 0. || *w > 1.)
                        || !sum.is_finite()
                        || sum <= 0.
                    {
                        return Err(invalid("invalid skin weights"));
                    }
                    for w in &mut weights {
                        *w = (f64::from(*w) / sum) as f32;
                    }
                    Ok(Influences { joints, weights })
                })
                .collect()
        }
        _ => Err(invalid("JOINTS_0 and WEIGHTS_0 must occur together")),
    }
}
pub(crate) fn expand(weights: &mut Vec<Influences>, indices: &[u32]) -> Result<()> {
    if weights.is_empty() {
        return Ok(());
    }
    if indices.len() > crate::MAX_VERTICES {
        return Err(AssetError::Limit("expanded influences"));
    }
    *weights = indices.iter().map(|&i| weights[i as usize]).collect();
    Ok(())
}

pub(crate) fn preflight(doc: &gltf::Document) -> Result<()> {
    if doc.scenes().len() > MAX_NODES {
        return Err(AssetError::Limit("animation scenes"));
    }
    let mut references = 0usize;
    for node in doc.nodes() {
        checked_name(node.name())?;
        let meshes = node.mesh().map_or(0, |m| m.primitives().len());
        references = references
            .checked_add(node.children().len())
            .and_then(|v| v.checked_add(meshes))
            .ok_or(AssetError::Limit("animation node references"))?;
        if references > MAX_NODE_REFERENCES {
            return Err(AssetError::Limit("animation node references"));
        }
    }
    for scene in doc.scenes() {
        references = references
            .checked_add(scene.nodes().len())
            .ok_or(AssetError::Limit("animation node references"))?;
        if references > MAX_NODE_REFERENCES {
            return Err(AssetError::Limit("animation node references"));
        }
    }
    // Reserve the aggregate encoded footprint before allocating channel/joint data.
    let mut budget = 28usize + doc.nodes().len() * 45;
    let mut influences = 0usize;
    for mesh in doc.meshes() {
        for p in mesh.primitives() {
            charge(&mut budget, 4)?;
            if p.get(&gltf::Semantic::Joints(0)).is_some()
                || p.get(&gltf::Semantic::Weights(0)).is_some()
            {
                let n = p.get(&gltf::Semantic::Positions).map_or(0, |a| a.count());
                let expanded = p.get(&gltf::Semantic::Normals).is_none()
                    || (p.get(&gltf::Semantic::Tangents).is_none()
                        && p.material().normal_texture().is_some());
                let n = if expanded {
                    p.indices().map_or(n, |a| a.count())
                } else {
                    n
                };
                influences = influences
                    .checked_add(n)
                    .ok_or(AssetError::Limit("influences"))?;
                if influences > crate::MAX_VERTICES {
                    return Err(AssetError::Limit("influences"));
                }
                charge(&mut budget, n * 24)?;
            }
        }
    }
    let (mut joints, mut channels, mut keys) = (0, 0, 0);
    for skin in doc.skins() {
        if let Some(a) = skin.inverse_bind_matrices() {
            float_accessor(&a, gltf::accessor::Dimensions::Mat4)?;
        }
        let n = skin.joints().len();
        joints += n;
        if n > MAX_JOINTS_PER_SKIN || joints > MAX_JOINTS {
            return Err(AssetError::Limit("skin joints"));
        }
        charge(
            &mut budget,
            12 + checked_name(skin.name())?.map_or(0, str::len) + n * 68,
        )?;
    }
    for clip in doc.animations() {
        charge(
            &mut budget,
            8 + checked_name(clip.name())?.map_or(0, str::len),
        )?;
        for channel in clip.channels() {
            float_accessor(
                &channel.sampler().input(),
                gltf::accessor::Dimensions::Scalar,
            )?;
            let dimensions = match channel.target().property() {
                GltfProperty::Translation | GltfProperty::Scale => gltf::accessor::Dimensions::Vec3,
                GltfProperty::Rotation => gltf::accessor::Dimensions::Vec4,
                GltfProperty::MorphTargetWeights => {
                    return Err(AssetError::Unsupported("glTF morph animation".into()));
                }
            };
            float_accessor(&channel.sampler().output(), dimensions)?;
            channels += 1;
            let n = channel.sampler().input().count();
            keys += n;
            if channels > MAX_CHANNELS || keys > MAX_KEYS {
                return Err(AssetError::Limit("animation channels/keys"));
            }
            if channel.sampler().output().count() != n {
                return Err(AssetError::Unsupported(
                    "animation output count/interpolation".into(),
                ));
            }
            charge(&mut budget, 10 + n * 20)?;
            if channel.sampler().interpolation() == GltfInterpolation::CubicSpline {
                return Err(AssetError::Unsupported("glTF CUBICSPLINE animation".into()));
            }
        }
    }
    Ok(())
}

// gltf's convenience readers assume these semantic types and include unreachable!
// arms. Generic JSON/accessor bounds validation is not sufficient for that contract.
pub(crate) fn validate_primitive(primitive: &gltf::Primitive<'_>) -> Result<()> {
    use gltf::{
        Semantic,
        accessor::{DataType as T, Dimensions as D},
    };
    for (semantic, a) in primitive.attributes() {
        let valid = match semantic {
            Semantic::Positions | Semantic::Normals => {
                a.dimensions() == D::Vec3 && a.data_type() == T::F32 && !a.normalized()
            }
            Semantic::Tangents => {
                a.dimensions() == D::Vec4 && a.data_type() == T::F32 && !a.normalized()
            }
            Semantic::TexCoords(0) => a.dimensions() == D::Vec2 && weight_component(&a),
            Semantic::Joints(0) => {
                a.dimensions() == D::Vec4
                    && matches!(a.data_type(), T::U8 | T::U16)
                    && !a.normalized()
            }
            Semantic::Weights(0) => a.dimensions() == D::Vec4 && weight_component(&a),
            _ => false,
        };
        if !valid {
            return Err(invalid(format!(
                "invalid {semantic:?} accessor type/normalization"
            )));
        }
    }
    if let Some(a) = primitive.indices()
        && (a.dimensions() != D::Scalar
            || !matches!(a.data_type(), T::U8 | T::U16 | T::U32)
            || a.normalized())
    {
        return Err(invalid("invalid index accessor type/normalization"));
    }
    Ok(())
}
fn weight_component(a: &gltf::Accessor<'_>) -> bool {
    use gltf::accessor::DataType as T;
    match a.data_type() {
        T::F32 => !a.normalized(),
        T::U8 | T::U16 => a.normalized(),
        _ => false,
    }
}
fn float_accessor(a: &gltf::Accessor<'_>, dimensions: gltf::accessor::Dimensions) -> Result<()> {
    if a.data_type() != gltf::accessor::DataType::F32
        || a.dimensions() != dimensions
        || a.normalized()
    {
        return Err(invalid("invalid animation accessor type/normalization"));
    }
    Ok(())
}

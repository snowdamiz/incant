//! Typed, bounded glTF skeletal and TRS animation data. This is CPU data/sampling;
//! static render and navigation consumers do not yet implement animated models.
mod binary;
mod player;
use crate::{AssetError, Mesh, ModelNode, Result, invalid};
pub use binary::decode_animation;
use glam::{DMat4, DQuat, DVec3};
pub use player::AnimationPlayer;
use std::collections::BTreeSet;

pub const MAX_ANIMATION_BYTES: usize = 32 * 1024 * 1024;
pub const MAX_NODES: usize = 4096;
pub const MAX_SKINS: usize = 256;
pub const MAX_JOINTS_PER_SKIN: usize = 256;
pub const MAX_JOINTS: usize = 4096;
pub const MAX_CLIPS: usize = 256;
pub const MAX_CHANNELS: usize = 8192;
pub const MAX_KEYS: usize = 1_000_000;
pub const MAX_NAME_BYTES: usize = 1024;
pub const MAX_NODE_REFERENCES: usize = 65_536;

#[derive(Debug, Clone, PartialEq)]
pub struct NodeTrs {
    pub translation: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}
impl NodeTrs {
    pub(crate) fn matrix(&self) -> DMat4 {
        DMat4::from_scale_rotation_translation(
            DVec3::from_array(self.scale.map(f64::from)),
            DQuat::from_array(self.rotation.map(f64::from)).normalize(),
            DVec3::from_array(self.translation.map(f64::from)),
        )
    }
}
#[derive(Debug, Clone, PartialEq)]
pub struct Skin {
    pub name: Option<String>,
    pub skeleton: Option<usize>,
    pub joints: Vec<usize>,
    pub inverse_bind: Vec<[[f32; 4]; 4]>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Influences {
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interpolation {
    Linear,
    Step,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Property {
    Translation,
    Rotation,
    Scale,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    pub node: usize,
    pub property: Property,
    pub interpolation: Interpolation,
    pub times: Vec<f32>,
    /// Rotation is XYZW; translation/scale have a canonical zero fourth lane.
    pub values: Vec<[f32; 4]>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Clip {
    pub name: Option<String>,
    pub channels: Vec<Channel>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationData {
    /// None retains the node's static matrix. Such nodes cannot be clip targets.
    pub rest: Vec<Option<NodeTrs>>,
    pub skins: Vec<Skin>,
    pub node_skins: Vec<Option<usize>>,
    /// One stream per flattened model primitive; empty means no skin attributes.
    pub influences: Vec<Vec<Influences>>,
    pub clips: Vec<Clip>,
}
fn cap(n: usize, max: usize, what: &'static str) -> Result<()> {
    if n > max {
        Err(AssetError::Limit(what))
    } else {
        Ok(())
    }
}
fn name(v: &Option<String>) -> Result<()> {
    cap(
        v.as_ref().map_or(0, String::len),
        MAX_NAME_BYTES,
        "animation name",
    )
}
pub(crate) fn affine(m: &[[f32; 4]; 4]) -> bool {
    m.iter().flatten().all(|f| f.is_finite())
        && m[0][3] == 0.
        && m[1][3] == 0.
        && m[2][3] == 0.
        && m[3][3] == 1.
}
pub(crate) fn quaternion(v: &[f32; 4]) -> bool {
    let norm: f64 = v.iter().map(|&f| f64::from(f).powi(2)).sum();
    v.iter().all(|f| f.is_finite()) && (norm - 1.).abs() <= 0.002
}
pub(crate) fn matrix(m: &[[f32; 4]; 4]) -> DMat4 {
    DMat4::from_cols_array_2d(&m.map(|c| c.map(f64::from)))
}
impl AnimationData {
    /// Shared importer/decoder/startup validation. Public mutable cooked structs
    /// are never treated as already validated by the retained AnimationPlayer.
    pub fn validate(
        &self,
        nodes: &[ModelNode],
        meshes: &[Mesh],
        scenes: &[Vec<usize>],
    ) -> Result<()> {
        cap(nodes.len(), MAX_NODES, "animation nodes")?;
        cap(meshes.len(), 10_000, "animation primitives")?;
        cap(scenes.len(), MAX_NODES, "animation scenes")?;
        let mut references = 0usize;
        for n in nodes {
            references = references
                .checked_add(n.children.len())
                .and_then(|v| v.checked_add(n.meshes.len()))
                .ok_or(AssetError::Limit("animation node references"))?;
            name(&n.name)?;
        }
        for s in scenes {
            references = references
                .checked_add(s.len())
                .ok_or(AssetError::Limit("animation node references"))?;
        }
        cap(references, MAX_NODE_REFERENCES, "animation node references")?;
        cap(self.skins.len(), MAX_SKINS, "skins")?;
        cap(self.clips.len(), MAX_CLIPS, "clips")?;
        if self.skins.is_empty() && self.clips.is_empty() {
            return Err(invalid("empty animation section"));
        }
        if self.rest.len() != nodes.len()
            || self.node_skins.len() != nodes.len()
            || self.influences.len() != meshes.len()
        {
            return Err(invalid("animation table lengths"));
        }
        crate::model_node::validate_nodes(nodes)?;
        let mut parents = vec![None; nodes.len()];
        for (i, node) in nodes.iter().enumerate() {
            if !affine(&node.transform) || node.meshes.iter().any(|&m| m >= meshes.len()) {
                return Err(invalid("animated model node matrix/reference"));
            }
            if self.rest[i].is_none() {
                let m = matrix(&node.transform);
                let columns = [
                    m.x_axis.truncate(),
                    m.y_axis.truncate(),
                    m.z_axis.truncate(),
                ];
                for a in 0..3 {
                    for b in a + 1..3 {
                        let bound = columns[a].length() * columns[b].length();
                        if bound > 0. && columns[a].dot(columns[b]).abs() > bound * 1e-5 {
                            return Err(AssetError::Unsupported(
                                "sheared matrix in animated model".into(),
                            ));
                        }
                    }
                }
            }
            for &child in &node.children {
                parents[child] = Some(i);
            }
            if let Some(trs) = &self.rest[i] {
                if !quaternion(&trs.rotation)
                    || trs
                        .translation
                        .iter()
                        .chain(&trs.scale)
                        .any(|v| !v.is_finite())
                {
                    return Err(invalid("invalid animation rest TRS"));
                }
                // Prevent the metadata and typed rest pose from disagreeing.
                let a = trs.matrix().to_cols_array();
                let b = matrix(&node.transform).to_cols_array();
                if a.iter()
                    .zip(b)
                    .any(|(a, b)| (a - b).abs() > 1e-5 * (1. + b.abs()))
                {
                    return Err(invalid("rest TRS differs from node matrix"));
                }
            }
        }
        for scene in scenes {
            if scene
                .iter()
                .any(|&r| r >= nodes.len() || parents[r].is_some())
            {
                return Err(invalid("animation scene root"));
            }
        }
        let root = |mut n: usize| {
            while let Some(p) = parents[n] {
                n = p;
            }
            n
        };
        let mut joints = 0usize;
        for skin in &self.skins {
            name(&skin.name)?;
            cap(skin.joints.len(), MAX_JOINTS_PER_SKIN, "skin joints")?;
            joints = joints
                .checked_add(skin.joints.len())
                .ok_or(AssetError::Limit("total joints"))?;
            cap(joints, MAX_JOINTS, "total joints")?;
            if skin.joints.is_empty()
                || skin.joints.len() != skin.inverse_bind.len()
                || skin.joints.iter().any(|&j| j >= nodes.len())
                || skin.skeleton.is_some_and(|s| s >= nodes.len())
                || skin.inverse_bind.iter().any(|m| !affine(m))
            {
                return Err(invalid("invalid skin joint/bind matrix"));
            }
            let unique: BTreeSet<_> = skin.joints.iter().copied().collect();
            if unique.len() != skin.joints.len()
                || skin.joints.iter().any(|&j| root(j) != root(skin.joints[0]))
            {
                return Err(invalid("skin joints need one hierarchy and unique nodes"));
            }
            if let Some(s) = skin.skeleton {
                for &j in &skin.joints {
                    let mut n = j;
                    while n != s {
                        n = parents[n]
                            .ok_or_else(|| invalid("skeleton is not a joint ancestor"))?;
                    }
                }
            }
        }
        let (mut vertices, mut indices) = (0usize, 0usize);
        for mesh in meshes {
            vertices = vertices
                .checked_add(mesh.vertices.len())
                .ok_or(AssetError::Limit("total cached geometry"))?;
            indices = indices
                .checked_add(mesh.indices.len())
                .ok_or(AssetError::Limit("total cached geometry"))?;
            if vertices > crate::MAX_VERTICES || indices > crate::MAX_INDICES {
                return Err(AssetError::Limit("total cached geometry"));
            }
        }
        let mut maximum_joints = Vec::with_capacity(meshes.len());
        let mut count = 0usize;
        for (m, stream) in meshes.iter().zip(&self.influences) {
            m.validate()?;
            count = count
                .checked_add(stream.len())
                .ok_or(AssetError::Limit("influences"))?;
            cap(count, crate::MAX_VERTICES, "influences")?;
            if !stream.is_empty() && stream.len() != m.vertices.len() {
                return Err(invalid("influence/vertex count mismatch"));
            }
            let mut maximum = 0usize;
            for v in stream {
                for &j in &v.joints {
                    maximum = maximum.max(j as usize);
                }
                let sum: f64 = v.weights.iter().map(|&w| f64::from(w)).sum();
                if v.weights.iter().any(|w| !w.is_finite() || *w < 0.) || (sum - 1.).abs() > 1e-5 {
                    return Err(invalid("invalid skin weights"));
                }
            }
            maximum_joints.push(maximum);
        }
        for (i, skin) in self.node_skins.iter().enumerate() {
            if let Some(s) = skin {
                let skin = self
                    .skins
                    .get(*s)
                    .ok_or_else(|| invalid("node skin reference"))?;
                if nodes[i].meshes.is_empty() {
                    return Err(invalid("skin node has no mesh"));
                }
                for &m in &nodes[i].meshes {
                    let stream = &self.influences[m];
                    if stream.is_empty() || maximum_joints[m] >= skin.joints.len() {
                        return Err(invalid("skin vertex joint reference"));
                    }
                }
                // Each referencing scene must also contain the complete joint tree.
                for scene in scenes {
                    if scene.contains(&root(i)) && !scene.contains(&root(skin.joints[0])) {
                        return Err(invalid("skin joints absent from referencing scene"));
                    }
                }
            }
        }
        let (mut channels, mut keys) = (0usize, 0usize);
        for clip in &self.clips {
            name(&clip.name)?;
            if clip.channels.is_empty() {
                return Err(invalid("empty animation clip"));
            }
            channels = channels
                .checked_add(clip.channels.len())
                .ok_or(AssetError::Limit("channels"))?;
            cap(channels, MAX_CHANNELS, "channels")?;
            let mut targets = BTreeSet::new();
            for ch in &clip.channels {
                keys = keys
                    .checked_add(ch.times.len())
                    .ok_or(AssetError::Limit("keys"))?;
                cap(keys, MAX_KEYS, "keys")?;
                if ch.node >= nodes.len()
                    || self.rest[ch.node].is_none()
                    || !targets.insert((ch.node, ch.property))
                {
                    return Err(invalid("duplicate/invalid/matrix animation target"));
                }
                if ch.times.is_empty()
                    || ch.times.len() != ch.values.len()
                    || ch.times.iter().any(|t| !t.is_finite() || *t < 0.)
                    || ch.times.windows(2).any(|t| t[0] >= t[1])
                {
                    return Err(invalid(
                        "animation times must be finite, nonnegative, strictly increasing",
                    ));
                }
                for v in &ch.values {
                    if v.iter().any(|f| !f.is_finite())
                        || (ch.property == Property::Rotation && !quaternion(v))
                        || (ch.property != Property::Rotation && v[3] != 0.)
                    {
                        return Err(invalid("invalid animation value"));
                    }
                }
            }
        }
        self.encoded_len()?;
        Ok(())
    }
}

impl AnimationData {
    /// Checked serialized footprint, also used before authoring writer allocation.
    pub fn encoded_len(&self) -> Result<usize> {
        let mut n = 8usize;
        let mut add = |count: usize, stride: usize| -> Result<()> {
            n = count
                .checked_mul(stride)
                .and_then(|v| n.checked_add(v))
                .filter(|&v| v <= MAX_ANIMATION_BYTES)
                .ok_or(AssetError::Limit("animation bytes"))?;
            Ok(())
        };
        add(5, 4)?;
        add(self.rest.len(), 1)?;
        add(self.rest.iter().filter(|t| t.is_some()).count(), 40)?;
        add(self.node_skins.len(), 4)?;
        for s in &self.skins {
            add(1, 12)?;
            add(s.name.as_ref().map_or(0, String::len), 1)?;
            add(s.joints.len(), 68)?;
        }
        for stream in &self.influences {
            add(1, 4)?;
            add(stream.len(), 24)?;
        }
        for clip in &self.clips {
            add(1, 8)?;
            add(clip.name.as_ref().map_or(0, String::len), 1)?;
            for ch in &clip.channels {
                add(1, 10)?;
                add(ch.times.len(), 20)?;
            }
        }
        Ok(n)
    }
}

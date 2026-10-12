use super::*;
use crate::CookedModel;
#[path = "deform.rs"]
mod deform;
pub use deform::SkinPose;
#[path = "weighted.rs"]
mod weighted;
pub use weighted::{MAX_POSE_SAMPLES, WeightedClipSample};
#[path = "grouped.rs"]
mod grouped;
pub use grouped::PoseSource;

/// One retained, immutable model animation copy and two preallocated pose buffers.
/// Sampling performs no allocation, source IO or JSON. There is no implicit clock:
/// finite seconds clamp independently to each channel's authored key interval.
/// Module is CPU-only; it makes no cross-device bit-exact or performance claim.
pub struct AnimationPlayer {
    data: AnimationData,
    nodes: Vec<ModelNode>,
    parents: Vec<Option<usize>>,
    order: Vec<usize>,
    offsets: Vec<usize>,
    durations: Vec<f32>,
    live: Pose,
    staged: Pose,
    trs: Vec<Option<NodeTrs>>,
    blend_trs: Vec<Option<NodeTrs>>,
    group_trs: Vec<Option<NodeTrs>>,
}
struct Pose {
    local: Vec<DMat4>,
    global: Vec<DMat4>,
    joints: Vec<DMat4>,
}
impl Pose {
    fn new(nodes: usize, joints: usize) -> Self {
        Self {
            local: vec![DMat4::IDENTITY; nodes],
            global: vec![DMat4::IDENTITY; nodes],
            joints: vec![DMat4::IDENTITY; joints],
        }
    }
}
impl AnimationPlayer {
    /// Logical owned allocation payload for one player, excluding allocator
    /// headers and validation scratch. Counts cloned keys/topology as well as
    /// both pose buffers and three TRS scratch arrays. Callers can charge instances before constructing
    /// them. Shared source pixels/geometry are not copied by a player.
    pub fn retained_bytes(model: &CookedModel) -> Result<usize> {
        let data = model
            .animation
            .as_ref()
            .ok_or_else(|| invalid("model has no animation data"))?;
        let mut bytes = data.retained_bytes()?;
        let mut charge = |count: usize, size: usize| -> Result<()> {
            bytes = count
                .checked_mul(size)
                .and_then(|n| bytes.checked_add(n))
                .ok_or(AssetError::Limit("retained animation bytes"))?;
            Ok(())
        };
        let nodes = model.metadata.nodes.len();
        charge(nodes, size_of::<ModelNode>())?;
        for node in &model.metadata.nodes {
            charge(node.children.len(), size_of::<usize>())?;
            charge(node.meshes.len(), size_of::<usize>())?;
            charge(node.name.as_ref().map_or(0, String::len), 1)?;
        }
        charge(
            nodes,
            size_of::<Option<usize>>() + size_of::<usize>() + 3 * size_of::<Option<NodeTrs>>(),
        )?;
        charge(
            data.skins
                .len()
                .checked_add(1)
                .ok_or(AssetError::Limit("retained animation bytes"))?,
            size_of::<usize>(),
        )?;
        charge(data.clips.len(), size_of::<f32>())?;
        charge(nodes, 4 * size_of::<DMat4>())?;
        for skin in &data.skins {
            charge(skin.joints.len(), 2 * size_of::<DMat4>())?;
        }
        Ok(bytes)
    }
    pub fn new(model: &CookedModel) -> Result<Self> {
        let data = model
            .animation
            .as_ref()
            .ok_or_else(|| invalid("model has no animation data"))?;
        data.validate(&model.metadata.nodes, &model.meshes, &model.metadata.scenes)?;
        let nodes = model.metadata.nodes.clone();
        let mut parents = vec![None; nodes.len()];
        for (i, node) in nodes.iter().enumerate() {
            for &c in &node.children {
                parents[c] = Some(i);
            }
        }
        let mut order = Vec::with_capacity(nodes.len());
        order.extend(
            parents
                .iter()
                .enumerate()
                .filter_map(|(i, p)| p.is_none().then_some(i)),
        );
        let mut cursor = 0;
        while cursor < order.len() {
            let i = order[cursor];
            order.extend_from_slice(&nodes[i].children);
            cursor += 1;
        }
        let mut offsets = Vec::with_capacity(data.skins.len() + 1);
        offsets.push(0);
        for skin in &data.skins {
            offsets.push(offsets.last().unwrap() + skin.joints.len());
        }
        let count = *offsets.last().unwrap();
        let mut out = Self {
            data: data.clone(),
            trs: data.rest.clone(),
            blend_trs: data.rest.clone(),
            group_trs: data.rest.clone(),
            live: Pose::new(nodes.len(), count),
            staged: Pose::new(nodes.len(), count),
            nodes,
            parents,
            order,
            offsets,
            durations: data
                .clips
                .iter()
                .map(|clip| {
                    clip.channels.iter().fold(0.0f32, |duration, channel| {
                        duration.max(*channel.times.last().expect("validated keys"))
                    })
                })
                .collect(),
        };
        out.sample(None, 0.)?;
        Ok(out)
    }
    /// Canonical duration from the last key of every channel, retained at startup.
    pub fn clip_duration(&self, clip: usize) -> Option<f32> {
        self.durations.get(clip).copied()
    }
    pub fn clips(&self) -> &[Clip] {
        &self.data.clips
    }
    pub fn globals(&self) -> &[DMat4] {
        &self.live.global
    }
    pub fn locals(&self) -> &[DMat4] {
        &self.live.local
    }
    /// Model-space matrices: global joint * inverse bind. glTF ignores the mesh
    /// node transform for skinning. Non-joint ancestors are included in globals.
    pub fn joint_palette(&self, skin: usize) -> Result<&[DMat4]> {
        if skin >= self.data.skins.len() {
            return Err(invalid("skin index"));
        }
        Ok(&self.live.joints[self.offsets[skin]..self.offsets[skin + 1]])
    }
    /// Convert a palette to mesh-local space into a caller-owned buffer. A renderer
    /// using this must apply the mesh global afterward. Singular transforms refuse.
    /// Preflight precedes all output writes, preserving the caller buffer on error.
    pub fn mesh_local_palette(&self, node: usize, out: &mut [DMat4]) -> Result<()> {
        let skin = self
            .data
            .node_skins
            .get(node)
            .copied()
            .flatten()
            .ok_or_else(|| invalid("node has no skin"))?;
        let palette = self.joint_palette(skin)?;
        if out.len() != palette.len() {
            return Err(invalid("palette output length"));
        }
        let global = self.live.global[node];
        if global.determinant() == 0. {
            return Err(invalid("singular mesh transform"));
        }
        let inverse = global.inverse();
        if !inverse.is_finite() || palette.iter().any(|m| !(inverse * *m).is_finite()) {
            return Err(invalid("nonfinite mesh-local palette"));
        }
        for (out, m) in out.iter_mut().zip(palette) {
            *out = inverse * *m;
        }
        Ok(())
    }
    /// CPU reference linear-blend position, in model space. Position is supplied
    /// by the caller from the matching cooked primitive; this is not a GPU path.
    pub fn skin_position(
        &self,
        node: usize,
        primitive: usize,
        vertex: usize,
        position: [f32; 3],
    ) -> Result<[f64; 3]> {
        if position.iter().any(|v| !v.is_finite())
            || !self
                .nodes
                .get(node)
                .is_some_and(|n| n.meshes.contains(&primitive))
        {
            return Err(invalid("skin position arguments"));
        }
        let skin = self.data.node_skins[node].ok_or_else(|| invalid("node has no skin"))?;
        let weights = self
            .data
            .influences
            .get(primitive)
            .and_then(|s| s.get(vertex))
            .ok_or_else(|| invalid("skin vertex index"))?;
        let palette = self.joint_palette(skin)?;
        let p = DVec3::from_array(position.map(f64::from));
        let mut out = DVec3::ZERO;
        for i in 0..4 {
            out += palette[weights.joints[i] as usize].transform_point3(p)
                * f64::from(weights.weights[i]);
        }
        if !out.is_finite() {
            return Err(invalid("nonfinite skinned position"));
        }
        Ok(out.to_array())
    }
    /// None evaluates the authored rest pose. Invalid clip/time or nonfinite
    /// composed values leave the previously published complete pose unchanged.
    pub fn sample(&mut self, clip: Option<usize>, seconds: f64) -> Result<()> {
        let sample = ClipSample { clip, seconds };
        sample.validate(&self.data)?;
        sampling::sample(&self.data, sample, &mut self.trs);
        self.publish()
    }
    /// Blend two independently sampled clips in local TRS space, then compose
    /// the complete hierarchy and palettes. Translation and scale interpolate
    /// linearly; rotation uses normalized shortest-arc spherical interpolation.
    /// Weight is finite and in [0,1]. Both clocks/clip indices are validated even
    /// at exact endpoints; endpoints preserve the single-sample result exactly.
    /// Any refusal preserves every previously published pose/palette. This is a
    /// two-input runtime primitive, not an authored graph or transition clock.
    pub fn sample_blend(&mut self, from: ClipSample, to: ClipSample, weight: f64) -> Result<()> {
        from.validate(&self.data)?;
        to.validate(&self.data)?;
        if !weight.is_finite() || !(0. ..=1.).contains(&weight) {
            return Err(invalid("animation blend weight"));
        }
        if weight == 0. {
            return self.sample(from.clip, from.seconds);
        }
        if weight == 1. {
            return self.sample(to.clip, to.seconds);
        }
        sampling::sample(&self.data, from, &mut self.trs);
        sampling::sample(&self.data, to, &mut self.blend_trs);
        sampling::blend(&mut self.trs, &self.blend_trs, weight);
        self.publish()
    }
    fn publish(&mut self) -> Result<()> {
        for &i in &self.order {
            let local = self.trs[i]
                .as_ref()
                .map_or_else(|| matrix(&self.nodes[i].transform), NodeTrs::matrix);
            let global = self.parents[i].map_or(local, |p| self.staged.global[p] * local);
            if !local.is_finite() || !global.is_finite() {
                return Err(invalid("nonfinite composed animation pose"));
            }
            self.staged.local[i] = local;
            self.staged.global[i] = global;
        }
        for (s, skin) in self.data.skins.iter().enumerate() {
            for (j, (&node, bind)) in skin.joints.iter().zip(&skin.inverse_bind).enumerate() {
                let value = self.staged.global[node] * matrix(bind);
                if !value.is_finite() {
                    return Err(invalid("nonfinite joint palette"));
                }
                self.staged.joints[self.offsets[s] + j] = value;
            }
        }
        std::mem::swap(&mut self.live, &mut self.staged);
        Ok(())
    }
}

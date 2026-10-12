use super::*;
use crate::CookedModel;

/// One retained, immutable model animation copy and two preallocated pose buffers.
/// `sample` performs no allocation, source IO or JSON. It has no implicit clock:
/// finite seconds clamp independently to each channel's authored key interval.
/// Module is CPU-only; it makes no cross-device bit-exact or performance claim.
pub struct AnimationPlayer {
    data: AnimationData,
    nodes: Vec<ModelNode>,
    parents: Vec<Option<usize>>,
    order: Vec<usize>,
    offsets: Vec<usize>,
    live: Pose,
    staged: Pose,
    trs: Vec<Option<NodeTrs>>,
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
        let mut order: Vec<_> = parents
            .iter()
            .enumerate()
            .filter_map(|(i, p)| p.is_none().then_some(i))
            .collect();
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
            live: Pose::new(nodes.len(), count),
            staged: Pose::new(nodes.len(), count),
            nodes,
            parents,
            order,
            offsets,
        };
        out.sample(None, 0.)?;
        Ok(out)
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
        if !seconds.is_finite() {
            return Err(invalid("nonfinite animation time"));
        }
        let clip = clip
            .map(|i| self.data.clips.get(i).ok_or_else(|| invalid("clip index")))
            .transpose()?;
        self.trs.clone_from_slice(&self.data.rest);
        if let Some(clip) = clip {
            for ch in &clip.channels {
                let upper = ch.times.partition_point(|&t| f64::from(t) <= seconds);
                let a = upper.saturating_sub(1).min(ch.times.len() - 1);
                let b = upper.min(ch.times.len() - 1);
                let alpha = if a == b || ch.interpolation == Interpolation::Step {
                    0.
                } else {
                    ((seconds - f64::from(ch.times[a])) / f64::from(ch.times[b] - ch.times[a]))
                        .clamp(0., 1.)
                };
                let first = ch.values[a];
                let next = ch.values[b];
                let trs = self.trs[ch.node].as_mut().unwrap();
                match ch.property {
                    Property::Rotation => {
                        let q = DQuat::from_array(first.map(f64::from)).normalize();
                        let r = DQuat::from_array(next.map(f64::from)).normalize();
                        trs.rotation = q.slerp(r, alpha).normalize().to_array().map(|v| v as f32);
                    }
                    Property::Translation | Property::Scale => {
                        let value = std::array::from_fn(|i| {
                            (f64::from(first[i]) * (1. - alpha) + f64::from(next[i]) * alpha) as f32
                        });
                        if ch.property == Property::Translation {
                            trs.translation = value;
                        } else {
                            trs.scale = value;
                        }
                    }
                }
            }
        }
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

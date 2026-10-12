use super::*;
use incant_types::{
    MAX_ANIMATION_BLEND_POINTS, MAX_ANIMATION_CLIPS, MAX_ANIMATION_GRAPH_PARAMETERS,
    MAX_ANIMATION_GRAPH_VALUE,
};

pub(super) fn validate(
    nodes: &[NativeBlendTreeNode],
    parameters: &[NativeAnimationParameter],
    durations: &[f32],
) -> Result<(), SceneError> {
    let error = SceneError::Invalid;
    let bounded = |v: f64| v.is_finite() && v.abs() <= MAX_ANIMATION_GRAPH_VALUE;
    if nodes.is_empty()
        || nodes.len() > MAX_BLEND_TREE_NODES
        || parameters.len() > MAX_ANIMATION_GRAPH_PARAMETERS
        || parameters.windows(2).any(|p| p[0].id >= p[1].id)
        || parameters.iter().any(|p| {
            !bounded(p.min)
                || !bounded(p.max)
                || !bounded(p.initial)
                || p.min > p.max
                || p.initial < p.min
                || p.initial > p.max
        })
        || durations.len() > MAX_ANIMATION_CLIPS
        || durations
            .iter()
            .any(|n| !n.is_finite() || *n < 0.0 || n.to_bits() == (-0.0f32).to_bits())
    {
        return Err(error("animation blend tree tables"));
    }
    let mut depths = [0usize; MAX_BLEND_TREE_NODES];
    let mut poses = [0usize; MAX_BLEND_TREE_NODES];
    let mut referenced = [false; MAX_BLEND_TREE_NODES];
    for (index, node) in nodes.iter().enumerate() {
        let mut depth = 1;
        let mut child = |n: u8| -> Result<usize, SceneError> {
            let n = usize::from(n);
            if n >= index {
                return Err(error("animation blend tree child order"));
            }
            referenced[n] = true;
            depth = depth.max(depths[n] + 1);
            Ok(poses[n])
        };
        let axis = |parameter: u8, values: &[f64]| -> Result<(), SceneError> {
            let p = parameters
                .get(usize::from(parameter))
                .ok_or(error("animation blend tree parameter"))?;
            if !(2..=MAX_ANIMATION_BLEND_POINTS).contains(&values.len())
                || values
                    .iter()
                    .any(|v| !bounded(*v) || *v < p.min || *v > p.max)
                || values.windows(2).any(|p| p[0] >= p[1])
            {
                return Err(error("animation blend tree axis"));
            }
            Ok(())
        };
        let count = match node {
            NativeBlendTreeNode::Clip(clip) => {
                if clip.is_some_and(|clip| usize::from(clip) >= durations.len()) {
                    return Err(error("animation blend tree clip"));
                }
                1
            }
            NativeBlendTreeNode::Blend1d { parameter, points } => {
                if !(2..=MAX_ANIMATION_BLEND_POINTS).contains(&points.len()) {
                    return Err(error("animation blend tree point count"));
                }
                let mut values = [0.0; MAX_ANIMATION_BLEND_POINTS];
                let mut counts = [0; MAX_ANIMATION_BLEND_POINTS];
                for (slot, p) in points.iter().enumerate() {
                    values[slot] = p.threshold;
                    counts[slot] = child(p.child)?;
                }
                axis(*parameter, &values[..points.len()])?;
                counts[..points.len()]
                    .windows(2)
                    .map(|p| p[0] + p[1])
                    .max()
                    .unwrap_or(0)
            }
            NativeBlendTreeNode::Blend2d {
                parameters,
                x,
                y,
                children,
            } => {
                if parameters[0] == parameters[1] {
                    return Err(error("animation blend tree distinct axes"));
                }
                axis(parameters[0], x)?;
                axis(parameters[1], y)?;
                if children.len() != x.len() * y.len() {
                    return Err(error("animation blend tree grid shape"));
                }
                let mut counts = [0; MAX_ANIMATION_BLEND_POINTS * MAX_ANIMATION_BLEND_POINTS];
                for (slot, n) in children.iter().enumerate() {
                    counts[slot] = child(*n)?;
                }
                let mut max = 0;
                for row in 0..y.len() - 1 {
                    for col in 0..x.len() - 1 {
                        let i = row * x.len() + col;
                        max = max.max(
                            counts[i]
                                + counts[i + 1]
                                + counts[i + x.len()]
                                + counts[i + x.len() + 1],
                        );
                    }
                }
                max
            }
        };
        if depth > MAX_BLEND_TREE_DEPTH || count > MAX_BLEND_TREE_POSES {
            return Err(error("animation blend tree depth or active pose capacity"));
        }
        depths[index] = depth;
        poses[index] = count;
    }
    // Topological ordering guarantees that every referenced node reaches the
    // last node if and only if no earlier node is an unreferenced island.
    if referenced[..nodes.len() - 1].iter().any(|used| !used) {
        return Err(error("animation blend tree unreachable node"));
    }
    Ok(())
}

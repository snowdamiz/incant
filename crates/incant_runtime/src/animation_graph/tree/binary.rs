//! v15 tree node table. Counts are checked before allocating or reading arrays.
use super::*;
use crate::binary::Reader;
use incant_types::MAX_ANIMATION_BLEND_POINTS;

pub(crate) fn size(tree: &NativeAnimationBlendTree) -> usize {
    1 + tree
        .nodes()
        .iter()
        .map(|node| match node {
            NativeBlendTreeNode::Clip(_) => 3,
            NativeBlendTreeNode::Blend1d { points, .. } => 3 + 9 * points.len(),
            NativeBlendTreeNode::Blend2d { x, y, children, .. } => {
                5 + 8 * (x.len() + y.len()) + children.len()
            }
        })
        .sum::<usize>()
}
pub(crate) fn write(out: &mut Vec<u8>, tree: &NativeAnimationBlendTree) {
    out.push(tree.nodes().len() as u8);
    for node in tree.nodes() {
        match node {
            NativeBlendTreeNode::Clip(clip) => {
                out.push(0);
                out.extend(clip.unwrap_or(u16::MAX).to_le_bytes());
            }
            NativeBlendTreeNode::Blend1d { parameter, points } => {
                out.extend([1, *parameter, points.len() as u8]);
                for point in points.iter() {
                    out.extend(point.threshold.to_le_bytes());
                    out.push(point.child);
                }
            }
            NativeBlendTreeNode::Blend2d {
                parameters,
                x,
                y,
                children,
            } => {
                out.extend([
                    2,
                    parameters[0],
                    parameters[1],
                    x.len() as u8,
                    y.len() as u8,
                ]);
                for v in x.iter().chain(y.iter()) {
                    out.extend(v.to_le_bytes());
                }
                out.extend(children.iter());
            }
        }
    }
}
pub(crate) fn read(
    r: &mut Reader<'_>,
    parameters: &[NativeAnimationParameter],
    durations: &[f32],
) -> Result<NativeAnimationBlendTree, SceneError> {
    let count = usize::from(r.byte()?);
    if !(1..=MAX_BLEND_TREE_NODES).contains(&count) || count > r.remaining_len() / 3 {
        return Err(SceneError::Invalid("animation tree node count"));
    }
    let mut nodes = Vec::with_capacity(count);
    for _ in 0..count {
        nodes.push(match r.byte()? {
            0 => {
                let clip = u16::from_le_bytes(r.array()?);
                NativeBlendTreeNode::Clip((clip != u16::MAX).then_some(clip))
            }
            1 => {
                let parameter = r.byte()?;
                let n = usize::from(r.byte()?);
                if !(2..=MAX_ANIMATION_BLEND_POINTS).contains(&n) || n > r.remaining_len() / 9 {
                    return Err(SceneError::Invalid("animation tree point count"));
                }
                let mut points = Vec::with_capacity(n);
                for _ in 0..n {
                    let [threshold] = r.floats()?;
                    points.push(NativeBlendTreePoint {
                        threshold,
                        child: r.byte()?,
                    });
                }
                NativeBlendTreeNode::Blend1d {
                    parameter,
                    points: points.into(),
                }
            }
            2 => {
                let parameters = r.array()?;
                let nx = usize::from(r.byte()?);
                let ny = usize::from(r.byte()?);
                if !(2..=MAX_ANIMATION_BLEND_POINTS).contains(&nx)
                    || !(2..=MAX_ANIMATION_BLEND_POINTS).contains(&ny)
                    || (nx + ny) * 8 + nx * ny > r.remaining_len()
                {
                    return Err(SceneError::Invalid("animation tree grid counts"));
                }
                let mut x = Vec::with_capacity(nx);
                let mut y = Vec::with_capacity(ny);
                for _ in 0..nx {
                    let [v] = r.floats()?;
                    x.push(v);
                }
                for _ in 0..ny {
                    let [v] = r.floats()?;
                    y.push(v);
                }
                NativeBlendTreeNode::Blend2d {
                    parameters,
                    x: x.into(),
                    y: y.into(),
                    children: Arc::from(r.bytes(nx * ny)?),
                }
            }
            _ => return Err(SceneError::Invalid("animation tree node tag")),
        });
    }
    NativeAnimationBlendTree::new(nodes, Arc::from(parameters), Arc::from(durations))
}

use super::*;
use incant_runtime::{NativeAnimationBlendTree, NativeBlendTreeNode, NativeBlendTreePoint};
use incant_types::{AnimationTree, AnimationTreeNode};

pub(super) fn compile(
    tree: &AnimationTree,
    graph: &AnimationGraph,
    parameters: &BTreeMap<&str, u8>,
    native_parameters: &[NativeAnimationParameter],
    durations: &[f32],
) -> Result<NativeAnimationBlendTree, CookError> {
    let order = tree
        .compile_order(&graph.parameters)
        .map_err(incant_runtime::SceneError::Invalid)?;
    let indices: BTreeMap<_, _> = order
        .iter()
        .enumerate()
        .map(|(i, id)| (*id, i as u8))
        .collect();
    let nodes = order
        .into_iter()
        .map(|id| match &tree.nodes[id] {
            AnimationTreeNode::Clip { clip } => NativeBlendTreeNode::Clip(*clip),
            AnimationTreeNode::Blend1d { parameter, points } => NativeBlendTreeNode::Blend1d {
                parameter: parameters[parameter.as_str()],
                points: points
                    .iter()
                    .map(|p| NativeBlendTreePoint {
                        threshold: p.threshold,
                        child: indices[p.node.as_str()],
                    })
                    .collect(),
            },
            AnimationTreeNode::Blend2d {
                parameters: axes,
                x,
                y,
                children,
            } => NativeBlendTreeNode::Blend2d {
                parameters: axes.each_ref().map(|id| parameters[id.as_str()]),
                x: x.clone().into(),
                y: y.clone().into(),
                children: children.iter().map(|id| indices[id.as_str()]).collect(),
            },
        })
        .collect();
    Ok(NativeAnimationBlendTree::new(
        nodes,
        Arc::from(native_parameters),
        Arc::from(durations),
    )?)
}

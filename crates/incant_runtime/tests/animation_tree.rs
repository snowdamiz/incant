use incant_runtime::*;
use std::sync::Arc;

fn parameters() -> Arc<[NativeAnimationParameter]> {
    (0..3)
        .map(|n| NativeAnimationParameter {
            id: StableId([n; 16]),
            initial: 0.0,
            min: -2.0,
            max: 2.0,
        })
        .collect::<Vec<_>>()
        .into()
}
fn line(parameter: u8, children: [u8; 2]) -> NativeBlendTreeNode {
    NativeBlendTreeNode::Blend1d {
        parameter,
        points: Arc::from([
            NativeBlendTreePoint {
                threshold: 0.0,
                child: children[0],
            },
            NativeBlendTreePoint {
                threshold: 1.0,
                child: children[1],
            },
        ]),
    }
}
fn grid(children: [u8; 4]) -> NativeBlendTreeNode {
    NativeBlendTreeNode::Blend2d {
        parameters: [0, 1],
        x: Arc::from([0.0, 1.0]),
        y: Arc::from([0.0, 1.0]),
        children: Arc::from(children),
    }
}
fn nested_nodes() -> Vec<NativeBlendTreeNode> {
    let mut nodes: Vec<_> = (0..8).map(|n| NativeBlendTreeNode::Clip(Some(n))).collect();
    nodes.extend((0..4).map(|n| line(2, [n * 2, n * 2 + 1])));
    nodes.push(grid([8, 9, 10, 11]));
    nodes
}
fn tree(nodes: Vec<NativeBlendTreeNode>) -> Result<NativeAnimationBlendTree, SceneError> {
    NativeAnimationBlendTree::new(
        nodes,
        parameters(),
        Arc::from([1., 2., 3., 4., 5., 6., 7., 8.]),
    )
}
fn weight(pose: &AnimationPoseSamples, clip: Option<u16>) -> f64 {
    pose.samples()
        .iter()
        .filter(|p| p.sample.clip == clip)
        .map(|p| p.weight)
        .sum()
}
fn near(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-14, "{actual} != {expected}");
}

#[test]
fn nested_bilinear_and_linear_weights_match_independent_tensor_product() {
    let tree = tree(nested_nodes()).unwrap();
    let pose = tree.sample(&[0.25, 0.75, 0.4], 0.5).unwrap();
    assert_eq!(pose.samples().len(), 8);
    // Row-major grid with one low/high Z pair at each corner. This tensor
    // product is independent of the evaluator's topological propagation.
    let expected = [0.1125, 0.075, 0.0375, 0.025, 0.3375, 0.225, 0.1125, 0.075];
    for (clip, expected) in expected.into_iter().enumerate() {
        near(weight(&pose, Some(clip as u16)), expected);
        let sample = pose
            .samples()
            .iter()
            .find(|p| p.sample.clip == Some(clip as u16))
            .unwrap();
        assert_eq!(sample.sample.seconds, (clip + 1) as f64 * 0.5);
    }
    near(pose.samples().iter().map(|p| p.weight).sum(), 1.0);
    assert_eq!(pose, tree.sample(&[0.25, 0.75, 0.4], 0.5).unwrap());
}

#[test]
fn corners_and_outside_grid_clamp_exactly_without_extrapolation() {
    let tree = tree(nested_nodes()).unwrap();
    for (x, y, z, clip) in [
        (0., 0., 0., 0),
        (1., 0., 1., 3),
        (0., 1., 0., 4),
        (1., 1., 1., 7),
        (-2., 2., 2., 5),
    ] {
        let pose = tree.sample(&[x, y, z], 1.).unwrap();
        assert_eq!(
            pose.samples(),
            &[WeightedAnimationSample {
                sample: AnimationSample {
                    clip: Some(clip),
                    seconds: f64::from(clip + 1)
                },
                weight: 1.0,
            }]
        );
    }
}

#[test]
fn shared_children_and_duplicate_clips_coalesce_before_pose_sampling() {
    let tree = tree(vec![
        NativeBlendTreeNode::Clip(None),
        NativeBlendTreeNode::Clip(Some(0)),
        grid([0, 0, 1, 1]),
    ])
    .unwrap();
    let pose = tree.sample(&[0.3, 0.25, 0.], 0.5).unwrap();
    assert_eq!(pose.samples().len(), 2);
    near(weight(&pose, None), 0.75);
    near(weight(&pose, Some(0)), 0.25);
    assert_eq!(
        pose.samples()
            .iter()
            .find(|p| p.sample.clip.is_none())
            .unwrap()
            .sample
            .seconds,
        0.0
    );
    let duplicates = super_tree_duplicates();
    let pose = duplicates.sample(&[0.3, 0., 0.], 0.5).unwrap();
    assert_eq!(pose.samples().len(), 1);
    assert_eq!(pose.samples()[0].weight, 1.0);
}
fn super_tree_duplicates() -> NativeAnimationBlendTree {
    tree(vec![
        NativeBlendTreeNode::Clip(Some(0)),
        NativeBlendTreeNode::Clip(Some(0)),
        line(0, [0, 1]),
    ])
    .unwrap()
}

#[test]
fn piecewise_axes_select_only_adjacent_children_and_have_continuous_weights() {
    let points = [-1.0, 0.25, 1.5]
        .into_iter()
        .enumerate()
        .map(|(n, threshold)| NativeBlendTreePoint {
            threshold,
            child: n as u8,
        })
        .collect::<Vec<_>>();
    let tree = tree(vec![
        NativeBlendTreeNode::Clip(Some(0)),
        NativeBlendTreeNode::Clip(Some(1)),
        NativeBlendTreeNode::Clip(Some(2)),
        NativeBlendTreeNode::Blend1d {
            parameter: 0,
            points: points.into(),
        },
    ])
    .unwrap();
    for (v, expected) in [
        (-2., [1., 0., 0.]),
        (-0.375, [0.5, 0.5, 0.]),
        (0.25, [0., 1., 0.]),
        (0.875, [0., 0.5, 0.5]),
        (2., [0., 0., 1.]),
    ] {
        let pose = tree.sample(&[v, 0., 0.], 0.2).unwrap();
        for (clip, expected) in expected.into_iter().enumerate() {
            near(weight(&pose, Some(clip as u16)), expected);
        }
    }
}

#[test]
fn fades_retain_sixteen_distinct_clip_phases_and_exact_endpoints() {
    let tree = tree(nested_nodes()).unwrap();
    let source = tree.sample(&[0.5; 3], 0.25).unwrap();
    let destination = tree.sample(&[0.5; 3], 0.75).unwrap();
    let mixed = source.blend(&destination, 0.25).unwrap();
    assert_eq!(mixed.samples().len(), 16);
    for sample in mixed.samples() {
        let duration = f64::from(sample.sample.clip.unwrap() + 1);
        assert_eq!(
            sample.weight,
            if sample.sample.seconds == duration * 0.25 {
                0.09375
            } else {
                0.03125
            }
        );
    }
    assert_eq!(source.blend(&destination, 0.).unwrap(), source);
    assert_eq!(source.blend(&destination, 1.).unwrap(), destination);
    assert_eq!(source.blend(&source, 0.25).unwrap(), source);
    let third = tree.sample(&[0.5; 3], 0.5).unwrap();
    assert!(mixed.blend(&third, 0.5).is_err());
    assert_eq!(mixed.samples().len(), 16); // failure did not mutate either input
    for weight in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert!(source.blend(&destination, weight).is_err());
    }
}

#[test]
fn invalid_samples_are_refused_without_changing_the_immutable_tree() {
    let tree = tree(nested_nodes()).unwrap();
    let original = tree.clone();
    for phase in [-1., 1.1, f64::INFINITY, f64::NAN] {
        assert!(tree.sample(&[0.; 3], phase).is_err());
    }
    for values in [
        vec![],
        vec![0.; 2],
        vec![0.; 4],
        vec![f64::NAN, 0., 0.],
        vec![0., f64::INFINITY, 0.],
        vec![0., 0., 3.],
    ] {
        assert!(tree.sample(&values, 0.5).is_err());
    }
    assert_eq!(tree, original);
    assert!(
        tree.sample(&[0.; 3], -0.0)
            .unwrap()
            .samples()
            .iter()
            .all(|s| s.sample.seconds.to_bits() == 0)
    );
}

#[test]
fn malformed_topology_model_axes_and_capacity_fail_at_admission() {
    assert!(tree(vec![]).is_err());
    assert!(tree(vec![NativeBlendTreeNode::Clip(Some(8))]).is_err());
    assert!(
        tree(vec![
            NativeBlendTreeNode::Clip(Some(0));
            MAX_BLEND_TREE_NODES + 1
        ])
        .is_err()
    );
    assert!(
        tree(vec![
            NativeBlendTreeNode::Clip(Some(0)),
            NativeBlendTreeNode::Clip(Some(1))
        ])
        .is_err()
    ); // island
    assert!(tree(vec![NativeBlendTreeNode::Clip(Some(0)), line(0, [0, 1])]).is_err()); // self
    assert!(tree(vec![line(0, [1, 1]), NativeBlendTreeNode::Clip(Some(0))]).is_err()); // forward
    assert!(tree(vec![NativeBlendTreeNode::Clip(Some(0)), line(3, [0, 0])]).is_err());
    for thresholds in [[0., 0.], [1., 0.], [0., f64::NAN], [-3., 1.]] {
        let points = thresholds
            .into_iter()
            .map(|threshold| NativeBlendTreePoint {
                threshold,
                child: 0,
            })
            .collect::<Vec<_>>();
        assert!(
            tree(vec![
                NativeBlendTreeNode::Clip(Some(0)),
                NativeBlendTreeNode::Blend1d {
                    parameter: 0,
                    points: points.into()
                }
            ])
            .is_err()
        );
    }
    for (axes, x, y, children) in [
        ([0, 0], vec![0., 1.], vec![0., 1.], vec![0; 4]),
        ([0, 1], vec![0.], vec![0., 1.], vec![0; 2]),
        ([0, 1], vec![0., 1.], vec![0., 1.], vec![0; 3]),
    ] {
        assert!(
            tree(vec![
                NativeBlendTreeNode::Clip(None),
                NativeBlendTreeNode::Blend2d {
                    parameters: axes,
                    x: x.into(),
                    y: y.into(),
                    children: children.into()
                }
            ])
            .is_err()
        );
    }
    // Both children can be eight-pose trees: refuse before a sixteen-pose
    // state could overflow the two-state fade's fixed capacity.
    let mut too_many = nested_nodes();
    too_many.push(line(0, [12, 12]));
    assert!(tree(too_many).is_err());
}

#[test]
fn parameter_and_duration_tables_are_finite_bounded_and_canonical() {
    for duration in [-0.0, -1., f32::NAN, f32::INFINITY] {
        assert!(
            NativeAnimationBlendTree::new(
                vec![NativeBlendTreeNode::Clip(Some(0))],
                parameters(),
                Arc::from([duration])
            )
            .is_err()
        );
    }
    let zero = NativeAnimationBlendTree::new(
        vec![NativeBlendTreeNode::Clip(Some(0))],
        Arc::from([]),
        Arc::from([0.]),
    )
    .unwrap();
    assert_eq!(
        zero.sample(&[], 1.).unwrap().samples()[0].sample.seconds,
        0.
    );
    let rest = NativeAnimationBlendTree::new(
        vec![NativeBlendTreeNode::Clip(None)],
        Arc::from([]),
        Arc::from([]),
    )
    .unwrap();
    assert_eq!(
        rest.sample(&[], 1.).unwrap().samples()[0].sample,
        AnimationSample::REST
    );
    for modification in 0..5 {
        let mut p = parameters().to_vec();
        match modification {
            0 => p[1].id = p[0].id,
            1 => p.reverse(),
            2 => p[0].max = f64::INFINITY,
            3 => p[0].initial = 3.,
            _ => p[0].min = 3.,
        }
        assert!(
            NativeAnimationBlendTree::new(
                vec![NativeBlendTreeNode::Clip(None)],
                p.into(),
                Arc::from([])
            )
            .is_err()
        );
    }
}

#[test]
fn nonuniform_rectangular_grid_uses_the_correct_cell_and_row_stride() {
    let mut nodes = (0..6)
        .map(|n| NativeBlendTreeNode::Clip(Some(n)))
        .collect::<Vec<_>>();
    nodes.push(NativeBlendTreeNode::Blend2d {
        parameters: [0, 1],
        x: Arc::from([-1.0, 0.0, 2.0]),
        y: Arc::from([-2.0, 1.0]),
        children: Arc::from([0, 1, 2, 3, 4, 5]),
    });
    let pose = tree(nodes).unwrap().sample(&[1., -0.5, 0.], 0.5).unwrap();
    assert_eq!(pose.samples().len(), 4);
    for clip in 0..6 {
        assert_eq!(
            weight(&pose, Some(clip)),
            if [1, 2, 4, 5].contains(&clip) {
                0.25
            } else {
                0.0
            }
        );
    }
}

#[test]
fn full_node_table_and_maximum_depth_have_bounded_active_samples() {
    let mut nodes = (0..MAX_BLEND_TREE_NODES - 1)
        .map(|n| NativeBlendTreeNode::Clip(Some((n % 8) as u16)))
        .collect::<Vec<_>>();
    let axis: Arc<[f64]> = (0..8)
        .map(|n| -2.0 + n as f64 * 0.5)
        .collect::<Vec<_>>()
        .into();
    let children: Arc<[u8]> = (0..64).map(|n| n.min(62)).collect::<Vec<_>>().into();
    nodes.push(NativeBlendTreeNode::Blend2d {
        parameters: [0, 1],
        x: axis.clone(),
        y: axis,
        children,
    });
    let pose = tree(nodes).unwrap().sample(&[0.25, 1.25, 0.], 0.3).unwrap();
    assert!(pose.samples().len() <= 4);
    let mut chain = vec![NativeBlendTreeNode::Clip(None)];
    for index in 1..MAX_BLEND_TREE_DEPTH {
        chain.push(line(0, [0, (index - 1) as u8]));
    }
    assert_eq!(
        tree(chain.clone())
            .unwrap()
            .sample(&[0.5; 3], 0.5)
            .unwrap()
            .samples()
            .len(),
        1
    );
    chain.push(line(0, [0, (MAX_BLEND_TREE_DEPTH - 1) as u8]));
    assert!(tree(chain).is_err());
}

#[path = "support/alloc.rs"]
mod alloc;
#[global_allocator]
static ALLOC: alloc::CountAlloc = alloc::CountAlloc;

#[test]
fn nested_sampling_and_full_fades_allocate_nothing_after_admission() {
    let tree = tree(nested_nodes()).unwrap();
    let ((), count) = alloc::measured(|| {
        for tick in 0..4096 {
            let t = (tick % 257) as f64 / 256.;
            let first = tree.sample(&[t, 1. - t, 0.5], t).unwrap();
            let second = tree.sample(&[0.5, t, 1. - t], 1. - t).unwrap();
            std::hint::black_box(first.blend(&second, t).unwrap());
        }
    });
    assert_eq!(count, 0);
}

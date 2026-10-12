use super::*;

pub(super) fn scene() -> CookedScene {
    let durations: Arc<[f32]> = Arc::from([1., 2., 3., 4., 5., 6., 7., 8.]);
    let mut clips = Vec::new();
    for (i, &duration) in durations.iter().enumerate() {
        clips.push(
            RootMotionClip::new(
                duration,
                Some(
                    RootMotionTrack::new(
                        vec![0., duration],
                        vec![[0.; 4], [duration * (i + 1) as f32, 0., 0., 0.]],
                        Interpolation::Linear,
                        Kind::Vector,
                    )
                    .unwrap(),
                ),
                None,
            )
            .unwrap(),
        );
    }
    let root = NativeRootMotion {
        catalog: Arc::new(
            RootMotionCatalog::new(
                [1; 32],
                0,
                [0.; 3],
                [0., 0., 0., 1.],
                RootMotionDelta::IDENTITY,
                clips,
            )
            .unwrap(),
        ),
        application: RootMotionApplication::Transform,
        applying: true,
    };
    let parameters: Arc<[_]> = [0.25, 0.75, 0.4]
        .into_iter()
        .enumerate()
        .map(|(n, initial)| NativeAnimationParameter {
            id: id(20 + n as u8),
            initial,
            min: 0.,
            max: 1.,
        })
        .collect();
    let mut nodes: Vec<_> = (0..8)
        .map(|n| NativeBlendTreeNode::Clip((n != 7).then_some(n)))
        .collect();
    for n in 0..4 {
        nodes.push(NativeBlendTreeNode::Blend1d {
            parameter: 2,
            points: Arc::from([
                NativeBlendTreePoint {
                    threshold: 0.,
                    child: n * 2,
                },
                NativeBlendTreePoint {
                    threshold: 1.,
                    child: n * 2 + 1,
                },
            ]),
        });
    }
    nodes.push(NativeBlendTreeNode::Blend2d {
        parameters: [0, 1],
        x: Arc::from([0., 1.]),
        y: Arc::from([0., 1.]),
        children: Arc::from([8, 9, 10, 11]),
    });
    let tree = Arc::new(
        NativeAnimationBlendTree::new(nodes, parameters.clone(), durations.clone()).unwrap(),
    );
    let graph = NativeAnimationGraph::new(
        0,
        parameters.to_vec(),
        [4., 2.]
            .into_iter()
            .enumerate()
            .map(|(n, cycle_seconds)| NativeAnimationGraphState {
                id: id(1 + n as u8),
                motion: NativeAnimationMotion::Tree {
                    cycle_seconds,
                    tree: tree.clone(),
                },
                looping: true,
            })
            .collect(),
        vec![NativeAnimationGraphTransition {
            id: id(7),
            from: 0,
            to: 1,
            priority: 0,
            fade_seconds: 0.3,
            after_seconds: Some(0.1),
            conditions: Arc::from([]),
        }],
        &durations,
    )
    .unwrap();
    let animation = Arc::new(NativeAnimation {
        mesh_fingerprint: [1; 32],
        clip_durations: durations,
        initial: Animation {
            clip: None,
            seconds: 0.,
            speed: 1.,
            playing: true,
            looping: true,
        },
        graph: Some(Arc::new(graph)),
    });
    let mut node = CookedSpatial::empty(id(10));
    node.mesh = Some(NativeMeshRenderer {
        mesh: id(11),
        cast_shadows: false,
    });
    node.animation = Some(animation);
    node.root_motion = Some(Arc::new(root));
    CookedScene::new(
        id(0),
        8,
        vec![CookedEntity {
            id: id(10),
            parent: None,
            transform: Default::default(),
            velocity: None,
        }],
    )
    .unwrap()
    .with_spatial(
        vec![VisualAssetRef {
            id: id(11),
            kind: VisualAssetKind::Model,
            fingerprint: [1; 32],
        }],
        vec![node],
    )
    .unwrap()
}

#[test]
fn nested_one_and_two_dimensional_tree_motion_matches_independent_tensor_product_through_fade() {
    let mut world = NativeWorld::new(scene());
    let mut x = 0.25;
    let y = 0.75;
    let z = 0.4;
    let mut position = 0.;
    for tick in 0..24 {
        if tick == 2 {
            x = 0.5;
            command(
                &mut world,
                StructuralCommand::ControlAnimationGraph {
                    entity: id(10),
                    control: AnimationGraphControl::SetParameter {
                        parameter: id(20),
                        value: x,
                    },
                },
            );
        }
        let weights = [
            (1. - x) * (1. - y) * (1. - z),
            (1. - x) * (1. - y) * z,
            x * (1. - y) * (1. - z),
            x * (1. - y) * z,
            (1. - x) * y * (1. - z),
            (1. - x) * y * z,
            x * y * (1. - z),
            x * y * z,
        ];
        let rate: f64 = weights
            .into_iter()
            .enumerate()
            .take(7)
            .map(|(n, w)| w * ((n + 1) * (n + 1)) as f64)
            .sum();
        let factor = if tick == 0 {
            0.25
        } else if tick <= 3 {
            let start = (tick - 1) as f64 * 0.125;
            let stop = (start + 0.125).min(0.3);
            let average =
                ((stop * stop - start * start) / (2. * 0.3) + (start + 0.125 - stop)) / 0.125;
            0.25 * (1. - average) + 0.5 * average
        } else {
            0.5
        };
        position += rate * factor * 0.125;
        world.step().unwrap();
        assert!((world.inspect(id(10)).unwrap().transform.translation[0] - position).abs() < 3e-6);
        let rows = &mut Vec::new();
        world.snapshot_animations(rows).unwrap();
        let pose = rows[0].pose.unwrap();
        if tick <= 2 {
            assert_eq!(pose.samples().len(), 16);
            assert!(pose.outer_weight().is_some());
        } else {
            assert_eq!(pose.samples().len(), 8);
        }
    }
}

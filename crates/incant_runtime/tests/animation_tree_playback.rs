use incant_runtime::*;
use incant_types::{Animation, Transform};
use std::sync::Arc;

fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}

fn scene(looping: bool, rest: bool) -> CookedScene {
    let parameters: Arc<[_]> = (0..3)
        .map(|i| NativeAnimationParameter {
            id: id(10 + i),
            initial: [0.25, 0.5, 0.75][i as usize],
            min: 0.0,
            max: 1.0,
        })
        .collect();
    let durations: Arc<[f32]> = (1..=8).map(|n| n as f32).collect();
    let mut nodes: Vec<_> = (0..8)
        .map(|n| NativeBlendTreeNode::Clip(if rest { None } else { Some(n) }))
        .collect();
    for offset in [0, 4] {
        nodes.push(NativeBlendTreeNode::Blend2d {
            parameters: [0, 1],
            x: Arc::from([0.0, 1.0]),
            y: Arc::from([0.0, 1.0]),
            children: Arc::from([offset, offset + 1, offset + 2, offset + 3]),
        });
    }
    nodes.push(NativeBlendTreeNode::Blend1d {
        parameter: 2,
        points: Arc::from([
            NativeBlendTreePoint {
                threshold: 0.0,
                child: 8,
            },
            NativeBlendTreePoint {
                threshold: 1.0,
                child: 9,
            },
        ]),
    });
    let tree = NativeAnimationBlendTree::new(nodes, parameters.clone(), durations.clone()).unwrap();
    let graph = NativeAnimationGraph::new(
        0,
        parameters.to_vec(),
        vec![NativeAnimationGraphState {
            id: id(20),
            motion: NativeAnimationMotion::Tree {
                cycle_seconds: 2.0,
                tree: Arc::new(tree),
            },
            looping,
        }],
        vec![],
        &durations,
    )
    .unwrap();
    let mut node = CookedSpatial::empty(id(1));
    node.mesh = Some(NativeMeshRenderer {
        mesh: id(2),
        cast_shadows: false,
    });
    node.animation = Some(Arc::new(NativeAnimation {
        mesh_fingerprint: [4; 32],
        clip_durations: durations,
        initial: Animation {
            clip: None,
            seconds: 0.0,
            speed: 1.0,
            playing: true,
            looping,
        },
        graph: Some(Arc::new(graph)),
    }));
    CookedScene::new(
        id(0),
        8,
        vec![CookedEntity {
            id: id(1),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        }],
    )
    .unwrap()
    .with_spatial(
        vec![VisualAssetRef {
            id: id(2),
            kind: VisualAssetKind::Model,
            fingerprint: [4; 32],
        }],
        vec![node],
    )
    .unwrap()
}
fn apply(world: &mut NativeWorld, control: AnimationControl) {
    let mut c = FrameCommands::default();
    c.push(StructuralCommand::ControlAnimation {
        entity: id(1),
        control,
    })
    .unwrap();
    world.apply(&mut c).unwrap();
}
fn parameter(world: &mut NativeWorld, value: f64) {
    let mut c = FrameCommands::default();
    c.push(StructuralCommand::ControlAnimationGraph {
        entity: id(1),
        control: AnimationGraphControl::SetParameter {
            parameter: id(10),
            value,
        },
    })
    .unwrap();
    world.apply(&mut c).unwrap();
}

#[test]
fn tree_clocks_publish_all_eight_samples_with_tensor_weights_and_owned_snapshots() {
    let mut world = NativeWorld::new(scene(true, false));
    world.step().unwrap();
    let row = world.animation_graph_state(id(1)).unwrap();
    let pose = row.pose.unwrap();
    assert_eq!(pose.samples().len(), 8);
    for entry in pose.samples() {
        let clip = entry.sample.clip.unwrap();
        let x = if clip % 2 == 0 { 0.75 } else { 0.25 };
        let y = 0.5;
        let z = if clip < 4 { 0.25 } else { 0.75 };
        assert_eq!(entry.weight, x * y * z);
        assert_eq!(entry.sample.seconds, (f64::from(clip) + 1.0) / 16.0);
    }
    let mut rows = Vec::new();
    world.snapshot_animations(&mut rows).unwrap();
    assert_eq!(rows[0].pose, Some(pose));
    assert!(rows[0].blend.is_none());
    assert_eq!(
        world
            .spatial_view()
            .unwrap()
            .nodes()
            .next()
            .unwrap()
            .animation_pose,
        Some(pose)
    );
    parameter(&mut world, 1.0);
    assert_eq!(rows[0].pose, Some(pose));
    assert_eq!(
        world
            .animation_graph_state(id(1))
            .unwrap()
            .pose
            .unwrap()
            .samples()
            .len(),
        4
    );
}

#[test]
fn binary_v15_and_existing_save_envelope_resume_active_paused_completed_and_rest_trees() {
    for (looping, rest, paused, steps) in [
        (true, false, false, 5),
        (true, false, true, 5),
        (false, false, false, 20),
        (false, true, false, 20),
        (false, true, true, 20),
    ] {
        let original = scene(looping, rest);
        let bytes = original.to_bytes();
        assert_eq!(&bytes[8..12], &15u32.to_le_bytes());
        assert_eq!(CookedScene::from_bytes(&bytes).unwrap().to_bytes(), bytes);
        let mut world = NativeWorld::from_bytes(&bytes).unwrap();
        for _ in 0..steps {
            world.step().unwrap();
        }
        if paused {
            apply(&mut world, AnimationControl::SetPlaying(false));
        }
        let save = world.save_bytes().unwrap();
        let mut resumed = NativeWorld::from_save_bytes_for_scene(&save, &original).unwrap();
        assert_eq!(resumed.save_bytes().unwrap(), save);
        for _ in 0..19 {
            world.step().unwrap();
            resumed.step().unwrap();
            assert_eq!(world.save_bytes().unwrap(), resumed.save_bytes().unwrap());
            assert_eq!(
                world.animation_graph_state(id(1)),
                resumed.animation_graph_state(id(1))
            );
        }
        if !looping {
            assert!(world.animation_state(id(1)).unwrap().completed);
        }
    }
}

#[test]
fn tree_pause_reverse_parameter_rejection_and_numeric_rollback_are_observable() {
    let mut world = NativeWorld::new(scene(true, false));
    for _ in 0..4 {
        world.step().unwrap();
    }
    apply(&mut world, AnimationControl::SetSpeed(-2.0));
    world.step().unwrap();
    assert_eq!(world.animation_graph_state(id(1)).unwrap().seconds, 0.25);
    apply(&mut world, AnimationControl::SetPlaying(false));
    let before = world.animation_graph_state(id(1)).unwrap();
    world.step().unwrap();
    assert_eq!(world.animation_graph_state(id(1)).unwrap(), before);
    let mut c = FrameCommands::default();
    for value in [0.5, f64::NAN] {
        c.push(StructuralCommand::ControlAnimationGraph {
            entity: id(1),
            control: AnimationGraphControl::SetParameter {
                parameter: id(10),
                value,
            },
        })
        .unwrap();
    }
    assert!(world.apply(&mut c).is_err());
    assert_eq!(world.animation_graph_state(id(1)).unwrap(), before);
    apply(&mut world, AnimationControl::SetPlaying(true));
    world.step().unwrap();
    world.step().unwrap();
    assert_eq!(world.animation_graph_state(id(1)).unwrap().seconds, 1.75);
}

fn checksum(bytes: &mut [u8]) {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(&bytes[..40]);
    h.update(&bytes[72..]);
    bytes[40..72].copy_from_slice(&h.finalize());
}
#[test]
fn checksum_valid_tree_downgrade_truncation_and_foreign_context_are_refused() {
    let original = scene(true, false);
    let bytes = original.to_bytes();
    let mut bad = bytes.clone();
    bad[8..12].copy_from_slice(&14u32.to_le_bytes());
    checksum(&mut bad);
    assert!(CookedScene::from_bytes(&bad).is_err());
    for end in bytes.len() - 80..bytes.len() {
        assert!(CookedScene::from_bytes(&bytes[..end]).is_err());
    }
    let graph = original.spatial()[0]
        .animation
        .as_ref()
        .unwrap()
        .graph
        .as_ref()
        .unwrap();
    let mut parameters = graph.parameters().to_vec();
    parameters[0].initial = 0.75;
    assert!(
        NativeAnimationGraph::new(
            0,
            parameters,
            graph.states().to_vec(),
            vec![],
            &[1., 2., 3., 4., 5., 6., 7., 8.]
        )
        .is_err()
    );
}

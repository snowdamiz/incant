//! Real native tree, scalar fade and parameter pair; no copied ABI decoder.
use incant_runtime::*;
use incant_types::{Animation, Transform};
use std::sync::Arc;
pub fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
pub fn scene() -> CookedScene {
    let parameters: Arc<[_]> = (0..3)
        .map(|i| NativeAnimationParameter {
            id: id(10 + i),
            initial: [0.25, 0.5, 0.75][i as usize],
            min: 0.0,
            max: 1.0,
        })
        .collect();
    let durations: Arc<[f32]> = (1..=8).map(|n| n as f32).collect();
    let mut tree_nodes: Vec<_> = (0..8).map(|n| NativeBlendTreeNode::Clip(Some(n))).collect();
    for offset in [0, 4] {
        tree_nodes.push(NativeBlendTreeNode::Blend2d {
            parameters: [0, 1],
            x: Arc::from([0.0, 1.0]),
            y: Arc::from([0.0, 1.0]),
            children: Arc::from([offset, offset + 1, offset + 2, offset + 3]),
        });
    }
    tree_nodes.push(NativeBlendTreeNode::Blend1d {
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
    let tree =
        NativeAnimationBlendTree::new(tree_nodes, parameters.clone(), durations.clone()).unwrap();
    let rest = NativeAnimationBlendTree::new(
        vec![NativeBlendTreeNode::Clip(None)],
        parameters.clone(),
        durations.clone(),
    )
    .unwrap();
    let graph = NativeAnimationGraph::new(
        0,
        parameters.to_vec(),
        vec![
            NativeAnimationGraphState {
                id: id(20),
                looping: true,
                motion: NativeAnimationMotion::Tree {
                    cycle_seconds: 2.0,
                    tree: Arc::new(tree),
                },
            },
            NativeAnimationGraphState {
                id: id(21),
                looping: false,
                motion: NativeAnimationMotion::Tree {
                    cycle_seconds: 0.04,
                    tree: Arc::new(rest),
                },
            },
        ],
        vec![],
        &durations,
    )
    .unwrap();
    let pair = NativeAnimationGraph::new(
        0,
        vec![parameters[0]],
        vec![NativeAnimationGraphState {
            id: id(20),
            looping: true,
            motion: NativeAnimationMotion::Blend1d {
                parameter: 0,
                cycle_seconds: 1.0,
                points: Arc::from([
                    NativeAnimationBlendPoint {
                        threshold: 0.0,
                        clip: 0,
                    },
                    NativeAnimationBlendPoint {
                        threshold: 1.0,
                        clip: 1,
                    },
                ]),
            },
        }],
        vec![],
        &durations,
    )
    .unwrap();
    let nodes = [Some(Arc::new(graph)), None, Some(Arc::new(pair))]
        .into_iter()
        .enumerate()
        .map(|(index, graph)| {
            let mut node = CookedSpatial::empty(id(index as u128 + 1));
            node.mesh = Some(NativeMeshRenderer {
                mesh: id(50),
                cast_shadows: false,
            });
            node.animation = Some(Arc::new(NativeAnimation {
                mesh_fingerprint: [4; 32],
                clip_durations: durations.clone(),
                graph,
                initial: Animation {
                    clip: if index == 1 { Some(0) } else { None },
                    seconds: 0.0,
                    speed: 1.0,
                    playing: true,
                    looping: true,
                },
            }));
            node
        })
        .collect();
    let entities = (1..=4)
        .chain(100..148)
        .chain(200..209)
        .map(|n| CookedEntity {
            id: id(n),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        })
        .collect();
    CookedScene::new(id(99), 60, entities)
        .unwrap()
        .with_spatial(
            vec![VisualAssetRef {
                id: id(50),
                kind: VisualAssetKind::Model,
                fingerprint: [4; 32],
            }],
            nodes,
        )
        .unwrap()
}

pub fn controls(turn: u32) -> Vec<StructuralCommand> {
    let animation = |entity, control| StructuralCommand::ControlAnimation {
        entity: id(entity),
        control,
    };
    let parameter = |key, value| StructuralCommand::ControlAnimationGraph {
        entity: id(1),
        control: AnimationGraphControl::SetParameter {
            parameter: id(key),
            value,
        },
    };
    let select = |key| StructuralCommand::ControlAnimationGraph {
        entity: id(1),
        control: AnimationGraphControl::SelectState(id(key)),
    };
    let first = match turn {
        0 | 4 | 5 | 6 | 16 => Some(parameter(
            10,
            match turn {
                0 => 0.75,
                4 => 0.0,
                5 => 1.0,
                16 => 0.25,
                _ => 0.5,
            },
        )),
        1 | 12 => Some(animation(1, AnimationControl::SetPlaying(false))),
        2 | 14 => Some(select(20)),
        3 | 13 => Some(animation(1, AnimationControl::SetPlaying(true))),
        7 => Some(animation(1, AnimationControl::SetSpeed(-1.0))),
        8 => Some(animation(1, AnimationControl::SetSpeed(1.0))),
        9 => Some(select(21)),
        10 => Some(animation(1, AnimationControl::SetSpeed(1.0))),
        15 => Some(parameter(12, 0.25)),
        _ => None,
    };
    let second = match turn {
        0 => Some(AnimationControl::Crossfade {
            target: AnimationSample {
                clip: Some(1),
                seconds: 0.5,
            },
            duration: 0.2,
        }),
        1 | 10 => Some(AnimationControl::SetPlaying(false)),
        3 | 12 => Some(AnimationControl::SetPlaying(true)),
        4 => Some(AnimationControl::SetSpeed(-1.0)),
        7 => Some(AnimationControl::Seek(0.7)),
        8 => Some(AnimationControl::Crossfade {
            target: AnimationSample::REST,
            duration: 0.1,
        }),
        15 => Some(AnimationControl::Select(AnimationSample {
            clip: Some(0),
            seconds: 0.0,
        })),
        _ => None,
    }
    .map(|control| animation(2, control));
    first.into_iter().chain(second).collect()
}

/// Independent native observations: retain every sample, never hash or decode
/// guest bytes. Scalar clocks, flags and interpolation meaning are separate.
pub fn observe(world: &mut NativeWorld) -> ([[f64; 3]; 48], [[f64; 3]; 9]) {
    let mut rows = Vec::new();
    world.snapshot_animations(&mut rows).unwrap();
    rows.sort_unstable_by_key(|r| r.id);
    let mut observations = [[-2.0, 0.0, 0.0]; 48];
    let mut metadata = [[0.0; 3]; 9];
    for (index, row) in rows.iter().enumerate() {
        let (kind, samples): (_, Vec<_>) = if let Some(pose) = row.pose {
            (
                3.0,
                pose.samples()
                    .iter()
                    .map(|v| (v.sample, v.weight))
                    .collect(),
            )
        } else if let Some(blend) = row.blend {
            (
                if row.state.transition.is_some() {
                    1.0
                } else {
                    2.0
                },
                vec![
                    (blend.sample, 1.0 - blend.weight),
                    (row.state.sample, blend.weight),
                ],
            )
        } else {
            (0.0, vec![(row.state.sample, 1.0)])
        };
        for (slot, (sample, weight)) in samples.iter().enumerate() {
            observations[index * 16 + slot] =
                [sample.clip.map_or(-1.0, f64::from), sample.seconds, *weight];
        }
        let state = row.state;
        metadata[index * 3] = [samples.len() as f64, kind, state.sample.seconds];
        metadata[index * 3 + 1] = [
            state.speed,
            f64::from(u8::from(state.playing)),
            f64::from(u8::from(state.completed)),
        ];
        metadata[index * 3 + 2] = [
            f64::from(u8::from(state.looping)),
            state.transition.map_or(-1.0, |t| t.elapsed),
            state.transition.map_or(-1.0, |t| t.duration),
        ];
    }
    (observations, metadata)
}
pub fn oracle(world: &mut NativeWorld, turn: u32) {
    let (observations, metadata) = observe(world);
    world.with_numeric_columns(|view| {
        view.for_each_chunk(|chunk| {
            for (entity, transform) in chunk
                .ids
                .iter()
                .zip(chunk.transforms.as_chunks_mut::<10>().0.iter_mut())
            {
                let n = u128::from_be_bytes(entity.0);
                if n == 4 {
                    transform[0] = f64::from(turn + 1);
                } else if (100..148).contains(&n) {
                    transform[..3].copy_from_slice(&observations[(n - 100) as usize]);
                } else if (200..209).contains(&n) {
                    transform[..3].copy_from_slice(&metadata[(n - 200) as usize]);
                }
            }
        })
    });
    let mut commands = FrameCommands::default();
    for command in controls(turn) {
        commands.push(command).unwrap();
    }
    world.apply(&mut commands).unwrap();
}

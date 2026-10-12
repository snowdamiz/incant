//! Actual native state and independent typed observation/control oracle.
#[allow(dead_code)] // Shared scene fixture also supplies independent older-protocol oracles.
#[path = "animation_ik.rs"]
mod ik;
pub use ik::id;
use incant_runtime::*;
use std::sync::Arc;
pub const VALUES: usize = 384;
pub fn scene() -> CookedScene {
    let base = ik::scene();
    let mut nodes = base.spatial().to_vec();
    let node = nodes.iter_mut().find(|n| n.entity == id(1)).unwrap();
    let a = Arc::make_mut(node.animation.as_mut().unwrap());
    let old = a.graph.as_ref().unwrap();
    let mut states = old.states().to_vec();
    states[1].motion = match &states[0].motion {
        NativeAnimationMotion::Tree { tree, .. } => NativeAnimationMotion::Tree {
            tree: tree.clone(),
            cycle_seconds: 1.,
        },
        _ => unreachable!(),
    };
    states.push(NativeAnimationGraphState {
        id: id(22),
        looping: false,
        motion: NativeAnimationMotion::Blend1d {
            parameter: 0,
            cycle_seconds: 0.25,
            points: Arc::from([
                NativeAnimationBlendPoint {
                    threshold: 0.,
                    clip: 0,
                },
                NativeAnimationBlendPoint {
                    threshold: 1.,
                    clip: 1,
                },
            ]),
        },
    });
    states.push(NativeAnimationGraphState {
        id: id(23),
        looping: true,
        motion: NativeAnimationMotion::Clip(Some(0)),
    });
    let transitions = (0..4)
        .map(|n| NativeAnimationGraphTransition {
            id: id(40 + u128::from(n)),
            from: n,
            to: (n + 1) % 4,
            priority: 0,
            fade_seconds: 0.2,
            after_seconds: Some(0.1),
            conditions: Arc::from([]),
        })
        .collect();
    a.graph = Some(Arc::new(
        NativeAnimationGraph::new(
            0,
            old.parameters().to_vec(),
            states,
            transitions,
            &a.clip_durations,
        )
        .unwrap(),
    ));
    let entities = (1..=4)
        .chain(100..228)
        .map(|n| CookedEntity {
            id: id(n),
            parent: None,
            transform: incant_types::Transform::default(),
            velocity: None,
        })
        .collect();
    CookedScene::new(id(99), 60, entities)
        .unwrap()
        .with_spatial(base.visual_assets().to_vec(), nodes)
        .unwrap()
}
pub fn observations(world: &mut NativeWorld) -> [f64; VALUES] {
    let mut values = [0.; VALUES];
    let mut index = 0;
    let mut push = |v| {
        values[index] = v;
        index += 1;
    };
    let mut poses = Vec::new();
    world.snapshot_animations(&mut poses).unwrap();
    poses.sort_unstable_by_key(|r| r.id);
    push(poses.len() as f64);
    for row in poses {
        let s = row.state;
        let kind = if let Some(p) = row.pose {
            if p.outer_weight().is_some() {
                4.
            } else {
                match p.group(0).unwrap().interpolation {
                    incant_types::PoseInterpolation::Clip => 0.,
                    incant_types::PoseInterpolation::SphericalPair => 2.,
                    incant_types::PoseInterpolation::Weighted => 3.,
                }
            }
        } else if row.blend.is_some() {
            if s.transition.is_some() { 1. } else { 2. }
        } else {
            0.
        };
        for v in [
            u128::from_be_bytes(row.id.0) as f64,
            s.sample.clip.map_or(-1., f64::from),
            s.sample.seconds,
            s.speed,
            f64::from(s.playing),
            f64::from(s.looping),
            f64::from(s.completed),
            kind,
            row.pose.and_then(|p| p.outer_weight()).unwrap_or(-1.),
        ] {
            push(v);
        }
        let t = s.transition;
        for v in [
            t.map_or(-1., |v| v.source.clip.map_or(-1., f64::from)),
            t.map_or(-1., |v| v.source.seconds),
            t.map_or(-1., |v| v.speed),
            t.map_or(-1., |v| f64::from(v.playing)),
            t.map_or(-1., |v| f64::from(v.looping)),
            t.map_or(-1., |v| v.elapsed),
            t.map_or(-1., |v| v.duration),
        ] {
            push(v);
        }
        if let Some(pose) = row.pose {
            for i in 0..2 {
                if let Some(g) = pose.group(i) {
                    push(g.interpolation as u8 as f64);
                    push(g.samples.len() as f64);
                    for s in g.samples {
                        push(s.sample.clip.map_or(-1., f64::from));
                        push(s.sample.seconds);
                        push(s.weight);
                    }
                } else {
                    push(-1.);
                    push(0.);
                }
            }
        } else {
            if let Some(blend) = row.blend {
                push(1.);
                push(2.);
                for (s, w) in [(blend.sample, 1. - blend.weight), (s.sample, blend.weight)] {
                    push(s.clip.map_or(-1., f64::from));
                    push(s.seconds);
                    push(w);
                }
            } else {
                push(0.);
                push(1.);
                push(s.sample.clip.map_or(-1., f64::from));
                push(s.sample.seconds);
                push(1.);
            }
            push(-1.);
            push(0.);
        }
    }
    let mut graphs = Vec::new();
    world.snapshot_animation_graphs(&mut graphs).unwrap();
    graphs.sort_unstable_by_key(|r| r.entity);
    push(graphs.len() as f64);
    for g in graphs {
        for v in [
            u128::from_be_bytes(g.entity.0) as f64,
            u128::from_be_bytes(g.state.0) as f64,
            f64::from(g.paused),
            g.seconds,
            g.elapsed,
        ] {
            push(v);
        }
        let f = g.fade;
        for v in [
            f.map_or(-1., |v| u128::from_be_bytes(v.state.0) as f64),
            f.map_or(-1., |v| v.seconds),
            f.map_or(-1., |v| v.speed),
            f.map_or(-1., |v| f64::from(v.playing)),
            f.map_or(-1., |v| v.elapsed),
            f.map_or(-1., |v| v.duration),
        ] {
            push(v);
        }
        push(f64::from(g.parameter_count));
        for n in 0..usize::from(g.parameter_count) {
            push(u128::from_be_bytes(g.parameter_ids[n].0) as f64);
            push(g.parameters[n]);
        }
    }
    let mut constraints = Vec::new();
    world.snapshot_ik(&mut constraints).unwrap();
    push(constraints.len() as f64);
    for r in constraints {
        push(u128::from_be_bytes(r.entity.0) as f64);
        push(u128::from_be_bytes(r.sample.id.0) as f64);
        for v in r.sample.chain {
            push(f64::from(v));
        }
        for v in r.sample.target.into_iter().chain(r.sample.pole) {
            push(v);
        }
        push(r.sample.weight);
    }
    values
}
pub fn controls(world: &mut NativeWorld, turn: u32) {
    let mut commands = FrameCommands::default();
    let mut push = |c| commands.push(c).unwrap();
    let animation = match turn {
        4 | 9 => Some(AnimationControl::SetPlaying(false)),
        7 | 12 => Some(AnimationControl::SetPlaying(true)),
        10 | 18 => Some(AnimationControl::SetSpeed(-2.)),
        16 => Some(AnimationControl::SetSpeed(1.)),
        _ => None,
    };
    if let Some(control) = animation {
        push(StructuralCommand::ControlAnimation {
            entity: id(if turn == 9 || turn == 12 || turn == 18 {
                2
            } else {
                1
            }),
            control,
        });
    }
    if turn == 2 {
        push(StructuralCommand::ControlAnimation {
            entity: id(2),
            control: AnimationControl::Crossfade {
                target: AnimationSample {
                    clip: Some(1),
                    seconds: 0.3,
                },
                duration: 0.2,
            },
        });
    }
    if turn == 24 || turn == 28 {
        push(StructuralCommand::ControlAnimationGraph {
            entity: id(1),
            control: AnimationGraphControl::SelectState(id(if turn == 24 { 23 } else { 20 })),
        });
    }
    if turn == 1 || turn == 20 {
        push(StructuralCommand::ControlAnimationGraph {
            entity: id(1),
            control: AnimationGraphControl::SetParameter {
                parameter: id(10),
                value: if turn == 1 { 0.75 } else { 0.25 },
            },
        });
    }
    let mut rows = Vec::new();
    world.snapshot_ik(&mut rows).unwrap();
    for (n, r) in rows.iter().enumerate() {
        push(StructuralCommand::ControlIk {
            entity: r.entity,
            control: IkControl {
                constraint: r.sample.id,
                target: [f64::from(turn) / 8., n as f64, 0.],
                pole: [0., 1., f64::from(turn) / 64.],
                weight: f64::from(turn % 3) / 2.,
            },
        });
    }
    world.apply(&mut commands).unwrap();
}
pub fn oracle(world: &mut NativeWorld, turn: u32) {
    let values = observations(world);
    world.with_numeric_columns(|view| {
        view.for_each_chunk(|chunk| {
            for (id, t) in chunk
                .ids
                .iter()
                .zip(chunk.transforms.as_chunks_mut::<10>().0)
            {
                let n = u128::from_be_bytes(id.0);
                if n == 4 {
                    t[0] += 1.;
                } else if (100..228).contains(&n) {
                    let offset = (n - 100) as usize * 3;
                    t[..3].copy_from_slice(&values[offset..offset + 3]);
                }
            }
        })
    });
    controls(world, turn);
}

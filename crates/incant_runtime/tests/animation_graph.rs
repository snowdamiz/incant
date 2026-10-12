use incant_runtime::*;
use incant_types::{Animation, AnimationComparison, Transform};
use std::sync::Arc;
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn graph(transitions: Vec<NativeAnimationGraphTransition>) -> NativeAnimationGraph {
    NativeAnimationGraph::new(
        0,
        vec![NativeAnimationParameter {
            id: id(10),
            initial: 0.0,
            min: -1.0,
            max: 2.0,
        }],
        vec![
            NativeAnimationGraphState {
                id: id(20),
                motion: NativeAnimationMotion::Clip(Some(0)),
                looping: true,
            },
            NativeAnimationGraphState {
                id: id(21),
                motion: NativeAnimationMotion::Clip(Some(1)),
                looping: true,
            },
            NativeAnimationGraphState {
                id: id(22),
                motion: NativeAnimationMotion::Blend1d {
                    parameter: 0,
                    cycle_seconds: 2.0,
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
                looping: true,
            },
        ],
        transitions,
        &[2.0, 4.0],
    )
    .unwrap()
}
fn edge(n: u128, from: u8, to: u8, priority: i16, fade: f64) -> NativeAnimationGraphTransition {
    NativeAnimationGraphTransition {
        id: id(n),
        from,
        to,
        priority,
        fade_seconds: fade,
        after_seconds: None,
        conditions: Arc::from([NativeAnimationCondition {
            parameter: 0,
            comparison: AnimationComparison::GreaterEqual,
            value: 0.5,
        }]),
    }
}
fn scene(graph: NativeAnimationGraph) -> CookedScene {
    let definition = Arc::new(NativeAnimation {
        mesh_fingerprint: [4; 32],
        clip_durations: Arc::from([2.0, 4.0]),
        initial: Animation {
            clip: Some(0),
            seconds: 0.0,
            speed: 1.0,
            playing: true,
            looping: true,
        },
        graph: Some(Arc::new(graph)),
    });
    let mut node = CookedSpatial::empty(id(1));
    node.mesh = Some(NativeMeshRenderer {
        mesh: id(2),
        cast_shadows: false,
    });
    node.animation = Some(definition);
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
fn command(world: &mut NativeWorld, control: AnimationGraphControl) {
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::ControlAnimationGraph {
            entity: id(1),
            control,
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
}
fn parameter(world: &mut NativeWorld, value: f64) {
    command(
        world,
        AnimationGraphControl::SetParameter {
            parameter: id(10),
            value,
        },
    );
}
fn select(world: &mut NativeWorld, state: u128) {
    command(world, AnimationGraphControl::SelectState(id(state)));
}
fn master(world: &mut NativeWorld, control: AnimationControl) {
    let mut c = FrameCommands::default();
    c.push(StructuralCommand::ControlAnimation {
        entity: id(1),
        control,
    })
    .unwrap();
    world.apply(&mut c).unwrap();
}
fn snapshot(world: &NativeWorld) -> AnimationGraphSnapshot {
    world.animation_graph_state(id(1)).unwrap()
}
fn close(a: f64, b: f64) {
    assert!((a - b).abs() < 1e-12, "{a} != {b}");
}

#[test]
fn priority_then_stable_id_and_one_edge_per_tick_are_observable() {
    let mut world = NativeWorld::new(scene(graph(vec![
        edge(30, 0, 1, -2, 0.0),
        edge(31, 0, 2, -2, 0.0),
        edge(32, 1, 2, 0, 0.0),
    ])));
    world.step().unwrap();
    assert_eq!(snapshot(&world).state, id(20));
    parameter(&mut world, 0.5);
    world.step().unwrap();
    assert_eq!(snapshot(&world).state, id(21));
    close(snapshot(&world).seconds, 0.0);
    world.step().unwrap();
    assert_eq!(snapshot(&world).state, id(22));
    assert_eq!(snapshot(&world).blend.unwrap().weight, 0.5);
}
#[test]
fn adjacent_blend_uses_normalized_phase_clamps_and_preserves_pause_reverse() {
    let mut world = NativeWorld::new(scene(graph(vec![])));
    select(&mut world, 22);
    parameter(&mut world, 0.25);
    for _ in 0..4 {
        world.step().unwrap();
    }
    let row = snapshot(&world);
    close(row.seconds, 0.5);
    close(row.blend.unwrap().weight, 0.25);
    close(row.blend.unwrap().sample.seconds, 0.5);
    close(world.animation_state(id(1)).unwrap().sample.seconds, 1.0);
    let pose = world.spatial_view().unwrap().nodes().next().unwrap();
    assert_eq!(pose.animation_blend, row.blend);
    master(&mut world, AnimationControl::SetPlaying(false));
    world.step().unwrap();
    assert_eq!(
        snapshot(&world),
        AnimationGraphSnapshot {
            paused: true,
            ..row
        }
    );
    parameter(&mut world, -1.0);
    assert!(snapshot(&world).blend.is_none());
    assert_eq!(world.animation_state(id(1)).unwrap().sample.clip, Some(0));
    parameter(&mut world, 2.0);
    assert!(snapshot(&world).blend.is_none());
    assert_eq!(world.animation_state(id(1)).unwrap().sample.clip, Some(1));
    master(&mut world, AnimationControl::SetSpeed(-1.0));
    master(&mut world, AnimationControl::SetPlaying(true));
    for _ in 0..5 {
        world.step().unwrap();
    }
    close(snapshot(&world).seconds, 1.875);
}
#[test]
fn active_clip_fade_owns_its_step_and_manual_selection_cancels_it() {
    let mut world = NativeWorld::new(scene(graph(vec![
        edge(30, 0, 1, 0, 0.5),
        edge(31, 1, 2, 0, 0.0),
    ])));
    parameter(&mut world, 0.5);
    world.step().unwrap();
    assert_eq!(snapshot(&world).state, id(21));
    for i in 1..=4 {
        world.step().unwrap();
        assert_eq!(snapshot(&world).state, id(21));
        if i < 4 {
            close(snapshot(&world).blend.unwrap().weight, f64::from(i) / 4.0);
        }
    }
    assert!(world.animation_state(id(1)).unwrap().transition.is_none());
    world.step().unwrap();
    assert_eq!(snapshot(&world).state, id(22));
    select(&mut world, 20);
    world.step().unwrap();
    assert!(world.animation_state(id(1)).unwrap().transition.is_some());
    master(&mut world, AnimationControl::SetPlaying(false));
    let stopped = snapshot(&world);
    world.step().unwrap();
    assert_eq!(snapshot(&world), stopped);
    select(&mut world, 22);
    assert!(world.animation_state(id(1)).unwrap().transition.is_none());
    assert!(!world.animation_state(id(1)).unwrap().playing);
}
#[test]
fn mixed_control_failure_rolls_back_and_legacy_clock_owners_refuse() {
    let mut world = NativeWorld::new(scene(graph(vec![])));
    let before = snapshot(&world);
    for invalid in [
        AnimationGraphControl::SetParameter {
            parameter: id(10),
            value: 3.0,
        },
        AnimationGraphControl::SetParameter {
            parameter: id(99),
            value: 0.5,
        },
        AnimationGraphControl::SelectState(id(99)),
    ] {
        let mut c = FrameCommands::default();
        c.push(StructuralCommand::ControlAnimationGraph {
            entity: id(1),
            control: AnimationGraphControl::SelectState(id(22)),
        })
        .unwrap();
        c.push(StructuralCommand::ControlAnimationGraph {
            entity: id(1),
            control: invalid,
        })
        .unwrap();
        assert!(world.apply(&mut c).is_err());
        assert_eq!(snapshot(&world), before);
    }
    for control in [
        AnimationControl::Seek(0.5),
        AnimationControl::SetLooping(false),
        AnimationControl::Select(AnimationSample {
            clip: Some(1),
            seconds: 0.0,
        }),
        AnimationControl::Crossfade {
            target: AnimationSample {
                clip: Some(1),
                seconds: 0.0,
            },
            duration: 0.5,
        },
    ] {
        let mut c = FrameCommands::default();
        c.push(StructuralCommand::ControlAnimation {
            entity: id(1),
            control,
        })
        .unwrap();
        assert!(world.apply(&mut c).is_err());
        assert_eq!(snapshot(&world), before);
    }
}
#[test]
fn invalid_tables_and_oversized_graphs_refuse() {
    let g = graph(vec![]);
    for transitions in [
        vec![edge(31, 0, 1, 0, 0.0), edge(30, 0, 2, 0, 0.0)],
        vec![edge(30, 0, 1, 0, 0.0); 129],
    ] {
        assert!(
            NativeAnimationGraph::new(
                0,
                g.parameters().to_vec(),
                g.states().to_vec(),
                transitions,
                &[2.0, 4.0]
            )
            .is_err()
        );
    }
    let mut states = g.states().to_vec();
    if let NativeAnimationMotion::Blend1d { points, .. } = &mut states[2].motion {
        *points = Arc::from([
            NativeAnimationBlendPoint {
                threshold: 0.0,
                clip: 0,
            },
            NativeAnimationBlendPoint {
                threshold: 0.0,
                clip: 1,
            },
        ]);
    }
    assert!(
        NativeAnimationGraph::new(0, g.parameters().to_vec(), states, vec![], &[2.0, 4.0]).is_err()
    );
}

#[test]
fn v14_v6_source_free_roundtrip_and_exact_active_paused_blend_continuation() {
    let scene = scene(graph(vec![edge(30, 0, 1, 0, 0.5), edge(31, 1, 2, 0, 0.0)]));
    let bytes = scene.to_bytes();
    assert_eq!(&bytes[8..12], &14u32.to_le_bytes());
    assert_eq!(CookedScene::from_bytes(&bytes).unwrap().to_bytes(), bytes);
    let mut world = NativeWorld::from_bytes(&bytes).unwrap();
    parameter(&mut world, 0.25);
    for phase in 0..4 {
        match phase {
            0 => parameter(&mut world, 0.5),
            1 => {
                select(&mut world, 20);
                master(&mut world, AnimationControl::SetPlaying(true));
                world.step().unwrap();
                master(&mut world, AnimationControl::SetPlaying(false));
            }
            2 => {
                master(&mut world, AnimationControl::SetPlaying(true));
                select(&mut world, 22);
                parameter(&mut world, 0.25);
            }
            _ => {
                select(&mut world, 20);
            }
        }
        world.step().unwrap();
        let saved = world.save_bytes().unwrap();
        assert_eq!(&saved[8..12], &6u32.to_le_bytes());
        let mut fresh = NativeWorld::from_save_bytes_for_scene(&saved, &scene).unwrap();
        let mut restored = NativeWorld::new(scene.clone());
        restored.restore_bytes(&saved).unwrap();
        assert_eq!(fresh.save_bytes().unwrap(), saved);
        assert_eq!(restored.save_bytes().unwrap(), saved);
        for _ in 0..19 {
            world.step().unwrap();
            fresh.step().unwrap();
            restored.step().unwrap();
            assert_eq!(world.save_bytes().unwrap(), fresh.save_bytes().unwrap());
            assert_eq!(world.save_bytes().unwrap(), restored.save_bytes().unwrap());
            assert_eq!(snapshot(&world), snapshot(&fresh));
        }
    }
}
fn save_checksum(bytes: &mut [u8]) {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(&bytes[..32]);
    hash.update(&bytes[64..]);
    bytes[32..64].copy_from_slice(&hash.finalize());
}
fn scene_checksum(bytes: &mut [u8]) {
    use sha2::{Digest, Sha256};
    let mut hash = Sha256::new();
    hash.update(&bytes[..40]);
    hash.update(&bytes[72..]);
    bytes[40..72].copy_from_slice(&hash.finalize());
}
#[test]
fn checksum_valid_bad_graph_records_and_saved_membership_refuse_atomically() {
    let scene = scene(graph(vec![]));
    let mut world = NativeWorld::new(scene.clone());
    select(&mut world, 22);
    parameter(&mut world, 0.25);
    world.step().unwrap();
    let saved = world.save_bytes().unwrap();
    for variant in 0..8 {
        let mut bad = saved.clone();
        let row = bad.len() - 168;
        match variant {
            0 => bad[row + 16] = 255,
            1 => bad[row + 17] = 2,
            2 => bad[row + 24..row + 32].copy_from_slice(&3.0f64.to_le_bytes()),
            3 => bad[row + 32..row + 40].copy_from_slice(&f64::NAN.to_le_bytes()),
            4 => bad[row + 40..row + 48].copy_from_slice(&3.0f64.to_le_bytes()),
            5 => bad[row + 48..row + 56].copy_from_slice(&(-0.0f64).to_le_bytes()),
            6 => bad[row..row + 16].copy_from_slice(&id(2).0),
            _ => bad[108..112].copy_from_slice(&0u32.to_le_bytes()),
        }
        save_checksum(&mut bad);
        assert!(world.restore_bytes(&bad).is_err(), "variant {variant}");
        assert_eq!(world.save_bytes().unwrap(), saved);
    }
    let bytes = scene.to_bytes();
    let suffix = bytes.len() - 152;
    for (offset, value) in [(suffix, 1025u32), (suffix + 20, 0x00802000u32)] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        scene_checksum(&mut bad);
        assert!(CookedScene::from_bytes(&bad).is_err());
    }
    let mut bad = bytes.clone();
    bad[suffix + 4..suffix + 20].copy_from_slice(&id(2).0);
    scene_checksum(&mut bad);
    assert!(CookedScene::from_bytes(&bad).is_err());
}
#[test]
fn timed_edge_fires_at_integral_tick_even_with_accumulated_roundoff() {
    let mut e = edge(30, 0, 1, 0, 0.0);
    e.conditions = Arc::from([]);
    e.after_seconds = Some(1.0);
    let original = scene(graph(vec![e]));
    let scene = CookedScene::new(original.id(), 60, original.entities().to_vec())
        .unwrap()
        .with_spatial(
            original.visual_assets().to_vec(),
            original.spatial().to_vec(),
        )
        .unwrap();
    let mut world = NativeWorld::new(scene);
    for _ in 0..59 {
        world.step().unwrap();
        assert_eq!(snapshot(&world).state, id(20));
    }
    world.step().unwrap();
    assert_eq!(snapshot(&world).state, id(21));
}

#[test]
fn graph_v14_preserves_ui_v13_suffix_and_its_explicit_checkpoint_refusal() {
    let base = scene(graph(vec![]));
    let mut entities = base.entities().to_vec();
    entities.push(CookedEntity {
        id: id(3),
        parent: None,
        transform: Transform::default(),
        velocity: None,
    });
    let scene = CookedScene::new(base.id(), base.tick_rate(), entities)
        .unwrap()
        .with_spatial(base.visual_assets().to_vec(), base.spatial().to_vec())
        .unwrap()
        .with_ui(vec![CookedUi {
            entity: id(3),
            node: Arc::new(incant_types::UiNode {
                kind: incant_types::UiKind::Root,
                style: Default::default(),
                order: 0,
                disabled: false,
            }),
        }])
        .unwrap();
    let bytes = scene.to_bytes();
    let decoded = CookedScene::from_bytes(&bytes).unwrap();
    assert_eq!(decoded.to_bytes(), bytes);
    assert_eq!(decoded.ui(), scene.ui());
    assert!(decoded.has_animation_graphs());
    let mut world = NativeWorld::new(decoded);
    world.step().unwrap();
    assert!(world.animation_graph_state(id(1)).is_some());
    let saved = world.save_bytes().unwrap();
    let restored = NativeWorld::from_save_bytes(&saved).unwrap();
    assert_eq!(
        restored.animation_graph_state(id(1)),
        world.animation_graph_state(id(1))
    );
    assert_eq!(restored.save_bytes().unwrap(), saved);
}

#[test]
fn endpoint_transition_captures_a_stopped_source_and_starts_destination() {
    let mut edge = edge(30, 0, 1, 0, 0.5);
    edge.conditions = Arc::from([]);
    edge.after_seconds = Some(0.125);
    let base = scene(graph(vec![edge]));
    let mut nodes = base.spatial().to_vec();
    let animation = Arc::make_mut(nodes[0].animation.as_mut().unwrap());
    let graph = animation.graph.as_ref().unwrap();
    let mut states = graph.states().to_vec();
    states[0].looping = false;
    animation.graph = Some(Arc::new(
        NativeAnimationGraph::new(
            0,
            graph.parameters().to_vec(),
            states,
            graph.transitions().to_vec(),
            &animation.clip_durations,
        )
        .unwrap(),
    ));
    animation.initial.looping = false;
    animation.initial.speed = 16.0;
    let assets = base.visual_assets().to_vec();
    let mut world = NativeWorld::new(base.with_spatial(assets, nodes).unwrap());
    world.step().unwrap();
    let playback = world.animation_state(id(1)).unwrap();
    let fade = playback.transition.unwrap();
    assert!(playback.playing);
    assert!(!playback.completed);
    assert_eq!(playback.sample.clip, Some(1));
    assert_eq!(playback.sample.seconds, 0.0);
    assert_eq!(fade.source.seconds, 2.0);
    assert!(!fade.playing);
    assert!(!fade.looping);
    let saved = world.save_bytes().unwrap();
    let mut fresh = NativeWorld::from_save_bytes(&saved).unwrap();
    world.step().unwrap();
    fresh.step().unwrap();
    assert_eq!(world.save_bytes().unwrap(), fresh.save_bytes().unwrap());
}

fn completing_scene(timed: bool) -> CookedScene {
    let mut e = edge(30, 0, 1, 0, 0.5);
    if timed {
        e.conditions = Arc::from([]);
        e.after_seconds = Some(0.5);
    }
    let base = scene(graph(vec![e]));
    let mut nodes = base.spatial().to_vec();
    let a = Arc::make_mut(nodes[0].animation.as_mut().unwrap());
    let g = a.graph.as_ref().unwrap();
    let mut states = g.states().to_vec();
    states[0].looping = false;
    a.graph = Some(Arc::new(
        NativeAnimationGraph::new(
            0,
            g.parameters().to_vec(),
            states,
            g.transitions().to_vec(),
            &a.clip_durations,
        )
        .unwrap(),
    ));
    a.initial.looping = false;
    a.initial.speed = 16.0;
    let assets = base.visual_assets().to_vec();
    base.with_spatial(assets, nodes).unwrap()
}
#[test]
fn completed_pose_takes_later_parameter_and_time_edges_but_master_pause_freezes_both() {
    for timed in [false, true] {
        let initial = completing_scene(timed);
        let mut world = NativeWorld::new(initial.clone());
        world.step().unwrap();
        assert!(world.animation_state(id(1)).unwrap().completed);
        assert!(!snapshot(&world).paused);
        assert_eq!(snapshot(&world).state, id(20));
        master(&mut world, AnimationControl::SetPlaying(false));
        parameter(&mut world, 0.5);
        let frozen = snapshot(&world);
        let save = world.save_bytes().unwrap();
        let mut fresh = NativeWorld::from_save_bytes_for_scene(&save, &initial).unwrap();
        for _ in 0..8 {
            world.step().unwrap();
            fresh.step().unwrap();
            assert_eq!(snapshot(&world), frozen);
            assert_eq!(world.save_bytes().unwrap(), fresh.save_bytes().unwrap());
        }
        for w in [&mut world, &mut fresh] {
            master(w, AnimationControl::SetPlaying(true));
        }
        for _ in 0..3 {
            world.step().unwrap();
            fresh.step().unwrap();
            assert_eq!(world.save_bytes().unwrap(), fresh.save_bytes().unwrap());
        }
        assert_eq!(snapshot(&world).state, id(21));
        assert!(!snapshot(&world).paused);
    }
    let mut world = NativeWorld::new(completing_scene(false));
    world.step().unwrap();
    let saved = world.save_bytes().unwrap();
    let mut resumed = NativeWorld::from_save_bytes(&saved).unwrap();
    for w in [&mut world, &mut resumed] {
        parameter(w, 0.5);
        w.step().unwrap();
    }
    assert_eq!(world.save_bytes().unwrap(), resumed.save_bytes().unwrap());
    let fade = world.animation_state(id(1)).unwrap().transition.unwrap();
    assert_eq!(snapshot(&world).state, id(21));
    assert!(!fade.playing);
    assert_eq!(fade.source.seconds, 2.0);
    let mut world = NativeWorld::new(completing_scene(true));
    world.step().unwrap();
    for _ in 0..2 {
        world.step().unwrap();
        assert_eq!(snapshot(&world).state, id(20));
    }
    world.step().unwrap();
    assert_eq!(snapshot(&world).state, id(21));
}
#[test]
fn manual_state_selection_uses_master_pause_after_natural_completion() {
    let mut world = NativeWorld::new(completing_scene(false));
    world.step().unwrap();
    select(&mut world, 22);
    assert!(world.animation_state(id(1)).unwrap().playing);
    assert!(!snapshot(&world).paused);
    master(&mut world, AnimationControl::SetPlaying(false));
    select(&mut world, 20);
    assert!(!world.animation_state(id(1)).unwrap().playing);
    assert!(snapshot(&world).paused);
}

#[path = "animation_graph/fades.rs"]
mod fades;

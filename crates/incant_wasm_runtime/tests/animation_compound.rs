#[path = "support/animation_graph.rs"]
mod graph;
use incant_module::{
    AnimationGraphSnapshotV2, AnimationIkSnapshotV2, AnimationPoseKindV5, AnimationPoseSnapshotV5,
};
use incant_runtime::*;
use incant_wasm_runtime::{BoundaryError, Call, Capability};
use std::sync::Arc;
fn scene(ik: bool) -> CookedScene {
    let base = graph::scene(0.5);
    let mut nodes = base.spatial().to_vec();
    for n in &mut nodes {
        let a = Arc::make_mut(n.animation.as_mut().unwrap());
        let old = a.graph.as_ref().unwrap();
        let mut states = old.states().to_vec();
        states.push(NativeAnimationGraphState {
            id: StableId([21; 16]),
            motion: NativeAnimationMotion::Clip(Some(0)),
            looping: true,
        });
        a.graph = Some(Arc::new(
            NativeAnimationGraph::new(
                0,
                old.parameters().to_vec(),
                states,
                vec![NativeAnimationGraphTransition {
                    id: StableId([30; 16]),
                    from: 0,
                    to: 1,
                    priority: 0,
                    fade_seconds: 0.5,
                    after_seconds: None,
                    conditions: Arc::from([]),
                }],
                &[1., 2.],
            )
            .unwrap(),
        ));
        if ik {
            n.ik = Some(Arc::new(NativeIk {
                mesh_fingerprint: [4; 32],
                initial: Arc::from([AnimationIkSample {
                    id: StableId([9; 16]),
                    chain: [0, 1, 2],
                    target: [1., 1., 0.],
                    pole: [0., 1., 0.],
                    weight: 0.5,
                }]),
            }));
        }
    }
    base.clone()
        .with_spatial(base.visual_assets().to_vec(), nodes)
        .unwrap()
}
fn read(c: &mut Capability, m: &mut [u8], e: i64, call: Call) -> Vec<u8> {
    let n = c.invoke(m, e, call).unwrap() as usize;
    m[3..3 + n].to_vec()
}
#[test]
fn actual_graph_compound_clock_and_pose_reads_are_captured_before_controls() {
    let mut w = NativeWorld::new(scene(false));
    w.step().unwrap();
    let mut c = Capability::default();
    let mut m = vec![0xa5; 8192];
    for turn in 0..35 {
        let e = c.begin(&mut w, 1. / 60.).unwrap();
        let bytes = read(
            &mut c,
            &mut m,
            e,
            Call::ReadAnimationsV5 {
                ptr: 3,
                capacity: 8000,
            },
        );
        let graphs = read(
            &mut c,
            &mut m,
            e,
            Call::ReadAnimationGraphsV2 {
                ptr: 3,
                capacity: 8000,
            },
        );
        let view = AnimationPoseSnapshotV5::parse(&bytes).unwrap();
        let gs = AnimationGraphSnapshotV2::parse(&graphs).unwrap();
        let mut native = Vec::new();
        w.snapshot_animations(&mut native).unwrap();
        for (copy, n) in view.iter().zip(&native) {
            assert_eq!(copy.entity.0, n.id.0);
            assert_eq!(copy.clip, n.state.sample.clip);
            assert_eq!(copy.seconds, n.state.sample.seconds);
            if let Some(pose) = n.pose {
                assert_eq!(copy.outer_weight, pose.outer_weight());
                for i in 0..2 {
                    match (copy.group(i), pose.group(i)) {
                        (Some(a), Some(b)) => {
                            assert_eq!(a.interpolation as u8, b.interpolation as u8);
                            assert_eq!(a.samples().len(), b.samples.len());
                            for (a, b) in a.samples().zip(b.samples) {
                                assert_eq!(
                                    (a.clip, a.seconds, a.weight),
                                    (b.sample.clip, b.sample.seconds, b.weight)
                                );
                            }
                        }
                        (None, None) => {}
                        _ => panic!("group mismatch"),
                    }
                }
            }
        }
        let mut native_graphs = Vec::new();
        w.snapshot_animation_graphs(&mut native_graphs).unwrap();
        for (a, b) in gs.fades().zip(native_graphs.iter().filter_map(|r| r.fade)) {
            assert_eq!(a.state.0, b.state.0);
            assert_eq!(
                (a.seconds, a.speed, a.elapsed, a.duration, a.playing),
                (b.seconds, b.speed, b.elapsed, b.duration, b.playing)
            );
        }
        if turn == 0 {
            assert_eq!(
                view.iter().next().unwrap().kind,
                AnimationPoseKindV5::Compound
            );
            assert_eq!(gs.fades().len(), 2);
        }
        m[..16].copy_from_slice(&StableId([1; 16]).0);
        m[32..56].copy_from_slice(
            &incant_wasm_abi::AnimationCommand {
                kind: 4,
                clip: 0,
                seconds: 0.,
                value: -1.,
            }
            .encode(),
        );
        c.invoke(&mut m, e, Call::ControlAnimationV2 { id: 0, ptr: 32 })
            .unwrap();
        assert_eq!(
            read(
                &mut c,
                &mut m,
                e,
                Call::ReadAnimationsV5 {
                    ptr: 3,
                    capacity: 8000
                }
            ),
            bytes
        );
        assert_eq!(
            read(
                &mut c,
                &mut m,
                e,
                Call::ReadAnimationGraphsV2 {
                    ptr: 3,
                    capacity: 8000
                }
            ),
            graphs
        );
        c.finish(&mut w).unwrap();
        w.step().unwrap();
    }
}
#[test]
fn complete_ik_v2_does_not_enable_legacy_or_incomplete_readers_from_shared_caches() {
    for old in [
        Call::AnimationLen,
        Call::AnimationLenV2,
        Call::AnimationLenV3,
        Call::AnimationLenV4,
        Call::AnimationLenV5,
        Call::AnimationIkLen,
        Call::AnimationGraphLen,
        Call::ReadAnimations {
            ptr: 3,
            capacity: 8000,
        },
        Call::ReadAnimationsV2 {
            ptr: 3,
            capacity: 8000,
        },
        Call::ReadAnimationsV3 {
            ptr: 3,
            capacity: 8000,
        },
        Call::ReadAnimationsV4 {
            ptr: 3,
            capacity: 8000,
        },
        Call::ReadAnimationsV5 {
            ptr: 3,
            capacity: 8000,
        },
        Call::ReadAnimationIk {
            ptr: 3,
            capacity: 8000,
        },
        Call::ReadAnimationGraphs {
            ptr: 3,
            capacity: 8000,
        },
    ] {
        let mut w = NativeWorld::new(scene(true));
        w.step().unwrap();
        let before = w.save_bytes().unwrap();
        let mut c = Capability::default();
        let mut m = vec![0xa5; 8192];
        let e = c.begin(&mut w, 1. / 60.).unwrap();
        let bytes = read(
            &mut c,
            &mut m,
            e,
            Call::ReadAnimationIkV2 {
                ptr: 3,
                capacity: 8000,
            },
        );
        let view = AnimationIkSnapshotV2::parse(&bytes).unwrap();
        assert_eq!(view.constraints().len(), 2);
        assert_eq!(
            view.poses().iter().next().unwrap().kind,
            AnimationPoseKindV5::Compound
        );
        read(
            &mut c,
            &mut m,
            e,
            Call::ReadAnimationGraphsV2 {
                ptr: 3,
                capacity: 8000,
            },
        );
        let untouched = m.clone();
        assert!(matches!(
            c.invoke(&mut m, e, old),
            Err(BoundaryError::Abi(_))
        ));
        assert_eq!(m, untouched);
        assert!(c.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
    }
}
#[test]
fn all_new_readers_refuse_bounds_before_copy_and_poison_only_current_epoch() {
    for call in [
        Call::ReadAnimationsV5 {
            ptr: 3,
            capacity: 1,
        },
        Call::ReadAnimationIkV2 {
            ptr: -1,
            capacity: 8000,
        },
        Call::ReadAnimationGraphsV2 {
            ptr: 8190,
            capacity: 8000,
        },
    ] {
        let mut w = NativeWorld::new(scene(false));
        w.step().unwrap();
        let before = w.save_bytes().unwrap();
        let mut c = Capability::default();
        let mut m = vec![0xa5; 8192];
        let e = c.begin(&mut w, 1. / 60.).unwrap();
        assert!(c.invoke(&mut m, e - 1, call).is_err());
        c.finish(&mut w).unwrap();
        let e = c.begin(&mut w, 1. / 60.).unwrap();
        assert!(c.invoke(&mut m, e, call).is_err());
        assert!(m.iter().all(|v| *v == 0xa5));
        assert!(c.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
    }
}

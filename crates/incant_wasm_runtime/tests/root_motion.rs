#[allow(dead_code)] // Shared fixture also provides the independent actual-host oracle.
#[path = "support/root_motion.rs"]
mod fixture;
use fixture::{id, scene};
use incant_module::{RootMotionClockOwner, RootMotionSnapshot};
use incant_runtime::*;
use incant_wasm_abi as abi;
use incant_wasm_runtime::{Call, Capability};
const DT: f64 = 1. / 60.;
fn read(c: &mut Capability, m: &mut [u8], e: i64) -> Vec<u8> {
    let len = c.invoke(m, e, Call::RootMotionLen).unwrap();
    let before = m.to_vec();
    assert_eq!(
        c.invoke(
            m,
            e,
            Call::ReadRootMotion {
                ptr: 7,
                capacity: len
            }
        )
        .unwrap(),
        len
    );
    assert_eq!(&m[..7], &before[..7]);
    assert_eq!(&m[7 + len as usize..], &before[7 + len as usize..]);
    m[7..7 + len as usize].to_vec()
}
fn control(
    c: &mut Capability,
    m: &mut [u8],
    e: i64,
    entity: StableId,
    kind: u32,
    value: u32,
) -> Result<i32, incant_wasm_runtime::BoundaryError> {
    let b = abi::RootMotionCommand {
        entity: entity.0,
        kind,
        value,
    }
    .encode();
    m[15000..15000 + b.len()].copy_from_slice(&b);
    c.invoke(
        m,
        e,
        Call::ControlRootMotion {
            ptr: 15000,
            len: b.len() as i32,
        },
    )
}
#[test]
fn complete_composed_frame_matches_all_native_fields_and_controls_capture_before_commands() {
    let mut w = NativeWorld::new(scene());
    let mut c = Capability::default();
    let mut m = vec![0xa5; 32768];
    let mut compound = false;
    for turn in 0..96 {
        w.step().unwrap();
        let e = c.begin(&mut w, DT).unwrap();
        let bytes = read(&mut c, &mut m, e);
        let s = RootMotionSnapshot::parse(&bytes).unwrap();
        let mut rows = Vec::new();
        w.snapshot_root_motion(&mut rows).unwrap();
        assert_eq!(s.scene().0, w.scene_id().0);
        assert_eq!(s.len(), rows.len());
        for (a, b) in s.iter().zip(&rows) {
            assert_eq!(a.entity.0, b.entity.0);
            assert_eq!(a.fingerprint, b.fingerprint);
            assert_eq!(a.root, b.root);
            assert_eq!(a.applying, b.applying);
            assert_eq!(a.tick, b.tick);
            for (a, b) in [(Some(a.current), Some(b.current)), (a.outgoing, b.outgoing)] {
                assert_eq!(a.is_some(), b.is_some());
                if let (Some(a), Some(b)) = (a, b) {
                    assert_eq!(a.cycle, b.cycle);
                    assert_eq!(a.seconds.to_bits(), b.seconds.to_bits());
                    match a.owner {
                        RootMotionClockOwner::State(s) => assert_eq!(Some(StableId(s.0)), b.state),
                        RootMotionClockOwner::Clip(n) => assert_eq!(Some(n), b.clip),
                        RootMotionClockOwner::Rest => {
                            assert!(b.state.is_none() && b.clip.is_none())
                        }
                    }
                }
            }
            for (a, b) in [
                (a.requested, b.requested),
                (a.applied, b.applied),
                (a.accumulated_requested, b.accumulated_requested),
                (a.accumulated_applied, b.accumulated_applied),
            ] {
                assert_eq!(a.translation, b.translation);
                assert_eq!(a.rotation, b.rotation);
            }
        }
        compound |= s
            .poses()
            .iter()
            .any(|p| p.kind == incant_module::AnimationPoseKindV5::Compound);
        assert_eq!(s.constraints().len(), 4);
        control(&mut c, &mut m, e, id(1), 0, u32::from(turn % 2 == 0)).unwrap();
        assert_eq!(read(&mut c, &mut m, e), bytes);
        c.finish(&mut w).unwrap();
    }
    assert!(compound);
}
#[test]
fn every_legacy_complete_reader_refuses_root_strip_before_and_after_cache_and_rolls_back() {
    for inactive_extract_only in [false, true] {
        for cached in [false, true] {
            for call in [
                Call::AnimationLen,
                Call::ReadAnimations {
                    ptr: 0,
                    capacity: 32768,
                },
                Call::AnimationLenV2,
                Call::ReadAnimationsV2 {
                    ptr: 0,
                    capacity: 32768,
                },
                Call::AnimationLenV3,
                Call::ReadAnimationsV3 {
                    ptr: 0,
                    capacity: 32768,
                },
                Call::AnimationLenV4,
                Call::ReadAnimationsV4 {
                    ptr: 0,
                    capacity: 32768,
                },
                Call::AnimationLenV5,
                Call::ReadAnimationsV5 {
                    ptr: 0,
                    capacity: 32768,
                },
                Call::AnimationIkLen,
                Call::ReadAnimationIk {
                    ptr: 0,
                    capacity: 32768,
                },
                Call::AnimationIkLenV2,
                Call::ReadAnimationIkV2 {
                    ptr: 0,
                    capacity: 32768,
                },
            ] {
                let mut selected = scene();
                if inactive_extract_only {
                    let mut nodes = selected.spatial().to_vec();
                    for node in &mut nodes {
                        if node.entity == id(1) {
                            let root = std::sync::Arc::make_mut(node.root_motion.as_mut().unwrap());
                            root.application = incant_types::RootMotionApplication::ExtractOnly;
                            root.applying = false;
                        } else {
                            node.root_motion = None;
                        }
                    }
                    selected = CookedScene::new(
                        selected.id(),
                        selected.tick_rate(),
                        selected.entities().to_vec(),
                    )
                    .unwrap()
                    .with_spatial(selected.visual_assets().to_vec(), nodes)
                    .unwrap();
                }
                let mut w = NativeWorld::new(selected);
                let before = w.save_bytes().unwrap();
                let mut c = Capability::default();
                let mut m = vec![0xa5; 32768];
                let e = c.begin(&mut w, DT).unwrap();
                if cached {
                    read(&mut c, &mut m, e);
                }
                control(&mut c, &mut m, e, id(1), 0, 0).unwrap();
                let old = m.clone();
                assert!(
                    c.invoke(&mut m, e, call)
                        .unwrap_err()
                        .to_string()
                        .contains("cannot represent root stripping")
                );
                assert_eq!(m, old);
                assert!(c.finish(&mut w).is_err());
                assert_eq!(w.save_bytes().unwrap(), before);
            }
        }
    }
}
#[test]
fn invalid_caught_commands_buffers_and_late_owner_failure_discard_whole_prefix() {
    for mode in 0..8 {
        let mut w = NativeWorld::new(scene());
        let before = w.save_bytes().unwrap();
        let mut c = Capability::default();
        let mut m = vec![0xa5; 32768];
        let e = c.begin(&mut w, DT).unwrap();
        control(&mut c, &mut m, e, id(1), 0, 0).unwrap();
        let result = match mode {
            0 => control(&mut c, &mut m, e, id(1), 0, 2),
            1 => control(&mut c, &mut m, e, id(1), 1, 1),
            2 => control(&mut c, &mut m, e, id(1), 2, 0),
            3 => control(&mut c, &mut m, e, id(999), 1, 0),
            4 => c.invoke(
                &mut m,
                e,
                Call::ControlRootMotion {
                    ptr: 15000,
                    len: 23,
                },
            ),
            _ => {
                let old = m.clone();
                let r = c.invoke(
                    &mut m,
                    e,
                    Call::ReadRootMotion {
                        ptr: if mode == 5 { -1 } else { 7 },
                        capacity: if mode == 6 { 1 } else { 32768 },
                    },
                );
                assert_eq!(m, old);
                r
            }
        };
        assert_eq!(result.is_err(), mode != 3);
        assert!(c.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
    }
}
#[test]
fn stale_epoch_cannot_poison_new_callback_and_world_identity_never_aliases() {
    let mut w = NativeWorld::new(scene());
    let mut c = Capability::default();
    let mut m = vec![0; 32768];
    let old = c.begin(&mut w, DT).unwrap();
    c.finish(&mut w).unwrap();
    let e = c.begin(&mut w, DT).unwrap();
    assert!(control(&mut c, &mut m, old, id(1), 0, 0).is_err());
    control(&mut c, &mut m, e, id(1), 0, 0).unwrap();
    c.finish(&mut w).unwrap();
    let mut rows = Vec::new();
    w.snapshot_root_motion(&mut rows).unwrap();
    assert!(!rows[0].applying);
    let mut other = NativeWorld::new(scene());
    assert!(c.begin(&mut other, DT).is_err());
}

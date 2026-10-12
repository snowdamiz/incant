#[path = "support/animation_ik.rs"]
mod fixture;
use fixture::{id, scene};
use incant_module::AnimationIkSnapshot;
use incant_runtime::*;
use incant_wasm_abi as abi;
use incant_wasm_runtime::{Call, Capability};
const DT: f64 = 1. / 60.;
fn read(cap: &mut Capability, m: &mut [u8], epoch: i64) -> Vec<u8> {
    let len = cap.invoke(m, epoch, Call::AnimationIkLen).unwrap();
    let old = m.to_vec();
    assert_eq!(
        cap.invoke(
            m,
            epoch,
            Call::ReadAnimationIk {
                ptr: 7,
                capacity: len
            }
        )
        .unwrap(),
        len
    );
    assert_eq!(&m[..7], &old[..7]);
    assert_eq!(&m[7 + len as usize..], &old[7 + len as usize..]);
    m[7..7 + len as usize].to_vec()
}
fn control(
    cap: &mut Capability,
    m: &mut [u8],
    epoch: i64,
    entity: StableId,
    constraint: StableId,
    x: f64,
) -> Result<i32, incant_wasm_runtime::BoundaryError> {
    let cmd = abi::AnimationIkCommand {
        entity: entity.0,
        constraint: constraint.0,
        target: [x, 1., 0.],
        pole: [0., 1., 0.],
        weight: 0.25,
    }
    .encode();
    m[12000..12000 + cmd.len()].copy_from_slice(&cmd);
    cap.invoke(
        m,
        epoch,
        Call::ControlAnimationIk {
            ptr: 12000,
            len: cmd.len() as i32,
        },
    )
}
#[test]
fn complete_frozen_pose_and_constraints_match_actual_native_state_then_publish_controls() {
    let mut w = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut m = vec![0xa5; 16384];
    for turn in 0..4 {
        w.step().unwrap();
        let epoch = cap.begin(&mut w, DT).unwrap();
        let bytes = read(&mut cap, &mut m, epoch);
        let s = AnimationIkSnapshot::parse(&bytes).unwrap();
        assert_eq!(s.poses().scene().0, w.scene_id().0);
        assert_eq!(s.poses().len(), 3);
        let mut native = Vec::new();
        w.snapshot_ik(&mut native).unwrap();
        for (a, b) in s.constraints().zip(&native) {
            assert_eq!(a.entity.0, b.entity.0);
            assert_eq!(a.constraint.0, b.sample.id.0);
            assert_eq!(a.chain, b.sample.chain);
            assert_eq!(a.target, b.sample.target);
            assert_eq!(a.weight, b.sample.weight);
        }
        assert_eq!(s.constraints().len(), native.len());
        control(&mut cap, &mut m, epoch, id(1), id(9), turn as f64).unwrap();
        assert_eq!(read(&mut cap, &mut m, epoch), bytes);
        cap.finish(&mut w).unwrap();
        assert_eq!(w.ik_state(id(1)).unwrap()[0].target[0], turn as f64);
        fixture::oracle(&mut w, turn);
    }
}
#[test]
fn caught_bad_control_or_late_missing_identity_rejects_whole_native_prefix() {
    for mode in 0..4 {
        let mut w = NativeWorld::new(scene());
        let before = w.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut m = vec![0xa5; 16384];
        let e = cap.begin(&mut w, DT).unwrap();
        control(&mut cap, &mut m, e, id(1), id(9), 5.).unwrap();
        let result = match mode {
            0 => control(&mut cap, &mut m, e, id(1), id(9), f64::NAN),
            1 => control(&mut cap, &mut m, e, id(1), id(9), 1e9 + 1.),
            2 => control(&mut cap, &mut m, e, id(1), id(99), 1.),
            _ => control(&mut cap, &mut m, e, id(99), id(9), 1.),
        };
        assert_eq!(result.is_err(), mode < 2);
        assert!(cap.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
    }
}
#[test]
fn short_or_out_of_bounds_destination_never_copies_and_poison_cannot_be_bypassed() {
    for mode in 0..3 {
        let mut w = NativeWorld::new(scene());
        let before = w.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut m = vec![0xa5; 16384];
        let e = cap.begin(&mut w, DT).unwrap();
        control(&mut cap, &mut m, e, id(1), id(9), 5.).unwrap();
        let len = cap.invoke(&mut m, e, Call::AnimationIkLen).unwrap();
        let old = m.clone();
        let call = match mode {
            0 => Call::ReadAnimationIk {
                ptr: 7,
                capacity: len - 1,
            },
            1 => Call::ReadAnimationIk {
                ptr: -1,
                capacity: len,
            },
            _ => Call::ReadAnimationIk {
                ptr: 16380,
                capacity: len,
            },
        };
        assert!(cap.invoke(&mut m, e, call).is_err());
        assert_eq!(m, old);
        assert!(cap.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
    }
}

#[test]
fn complete_frame_cache_never_authorizes_older_incomplete_pose_imports() {
    for call in [
        Call::AnimationLen,
        Call::ReadAnimations {
            ptr: 0,
            capacity: 4096,
        },
        Call::AnimationLenV2,
        Call::ReadAnimationsV2 {
            ptr: 0,
            capacity: 4096,
        },
        Call::AnimationLenV3,
        Call::ReadAnimationsV3 {
            ptr: 0,
            capacity: 4096,
        },
        Call::AnimationLenV4,
        Call::ReadAnimationsV4 {
            ptr: 0,
            capacity: 4096,
        },
    ] {
        let mut w = NativeWorld::new(scene());
        let before = w.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut m = vec![0; 16384];
        let e = cap.begin(&mut w, DT).unwrap();
        read(&mut cap, &mut m, e);
        control(&mut cap, &mut m, e, id(1), id(9), 42.).unwrap();
        let error = cap.invoke(&mut m, e, call).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("cannot represent IK constraints")
        );
        assert!(cap.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
    }
}

#[test]
fn stale_epochs_cannot_change_current_ik_and_wrong_world_rejects_publication() {
    let mut w = NativeWorld::new(scene());
    let mut other = NativeWorld::new(scene());
    let before = other.save_bytes().unwrap();
    let mut cap = Capability::default();
    let mut m = vec![0; 16384];
    let old = cap.begin(&mut w, DT).unwrap();
    cap.finish(&mut w).unwrap();
    let current = cap.begin(&mut w, DT).unwrap();
    assert!(control(&mut cap, &mut m, old, id(1), id(9), 99.).is_err());
    control(&mut cap, &mut m, current, id(1), id(9), 3.).unwrap();
    cap.finish(&mut w).unwrap();
    assert_eq!(w.ik_state(id(1)).unwrap()[0].target[0], 3.);
    let before_w = w.save_bytes().unwrap();
    let e = cap.begin(&mut w, DT).unwrap();
    control(&mut cap, &mut m, e, id(1), id(9), 7.).unwrap();
    assert!(cap.finish(&mut other).is_err());
    assert_eq!(w.save_bytes().unwrap(), before_w);
    assert_eq!(other.save_bytes().unwrap(), before);
}

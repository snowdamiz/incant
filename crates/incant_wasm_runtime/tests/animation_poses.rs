#[path = "support/animation_poses.rs"]
mod fixture;
use fixture::{id, scene};
use incant_module::{AnimationPoseKind as Kind, AnimationPoseSnapshot};
use incant_runtime::*;
use incant_wasm_abi as abi;
use incant_wasm_runtime::{Call, Capability, limits};
const DT: f64 = 1.0 / 60.0;

fn read(cap: &mut Capability, memory: &mut [u8], epoch: i64) -> Vec<u8> {
    let len = cap.invoke(memory, epoch, Call::AnimationLenV4).unwrap();
    let before = memory.to_vec();
    assert_eq!(
        cap.invoke(
            memory,
            epoch,
            Call::ReadAnimationsV4 {
                ptr: 7,
                capacity: len + 3
            }
        )
        .unwrap(),
        len
    );
    assert_eq!(&before[..7], &memory[..7]);
    assert_eq!(&before[7 + len as usize..], &memory[7 + len as usize..]);
    memory[7..7 + len as usize].to_vec()
}
fn prefix(cap: &mut Capability, memory: &mut [u8], epoch: i64) {
    cap.invoke(
        memory,
        epoch,
        Call::Column {
            chunk: 0,
            ptr: 4096,
            velocity: false,
            write: false,
        },
    )
    .unwrap();
    memory[4096..4104].copy_from_slice(&99f64.to_le_bytes());
    cap.invoke(
        memory,
        epoch,
        Call::Column {
            chunk: 0,
            ptr: 4096,
            velocity: false,
            write: true,
        },
    )
    .unwrap();
    memory[10000..10016].copy_from_slice(&id(1).0);
    memory[10032..10064].copy_from_slice(
        &abi::AnimationGraphCommand {
            kind: 0,
            reserved: 0,
            id: id(10).0,
            value: 0.75,
        }
        .encode(),
    );
    cap.invoke(
        memory,
        epoch,
        Call::ControlAnimationGraph {
            id: 10000,
            ptr: 10032,
        },
    )
    .unwrap();
    memory[10080..10086].copy_from_slice(b"prefix");
    cap.invoke(
        memory,
        epoch,
        Call::Log {
            level: 1,
            ptr: 10080,
            len: 6,
        },
    )
    .unwrap();
}
#[test]
fn complete_native_pose_samples_and_interpolation_semantics_remain_copied_across_controls() {
    let mut world = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut memory = vec![0xa5; 16384];
    for turn in 0..20 {
        world.step().unwrap();
        let mut native = Vec::new();
        world.snapshot_animations(&mut native).unwrap();
        let epoch = cap.begin(&mut world, DT).unwrap();
        let bytes = read(&mut cap, &mut memory, epoch);
        let snapshot = AnimationPoseSnapshot::parse(&bytes).unwrap();
        assert_eq!(snapshot.scene().0, id(99).0);
        assert_eq!(snapshot.len(), 3);
        for (index, row) in snapshot.iter().enumerate() {
            let original = native.iter().find(|n| n.id.0 == row.entity.0).unwrap();
            assert_eq!(original.state.sample.clip, row.clip);
            assert_eq!(
                original.state.sample.seconds.to_bits(),
                row.seconds.to_bits()
            );
            let values: Vec<_> = row.samples().collect();
            if let Some(pose) = original.pose {
                assert_eq!(row.kind, Kind::WeightedQuaternionMean);
                assert_eq!(values.len(), pose.samples().len());
                for (actual, expected) in values.iter().zip(pose.samples()) {
                    assert_eq!(actual.clip, expected.sample.clip);
                    assert_eq!(actual.seconds.to_bits(), expected.sample.seconds.to_bits());
                    assert_eq!(actual.weight.to_bits(), expected.weight.to_bits());
                }
                if turn == 0 {
                    assert_eq!(values.len(), 8);
                    for value in values {
                        let clip = value.clip.unwrap();
                        let x = if clip % 2 == 0 { 0.75 } else { 0.25 };
                        let z = if clip < 4 { 0.25 } else { 0.75 };
                        assert_eq!(value.weight, x * 0.5 * z);
                    }
                }
            } else if let Some(blend) = original.blend {
                assert_eq!(values.len(), 2);
                assert_eq!(values[0].clip, blend.sample.clip);
                assert_eq!(values[0].seconds, blend.sample.seconds);
                assert_eq!(values[0].weight, 1.0 - blend.weight);
                assert_eq!(values[1].weight, blend.weight);
                assert_eq!(
                    row.kind,
                    if original.state.transition.is_some() {
                        Kind::SlerpTransition
                    } else {
                        Kind::SlerpParameter
                    }
                );
            } else {
                assert_eq!(row.kind, Kind::Single);
                assert_eq!(values.len(), 1);
                assert_eq!(values[0].weight, 1.0);
            }
            assert_eq!(row.entity.0, id(index as u128 + 1).0);
        }
        prefix(&mut cap, &mut memory, epoch);
        assert_eq!(read(&mut cap, &mut memory, epoch), bytes);
        cap.finish(&mut world).unwrap();
        fixture::oracle(&mut world, turn);
        // Caller-owned copies still retain the old native frame after acceptance.
        assert_eq!(
            snapshot.iter().next().unwrap().seconds,
            native
                .iter()
                .find(|r| r.id == id(1))
                .unwrap()
                .state
                .sample
                .seconds
        );
    }
}
#[test]
fn whole_copy_preflight_and_authorized_failures_reject_all_staged_work() {
    for (ptr, capacity) in [(-1, 2048), (7, -1), (7, 1), (16000, 2048), (0, i32::MAX)] {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0xa5; 16384];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        let untouched = memory.clone();
        assert!(
            cap.invoke(&mut memory, epoch, Call::ReadAnimationsV4 { ptr, capacity })
                .is_err()
        );
        assert_eq!(memory, untouched);
        assert!(cap.finish(&mut world).is_err());
        assert_eq!(world.save_bytes().unwrap(), before);
        assert!(cap.logs().is_err());
    }
}
#[test]
fn stale_epoch_does_not_poison_next_callback_and_query_quota_does() {
    let mut world = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut memory = vec![0xa5; 16384];
    let old = cap.begin(&mut world, DT).unwrap();
    read(&mut cap, &mut memory, old);
    cap.finish(&mut world).unwrap();
    let epoch = cap.begin(&mut world, DT).unwrap();
    let before = memory.clone();
    for call in [
        Call::AnimationLenV4,
        Call::ReadAnimationsV4 {
            ptr: 7,
            capacity: 2048,
        },
    ] {
        assert!(cap.invoke(&mut memory, old, call).is_err());
    }
    assert_eq!(memory, before);
    read(&mut cap, &mut memory, epoch);
    cap.finish(&mut world).unwrap();
    let before = world.save_bytes().unwrap();
    let epoch = cap.begin(&mut world, DT).unwrap();
    prefix(&mut cap, &mut memory, epoch);
    let mut exhausted = false;
    for _ in 0..=limits::CALLS {
        if cap
            .invoke(&mut memory, epoch, Call::AnimationLenV4)
            .is_err()
        {
            exhausted = true;
            break;
        }
    }
    assert!(exhausted);
    assert!(cap.finish(&mut world).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
}

#[test]
fn exact_transfer_exhaustion_does_not_copy_a_partial_frame_or_accept_controls() {
    let base = scene();
    let template = base.spatial().iter().find(|n| n.entity == id(2)).unwrap();
    let entities = (1000..3048)
        .map(|n| CookedEntity {
            id: id(n),
            parent: None,
            transform: incant_types::Transform::default(),
            velocity: None,
        })
        .collect();
    let nodes = (1000..3048)
        .map(|n| {
            let mut node = template.clone();
            node.entity = id(n);
            node
        })
        .collect();
    let scene = CookedScene::new(id(99), 60, entities)
        .unwrap()
        .with_spatial(base.visual_assets().to_vec(), nodes)
        .unwrap();
    let mut world = NativeWorld::new(scene);
    let before = world.save_bytes().unwrap();
    let mut cap = Capability::default();
    let mut memory = vec![0xa5; 300000];
    let epoch = cap.begin(&mut world, DT).unwrap();
    memory[..16].copy_from_slice(&id(1000).0);
    memory[16..40].copy_from_slice(
        &abi::AnimationCommand {
            kind: 2,
            clip: 0,
            seconds: 0.0,
            value: 0.0,
        }
        .encode(),
    );
    cap.invoke(
        &mut memory,
        epoch,
        Call::ControlAnimationV2 { id: 0, ptr: 16 },
    )
    .unwrap();
    let len = cap
        .invoke(&mut memory, epoch, Call::AnimationLenV4)
        .unwrap();
    assert_eq!(len, 245800);
    for _ in 0..(limits::TRANSFERS - 40) / (len as usize) {
        cap.invoke(
            &mut memory,
            epoch,
            Call::ReadAnimationsV4 {
                ptr: 0,
                capacity: len,
            },
        )
        .unwrap();
    }
    memory.fill(0xa5);
    let error = cap
        .invoke(
            &mut memory,
            epoch,
            Call::ReadAnimationsV4 {
                ptr: 0,
                capacity: len,
            },
        )
        .unwrap_err();
    assert!(error.to_string().contains("transfer quota"));
    assert!(memory.iter().all(|v| *v == 0xa5));
    assert!(cap.finish(&mut world).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
}

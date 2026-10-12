#[path = "support/animations.rs"]
mod fixture;
use fixture::{id, scene};
use incant_runtime::*;
use incant_types::Transform;
use incant_wasm_abi as abi;
use incant_wasm_runtime::{BoundaryError, Call, Capability, limits};

const DT: f64 = 1. / 60.;
const ID: usize = 513;
const COMMAND: usize = 533;
const SNAPSHOT: usize = 7;
const LENGTH: usize = abi::AnimationHeader::SIZE + 2 * abi::AnimationRow::SIZE;

fn command(kind: u32, clip: u32, seconds: f64, value: f64) -> abi::AnimationCommand {
    abi::AnimationCommand {
        kind,
        clip,
        seconds,
        value,
    }
}
fn control(
    cap: &mut Capability,
    memory: &mut [u8],
    epoch: i64,
    entity: StableId,
    command: abi::AnimationCommand,
) -> Result<i32, BoundaryError> {
    memory[ID..ID + 16].copy_from_slice(&entity.0);
    memory[COMMAND..COMMAND + abi::AnimationCommand::SIZE].copy_from_slice(&command.encode());
    cap.invoke(
        memory,
        epoch,
        Call::ControlAnimation {
            id: ID as i32,
            ptr: COMMAND as i32,
        },
    )
}
fn read(cap: &mut Capability, memory: &mut [u8], epoch: i64) -> Vec<u8> {
    let length = cap.invoke(memory, epoch, Call::AnimationLen).unwrap();
    assert_eq!(length as usize, LENGTH);
    let before = memory.to_vec();
    assert_eq!(
        cap.invoke(
            memory,
            epoch,
            Call::ReadAnimations {
                ptr: SNAPSHOT as i32,
                capacity: length + 3,
            },
        )
        .unwrap(),
        length
    );
    assert_eq!(&memory[..SNAPSHOT], &before[..SNAPSHOT]);
    assert_eq!(&memory[SNAPSHOT + LENGTH..], &before[SNAPSHOT + LENGTH..]);
    memory[SNAPSHOT..SNAPSHOT + LENGTH].to_vec()
}
fn row(bytes: &[u8], entity: StableId) -> abi::AnimationRow {
    bytes[abi::AnimationHeader::SIZE..]
        .as_chunks::<{ abi::AnimationRow::SIZE }>()
        .0
        .iter()
        .map(|raw| abi::AnimationRow::decode(raw).unwrap())
        .find(|row| row.id == entity.0)
        .unwrap()
}
fn prefix(cap: &mut Capability, memory: &mut [u8], epoch: i64) {
    cap.invoke(
        memory,
        epoch,
        Call::Column {
            chunk: 0,
            ptr: 0,
            velocity: false,
            write: false,
        },
    )
    .unwrap();
    memory[..8].copy_from_slice(&99f64.to_le_bytes());
    cap.invoke(
        memory,
        epoch,
        Call::Column {
            chunk: 0,
            ptr: 0,
            velocity: false,
            write: true,
        },
    )
    .unwrap();
    memory[ID - 16..ID].copy_from_slice(&id(2).0);
    memory[ID..ID + 16].copy_from_slice(&id(1).0);
    cap.invoke(memory, epoch, Call::SetParent((ID - 16) as i32, ID as i32))
        .unwrap();
    control(cap, memory, epoch, id(1), command(2, 0, 0., 0.)).unwrap();
    memory[600..607].copy_from_slice(b"pending");
    cap.invoke(
        memory,
        epoch,
        Call::Log {
            level: 1,
            ptr: 600,
            len: 7,
        },
    )
    .unwrap();
}
fn rejected(cap: &mut Capability, world: &mut NativeWorld, before: &[u8]) {
    assert!(cap.finish(world).is_err());
    assert!(cap.is_failed());
    assert!(cap.logs().is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
}

#[test]
fn copied_unaligned_snapshot_is_owned_and_all_controls_publish_in_order() {
    let mut world = NativeWorld::new(scene());
    for _ in 0..12 {
        world.step().unwrap();
    }
    let old = world.animation_state(id(1)).unwrap();
    let other = world.animation_state(id(2)).unwrap();
    let mut cap = Capability::default();
    let mut memory = vec![0xa5; 1024];
    let epoch = cap.begin(&mut world, DT).unwrap();
    let bytes = read(&mut cap, &mut memory, epoch);
    let header = abi::AnimationHeader::decode(&bytes).unwrap();
    assert_eq!(header.magic, *abi::ANIMATION_MAGIC);
    assert_eq!(header.count, 2);
    assert_eq!(header.stride as usize, abi::AnimationRow::SIZE);
    assert!(bytes.len() <= abi::ANIMATION_MAX_BYTES);
    for entity in [id(1), id(2)] {
        let row = row(&bytes, entity);
        assert_eq!((row.clip, row.speed, row.flags), (0, 1., 1));
        assert_eq!(
            row.seconds,
            world.animation_state(entity).unwrap().sample.seconds
        );
        assert!((row.seconds - 0.2).abs() < 1e-12);
    }
    // The unaligned copied bytes are writable guest data, never a native loan.
    memory[SNAPSHOT..SNAPSHOT + LENGTH].fill(0);
    assert_eq!(read(&mut cap, &mut memory, epoch), bytes);
    for value in [
        command(0, 0, 0.5, 0.),
        command(1, 0, 0.75, 0.),
        command(4, 0, 0., -2.),
        command(3, 0, 0., 1.),
        command(2, 0, 0., 0.),
    ] {
        control(&mut cap, &mut memory, epoch, id(1), value).unwrap();
    }
    assert_eq!(read(&mut cap, &mut memory, epoch), bytes);
    assert_eq!(world.animation_state(id(1)), Some(old));
    cap.finish(&mut world).unwrap();
    let state = world.animation_state(id(1)).unwrap();
    assert_eq!(
        state.sample,
        AnimationSample {
            clip: Some(0),
            seconds: 0.75
        }
    );
    assert_eq!(state.speed, -2.);
    assert!(!state.playing && state.looping && !state.completed);
    assert_eq!(world.animation_state(id(2)), Some(other));
    let next = cap.begin(&mut world, DT).unwrap();
    let changed = read(&mut cap, &mut memory, next);
    assert_eq!(row(&changed, id(1)).flags, 2);
    assert_eq!(row(&changed, id(1)).seconds, 0.75);
    cap.abort();
}

#[test]
fn rest_and_zero_duration_snapshots_follow_native_completion_and_flags() {
    let mut world = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut memory = vec![0; 1024];
    let epoch = cap.begin(&mut world, DT).unwrap();
    control(
        &mut cap,
        &mut memory,
        epoch,
        id(1),
        command(0, u32::MAX, 0., 0.),
    )
    .unwrap();
    control(&mut cap, &mut memory, epoch, id(2), command(0, 1, 0., 0.)).unwrap();
    cap.finish(&mut world).unwrap();
    world.step().unwrap();
    let epoch = cap.begin(&mut world, DT).unwrap();
    let bytes = read(&mut cap, &mut memory, epoch);
    assert_eq!(row(&bytes, id(1)).clip, u32::MAX);
    assert_eq!(row(&bytes, id(1)).seconds, 0.);
    assert_eq!(row(&bytes, id(1)).flags, 1);
    assert_eq!(row(&bytes, id(2)).clip, 1);
    assert_eq!(row(&bytes, id(2)).seconds, 0.);
    assert_eq!(row(&bytes, id(2)).flags, 4);
    cap.finish(&mut world).unwrap();
}

#[test]
fn short_invalid_and_oob_ingress_copies_nothing_and_rejects_mixed_prefix() {
    for call in [
        Call::ReadAnimations {
            ptr: -1,
            capacity: LENGTH as i32,
        },
        Call::ReadAnimations {
            ptr: 7,
            capacity: -1,
        },
        Call::ReadAnimations {
            ptr: 7,
            capacity: LENGTH as i32 - 1,
        },
        Call::ReadAnimations {
            ptr: 1000,
            capacity: LENGTH as i32,
        },
        Call::ReadAnimations {
            ptr: 7,
            capacity: i32::MAX,
        },
        Call::ControlAnimation {
            id: ID as i32,
            ptr: -1,
        },
        Call::ControlAnimation {
            id: ID as i32,
            ptr: 1001,
        },
        Call::ControlAnimation {
            id: -1,
            ptr: COMMAND as i32,
        },
        Call::ControlAnimation {
            id: 1009,
            ptr: COMMAND as i32,
        },
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0x5a; 1024];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        let copied = memory.clone();
        assert!(cap.invoke(&mut memory, epoch, call).is_err());
        assert_eq!(memory, copied);
        rejected(&mut cap, &mut world, &before);
    }
}

#[test]
fn malformed_tags_reserved_fields_and_nonfinite_values_are_sticky_even_if_superseded() {
    for bad in [
        command(5, 0, 0., 0.),
        command(u32::MAX, 0, 0., 0.),
        command(0, 256, 0., 0.),
        command(0, u32::MAX, 0.1, 0.),
        command(0, 0, -0.1, 0.),
        command(0, 0, 0., 1.),
        command(0, 0, f64::NAN, 0.),
        command(0, 0, 0., f64::INFINITY),
        command(1, 1, 0., 0.),
        command(1, 0, 0., -0.),
        command(1, 0, -1., 0.),
        command(2, 0, 1., 1.),
        command(2, 0, 0., 0.5),
        command(3, 1, 0., 0.),
        command(3, 0, 0., f64::NAN),
        command(4, 0, 0., 16.0001),
        command(4, 0, 0., f64::NEG_INFINITY),
        command(4, 0, 1., 0.),
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; 1024];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        assert!(
            control(&mut cap, &mut memory, epoch, id(1), bad).is_err(),
            "{bad:?}"
        );
        control(&mut cap, &mut memory, epoch, id(1), command(1, 0, 0., 0.)).unwrap();
        rejected(&mut cap, &mut world, &before);
    }
}

#[test]
fn wire_valid_asset_clip_duration_and_unknown_entity_failures_restore_all_prefixes() {
    for (entity, bad) in [
        (id(1), command(0, 2, 0., 0.)),
        (id(1), command(0, 255, 0., 0.)),
        (id(1), command(1, 0, 1.01, 0.)),
        (id(1), command(0, 1, 0.01, 0.)),
        (id(9), command(2, 0, 0., 0.)),
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; 1024];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch);
        control(&mut cap, &mut memory, epoch, entity, bad).unwrap();
        control(&mut cap, &mut memory, epoch, id(1), command(1, 0, 0., 0.)).unwrap();
        rejected(&mut cap, &mut world, &before);
    }
}

#[test]
fn stale_and_inactive_animation_calls_do_not_poison_a_new_authorized_epoch() {
    let mut world = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut memory = vec![0; 1024];
    assert!(cap.invoke(&mut memory, 0, Call::AnimationLen).is_err());
    let old = cap.begin(&mut world, DT).unwrap();
    cap.finish(&mut world).unwrap();
    assert!(cap.invoke(&mut memory, old, Call::AnimationLen).is_err());
    let epoch = cap.begin(&mut world, DT).unwrap();
    prefix(&mut cap, &mut memory, epoch);
    assert!(cap.invoke(&mut memory, old, Call::AnimationLen).is_err());
    assert!(control(&mut cap, &mut memory, old, id(1), command(99, 0, 0., 0.)).is_err());
    assert!(cap.invoke(&mut memory, epoch, Call::ChunkLen(-1)).is_err());
    cap.finish(&mut world).unwrap();
    assert!(!world.animation_state(id(1)).unwrap().playing);
    assert_eq!(cap.logs().unwrap().entries()[0].message, "pending");
    assert_eq!(cap.logs().unwrap().entries()[0].tick, 2);
}

#[test]
fn structural_quota_is_shared_and_ignored_animation_overflow_cannot_publish_prefix() {
    let mut world = NativeWorld::new(scene());
    let before = world.save_bytes().unwrap();
    let mut cap = Capability::default();
    let mut memory = vec![0; 1024];
    let epoch = cap.begin(&mut world, DT).unwrap();
    prefix(&mut cap, &mut memory, epoch); // two queued structural commands
    memory[ID..ID + 16].copy_from_slice(&id(1).0);
    for _ in 2..limits::COMMANDS {
        cap.invoke(&mut memory, epoch, Call::SetParent(ID as i32, -1))
            .unwrap();
    }
    assert!(matches!(
        control(&mut cap, &mut memory, epoch, id(1), command(2, 0, 0., 1.)),
        Err(BoundaryError::Limit("structural command quota"))
    ));
    rejected(&mut cap, &mut world, &before);
}

#[test]
fn actual_shared_call_and_memory_limits_reject_authorized_animation_prefixes() {
    for memory_limit in [false, true] {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; 1024];
        let epoch = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, epoch); // five accepted imports
        let error = if memory_limit {
            cap.invoke(&mut vec![0; limits::MEMORY + 1], epoch, Call::AnimationLen)
                .unwrap_err()
        } else {
            for _ in 5..limits::CALLS {
                cap.invoke(&mut memory, epoch, Call::ChunkCount).unwrap();
            }
            cap.invoke(&mut memory, epoch, Call::AnimationLen)
                .unwrap_err()
        };
        assert!(matches!(error, BoundaryError::Limit(_)));
        rejected(&mut cap, &mut world, &before);
    }
}

#[test]
fn shared_transfer_limit_is_sticky_without_partial_animation_copy() {
    let initial = scene();
    let mut entities = initial.entities().to_vec();
    for n in 0..4094u64 {
        let mut raw = [0; 16];
        raw[8..].copy_from_slice(&(n + 1).to_be_bytes());
        entities.push(CookedEntity {
            id: StableId(raw),
            parent: None,
            transform: Transform::default(),
            velocity: None,
        });
    }
    let initial = CookedScene::new(id(0), 60, entities)
        .unwrap()
        .with_spatial(initial.visual_assets().to_vec(), initial.spatial().to_vec())
        .unwrap();
    let mut world = NativeWorld::new(initial);
    let before = world.save_bytes().unwrap();
    let mut cap = Capability::default();
    let mut memory = vec![0; 4096 * 80];
    let epoch = cap.begin(&mut world, DT).unwrap();
    prefix(&mut cap, &mut memory, epoch);
    // Large unrelated numeric reads reach the real transfer boundary without
    // exhausting CALLS. A rejected unrelated read retains its prior semantics.
    while cap
        .invoke(
            &mut memory,
            epoch,
            Call::Column {
                chunk: 0,
                ptr: 0,
                velocity: false,
                write: false,
            },
        )
        .is_ok()
    {}
    loop {
        memory.fill(0xa5);
        let copied = memory.clone();
        match cap.invoke(
            &mut memory,
            epoch,
            Call::ReadAnimations {
                ptr: SNAPSHOT as i32,
                capacity: LENGTH as i32,
            },
        ) {
            Ok(length) => assert_eq!(length as usize, LENGTH),
            Err(error) => {
                assert!(
                    matches!(error, BoundaryError::Limit("transfer quota")),
                    "{error}"
                );
                assert_eq!(memory, copied);
                break;
            }
        }
    }
    rejected(&mut cap, &mut world, &before);
}

#[test]
fn terminal_failure_hides_prior_logs_and_replacement_keeps_only_committed_history() {
    let mut world = NativeWorld::new(scene());
    let before = world.save_bytes().unwrap();
    let mut cap = Capability::default();
    let mut memory = vec![0; 1024];
    let epoch = cap.begin(&mut world, DT).unwrap();
    memory[..8].copy_from_slice(b"accepted");
    cap.invoke(
        &mut memory,
        epoch,
        Call::Log {
            level: 1,
            ptr: 0,
            len: 8,
        },
    )
    .unwrap();
    cap.finish(&mut world).unwrap();
    let epoch = cap.begin(&mut world, DT).unwrap();
    prefix(&mut cap, &mut memory, epoch);
    assert!(control(&mut cap, &mut memory, epoch, id(1), command(99, 0, 0., 0.)).is_err());
    rejected(&mut cap, &mut world, &before);
    let mut replacement = cap.replacement();
    assert_eq!(replacement.logs().unwrap().entries().len(), 1);
    assert_eq!(replacement.logs().unwrap().entries()[0].message, "accepted");
    let fresh = replacement.begin(&mut world, DT).unwrap();
    assert_eq!(
        row(&read(&mut replacement, &mut memory, fresh), id(1)).flags,
        1
    );
    replacement.finish(&mut world).unwrap();
    replacement.abort();
    assert!(
        replacement
            .invoke(&mut memory, fresh, Call::AnimationLen)
            .is_err()
    );
}

use super::*;
const V2_LENGTH: usize = abi::AnimationHeader::SIZE + 2 * abi::AnimationRowV2::SIZE;
fn v2control(
    cap: &mut Capability,
    memory: &mut [u8],
    epoch: i64,
    entity: StableId,
    c: abi::AnimationCommand,
) -> Result<i32, BoundaryError> {
    memory[ID..ID + 16].copy_from_slice(&entity.0);
    memory[COMMAND..COMMAND + abi::AnimationCommand::SIZE].copy_from_slice(&c.encode());
    cap.invoke(
        memory,
        epoch,
        Call::ControlAnimationV2 {
            id: ID as i32,
            ptr: COMMAND as i32,
        },
    )
}
fn read_v2(cap: &mut Capability, memory: &mut [u8], epoch: i64) -> Vec<u8> {
    let n = cap.invoke(memory, epoch, Call::AnimationLenV2).unwrap();
    assert_eq!(n as usize, V2_LENGTH);
    cap.invoke(
        memory,
        epoch,
        Call::ReadAnimationsV2 {
            ptr: SNAPSHOT as i32,
            capacity: n,
        },
    )
    .unwrap();
    memory[SNAPSHOT..SNAPSHOT + V2_LENGTH].to_vec()
}
fn v2row(bytes: &[u8], id: StableId) -> abi::AnimationRowV2 {
    bytes[abi::AnimationHeader::SIZE..]
        .as_chunks::<{ abi::AnimationRowV2::SIZE }>()
        .0
        .iter()
        .map(|b| abi::AnimationRowV2::decode(b).unwrap())
        .find(|r| r.id == id.0)
        .unwrap()
}
fn start(world: &mut NativeWorld, duration: f64) {
    let mut commands = FrameCommands::default();
    commands
        .push(StructuralCommand::ControlAnimation {
            entity: id(1),
            control: AnimationControl::Crossfade {
                target: AnimationSample {
                    clip: Some(0),
                    seconds: 0.75,
                },
                duration,
            },
        })
        .unwrap();
    world.apply(&mut commands).unwrap();
}

#[test]
fn copied_unaligned_v2_rows_publish_all_captured_fields_and_keep_controls_invisible() {
    let mut world = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut memory = vec![0xa5; 1024];
    let epoch = cap.begin(&mut world, DT).unwrap();
    let initial = read_v2(&mut cap, &mut memory, epoch);
    let header = abi::AnimationHeader::decode(&initial).unwrap();
    assert_eq!(header.magic, *abi::ANIMATION_V2_MAGIC);
    assert_eq!(header.stride, 80);
    let row = v2row(&initial, id(1));
    assert_eq!(
        (
            row.transition_flags,
            row.source_clip,
            row.source_seconds,
            row.source_speed,
            row.elapsed,
            row.duration
        ),
        (0, 0, 0., 0., 0., 0.)
    );
    v2control(
        &mut cap,
        &mut memory,
        epoch,
        id(1),
        command(5, 0, 0.75, 0.5),
    )
    .unwrap();
    v2control(&mut cap, &mut memory, epoch, id(1), command(4, 0, 0., -2.)).unwrap();
    v2control(&mut cap, &mut memory, epoch, id(1), command(3, 0, 0., 1.)).unwrap();
    assert_eq!(read_v2(&mut cap, &mut memory, epoch), initial);
    assert!(world.animation_state(id(1)).unwrap().transition.is_none());
    cap.finish(&mut world).unwrap();
    world.step().unwrap();
    let epoch = cap.begin(&mut world, DT).unwrap();
    let bytes = read_v2(&mut cap, &mut memory, epoch);
    let row = v2row(&bytes, id(1));
    assert_eq!(
        (
            row.source_clip,
            row.transition_flags,
            row.source_speed,
            row.duration
        ),
        (0, 3, 1., 0.5)
    );
    assert_eq!((row.source_seconds, row.elapsed), (DT, DT));
    assert!((row.seconds - (0.75 - 2. * DT)).abs() < 1e-12);
    memory[SNAPSHOT..SNAPSHOT + V2_LENGTH].fill(0);
    assert_eq!(read_v2(&mut cap, &mut memory, epoch), bytes);
    v2control(&mut cap, &mut memory, epoch, id(1), command(1, 0, 0.5, 0.)).unwrap();
    assert_eq!(read_v2(&mut cap, &mut memory, epoch), bytes);
    cap.finish(&mut world).unwrap();
    let epoch = cap.begin(&mut world, DT).unwrap();
    let bytes = read_v2(&mut cap, &mut memory, epoch);
    let row = v2row(&bytes, id(1));
    assert_eq!(
        (
            row.transition_flags,
            row.source_clip,
            row.source_seconds,
            row.source_speed,
            row.elapsed,
            row.duration
        ),
        (0, 0, 0., 0., 0., 0.)
    );
    assert_eq!(
        cap.invoke(&mut memory, epoch, Call::AnimationLen).unwrap() as usize,
        LENGTH
    );
    cap.finish(&mut world).unwrap();
}

#[test]
fn active_legacy_length_and_read_refusals_are_consistent_and_sticky() {
    for call in [
        Call::AnimationLen,
        Call::ReadAnimations {
            ptr: SNAPSHOT as i32,
            capacity: LENGTH as i32,
        },
    ] {
        let mut world = NativeWorld::new(scene());
        start(&mut world, 1.);
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0x5a; 1024];
        let e = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, e);
        let copied = memory.clone();
        assert!(matches!(
            cap.invoke(&mut memory, e, call),
            Err(BoundaryError::Abi(
                "active transitions require animation v2"
            ))
        ));
        assert_eq!(memory, copied);
        assert!(cap.invoke(&mut memory, e, Call::AnimationLenV2).is_ok());
        rejected(&mut cap, &mut world, &before);
    }
}
#[test]
fn bad_v2_buffers_and_controls_refuse_without_any_partial_copy_or_mixed_publication() {
    let calls = [
        Call::ReadAnimationsV2 {
            ptr: 7,
            capacity: V2_LENGTH as i32 - 1,
        },
        Call::ReadAnimationsV2 {
            ptr: -1,
            capacity: V2_LENGTH as i32,
        },
        Call::ReadAnimationsV2 {
            ptr: 1000,
            capacity: V2_LENGTH as i32,
        },
        Call::ReadAnimationsV2 {
            ptr: 7,
            capacity: i32::MAX,
        },
        Call::ControlAnimationV2 {
            id: ID as i32,
            ptr: -1,
        },
        Call::ControlAnimationV2 {
            id: 1009,
            ptr: COMMAND as i32,
        },
    ];
    for call in calls {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; 1024];
        let e = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, e);
        let old = memory.clone();
        assert!(cap.invoke(&mut memory, e, call).is_err());
        assert_eq!(memory, old);
        rejected(&mut cap, &mut world, &before);
    }
    for c in [
        command(5, 0, 0., -1.),
        command(5, 0, 0., 60.001),
        command(5, 0, 0., f64::NAN),
        command(5, 0, f64::INFINITY, 1.),
        command(5, 256, 0., 1.),
        command(5, u32::MAX, 1., 1.),
        command(6, 0, 0., 1.),
    ] {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; 1024];
        let e = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, e);
        assert!(v2control(&mut cap, &mut memory, e, id(1), c).is_err());
        v2control(&mut cap, &mut memory, e, id(1), command(5, 0, 0., 0.)).unwrap();
        rejected(&mut cap, &mut world, &before);
    }
}
#[test]
fn actual_native_active_retarget_duration_and_entity_refusals_roll_back_mixed_prefix() {
    for active in [false, true] {
        for (entity, c) in [
            (id(1), command(5, 0, 1.01, 1.)),
            (id(1), command(5, 1, 0.01, 1.)),
            (id(9), command(5, 0, 0., 1.)),
            (id(1), command(5, 0, 0., 1.)),
        ] {
            if !active && entity == id(1) && c.seconds == 0. {
                continue;
            }
            let mut world = NativeWorld::new(scene());
            if active {
                start(&mut world, 1.);
            }
            let before = world.save_bytes().unwrap();
            let mut cap = Capability::default();
            let mut memory = vec![0; 1024];
            let e = cap.begin(&mut world, DT).unwrap();
            prefix(&mut cap, &mut memory, e);
            v2control(&mut cap, &mut memory, e, entity, c).unwrap();
            rejected(&mut cap, &mut world, &before);
        }
    }
}
#[test]
fn shared_ingress_and_command_limits_latch_v2_but_stale_calls_do_not() {
    for memory_limit in [false, true] {
        let mut world = NativeWorld::new(scene());
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; 1024];
        let e = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, e);
        assert!(
            cap.invoke(&mut memory, e + 1, Call::AnimationLenV2)
                .is_err()
        );
        let error = if memory_limit {
            cap.invoke(&mut vec![0; limits::MEMORY + 1], e, Call::AnimationLenV2)
                .unwrap_err()
        } else {
            for _ in 5..limits::CALLS {
                cap.invoke(&mut memory, e, Call::ChunkCount).unwrap();
            }
            cap.invoke(&mut memory, e, Call::AnimationLenV2)
                .unwrap_err()
        };
        assert!(matches!(error, BoundaryError::Limit(_)));
        rejected(&mut cap, &mut world, &before);
    }
    let mut world = NativeWorld::new(scene());
    let before = world.save_bytes().unwrap();
    let mut cap = Capability::default();
    let mut memory = vec![0; 1024];
    let e = cap.begin(&mut world, DT).unwrap();
    prefix(&mut cap, &mut memory, e);
    for _ in 2..limits::COMMANDS {
        v2control(&mut cap, &mut memory, e, id(1), command(2, 0, 0., 1.)).unwrap();
    }
    assert!(v2control(&mut cap, &mut memory, e, id(1), command(5, 0, 0., 0.)).is_err());
    rejected(&mut cap, &mut world, &before);
}

#[test]
fn duration_zero_and_sixty_endpoints_are_admitted_without_hidden_transition_flags() {
    for duration in [0., 60.] {
        let mut world = NativeWorld::new(scene());
        let mut cap = Capability::default();
        let mut memory = vec![0; 1024];
        let e = cap.begin(&mut world, DT).unwrap();
        v2control(&mut cap, &mut memory, e, id(1), command(5, 0, 0., duration)).unwrap();
        cap.finish(&mut world).unwrap();
        let e = cap.begin(&mut world, DT).unwrap();
        let row = v2row(&read_v2(&mut cap, &mut memory, e), id(1));
        assert_eq!(row.duration, duration);
        assert_eq!(row.transition_flags, if duration == 0. { 0 } else { 3 });
        cap.finish(&mut world).unwrap();
    }
}

use super::*;
use incant_module::{
    AnimationBlendKind, AnimationGraphSnapshot as GuestGraphs, AnimationSnapshot as GuestAnimations,
};

fn read_snapshot(cap: &mut Capability, memory: &mut [u8], epoch: i64, graph: bool) -> Vec<u8> {
    let length = cap
        .invoke(
            memory,
            epoch,
            if graph {
                Call::AnimationGraphLen
            } else {
                Call::AnimationLenV3
            },
        )
        .unwrap();
    let before = memory.to_vec();
    let call = if graph {
        Call::ReadAnimationGraphs {
            ptr: SNAPSHOT as i32,
            capacity: length + 3,
        }
    } else {
        Call::ReadAnimationsV3 {
            ptr: SNAPSHOT as i32,
            capacity: length + 3,
        }
    };
    assert_eq!(cap.invoke(memory, epoch, call).unwrap(), length);
    let end = SNAPSHOT + length as usize;
    assert_eq!(&memory[..SNAPSHOT], &before[..SNAPSHOT]);
    assert_eq!(&memory[end..], &before[end..]);
    memory[SNAPSHOT..end].to_vec()
}
fn graph_command(kind: u32, key: u8, value: f64) -> abi::AnimationGraphCommand {
    abi::AnimationGraphCommand {
        kind,
        reserved: 0,
        id: id(key).0,
        value,
    }
}
fn graph_control(
    cap: &mut Capability,
    memory: &mut [u8],
    epoch: i64,
    entity: StableId,
    command: abi::AnimationGraphCommand,
) -> Result<i32, BoundaryError> {
    memory[ID..ID + 16].copy_from_slice(&entity.0);
    memory[COMMAND..COMMAND + abi::AnimationGraphCommand::SIZE].copy_from_slice(&command.encode());
    cap.invoke(
        memory,
        epoch,
        Call::ControlAnimationGraph {
            id: ID as i32,
            ptr: COMMAND as i32,
        },
    )
}

#[test]
fn actual_native_graph_and_two_pose_snapshots_are_owned_and_commands_publish_atomically() {
    let mut world = NativeWorld::new(graph_fixture::scene(0.25));
    for _ in 0..12 {
        world.step().unwrap();
    }
    let mut cap = Capability::default();
    let mut memory = vec![0xa5; 1024];
    let epoch = cap.begin(&mut world, DT).unwrap();
    let graph_bytes = read_snapshot(&mut cap, &mut memory, epoch, true);
    let pose_bytes = read_snapshot(&mut cap, &mut memory, epoch, false);
    let graphs = GuestGraphs::parse(&graph_bytes).unwrap();
    let poses = GuestAnimations::parse(&pose_bytes).unwrap();
    assert_eq!((graphs.len(), poses.len()), (2, 2));
    for graph in graphs.iter() {
        let native = world
            .animation_graph_state(StableId(graph.entity.0))
            .unwrap();
        assert_eq!(graph.state.0, id(20).0);
        assert_eq!(
            (graph.seconds, graph.elapsed, graph.paused),
            (native.seconds, native.elapsed, native.paused)
        );
        let parameter = graph.parameters().next().unwrap();
        assert_eq!((parameter.id.0, parameter.value), (id(10).0, 0.25));
    }
    for pose in poses.iter() {
        let blend = pose.pose_blend.unwrap();
        assert_eq!(
            (pose.clip, blend.clip, blend.weight),
            (Some(1), Some(0), 0.25)
        );
        assert_eq!(blend.kind, AnimationBlendKind::Parameter);
        assert!((pose.seconds - 0.4).abs() < 1e-12);
        assert!((blend.seconds - 0.2).abs() < 1e-12);
        assert!(pose.transition.is_none());
    }
    prefix(&mut cap, &mut memory, epoch);
    graph_control(
        &mut cap,
        &mut memory,
        epoch,
        id(1),
        graph_command(0, 10, 0.75),
    )
    .unwrap();
    graph_control(
        &mut cap,
        &mut memory,
        epoch,
        id(1),
        graph_command(1, 20, 0.),
    )
    .unwrap();
    memory[SNAPSHOT..SNAPSHOT + pose_bytes.len()].fill(0);
    assert_eq!(
        read_snapshot(&mut cap, &mut memory, epoch, true),
        graph_bytes
    );
    assert_eq!(
        read_snapshot(&mut cap, &mut memory, epoch, false),
        pose_bytes
    );
    assert_eq!(
        world.animation_graph_state(id(1)).unwrap().parameters[0],
        0.25
    );
    cap.finish(&mut world).unwrap();
    let native = world.animation_graph_state(id(1)).unwrap();
    assert_eq!(
        (
            native.parameters[0],
            native.seconds,
            native.elapsed,
            native.paused
        ),
        (0.75, 0., 0., true)
    );
    assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 99.);
    assert_eq!(cap.logs().unwrap().entries().len(), 1);
    let epoch = cap.begin(&mut world, DT).unwrap();
    let graph_bytes = read_snapshot(&mut cap, &mut memory, epoch, true);
    let graph = GuestGraphs::parse(&graph_bytes)
        .unwrap()
        .iter()
        .next()
        .unwrap();
    assert_eq!(graph.parameters().next().unwrap().value, 0.75);
    assert!(graph.paused);
    let poses = read_snapshot(&mut cap, &mut memory, epoch, false);
    assert_eq!(
        GuestAnimations::parse(&poses)
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .pose_blend
            .unwrap()
            .weight,
        0.75
    );
    cap.abort();
}

#[test]
fn v3_timed_fade_reports_actual_native_blend_and_graphless_world_has_empty_graph_frame() {
    let mut world = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut memory = vec![0; 1024];
    let e = cap.begin(&mut world, DT).unwrap();
    let empty = read_snapshot(&mut cap, &mut memory, e, true);
    assert!(GuestGraphs::parse(&empty).unwrap().is_empty());
    let poses = read_snapshot(&mut cap, &mut memory, e, false);
    assert!(
        GuestAnimations::parse(&poses)
            .unwrap()
            .iter()
            .all(|r| r.pose_blend.is_none())
    );
    memory[ID..ID + 16].copy_from_slice(&id(1).0);
    memory[COMMAND..COMMAND + 24].copy_from_slice(&command(5, 0, 0.75, 0.5).encode());
    cap.invoke(
        &mut memory,
        e,
        Call::ControlAnimationV2 {
            id: ID as i32,
            ptr: COMMAND as i32,
        },
    )
    .unwrap();
    cap.finish(&mut world).unwrap();
    world.step().unwrap();
    let e = cap.begin(&mut world, DT).unwrap();
    let poses = read_snapshot(&mut cap, &mut memory, e, false);
    let pose = GuestAnimations::parse(&poses)
        .unwrap()
        .iter()
        .find(|r| r.entity.0 == id(1).0)
        .unwrap();
    let blend = pose.pose_blend.unwrap();
    let fade = pose.transition.unwrap();
    assert_eq!(blend.kind, AnimationBlendKind::Transition);
    assert_eq!(
        (blend.clip, blend.seconds, blend.weight),
        (fade.clip, fade.seconds, DT / 0.5)
    );
    cap.finish(&mut world).unwrap();
}

#[test]
fn bad_graph_and_pose_destinations_copy_nothing_and_poison_caught_errors() {
    for graph in [false, true] {
        for (ptr, capacity) in [(-1, 300), (7, -1), (7, 1), (1000, 300), (7, i32::MAX)] {
            let mut world = NativeWorld::new(graph_fixture::scene(0.25));
            let before = world.save_bytes().unwrap();
            let mut cap = Capability::default();
            let mut memory = vec![0x5a; 1024];
            let e = cap.begin(&mut world, DT).unwrap();
            prefix(&mut cap, &mut memory, e);
            let copied = memory.clone();
            let call = if graph {
                Call::ReadAnimationGraphs { ptr, capacity }
            } else {
                Call::ReadAnimationsV3 { ptr, capacity }
            };
            assert!(cap.invoke(&mut memory, e, call).is_err());
            assert_eq!(memory, copied);
            cap.invoke(&mut memory, e, Call::ChunkCount).unwrap();
            rejected(&mut cap, &mut world, &before);
        }
    }
    for (id, ptr) in [
        (-1, COMMAND as i32),
        (1009, COMMAND as i32),
        (ID as i32, -1),
        (ID as i32, 1000),
    ] {
        let mut world = NativeWorld::new(graph_fixture::scene(0.25));
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; 1024];
        let e = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, e);
        let copied = memory.clone();
        assert!(
            cap.invoke(&mut memory, e, Call::ControlAnimationGraph { id, ptr })
                .is_err()
        );
        assert_eq!(memory, copied);
        rejected(&mut cap, &mut world, &before);
    }
}

#[test]
fn malformed_ingress_and_deferred_native_errors_discard_numeric_structural_and_log_prefixes() {
    for mode in 0..11 {
        let mut world = NativeWorld::new(graph_fixture::scene(0.25));
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0; 1024];
        let e = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, e);
        graph_control(&mut cap, &mut memory, e, id(1), graph_command(0, 10, 0.75)).unwrap();
        let mut c = graph_command(0, 10, 0.5);
        let mut entity = id(1);
        match mode {
            0 => c.kind = 2,
            1 => c.reserved = 1,
            2 => c.value = f64::NAN,
            3 => c.value = f64::INFINITY,
            4 => c.value = 1_000_001.,
            5 => c = graph_command(1, 20, -0.),
            6 => c = graph_command(1, 20, 0.5),
            7 => c.id = id(99).0,
            8 => c.value = 1.1,
            9 => c = graph_command(1, 99, 0.),
            _ => entity = id(99),
        }
        let result = graph_control(&mut cap, &mut memory, e, entity, c);
        assert_eq!(result.is_err(), mode < 7, "mode {mode}");
        rejected(&mut cap, &mut world, &before);
    }
}

#[test]
fn expired_epochs_do_not_poison_new_updates_but_command_exhaustion_does() {
    let mut world = NativeWorld::new(graph_fixture::scene(0.25));
    let mut cap = Capability::default();
    let mut memory = vec![0; 1024];
    let stale = cap.begin(&mut world, DT).unwrap();
    cap.finish(&mut world).unwrap();
    let valid = cap.begin(&mut world, DT).unwrap();
    for call in [
        Call::AnimationLenV3,
        Call::AnimationGraphLen,
        Call::ReadAnimationGraphs {
            ptr: 7,
            capacity: 300,
        },
        Call::ControlAnimationGraph {
            id: ID as i32,
            ptr: COMMAND as i32,
        },
    ] {
        assert!(cap.invoke(&mut memory, stale, call).is_err());
    }
    prefix(&mut cap, &mut memory, valid);
    cap.finish(&mut world).unwrap();
    let before = world.save_bytes().unwrap();
    let e = cap.begin(&mut world, DT).unwrap();
    for _ in 0..limits::COMMANDS {
        graph_control(&mut cap, &mut memory, e, id(1), graph_command(0, 10, 0.5)).unwrap();
    }
    assert!(graph_control(&mut cap, &mut memory, e, id(1), graph_command(0, 10, 0.75)).is_err());
    rejected(&mut cap, &mut world, &before);
}

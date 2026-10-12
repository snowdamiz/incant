use super::*;

#[test]
fn graph_blend_v1_v2_length_and_read_refuse_without_copy_and_poison_caught_errors() {
    for call in [
        Call::AnimationLen,
        Call::AnimationLenV2,
        Call::ReadAnimations {
            ptr: 7,
            capacity: 500,
        },
        Call::ReadAnimationsV2 {
            ptr: 7,
            capacity: 500,
        },
    ] {
        let mut world = NativeWorld::new(graph_fixture::scene(0.25));
        world.step().unwrap();
        let mut rows = Vec::new();
        world.snapshot_animations(&mut rows).unwrap();
        assert!(
            rows.iter()
                .all(|r| r.state.transition.is_none() && r.blend.unwrap().weight == 0.25)
        );
        let before = world.save_bytes().unwrap();
        let mut cap = Capability::default();
        let mut memory = vec![0x5a; 1024];
        let e = cap.begin(&mut world, DT).unwrap();
        prefix(&mut cap, &mut memory, e);
        let copied = memory.clone();
        assert!(matches!(
            cap.invoke(&mut memory, e, call),
            Err(BoundaryError::Abi(
                "animation graph blend requires graph snapshot ABI"
            ))
        ));
        assert_eq!(memory, copied);
        // Catching the import error and issuing a later valid operation cannot
        // publish the numeric, structural, playback or log prefix.
        cap.invoke(&mut memory, e, Call::ChunkCount).unwrap();
        rejected(&mut cap, &mut world, &before);
    }
}
#[test]
fn graph_endpoints_remain_representable_and_blends_allow_other_capabilities() {
    for value in [0., 0.25, 1.] {
        let mut world = NativeWorld::new(graph_fixture::scene(value));
        let mut cap = Capability::default();
        let mut memory = vec![0xa5; 1024];
        let e = cap.begin(&mut world, DT).unwrap();
        if value != 0.25 {
            let v1 = read(&mut cap, &mut memory, e);
            assert_eq!(row(&v1, id(1)).clip, if value == 0. { 0 } else { 1 });
            let n = cap.invoke(&mut memory, e, Call::AnimationLenV2).unwrap();
            assert_eq!(
                n as usize,
                abi::AnimationHeader::SIZE + 2 * abi::AnimationRowV2::SIZE
            );
            cap.invoke(
                &mut memory,
                e,
                Call::ReadAnimationsV2 {
                    ptr: 7,
                    capacity: n,
                },
            )
            .unwrap();
        }
        prefix(&mut cap, &mut memory, e);
        cap.finish(&mut world).unwrap();
        assert_eq!(world.inspect(id(1)).unwrap().transform.translation[0], 99.);
        assert!(world.animation_graph_state(id(1)).unwrap().paused);
        assert_eq!(cap.logs().unwrap().entries().len(), 1);
    }
}

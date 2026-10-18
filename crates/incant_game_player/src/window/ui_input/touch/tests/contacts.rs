use super::*;

#[test]
fn logical_contacts_keep_full_native_identity_and_reuse_only_after_end() {
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    let mut state = InputRuntime::default();
    // These IDs alias after a lossy u32 cast; the native adapter must not cast.
    for id in [1, (1u64 << 32) + 1, u64::MAX] {
        input
            .window(
                None,
                &mut physical,
                &native(id, NativePhase::Started, [24., 40.]),
                2.,
            )
            .unwrap();
    }
    state.advance(physical.events(), 1. / 60.).unwrap();
    assert_eq!(
        state.frame().touches.keys().copied().collect::<Vec<_>>(),
        [1, 2, 3]
    );
    assert!(
        state
            .frame()
            .touches
            .values()
            .all(|p| p.position == [12., 20.])
    );
    physical.clear();
    input
        .window(
            None,
            &mut physical,
            &native((1u64 << 32) + 1, NativePhase::Moved, [30., 44.]),
            2.,
        )
        .unwrap();
    state.advance(physical.events(), 1. / 60.).unwrap();
    assert_eq!(state.frame().touches[&2].delta, [3., 2.]);
    assert_eq!(state.frame().touches[&1].delta, [0.; 2]);
    physical.clear();
    deliver(
        &mut input,
        &mut physical,
        None,
        1,
        NativePhase::Ended,
        [12., 20.],
    );
    deliver(
        &mut input,
        &mut physical,
        None,
        1,
        NativePhase::Started,
        [1., 2.],
    );
    state.advance(physical.events(), 1. / 60.).unwrap();
    assert!(!state.frame().touches.contains_key(&1));
    assert_eq!(state.frame().touches[&4].position, [1., 2.]);
}

#[test]
fn contacts_and_identity_exhaustion_refuse_without_mutating_pending_packet() {
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    for id in 0..MAX_TOUCHES {
        deliver(
            &mut input,
            &mut physical,
            None,
            id as u64,
            NativePhase::Started,
            [id as f64, 0.],
        );
    }
    let before = physical.events().to_vec();
    assert!(matches!(
        input.window(
            None,
            &mut physical,
            &native(99, NativePhase::Started, [0., 0.]),
            1.
        ),
        Err(Error::TouchCapacity)
    ));
    assert_eq!(physical.events(), before);
    deliver(
        &mut input,
        &mut physical,
        None,
        0,
        NativePhase::Cancelled,
        [0., 0.],
    );
    deliver(
        &mut input,
        &mut physical,
        None,
        99,
        NativePhase::Started,
        [0., 0.],
    );
    let mut state = InputRuntime::default();
    state.advance(physical.events(), 1. / 60.).unwrap();
    assert_eq!(state.frame().touches.len(), MAX_TOUCHES);
    assert!(
        state
            .frame()
            .touches
            .contains_key(&(MAX_TOUCHES as u32 + 1))
    );
    input.touches.clear();
    physical.clear();
    input.touches.next_id = u32::MAX;
    assert!(matches!(
        input.window(
            None,
            &mut physical,
            &native(1, NativePhase::Started, [0., 0.]),
            1.
        ),
        Err(Error::TouchIdentity)
    ));
    assert!(physical.events().is_empty() && input.touches.slots.iter().all(Option::is_none));
}

use super::*;

fn save(s: &mut NativeSession) -> Vec<u8> {
    capture(s)
        .unwrap()
        .persisted_bytes(&mut payload(), &mut planar(), &mut spatial())
        .unwrap()
}
fn load(bytes: &[u8], code: CompiledScript) -> Result<PendingSessionCheckpoint> {
    PendingSessionCheckpoint::from_persisted_bytes(
        bytes,
        &scene(),
        code,
        Default::default(),
        &mut payload(),
        &mut planar(),
        &mut spatial(),
        &mut allocations(),
    )
}

#[test]
fn persisted_session_stages_then_atomically_replaces_opted_in_data_and_neutral_input() {
    let body = "t.state.set(Number(t.state.get()||0)+1);t.chunks[0].transforms[0]=t.state.get();";
    let code = program(body);
    let mut live = NativeSession::new(scene(), code.clone(), Default::default()).unwrap();
    for _ in 0..7 {
        live.tick(&[]).unwrap();
    }
    let bytes = save(&mut live);
    let mut fresh = load(&bytes, code).unwrap().start().unwrap();
    assert_ne!(live.world.incarnation(), fresh.world.incarnation());
    assert_eq!(live.world.snapshot(), fresh.world.snapshot());
    let before = live.world.snapshot();
    let incarnation = live.world.incarnation();
    let mut bad = bytes.clone();
    *bad.last_mut().unwrap() ^= 1;
    assert!(
        live.restore_persisted_bytes(
            &bad,
            &mut payload(),
            &mut planar(),
            &mut spatial(),
            &mut allocations()
        )
        .is_err()
    );
    assert_eq!(live.world.snapshot(), before);
    assert_eq!(live.world.incarnation(), incarnation);
    live.enable_restart_requests().unwrap();
    live.tick(&[InputEvent::Key {
        code: KeyCode::Space,
        down: true,
    }])
    .unwrap();
    live.restore_persisted_bytes(
        &bytes,
        &mut payload(),
        &mut planar(),
        &mut spatial(),
        &mut allocations(),
    )
    .unwrap();
    assert!(host(&live).unwrap().restart_requests);
    assert!(!host(&fresh).unwrap().restart_requests);
    assert!(live.input().unwrap().keyboard.held.is_empty());
    for _ in 0..11 {
        live.tick(&[]).unwrap();
        fresh.tick(&[]).unwrap();
        assert_eq!(live.world.snapshot(), fresh.world.snapshot());
    }
    assert!(load(&bytes, program("t.state.set(99);")).is_err());
    // Ordinary bytes remain a distinct format, without silently admitting v12.
    assert!(
        NativeSession::from_checkpoint_bytes(&bytes, &scene(), program(body), Default::default())
            .is_err()
    );
}

#[test]
fn persisted_haptics_pending_receiver_refuses_and_restored_authority_is_neutral() {
    use incant_input::haptics::HapticCapabilities;
    let code = program(
        "if(t.clock.tick>1){const targets=t.commands.hapticCapabilities();if(targets.length)t.commands.playHaptic(targets[0],{kind:'rumble',strong:0.5,weak:0,durationMs:125});}",
    );
    let mut live = NativeSession::new(scene(), code.clone(), Default::default()).unwrap();
    live.tick(&[]).unwrap();
    let bytes = save(&mut live);
    let checkpoint = capture(&mut live).unwrap();
    live.set_haptic_capabilities(HapticCapabilities::default().with_gamepad(0, 8).unwrap())
        .unwrap();
    live.tick(&[]).unwrap();
    let before = live.world.snapshot();
    assert!(matches!(
        live.prepare_persisted_restore(
            &bytes,
            &mut payload(),
            &mut planar(),
            &mut spatial(),
            &mut allocations()
        ),
        Err(CheckpointError::Unsupported("pending haptic output"))
    ));
    assert!(matches!(
        checkpoint.prepare_for(
            &live,
            &mut payload(),
            &mut planar(),
            &mut spatial(),
            &mut allocations()
        ),
        Err(CheckpointError::Unsupported("pending haptic output"))
    ));
    assert_eq!(live.world.snapshot(), before);
    assert_eq!(live.take_haptics().unwrap().len(), 1);
    live.restore_persisted_bytes(
        &bytes,
        &mut payload(),
        &mut planar(),
        &mut spatial(),
        &mut allocations(),
    )
    .unwrap();
    assert!(host(&live).unwrap().haptic_capabilities.is_empty());
    assert!(live.take_haptics().unwrap().is_empty());
    let mut fresh = load(&bytes, code).unwrap().start().unwrap();
    assert!(host(&fresh).unwrap().haptic_capabilities.is_empty());
    for _ in 0..3 {
        live.tick(&[]).unwrap();
        fresh.tick(&[]).unwrap();
        assert!(live.take_haptics().unwrap().is_empty());
        assert!(fresh.take_haptics().unwrap().is_empty());
    }
}

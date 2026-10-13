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

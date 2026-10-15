use super::*;

fn coroutine_code() -> CompiledScript {
    CompiledScript::compile(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/coroutine.cjs"
    )))
    .unwrap()
}
#[test]
fn active_coroutine_checkpoint_restores_continuation_without_replaying_effects() {
    let code = coroutine_code();
    let mut baseline = session(code.clone());
    baseline.tick(&[]).unwrap();
    baseline.tick(&[]).unwrap();
    assert_eq!(x(&baseline), 1.);
    let saved = baseline.checkpoint_bytes().unwrap();
    assert_eq!(u32::from_le_bytes(saved[8..12].try_into().unwrap()), 5);
    let mut restored =
        NativeSession::from_checkpoint_bytes(&saved, &scene(), code, Default::default()).unwrap();
    assert_eq!(x(&restored), 1.);
    assert!(restored.logs().unwrap().entries().is_empty());
    for _ in 0..4 {
        baseline.tick(&[]).unwrap();
        restored.tick(&[]).unwrap();
        assert_eq!(
            baseline.world().unwrap().snapshot(),
            restored.world().unwrap().snapshot()
        );
        assert_eq!(
            baseline.checkpoint_bytes().unwrap(),
            restored.checkpoint_bytes().unwrap()
        );
    }
    assert_eq!(x(&restored), 3.);
    assert_eq!(
        &baseline.logs().unwrap().entries()[1..],
        restored.logs().unwrap().entries()
    );
    assert_eq!(
        restored.logs().unwrap().entries()[0].message,
        "pulse:main:1:4"
    );
    assert!(host(&restored).unwrap().schedule.coroutines.is_none());
}
#[test]
fn coherent_coroutine_checkpoint_mutations_refuse_without_changing_receiver() {
    let mut live = session(coroutine_code());
    live.tick(&[]).unwrap();
    let saved = live.checkpoint_bytes().unwrap();
    let identity = live.world.incarnation();
    let section = codec::sections(
        &saved,
        super::super::identity(&host(&live).unwrap()._compiled),
    )
    .unwrap();
    let start = saved.len() - section.coroutines.len();
    // count4, global scope1, task name length4+4, routine length4+5,
    // optional owner1, due8, number value tag1+8.
    for (offset, value) in [
        (start + 4, 2),
        (start + 9, b'!'),
        (start + 17, b'x'),
        (start + 22, 2),
        (start + 23, 0),
    ] {
        let mut bad = saved.clone();
        bad[offset] = value;
        let mut hash = Sha256::new();
        hash.update(&bad[..108]);
        hash.update(&bad[140..]);
        bad[108..140].copy_from_slice(&hash.finalize());
        assert!(
            live.restore_checkpoint_bytes(&bad).is_err(),
            "offset {offset}"
        );
        assert_eq!(live.world.incarnation(), identity);
        assert_eq!(live.checkpoint_bytes().unwrap(), saved);
    }
}

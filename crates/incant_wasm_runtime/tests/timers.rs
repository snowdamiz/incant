use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_types::Transform;
use incant_wasm_abi as abi;
use incant_wasm_runtime::{
    BoundaryError, Call, Capability,
    persistent::{Declaration, PersistentSnapshot},
};

fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            StableId([9; 16]),
            64,
            vec![CookedEntity {
                id: StableId([1; 16]),
                parent: None,
                transform: Transform::default(),
                velocity: None,
            }],
        )
        .unwrap(),
    )
}
fn cap() -> Capability {
    Capability::with_persistence(Some(Declaration::new(16, [1; 32]).unwrap()))
}
fn begin(c: &mut Capability, w: &mut NativeWorld) -> i64 {
    w.step().unwrap();
    c.begin(w, 0.25).unwrap()
}
fn command(
    c: &mut Capability,
    e: i64,
    id: &str,
    kind: u32,
    delay: u32,
    interval: u32,
) -> Result<i32, BoundaryError> {
    let (id, id_len) = abi::timers::encode_id(id).unwrap();
    let mut bytes = abi::TimerCommand {
        id,
        id_len,
        kind,
        delay,
        interval,
    }
    .encode();
    c.invoke(
        &mut bytes,
        e,
        Call::CommandTimer {
            ptr: 0,
            len: abi::TimerCommand::SIZE as i32,
        },
    )
}
fn snapshot(c: &mut Capability, e: i64) -> (u64, f64, Vec<(String, u64)>) {
    let mut bytes = vec![0xa5; abi::TIMER_MAX_BYTES + 2];
    let len = c.invoke(&mut bytes, e, Call::TimerLen).unwrap();
    assert_eq!(
        c.invoke(
            &mut bytes,
            e,
            Call::ReadTimers {
                ptr: 1,
                capacity: len
            }
        )
        .unwrap(),
        len
    );
    assert_eq!(bytes[0], 0xa5);
    assert_eq!(bytes[len as usize + 1], 0xa5);
    let s = abi::timers::Snapshot::parse(&bytes[1..len as usize + 1]).unwrap();
    (
        s.tick(),
        s.elapsed_seconds(),
        s.events().map(|(id, t)| (id.to_owned(), t)).collect(),
    )
}

#[test]
fn fixed_tick_order_replace_and_cancel_match_the_typescript_contract() {
    let mut w = world();
    let mut c = cap();
    let e = begin(&mut c, &mut w);
    assert_eq!(snapshot(&mut c, e), (1, 0.25, vec![]));
    command(&mut c, e, "z", 0, 1, 1).unwrap();
    command(&mut c, e, "a", 0, 1, 0).unwrap();
    command(&mut c, e, "later", 0, 1, 0).unwrap();
    command(&mut c, e, "later", 0, 3, 0).unwrap();
    assert!(snapshot(&mut c, e).2.is_empty());
    c.finish(&mut w).unwrap();
    let e = begin(&mut c, &mut w);
    let expected = (2, 0.5, vec![("a".into(), 2), ("z".into(), 2)]);
    assert_eq!(snapshot(&mut c, e), expected);
    command(&mut c, e, "z", 1, 0, 0).unwrap();
    command(&mut c, e, "absent", 1, 0, 0).unwrap();
    assert_eq!(snapshot(&mut c, e), expected);
    c.finish(&mut w).unwrap();
    let e = begin(&mut c, &mut w);
    assert_eq!(snapshot(&mut c, e), (3, 0.75, vec![]));
    c.finish(&mut w).unwrap();
    let e = begin(&mut c, &mut w);
    assert_eq!(snapshot(&mut c, e), (4, 1.0, vec![("later".into(), 4)]));
    c.finish(&mut w).unwrap();
}

#[test]
fn fresh_restore_and_reload_continue_timer_deadlines_and_elapsed_time() {
    let mut w = world();
    let mut c = cap();
    let e = begin(&mut c, &mut w);
    command(&mut c, e, "periodic", 0, 2, 3).unwrap();
    c.finish(&mut w).unwrap();
    let saved = c.persistent_checkpoint(&w).unwrap();
    let saved = PersistentSnapshot::decode(&saved.encode(), saved.declaration()).unwrap();
    let mut copy_world = NativeWorld::from_save_bytes(&w.save_bytes().unwrap()).unwrap();
    let mut copy = cap();
    copy.restore_persistent(&copy_world, &saved, 50, false)
        .unwrap();
    c.prepare_reload(&cap()).unwrap();
    c = c.replacement();
    for tick in 2..=8 {
        let e = begin(&mut c, &mut w);
        let re = begin(&mut copy, &mut copy_world);
        assert!(re > e);
        let a = snapshot(&mut c, e);
        assert_eq!(a, snapshot(&mut copy, re));
        assert_eq!(a.0, tick);
        assert_eq!(a.1, tick as f64 * 0.25);
        assert_eq!(
            a.2,
            if tick == 3 || tick == 6 {
                vec![("periodic".into(), tick)]
            } else {
                vec![]
            }
        );
        c.finish(&mut w).unwrap();
        copy.finish(&mut copy_world).unwrap();
    }
}

#[test]
fn pending_capacity_and_action_capacity_are_separate_and_replacements_fit() {
    let mut w = world();
    let mut c = cap();
    let e = begin(&mut c, &mut w);
    for i in 0..128 {
        command(&mut c, e, &format!("t{i:03}"), 0, 20, 0).unwrap();
    }
    for i in 0..128 {
        command(&mut c, e, &format!("t{i:03}"), 0, 1, 0).unwrap();
    }
    c.finish(&mut w).unwrap();
    let e = begin(&mut c, &mut w);
    assert_eq!(snapshot(&mut c, e).2.len(), 128);
    c.finish(&mut w).unwrap();
    for pending in [false, true] {
        let mut w = world();
        let mut c = cap();
        let e = begin(&mut c, &mut w);
        for i in 0..if pending { 128 } else { 256 } {
            command(
                &mut c,
                e,
                &format!("t{i:03}"),
                if pending { 0 } else { 1 },
                if pending { 1 } else { 0 },
                0,
            )
            .unwrap();
        }
        assert!(
            command(
                &mut c,
                e,
                "overflow",
                if pending { 0 } else { 1 },
                if pending { 1 } else { 0 },
                0
            )
            .is_err()
        );
        assert!(c.finish(&mut w).is_err());
        assert!(c.persistent_checkpoint(&w).is_err());
    }
}

#[test]
fn active_bad_calls_roll_back_numeric_and_persistent_writes_but_stale_calls_do_not() {
    for call in [
        Call::CommandTimer { ptr: -1, len: 80 },
        Call::CommandTimer { ptr: 0, len: 79 },
        Call::ReadTimers {
            ptr: 0,
            capacity: 0,
        },
    ] {
        let mut w = world();
        let mut c = cap();
        let e = begin(&mut c, &mut w);
        let before = w.save_bytes().unwrap();
        let mut bytes = [0; 80];
        c.invoke(
            &mut bytes,
            e,
            Call::Column {
                chunk: 0,
                ptr: 0,
                velocity: false,
                write: false,
            },
        )
        .unwrap();
        bytes[..8].copy_from_slice(&9f64.to_le_bytes());
        c.invoke(
            &mut bytes,
            e,
            Call::Column {
                chunk: 0,
                ptr: 0,
                velocity: false,
                write: true,
            },
        )
        .unwrap();
        c.invoke(
            &mut [7],
            e,
            Call::WritePersistent {
                ptr: 0,
                offset: 0,
                len: 1,
            },
        )
        .unwrap();
        command(&mut c, e, "queued", 0, 1, 0).unwrap();
        assert!(c.invoke(&mut bytes, e, call).is_err());
        assert!(c.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
        assert!(c.persistent_checkpoint(&w).is_err());
    }
    let mut w = world();
    let mut c = cap();
    let e = begin(&mut c, &mut w);
    assert!(
        c.invoke(
            &mut [],
            e - 1,
            Call::ReadTimers {
                ptr: -1,
                capacity: 0
            }
        )
        .is_err()
    );
    command(&mut c, e, "healthy", 0, 1, 0).unwrap();
    c.finish(&mut w).unwrap();
    let e = begin(&mut c, &mut w);
    assert_eq!(snapshot(&mut c, e).2, vec![("healthy".into(), 2)]);
    c.finish(&mut w).unwrap();
}

#[test]
fn malformed_timer_checkpoint_never_replaces_committed_state_and_legacy_decodes() {
    let mut w = world();
    let mut c = cap();
    let e = begin(&mut c, &mut w);
    command(&mut c, e, "saved", 0, 1, 2).unwrap();
    c.finish(&mut w).unwrap();
    let s = c.persistent_checkpoint(&w).unwrap();
    let bytes = s.encode();
    for n in 0..bytes.len() {
        assert!(
            PersistentSnapshot::decode(&bytes[..n], s.declaration()).is_err(),
            "prefix{n}"
        );
    }
    let timer = 8 + Declaration::SIZE + 16 + s.bytes().len();
    for offset in [
        timer,
        timer + 8,
        timer + 24,
        timer + 28,
        timer + abi::TimerHeader::SIZE + 64,
    ] {
        let mut bad = bytes.clone();
        bad[offset] ^= 0xff;
        assert!(PersistentSnapshot::decode(&bad, s.declaration()).is_err());
    }
    let mut due = bytes.clone();
    let start = timer + abi::TimerHeader::SIZE + 72;
    due[start..start + 8].copy_from_slice(&1u64.to_le_bytes());
    assert!(PersistentSnapshot::decode(&due, s.declaration()).is_err());
    let legacy = &bytes[8..timer];
    let old = PersistentSnapshot::decode(legacy, s.declaration()).unwrap();
    assert_eq!(old.bytes(), s.bytes());
    assert_eq!(old.tick(), s.tick());
}

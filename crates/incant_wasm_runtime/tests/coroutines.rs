use incant_runtime::{
    CookedEntity, CookedScene, FrameCommands, NativeWorld, StableId, StructuralCommand,
};
use incant_wasm_abi::{self as abi, coroutines::Routine};
use incant_wasm_runtime::{
    Call, Capability,
    persistent::{Declaration, PersistentSnapshot},
};

fn world() -> NativeWorld {
    NativeWorld::new(CookedScene::new(StableId([9; 16]), 64, vec![entity()]).unwrap())
}
fn entity() -> CookedEntity {
    CookedEntity {
        id: StableId([1; 16]),
        parent: None,
        transform: Default::default(),
        velocity: None,
    }
}
fn cap() -> Capability {
    Capability::with_persistence(Some(Declaration::new(8, [9; 32]).unwrap()))
        .with_coroutines(vec![Routine::new("walk", 8, [1; 32]).unwrap()])
        .unwrap()
}
fn begin(c: &mut Capability, w: &mut NativeWorld) -> i64 {
    w.step().unwrap();
    c.begin(w, 0.25).unwrap()
}
fn start(id: &str, owner: bool) -> Vec<u8> {
    let (id, id_len) = abi::timers::encode_id(id).unwrap();
    let (routine, routine_len) = abi::timers::encode_id("walk").unwrap();
    let h = abi::CoroutineHeader {
        magic: *abi::COROUTINE_MAGIC,
        kind: 0,
        delay: 0,
        id_len,
        routine_len,
        flags: u32::from(owner),
        state_len: 8,
        id,
        routine,
        entity: if owner { [1; 16] } else { [0; 16] },
    };
    [h.encode().as_slice(), &0u64.to_le_bytes()].concat()
}
fn invoke(
    c: &mut Capability,
    e: i64,
    mut bytes: Vec<u8>,
) -> Result<i32, incant_wasm_runtime::BoundaryError> {
    let len = bytes.len() as i32;
    c.invoke(&mut bytes, e, Call::CommandCoroutine { ptr: 0, len })
}
fn cancel(c: &mut Capability, e: i64, id: &str) {
    let (id, id_len) = abi::timers::encode_id(id).unwrap();
    invoke(
        c,
        e,
        abi::CoroutineHeader {
            magic: *abi::COROUTINE_MAGIC,
            kind: 1,
            delay: 0,
            id_len,
            routine_len: 0,
            flags: 0,
            state_len: 0,
            id,
            routine: [0; 64],
            entity: [0; 16],
        }
        .encode()
        .to_vec(),
    )
    .unwrap();
}
fn next(c: &mut Capability, e: i64) -> Vec<u8> {
    let mut bytes = vec![0xa5; abi::COROUTINE_MAX_BYTES + 2];
    let n = c
        .invoke(
            &mut bytes,
            e,
            Call::NextCoroutine {
                ptr: 1,
                capacity: abi::COROUTINE_MAX_BYTES as i32,
            },
        )
        .unwrap() as usize;
    assert_eq!(bytes[0], 0xa5);
    assert_eq!(bytes[n + 1], 0xa5);
    bytes[1..1 + n].to_vec()
}
fn finish(c: &mut Capability, e: i64, mut bytes: Vec<u8>, delay: u32) {
    let (mut h, _) = abi::coroutines::frame(&bytes).unwrap();
    if delay == 0 {
        h.kind = 2;
        h.state_len = 0;
        bytes.truncate(abi::CoroutineHeader::SIZE);
    } else {
        h.kind = 3;
        h.delay = delay;
    }
    bytes[..abi::CoroutineHeader::SIZE].copy_from_slice(&h.encode());
    invoke(c, e, bytes).unwrap();
}

#[test]
fn update_precedes_sorted_resumes_and_wait_is_from_current_tick() {
    let (mut c, mut w) = (cap(), world());
    let e = begin(&mut c, &mut w);
    for id in ["z", "a", "cancelled"] {
        invoke(&mut c, e, start(id, false)).unwrap();
    }
    assert!(next(&mut c, e).is_empty());
    c.finish(&mut w).unwrap();
    let e = begin(&mut c, &mut w);
    cancel(&mut c, e, "cancelled");
    let a = next(&mut c, e);
    assert_eq!(abi::coroutines::frame(&a).unwrap().0.id[0], b'a');
    finish(&mut c, e, a, 2);
    let z = next(&mut c, e);
    assert_eq!(abi::coroutines::frame(&z).unwrap().0.id[0], b'z');
    finish(&mut c, e, z, 0);
    assert!(next(&mut c, e).is_empty());
    c.finish(&mut w).unwrap();
    let e = begin(&mut c, &mut w);
    assert!(next(&mut c, e).is_empty());
    c.finish(&mut w).unwrap();
    let e = begin(&mut c, &mut w);
    let a = next(&mut c, e);
    assert!(!a.is_empty());
    finish(&mut c, e, a, 0);
    c.finish(&mut w).unwrap();
}

#[test]
fn cancellation_and_same_id_replacement_supersede_a_running_result() {
    for restart in [false, true] {
        let (mut c, mut w) = (cap(), world());
        let e = begin(&mut c, &mut w);
        invoke(&mut c, e, start("a", false)).unwrap();
        c.finish(&mut w).unwrap();
        let e = begin(&mut c, &mut w);
        let a = next(&mut c, e);
        cancel(&mut c, e, "a");
        if restart {
            invoke(&mut c, e, start("a", false)).unwrap();
        }
        finish(&mut c, e, a, 30);
        c.finish(&mut w).unwrap();
        let e = begin(&mut c, &mut w);
        let a = next(&mut c, e);
        assert_eq!(!a.is_empty(), restart);
        if restart {
            finish(&mut c, e, a, 0);
        }
        c.finish(&mut w).unwrap();
    }
}

#[test]
fn skipped_due_work_incomplete_resume_and_ignored_invalid_calls_reject() {
    for mode in 0..5 {
        let (mut c, mut w) = (cap(), world());
        let e = begin(&mut c, &mut w);
        invoke(&mut c, e, start("a", false)).unwrap();
        c.finish(&mut w).unwrap();
        let e = begin(&mut c, &mut w);
        let before = w.snapshot();
        match mode {
            0 => {}
            1 => {
                next(&mut c, e);
            }
            2 => {
                assert!(invoke(&mut c, e, start("a", false)).is_err());
            }
            3 => {
                let mut invalid = start("b", false);
                invalid[abi::CoroutineHeader::ROUTINE_OFFSET] = b'?';
                assert!(invoke(&mut c, e, invalid).is_err());
            }
            _ => {
                assert!(
                    c.invoke(
                        &mut [0; 8],
                        e,
                        Call::NextCoroutine {
                            ptr: 0,
                            capacity: 8
                        }
                    )
                    .is_err()
                );
            }
        }
        assert!(c.finish(&mut w).is_err());
        assert!(c.is_failed());
        assert_eq!(w.snapshot(), before);
        assert!(c.prepare_reload(&cap()).is_err());
    }
}

#[test]
fn stale_epoch_does_not_poison_and_pending_owner_despawn_cannot_resurrect() {
    let (mut c, mut w) = (cap(), world());
    let e = begin(&mut c, &mut w);
    invoke(&mut c, e, start("a", true)).unwrap();
    assert!(invoke(&mut c, e + 1, start("bad", false)).is_err());
    c.finish(&mut w).unwrap();
    let mut cmds = FrameCommands::default();
    cmds.push(StructuralCommand::Despawn(StableId([1; 16])))
        .unwrap();
    w.apply(&mut cmds).unwrap();
    cmds.push(StructuralCommand::Spawn(entity())).unwrap();
    w.apply(&mut cmds).unwrap();
    let e = begin(&mut c, &mut w);
    assert!(next(&mut c, e).is_empty());
    c.finish(&mut w).unwrap();
}

#[test]
fn explicit_schedule_survives_reload_and_canonical_snapshot_but_schema_changes_refuse() {
    let (mut c, mut w) = (cap(), world());
    let e = begin(&mut c, &mut w);
    invoke(&mut c, e, start("a", true)).unwrap();
    c.finish(&mut w).unwrap();
    let bytes = c.persistent_checkpoint(&w).unwrap().encode();
    assert_eq!(&bytes[..8], b"INCWPS03");
    let decoded =
        PersistentSnapshot::decode(&bytes, Declaration::new(8, [9; 32]).unwrap()).unwrap();
    assert_eq!(decoded.encode(), bytes);
    for len in 0..bytes.len() {
        assert!(
            PersistentSnapshot::decode(&bytes[..len], Declaration::new(8, [9; 32]).unwrap())
                .is_err()
        );
    }
    let mut restored = cap();
    restored.restore_persistent(&w, &decoded, 5, false).unwrap();
    c.prepare_reload(&cap()).unwrap();
    c = c.replacement();
    w.step().unwrap();
    for host in [&mut c, &mut restored] {
        let e = host.begin(&mut w, 0.25).unwrap();
        let a = next(host, e);
        assert!(!a.is_empty());
        finish(host, e, a, 0);
        host.finish(&mut w).unwrap();
    }
    let mut changed = Capability::with_persistence(Some(Declaration::new(8, [9; 32]).unwrap()))
        .with_coroutines(vec![Routine::new("walk", 8, [2; 32]).unwrap()])
        .unwrap();
    assert!(changed.restore_persistent(&w, &decoded, 0, false).is_err());
    assert!(c.prepare_reload(&changed).is_err());
}

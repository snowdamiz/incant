use incant_runtime::{CookedScene, NativeWorld, StableId};
use incant_wasm_runtime::{
    Call, Capability,
    persistent::{Declaration, PersistentSnapshot},
};

fn world() -> NativeWorld {
    NativeWorld::new(CookedScene::new(StableId([1; 16]), 64, vec![]).unwrap())
}
fn cap() -> Capability {
    Capability::with_persistence(Some(Declaration::new(16, [1; 32]).unwrap()))
}
fn write(ptr: i32, offset: i32, len: i32) -> Call {
    Call::WritePersistent { ptr, offset, len }
}
fn read(ptr: i32, offset: i32, len: i32) -> Call {
    Call::ReadPersistent { ptr, offset, len }
}
fn begin(cap: &mut Capability, world: &mut NativeWorld) -> i64 {
    world.step().unwrap();
    cap.begin(world, 1. / 64.).unwrap()
}

#[test]
fn committed_state_stages_reads_and_roundtrips_without_guest_callbacks() {
    let mut world = world();
    let mut cap = cap();
    assert_eq!(cap.persistent_checkpoint(&world).unwrap().bytes(), &[0; 16]);
    let epoch = begin(&mut cap, &mut world);
    cap.invoke(&mut [7; 4], epoch, write(0, 4, 4)).unwrap();
    let mut scratch = [0; 16];
    cap.invoke(&mut scratch, epoch, read(0, 0, 16)).unwrap();
    assert_eq!(&scratch[4..8], &[7; 4]);
    assert!(cap.persistent_checkpoint(&world).is_err());
    cap.finish(&mut world).unwrap();
    let snapshot = cap.persistent_checkpoint(&world).unwrap();
    assert_eq!(snapshot.bytes(), scratch);
    assert_eq!(
        PersistentSnapshot::decode(&snapshot.encode(), snapshot.declaration()).unwrap(),
        snapshot
    );
    for truncated in 0..snapshot.encode().len() {
        assert!(
            PersistentSnapshot::decode(&snapshot.encode()[..truncated], snapshot.declaration())
                .is_err()
        );
    }
}

#[test]
fn ignored_current_error_is_terminal_but_stale_calls_cannot_poison_current_tick() {
    for invalid in [
        write(0, -1, 1),
        write(0, 16, 1),
        write(0, 0, -1),
        read(1, 0, 1),
    ] {
        let mut world = world();
        let mut cap = cap();
        let epoch = begin(&mut cap, &mut world);
        let before = world.save_bytes().unwrap();
        cap.invoke(&mut [8], epoch, write(0, 0, 1)).unwrap();
        assert!(cap.invoke(&mut [9], epoch, invalid).is_err());
        assert!(cap.finish(&mut world).is_err());
        assert_eq!(world.save_bytes().unwrap(), before);
        assert!(cap.persistent_checkpoint(&world).is_err());
    }
    let mut world = world();
    let mut cap = cap();
    let epoch = begin(&mut cap, &mut world);
    assert!(cap.invoke(&mut [9], epoch - 1, write(0, 0, 1)).is_err());
    cap.finish(&mut world).unwrap();
    assert_eq!(cap.persistent_checkpoint(&world).unwrap().bytes(), &[0; 16]);
}

#[test]
fn undeclared_state_and_quota_overflow_poison_even_if_error_is_ignored() {
    let mut undeclared_world = world();
    let mut missing = Capability::default();
    let epoch = begin(&mut missing, &mut undeclared_world);
    assert!(missing.invoke(&mut [], epoch, read(0, 0, 0)).is_err());
    assert!(missing.finish(&mut undeclared_world).is_err());
    for traffic in [false, true] {
        let mut world = world();
        let mut cap = Capability::with_persistence(Some(Declaration::new(65536, [2; 32]).unwrap()));
        let epoch = begin(&mut cap, &mut world);
        let mut memory = vec![1; if traffic { 65536 } else { 0 }];
        let count = if traffic { 16 } else { 256 };
        let len = memory.len() as i32;
        for _ in 0..count {
            cap.invoke(&mut memory, epoch, write(0, 0, len)).unwrap();
        }
        assert!(cap.invoke(&mut memory, epoch, write(0, 0, len)).is_err());
        assert!(cap.finish(&mut world).is_err());
        assert!(cap.persistent_checkpoint(&world).is_err());
    }
}

#[test]
fn restore_revokes_saved_and_receiver_epochs_and_keeps_restart_owner_only() {
    let mut world = world();
    let mut source = cap();
    source.enable_restart_requests().unwrap();
    let old = begin(&mut source, &mut world);
    source.invoke(&mut [7], old, write(0, 0, 1)).unwrap();
    source.finish(&mut world).unwrap();
    let snapshot = source.persistent_checkpoint(&world).unwrap();
    let mut restored_world = NativeWorld::from_save_bytes(&world.save_bytes().unwrap()).unwrap();
    let mut restored = cap();
    restored
        .restore_persistent(&restored_world, &snapshot, 40, false)
        .unwrap();
    let current = begin(&mut restored, &mut restored_world);
    assert_eq!(current, 41);
    for stale in [old, 40] {
        assert!(restored.invoke(&mut [9], stale, write(0, 0, 1)).is_err());
    }
    restored.finish(&mut restored_world).unwrap();
    assert_eq!(
        restored
            .persistent_checkpoint(&restored_world)
            .unwrap()
            .bytes()[0],
        7
    );
    let epoch = begin(&mut restored, &mut restored_world);
    assert!(
        restored
            .invoke(&mut [], epoch, Call::RequestRestart)
            .is_err()
    );
    assert!(restored.finish(&mut restored_world).is_err());
}

#[test]
fn pending_restart_capture_refuses_without_consuming_and_reload_is_atomic() {
    let mut world = world();
    let mut source = cap();
    source.enable_restart_requests().unwrap();
    let epoch = begin(&mut source, &mut world);
    source.invoke(&mut [3], epoch, write(0, 0, 1)).unwrap();
    source.invoke(&mut [], epoch, Call::RequestRestart).unwrap();
    source.finish(&mut world).unwrap();
    assert!(source.persistent_checkpoint(&world).is_err());
    assert!(source.prepare_reload(&cap()).is_err());
    assert!(source.take_restart_request().unwrap());
    let before = source.persistent_checkpoint(&world).unwrap();
    assert!(
        source
            .prepare_reload(&Capability::with_persistence(Some(
                Declaration::new(15, [1; 32]).unwrap()
            )))
            .is_err()
    );
    assert_eq!(source.persistent_checkpoint(&world).unwrap(), before);
    source.prepare_reload(&cap()).unwrap();
    let mut next = source.replacement();
    assert_eq!(next.persistent_checkpoint(&world).unwrap(), before);
    let next_epoch = begin(&mut next, &mut world);
    assert!(next_epoch > epoch);
    next.finish(&mut world).unwrap();
}

#[test]
fn late_world_refusal_discards_staged_bytes_and_checkpoint_authority() {
    let mut world = world();
    let mut cap = cap();
    let epoch = begin(&mut cap, &mut world);
    let before = world.save_bytes().unwrap();
    cap.invoke(&mut [9], epoch, write(0, 0, 1)).unwrap();
    cap.invoke(&mut [2; 16], epoch, Call::Despawn(0)).unwrap();
    assert!(cap.finish(&mut world).is_err());
    assert_eq!(world.save_bytes().unwrap(), before);
    assert!(cap.persistent_checkpoint(&world).is_err());
    assert!(cap.replacement().persistent_checkpoint(&world).is_err());
}

#[test]
fn snapshot_corruption_and_extreme_watermark_refuse_without_partial_restore() {
    let world = world();
    let source = cap();
    let snapshot = source.persistent_checkpoint(&world).unwrap();
    let mut candidate = cap();
    let before = candidate.persistent_checkpoint(&world).unwrap();
    for watermark in [-1, i64::MAX] {
        assert!(
            candidate
                .restore_persistent(&world, &snapshot, watermark, false)
                .is_err()
        );
        assert_eq!(candidate.persistent_checkpoint(&world).unwrap(), before);
    }
    let mut max = snapshot.encode();
    max[64..72].copy_from_slice(&i64::MAX.to_le_bytes());
    assert!(PersistentSnapshot::decode(&max, snapshot.declaration()).is_err());
    for (offset, replacement) in [(12, 0), (16, 2), (48, 1)] {
        let mut bad = snapshot.encode();
        bad[offset] = replacement;
        assert!(PersistentSnapshot::decode(&bad, snapshot.declaration()).is_err());
    }
    let mut extra = snapshot.encode();
    extra.push(0);
    assert!(PersistentSnapshot::decode(&extra, snapshot.declaration()).is_err());
    candidate
        .restore_persistent(&world, &snapshot, i64::MAX - 1, false)
        .unwrap();
    // Last representable epoch may run once, but cannot yield a restorable
    // snapshot; the next begin refuses rather than wrap/reissue old authority.
    let mut world = world;
    let epoch = begin(&mut candidate, &mut world);
    assert_eq!(epoch, i64::MAX);
    candidate.finish(&mut world).unwrap();
    assert!(candidate.persistent_checkpoint(&world).is_err());
    assert!(candidate.begin(&mut world, 1. / 64.).is_err());
}

#[test]
fn empty_buffers_validate_both_guest_and_persistent_endpoints() {
    for (call, valid) in [
        (write(0, 16, 0), true),
        (read(0, 16, 0), true),
        (write(1, 16, 0), false),
        (read(-1, 0, 0), false),
        (read(0, 17, 0), false),
    ] {
        let mut world = world();
        let mut cap = cap();
        let epoch = begin(&mut cap, &mut world);
        assert_eq!(cap.invoke(&mut [], epoch, call).is_ok(), valid);
        assert_eq!(cap.finish(&mut world).is_ok(), valid);
    }
}

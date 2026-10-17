use incant_input::haptics::{HapticCapabilities, HapticEffect, HapticError, HapticTarget};
use incant_runtime::{CookedEntity, CookedScene, NativeWorld, StableId};
use incant_types::{Transform, Velocity};
use incant_wasm_abi as abi;
use incant_wasm_runtime::{BoundaryError, Call, Capability, persistent::Declaration};
fn world() -> NativeWorld {
    NativeWorld::new(
        CookedScene::new(
            StableId([9; 16]),
            64,
            vec![CookedEntity {
                id: StableId([1; 16]),
                parent: None,
                transform: Transform::default(),
                velocity: Some(Velocity { linear: [0.; 3] }),
            }],
        )
        .unwrap(),
    )
}
fn caps() -> HapticCapabilities {
    HapticCapabilities::default()
        .with_gamepad(0, 7)
        .unwrap()
        .with_device(9)
        .unwrap()
}
fn row() -> abi::HapticCommand {
    abi::HapticCommand {
        target_kind: 0,
        id: 0,
        generation: 7,
        effect_kind: 0,
        strong: 0.5,
        weak: 1.,
        duration_ms: 125,
        reserved: 0,
    }
}
fn batch(rows: &[abi::HapticCommand]) -> Vec<u8> {
    let mut bytes = abi::HapticCommandHeader {
        magic: *abi::HAPTIC_COMMAND_MAGIC,
        count: rows.len() as u32,
        stride: abi::HapticCommand::SIZE as u32,
    }
    .encode()
    .to_vec();
    for row in rows {
        bytes.extend(row.encode());
    }
    bytes
}
fn submit(
    cap: &mut Capability,
    epoch: i64,
    rows: &[abi::HapticCommand],
) -> Result<i32, BoundaryError> {
    let mut bytes = batch(rows);
    let len = bytes.len() as i32;
    cap.invoke(&mut bytes, epoch, Call::CommandHaptics { ptr: 0, len })
}
fn targets(cap: &mut Capability, epoch: i64) -> Vec<incant_module::HapticTarget> {
    let mut bytes = [0xa5; abi::HAPTIC_MAX_BYTES + 2];
    let len = cap.invoke(&mut bytes, epoch, Call::HapticLen).unwrap();
    assert_eq!(
        cap.invoke(
            &mut bytes,
            epoch,
            Call::ReadHaptics {
                ptr: 1,
                capacity: len
            }
        )
        .unwrap(),
        len
    );
    assert_eq!(bytes[0], 0xa5);
    assert_eq!(bytes[len as usize + 1], 0xa5);
    incant_module::HapticTargets::parse(&bytes[1..len as usize + 1])
        .unwrap()
        .iter()
        .collect()
}
fn prefix(cap: &mut Capability, epoch: i64) {
    let mut bytes = [0; 80];
    cap.invoke(
        &mut bytes,
        epoch,
        Call::Column {
            chunk: 0,
            ptr: 0,
            velocity: false,
            write: false,
        },
    )
    .unwrap();
    bytes[..8].copy_from_slice(&7f64.to_le_bytes());
    cap.invoke(
        &mut bytes,
        epoch,
        Call::Column {
            chunk: 0,
            ptr: 0,
            velocity: false,
            write: true,
        },
    )
    .unwrap();
}
#[test]
fn copied_targets_do_not_derive_from_input_and_all_supported_effects_commit_in_order() {
    let mut w = world();
    let mut cap = Capability::default();
    let e = cap.begin(&mut w, 0.25).unwrap();
    assert!(targets(&mut cap, e).is_empty());
    cap.finish(&mut w).unwrap();
    cap.set_haptic_capabilities(caps()).unwrap();
    let e = cap.begin(&mut w, 0.25).unwrap();
    assert_eq!(
        targets(&mut cap, e),
        [
            incant_module::HapticTarget::Gamepad {
                id: 0,
                generation: 7
            },
            incant_module::HapticTarget::Device { generation: 9 }
        ]
    );
    let stop = abi::HapticCommand {
        effect_kind: 2,
        strong: 0.,
        weak: 0.,
        duration_ms: 0,
        ..row()
    };
    let impact = abi::HapticCommand {
        target_kind: 1,
        generation: 9,
        effect_kind: 1,
        strong: 0.25,
        ..stop
    };
    submit(
        &mut cap,
        e,
        &[
            row(),
            impact,
            stop,
            abi::HapticCommand {
                effect_kind: 2,
                strong: 0.,
                ..impact
            },
        ],
    )
    .unwrap();
    assert!(cap.take_haptics().is_err());
    assert!(cap.set_haptic_capabilities(Default::default()).is_err());
    cap.finish(&mut w).unwrap();
    let output = cap.take_haptics().unwrap();
    assert_eq!(
        output.iter().map(|r| r.effect).collect::<Vec<_>>(),
        [
            HapticEffect::Rumble {
                strong: 0.5,
                weak: 1.,
                duration_ms: 125
            },
            HapticEffect::Impact { intensity: 0.25 },
            HapticEffect::Stop,
            HapticEffect::Stop
        ]
    );
    assert!(matches!(
        output.iter().nth(1).unwrap().target,
        HapticTarget::Device { generation: 9 }
    ));
    assert!(cap.take_haptics().unwrap().is_empty());
}
#[test]
fn stale_epochs_do_not_poison_an_active_tick_but_invalid_current_calls_discard_all_prefixes() {
    for invalid in [
        abi::HapticCommand {
            generation: 6,
            ..row()
        },
        abi::HapticCommand { id: 1, ..row() },
        abi::HapticCommand {
            strong: 1. + f64::EPSILON,
            ..row()
        },
        abi::HapticCommand {
            weak: f64::NAN,
            ..row()
        },
        abi::HapticCommand {
            duration_ms: 0,
            ..row()
        },
        abi::HapticCommand {
            target_kind: 1,
            ..row()
        },
    ] {
        let mut w = world();
        let mut cap = Capability::with_persistence(Some(Declaration::new(8, [1; 32]).unwrap()));
        cap.set_haptic_capabilities(caps()).unwrap();
        w.step().unwrap();
        let before = w.save_bytes().unwrap();
        let e = cap.begin(&mut w, 0.25).unwrap();
        prefix(&mut cap, e);
        cap.invoke(
            &mut [4; 8],
            e,
            Call::WritePersistent {
                ptr: 0,
                offset: 0,
                len: 8,
            },
        )
        .unwrap();
        submit(&mut cap, e, &[row()]).unwrap();
        assert!(submit(&mut cap, e, &[invalid]).is_err());
        assert!(cap.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
        assert!(cap.take_haptics().is_err());
        assert!(cap.persistent_checkpoint(&w).is_err());
    }
    let mut w = world();
    let mut cap = Capability::default();
    cap.set_haptic_capabilities(caps()).unwrap();
    let e = cap.begin(&mut w, 0.25).unwrap();
    assert!(submit(&mut cap, e - 1, &[row()]).is_err());
    prefix(&mut cap, e);
    submit(&mut cap, e, &[row()]).unwrap();
    cap.finish(&mut w).unwrap();
    assert_eq!(cap.take_haptics().unwrap().len(), 1);
    assert_eq!(w.snapshot()[0].transform.translation[0], 7.);
    assert!(submit(&mut cap, e, &[row()]).is_err());
}
#[test]
fn malformed_memory_reads_and_batches_poison_without_partial_guest_output() {
    for call in [
        Call::ReadHaptics {
            ptr: 0,
            capacity: 0,
        },
        Call::ReadHaptics {
            ptr: -1,
            capacity: 220,
        },
        Call::ReadHaptics {
            ptr: 1,
            capacity: 256,
        },
        Call::CommandHaptics { ptr: 0, len: -1 },
        Call::CommandHaptics { ptr: 0, len: 0 },
        Call::CommandHaptics { ptr: 0, len: 1297 },
        Call::CommandHaptics { ptr: 240, len: 56 },
    ] {
        let mut w = world();
        let mut cap = Capability::default();
        cap.set_haptic_capabilities(caps()).unwrap();
        let e = cap.begin(&mut w, 0.25).unwrap();
        prefix(&mut cap, e);
        let before = w.save_bytes().unwrap();
        let mut bytes = [0xa5; 256];
        assert!(cap.invoke(&mut bytes, e, call).is_err());
        assert_eq!(bytes, [0xa5; 256]);
        assert!(cap.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
    }
}
#[test]
fn pending_output_blocks_begin_capability_changes_capture_and_reload_without_consumption() {
    let mut w = world();
    let mut cap = Capability::with_persistence(Some(Declaration::new(8, [1; 32]).unwrap()));
    cap.set_haptic_capabilities(caps()).unwrap();
    w.step().unwrap();
    let e = cap.begin(&mut w, 0.25).unwrap();
    submit(&mut cap, e, &[row()]).unwrap();
    cap.finish(&mut w).unwrap();
    let before = w.save_bytes().unwrap();
    assert!(matches!(
        cap.begin(&mut w, 0.25),
        Err(BoundaryError::Haptics(HapticError::Pending))
    ));
    assert!(cap.set_haptic_capabilities(Default::default()).is_err());
    assert!(cap.checkpoint_authority(&w).is_err());
    assert!(cap.persistent_checkpoint(&w).is_err());
    assert!(
        cap.prepare_reload(&Capability::with_persistence(Some(
            Declaration::new(8, [1; 32]).unwrap()
        )))
        .is_err()
    );
    assert_eq!(w.save_bytes().unwrap(), before);
    assert_eq!(cap.take_haptics().unwrap().len(), 1);
    assert!(cap.persistent_checkpoint(&w).is_ok());
    let next = cap.begin(&mut w, 0.25).unwrap();
    assert_eq!(next, e + 1);
    cap.finish(&mut w).unwrap();
    cap.clear_haptics();
    let next = cap.begin(&mut w, 0.25).unwrap();
    assert!(targets(&mut cap, next).is_empty());
    cap.finish(&mut w).unwrap();
}
#[test]
fn haptic_and_restart_structural_requests_share_the_total_quota_in_both_orders() {
    for haptics_first in [false, true] {
        for overflow in [false, true] {
            let mut w = world();
            let before = w.save_bytes().unwrap();
            let mut cap = Capability::default();
            cap.set_haptic_capabilities(caps()).unwrap();
            cap.enable_restart_requests().unwrap();
            let e = cap.begin(&mut w, 0.25).unwrap();
            prefix(&mut cap, e);
            if haptics_first {
                submit(&mut cap, e, &[row(); 32]).unwrap();
            }
            let mut bytes = [1; 40];
            bytes[16..].fill(0);
            cap.invoke(&mut bytes, e, Call::SetVelocity(0, 16)).unwrap();
            for _ in 0..991 {
                cap.invoke(&mut [], e, Call::RequestRestart).unwrap();
            }
            if !haptics_first {
                submit(&mut cap, e, &[row(); 32]).unwrap();
            }
            if overflow {
                let result = if haptics_first {
                    cap.invoke(&mut [], e, Call::RequestRestart)
                } else {
                    submit(&mut cap, e, &[row()])
                };
                assert!(result.is_err());
                assert!(cap.finish(&mut w).is_err());
                assert_eq!(w.save_bytes().unwrap(), before);
            } else {
                cap.finish(&mut w).unwrap();
                assert_eq!(cap.take_haptics().unwrap().len(), 32);
                assert!(cap.take_restart_request().unwrap());
            }
        }
    }
}
#[test]
fn request_limit_and_failed_late_world_commit_expose_no_output() {
    for late in [false, true] {
        let mut w = world();
        let before = w.save_bytes().unwrap();
        let mut cap = Capability::default();
        cap.set_haptic_capabilities(caps()).unwrap();
        let e = cap.begin(&mut w, 0.25).unwrap();
        prefix(&mut cap, e);
        submit(&mut cap, e, &[row(); 32]).unwrap();
        if late {
            cap.invoke(&mut [9; 16], e, Call::Despawn(0)).unwrap();
        } else {
            assert!(submit(&mut cap, e, &[row()]).is_err());
        }
        assert!(cap.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
        assert!(cap.take_haptics().is_err());
    }
}
#[test]
fn restored_and_reloaded_hosts_keep_epoch_revocation_but_no_device_authority() {
    let declaration = Some(Declaration::new(8, [1; 32]).unwrap());
    let mut w = world();
    let mut old = Capability::with_persistence(declaration);
    old.set_haptic_capabilities(caps()).unwrap();
    w.step().unwrap();
    let e = old.begin(&mut w, 0.25).unwrap();
    old.finish(&mut w).unwrap();
    let snapshot = old.persistent_checkpoint(&w).unwrap();
    let mut restored = Capability::with_persistence(declaration);
    restored.set_haptic_capabilities(caps()).unwrap();
    restored
        .restore_persistent(&w, &snapshot, 40, false)
        .unwrap();
    old.prepare_reload(&Capability::with_persistence(declaration))
        .unwrap();
    let mut reloaded = old.replacement();
    for (cap, expected) in [(&mut restored, 41), (&mut reloaded, e + 1)] {
        let current = cap.begin(&mut w, 0.25).unwrap();
        assert_eq!(current, expected);
        assert!(cap.invoke(&mut [], e, Call::HapticLen).is_err());
        assert!(targets(cap, current).is_empty());
        cap.finish(&mut w).unwrap();
        assert!(cap.take_haptics().unwrap().is_empty());
    }
}

#[test]
fn ignored_current_epoch_host_call_limit_refuses_the_whole_haptic_transaction() {
    let mut w = world();
    let before = w.save_bytes().unwrap();
    let mut cap = Capability::default();
    cap.set_haptic_capabilities(caps()).unwrap();
    let e = cap.begin(&mut w, 0.25).unwrap();
    prefix(&mut cap, e); // two host calls
    submit(&mut cap, e, &[row()]).unwrap(); // one more
    for _ in 3..abi::CALLS {
        assert!(cap.invoke(&mut [], e, Call::HapticLen).is_ok());
    }
    assert!(matches!(
        cap.invoke(&mut [], e, Call::HapticLen),
        Err(BoundaryError::Limit("host call quota"))
    ));
    assert!(matches!(
        cap.finish(&mut w),
        Err(BoundaryError::Abi("rejected haptic transaction"))
    ));
    assert_eq!(w.save_bytes().unwrap(), before);
    assert!(cap.take_haptics().is_err());
}

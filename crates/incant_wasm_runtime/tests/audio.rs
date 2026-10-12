#[allow(dead_code)]
#[path = "support/audio_transport.rs"]
mod fixture;
use fixture::{id, scene};
use incant_module::AudioDeclarations;
use incant_runtime::*;
use incant_wasm_abi as abi;
use incant_wasm_runtime::{Call, Capability};
const DT: f64 = 1. / 60.;
fn read(c: &mut Capability, memory: &mut [u8], epoch: i64) -> Vec<u8> {
    let len = c.invoke(memory, epoch, Call::AudioLen).unwrap();
    let before = memory.to_vec();
    assert_eq!(
        c.invoke(
            memory,
            epoch,
            Call::ReadAudio {
                ptr: 3,
                capacity: len
            }
        )
        .unwrap(),
        len
    );
    assert_eq!(&before[..3], &memory[..3]);
    assert_eq!(&before[3 + len as usize..], &memory[3 + len as usize..]);
    memory[3..3 + len as usize].to_vec()
}
fn batch(rows: &[abi::AudioCommand]) -> Vec<u8> {
    let mut bytes = abi::AudioCommandHeader {
        magic: *abi::AUDIO_COMMAND_MAGIC,
        count: rows.len() as u32,
        stride: abi::AudioCommand::SIZE as u32,
    }
    .encode()
    .to_vec();
    for row in rows {
        bytes.extend(row.encode());
    }
    bytes
}
fn row(entity: u128, kind: u32, value: f64) -> abi::AudioCommand {
    abi::AudioCommand {
        entity: id(entity).0,
        kind,
        reserved: 0,
        value,
    }
}
fn submit(
    c: &mut Capability,
    m: &mut [u8],
    e: i64,
    bytes: &[u8],
) -> Result<i32, incant_wasm_runtime::BoundaryError> {
    m[32_768..32_768 + bytes.len()].copy_from_slice(bytes);
    c.invoke(
        m,
        e,
        Call::CommandAudio {
            ptr: 32_768,
            len: bytes.len() as i32,
        },
    )
}
fn native_rows(world: &NativeWorld) -> Vec<CookedAudio> {
    world.audio_nodes().map(|(r, _)| r.clone()).collect()
}
#[test]
fn copied_declarations_preserve_all_native_fields_and_queued_transport_is_not_visible() {
    let mut world = NativeWorld::new(scene());
    let mut cap = Capability::default();
    let mut memory = vec![0xa5; 131_072];
    for turn in 0..48 {
        world.step().unwrap();
        let epoch = cap.begin(&mut world, DT).unwrap();
        let bytes = read(&mut cap, &mut memory, epoch);
        let frame = AudioDeclarations::parse(&bytes).unwrap();
        assert_eq!(frame.scene().0, world.scene_id().0);
        assert_eq!(frame.tick(), world.tick());
        assert_eq!(frame.len(), world.audio_nodes().len());
        for ((native, matrix), copy) in world.audio_nodes().zip(frame.iter()) {
            assert_eq!(native.entity.0, copy.entity.0);
            assert_eq!(native.listener, copy.listener.is_some());
            assert_eq!(native.bus.is_some(), copy.bus.is_some());
            assert_eq!(native.source.is_some(), copy.source.is_some());
            if let (Some(a), Some(b)) = (&native.bus, copy.bus) {
                assert_eq!(a.parent.map(|p| p.0), b.parent.map(|p| p.0));
                assert_eq!(f64::from(a.gain_db), b.gain_db);
            }
            if let (Some(a), Some(b)) = (&native.source, copy.source) {
                assert_eq!(a.clip.0, b.clip.0);
                assert_eq!(a.bus.map(|p| p.0), b.bus.map(|p| p.0));
                assert_eq!(a.transport_epoch, b.transport_epoch);
                assert_eq!(
                    (
                        f64::from(a.gain_db),
                        f64::from(a.pan),
                        a.rate,
                        a.start_seconds
                    ),
                    (b.gain_db, b.pan, b.rate, b.start_seconds)
                );
                assert_eq!(
                    (a.playing, a.looping, a.streaming),
                    (b.playing, b.looping, b.streaming)
                );
                assert_eq!(a.spatial.is_some(), b.spatial.is_some());
                if let (Some(a), Some(b)) = (&a.spatial, b.spatial) {
                    assert_eq!(
                        [f64::from(a.min_distance), f64::from(a.max_distance)],
                        [b.min_distance, b.max_distance]
                    );
                    assert_eq!(b.world_position, matrix[3][..3]);
                }
            }
            if let Some(listener) = copy.listener {
                assert_eq!(listener.world_position, matrix[3][..3]);
                assert_eq!(
                    Some(listener.world_rotation),
                    world.world_rotation(native.entity)
                );
            }
        }
        let before = native_rows(&world);
        let epoch0 = before[1].source.as_ref().unwrap().transport_epoch;
        submit(
            &mut cap,
            &mut memory,
            epoch,
            &batch(&[
                row(2, 2, 0.),
                row(2, 3, f64::from(turn) / 1000.),
                row(2, 0, 0.),
            ]),
        )
        .unwrap();
        assert_eq!(read(&mut cap, &mut memory, epoch), bytes);
        cap.finish(&mut world).unwrap();
        let after = native_rows(&world);
        let s = after[1].source.as_ref().unwrap();
        assert_eq!(s.transport_epoch, epoch0 + 2);
        assert_eq!(s.start_seconds, f64::from(turn) / 1000.);
        assert!(!s.playing);
        assert_eq!(&after[..1], &before[..1]);
        assert_eq!(&after[2..], &before[2..]);
    }
}
#[test]
fn caught_authorized_buffer_and_batch_failures_poison_prefix_without_partial_copy() {
    for bad in [
        Call::ReadAudio {
            ptr: -1,
            capacity: 4096,
        },
        Call::ReadAudio {
            ptr: 0,
            capacity: -1,
        },
        Call::ReadAudio {
            ptr: 0,
            capacity: 1,
        },
        Call::ReadAudio {
            ptr: 131_070,
            capacity: 4096,
        },
        Call::CommandAudio { ptr: -1, len: 16 },
        Call::CommandAudio { ptr: 0, len: -1 },
        Call::CommandAudio { ptr: 0, len: 0 },
        Call::CommandAudio {
            ptr: 131_070,
            len: 48,
        },
    ] {
        let mut w = NativeWorld::new(scene());
        let saved = w.save_bytes().unwrap();
        let mut c = Capability::default();
        let mut m = vec![0xa5; 131_072];
        let e = c.begin(&mut w, DT).unwrap();
        submit(&mut c, &mut m, e, &batch(&[row(2, 2, 0.), row(1, 5, -20.)])).unwrap();
        let before = m.clone();
        assert!(c.invoke(&mut m, e, bad).is_err());
        assert_eq!(m, before);
        assert!(c.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), saved);
    }
    for mut invalid in [
        row(2, u32::MAX, 0.),
        row(2, 0, 2.),
        row(2, 1, -0.),
        row(2, 3, f64::NAN),
        row(2, 3, 3_600.000_000_001),
        row(2, 4, 24.000_000_001),
        row(1, 5, -80.000_000_001),
        row(2, 2, 0.),
    ] {
        if invalid.kind == 2 {
            invalid.reserved = 1;
        }
        let mut w = NativeWorld::new(scene());
        let saved = w.save_bytes().unwrap();
        let mut c = Capability::default();
        let mut m = vec![0; 131_072];
        let e = c.begin(&mut w, DT).unwrap();
        assert!(submit(&mut c, &mut m, e, &batch(&[row(2, 2, 0.), invalid])).is_err());
        assert!(c.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), saved);
    }
}
#[test]
fn final_owner_role_and_epoch_overflow_reject_all_prior_controls() {
    for (bad, overflow) in [
        (row(88, 2, 0.), false),
        (row(1, 2, 0.), false),
        (row(6, 2, 0.), false),
        (row(2, 5, -3.), false),
        (row(2, 2, 0.), true),
    ] {
        let mut w = NativeWorld::new(scene());
        if overflow {
            let mut source = native_rows(&w)[1].clone();
            source.source.as_mut().unwrap().transport_epoch = u64::MAX;
            let mut commands = FrameCommands::default();
            commands
                .push(StructuralCommand::SetAudio {
                    entity: id(2),
                    audio: Some(source),
                })
                .unwrap();
            w.apply(&mut commands).unwrap();
        }
        let before = w.save_bytes().unwrap();
        let mut c = Capability::default();
        let mut m = vec![0; 131_072];
        let e = c.begin(&mut w, DT).unwrap();
        let bytes = read(&mut c, &mut m, e);
        if overflow {
            assert_eq!(
                AudioDeclarations::parse(&bytes)
                    .unwrap()
                    .find(incant_module::EntityId(id(2).0))
                    .unwrap()
                    .source
                    .unwrap()
                    .transport_epoch,
                u64::MAX
            );
        }
        submit(&mut c, &mut m, e, &batch(&[row(1, 5, -12.), bad])).unwrap();
        assert!(c.finish(&mut w).is_err());
        assert_eq!(w.save_bytes().unwrap(), before);
    }
}
#[test]
fn shared_command_limit_is_exact_and_stale_epochs_do_not_poison_new_callback() {
    let mut w = NativeWorld::new(scene());
    let mut c = Capability::default();
    let mut m = vec![0; 131_072];
    let old = c.begin(&mut w, DT).unwrap();
    c.finish(&mut w).unwrap();
    let e = c.begin(&mut w, DT).unwrap();
    let before = m.clone();
    assert!(
        c.invoke(
            &mut m,
            old,
            Call::ReadAudio {
                ptr: 0,
                capacity: 4096
            }
        )
        .is_err()
    );
    assert_eq!(m, before);
    m[..16].copy_from_slice(&id(2).0);
    c.invoke(&mut m, e, Call::SetParent(0, -1)).unwrap();
    let commands = batch(&vec![row(2, 0, 1.); abi::COMMANDS - 1]);
    submit(&mut c, &mut m, e, &commands).unwrap();
    c.finish(&mut w).unwrap();
    let saved = w.save_bytes().unwrap();
    let e = c.begin(&mut w, DT).unwrap();
    c.invoke(&mut m, e, Call::SetParent(0, -1)).unwrap();
    submit(&mut c, &mut m, e, &commands).unwrap();
    assert!(submit(&mut c, &mut m, e, &batch(&[row(2, 2, 0.)])).is_err());
    assert!(c.finish(&mut w).is_err());
    assert_eq!(w.save_bytes().unwrap(), saved);
    let mut other = NativeWorld::new(scene());
    assert!(c.begin(&mut other, DT).is_err());
}

#[test]
fn authorized_call_and_transfer_quota_failures_poison_without_copy_or_publication() {
    let mut dense = fixture::scene();
    let source = dense.audio()[1].source.clone();
    let entities = (1..=161)
        .map(|n| CookedEntity {
            id: id(n),
            parent: None,
            transform: Default::default(),
            velocity: None,
        })
        .collect();
    dense = CookedScene::new(id(999), 60, entities)
        .unwrap()
        .with_audio(
            (1..=161)
                .map(|n| CookedAudio {
                    entity: id(n),
                    bus: (n <= 32).then_some(NativeAudioBus {
                        parent: None,
                        gain_db: -3.,
                    }),
                    source: (33..=160).contains(&n).then(|| source.clone().unwrap()),
                    listener: n == 161,
                })
                .collect(),
        )
        .unwrap();
    for transfer in [false, true] {
        let mut w = NativeWorld::new(dense.clone());
        let before = w.save_bytes().unwrap();
        let mut c = Capability::default();
        let mut m = vec![0xa5; 131_072];
        let e = c.begin(&mut w, DT).unwrap();
        let prefix = batch(&[row(33, 2, 0.)]);
        submit(&mut c, &mut m, e, &prefix).unwrap();
        let call = if transfer {
            Call::ReadAudio {
                ptr: 0,
                capacity: abi::AUDIO_MAX_BYTES as i32,
            }
        } else {
            Call::AudioLen
        };
        let accepted = if transfer {
            (abi::TRANSFERS - prefix.len()) / abi::AUDIO_MAX_BYTES
        } else {
            abi::CALLS - 1
        };
        for _ in 0..accepted {
            c.invoke(&mut m, e, call).unwrap();
        }
        let untouched = m.clone();
        let failure = c.invoke(&mut m, e, call).unwrap_err().to_string();
        assert!(failure.contains(if transfer {
            "transfer quota"
        } else {
            "host call quota"
        }));
        assert_eq!(m, untouched);
        assert!(
            c.finish(&mut w)
                .unwrap_err()
                .to_string()
                .contains("rejected audio")
        );
        assert_eq!(w.save_bytes().unwrap(), before);
    }
}

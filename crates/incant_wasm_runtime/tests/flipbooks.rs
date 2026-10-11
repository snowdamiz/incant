#[path = "../../incant_runtime/tests/support/flipbooks.rs"]
mod fixture;
use fixture::*;
use incant_runtime::*;
use incant_wasm_runtime::{
    Call, Capability, FLIPBOOK_MAGIC, FLIPBOOK_ROW_BYTES, MAX_FLIPBOOK_BYTES,
};
const DT: f64 = 1. / 64.;
fn read(cap: &mut Capability, epoch: i64, memory: &mut [u8]) -> Vec<u8> {
    let len = cap.invoke(memory, epoch, Call::FlipbookLen).unwrap();
    assert_eq!(
        cap.invoke(
            memory,
            epoch,
            Call::ReadFlipbooks {
                ptr: 8,
                capacity: len
            }
        )
        .unwrap(),
        len
    );
    memory[8..8 + len as usize].to_vec()
}
#[test]
fn snapshot_wire_is_bounded_aligned_owned_and_controls_are_commit_ordered() {
    let mut world = NativeWorld::new(scene(2));
    for _ in 0..12 {
        world.step().unwrap();
    }
    let mut cap = Capability::default();
    let epoch = cap.begin(&mut world, DT).unwrap();
    let mut memory = vec![0xa5; 256];
    let bytes = read(&mut cap, epoch, &mut memory);
    assert_eq!(&bytes[..8], FLIPBOOK_MAGIC);
    assert_eq!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 2);
    assert_eq!(
        u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize,
        FLIPBOOK_ROW_BYTES
    );
    for row in bytes[16..].as_chunks::<FLIPBOOK_ROW_BYTES>().0 {
        let entity = StableId(row[..16].try_into().unwrap());
        let state = world.flipbook_state(entity).unwrap();
        assert_eq!(
            u32::from_le_bytes(row[16..20].try_into().unwrap()),
            u32::from(state.frame())
        );
        assert_eq!(
            u32::from_le_bytes(row[20..24].try_into().unwrap()),
            u32::from(state.playing()) | (u32::from(state.completed()) << 1)
        );
    }
    assert!(
        memory[..8]
            .iter()
            .chain(memory[8 + bytes.len()..].iter())
            .all(|&b| b == 0xa5)
    );
    memory[128..144].copy_from_slice(&id(1).0);
    for (kind, frame) in [(1, 0), (3, 2)] {
        cap.invoke(
            &mut memory,
            epoch,
            Call::ControlFlipbook {
                id: 128,
                kind,
                frame,
            },
        )
        .unwrap();
    }
    assert_eq!(read(&mut cap, epoch, &mut memory), bytes);
    cap.finish(&mut world).unwrap();
    let state = world.flipbook_state(id(1)).unwrap();
    assert_eq!(state.frame(), 2);
    assert!(!state.playing());
    assert!(cap.invoke(&mut memory, epoch, Call::FlipbookLen).is_err());
    let next = cap.begin(&mut world, DT).unwrap();
    assert_ne!(read(&mut cap, next, &mut memory), bytes);
}
#[test]
fn short_oob_bad_epoch_and_control_tags_reject_without_partial_copy() {
    let mut world = NativeWorld::new(scene(1));
    let mut cap = Capability::default();
    let epoch = cap.begin(&mut world, DT).unwrap();
    let mut memory = vec![0xa5; 256];
    let len = cap.invoke(&mut memory, epoch, Call::FlipbookLen).unwrap();
    for (ptr, capacity) in [(-1, len), (0, len - 1), (240, len), (0, -1), (0, i32::MAX)] {
        assert!(
            cap.invoke(&mut memory, epoch, Call::ReadFlipbooks { ptr, capacity })
                .is_err()
        );
        assert!(memory.iter().all(|&b| b == 0xa5));
    }
    assert!(
        cap.invoke(&mut memory, epoch - 1, Call::FlipbookLen)
            .is_err()
    );
    memory[..16].copy_from_slice(&id(1).0);
    for (kind, frame) in [(4, 0), (-1, 0), (0, 1), (1, 1), (2, 1), (3, -1), (3, 65536)] {
        assert!(
            cap.invoke(
                &mut memory,
                epoch,
                Call::ControlFlipbook { id: 0, kind, frame }
            )
            .is_err()
        );
    }
    cap.abort();
    assert!(cap.finish(&mut world).is_err());
}
#[test]
fn native_invalid_control_restores_numeric_prefix_and_all_queued_controls() {
    let mut world = NativeWorld::new(scene(1));
    let before = world.snapshot();
    let state = world.flipbook_state(id(1));
    let mut cap = Capability::default();
    let epoch = cap.begin(&mut world, DT).unwrap();
    let mut memory = vec![0; 256];
    memory[128..144].copy_from_slice(&id(1).0);
    cap.invoke(
        &mut memory,
        epoch,
        Call::Column {
            chunk: 0,
            ptr: 0,
            velocity: false,
            write: false,
        },
    )
    .unwrap();
    memory[..8].copy_from_slice(&77f64.to_le_bytes());
    cap.invoke(
        &mut memory,
        epoch,
        Call::Column {
            chunk: 0,
            ptr: 0,
            velocity: false,
            write: true,
        },
    )
    .unwrap();
    for (kind, frame) in [(1, 0), (3, 3)] {
        cap.invoke(
            &mut memory,
            epoch,
            Call::ControlFlipbook {
                id: 128,
                kind,
                frame,
            },
        )
        .unwrap();
    }
    assert!(cap.finish(&mut world).is_err());
    assert!(cap.is_failed());
    assert_eq!(world.snapshot(), before);
    assert_eq!(world.flipbook_state(id(1)), state);
}
#[test]
fn full_supported_snapshot_fits_cap_and_shares_transfer_quota() {
    let mut world = NativeWorld::new(scene(MAX_SPRITE_ENTITIES));
    let mut cap = Capability::default();
    let epoch = cap.begin(&mut world, DT).unwrap();
    let mut memory = vec![0xa5; MAX_FLIPBOOK_BYTES];
    let len = cap.invoke(&mut memory, epoch, Call::FlipbookLen).unwrap();
    assert_eq!(len as usize, MAX_FLIPBOOK_BYTES);
    let mut copied = 0;
    loop {
        memory.fill(0xa5);
        match cap.invoke(
            &mut memory,
            epoch,
            Call::ReadFlipbooks {
                ptr: 0,
                capacity: len,
            },
        ) {
            Ok(_) => copied += len as usize,
            Err(e) => {
                assert!(e.to_string().contains("transfer quota"));
                assert!(memory.iter().all(|&b| b == 0xa5));
                break;
            }
        }
    }
    assert!(copied <= 128 * 1024 * 1024 && copied + len as usize > 128 * 1024 * 1024);
}
#[test]
fn controls_share_the_existing_command_quota_without_committing_a_prefix() {
    let mut world = NativeWorld::new(scene(1));
    let state = world.flipbook_state(id(1));
    let mut cap = Capability::default();
    let epoch = cap.begin(&mut world, DT).unwrap();
    let mut memory = id(1).0.to_vec();
    for _ in 0..1024 {
        cap.invoke(
            &mut memory,
            epoch,
            Call::ControlFlipbook {
                id: 0,
                kind: 1,
                frame: 0,
            },
        )
        .unwrap();
    }
    assert!(
        cap.invoke(
            &mut memory,
            epoch,
            Call::ControlFlipbook {
                id: 0,
                kind: 1,
                frame: 0
            }
        )
        .unwrap_err()
        .to_string()
        .contains("structural command quota")
    );
    cap.abort();
    assert_eq!(world.flipbook_state(id(1)), state);
}

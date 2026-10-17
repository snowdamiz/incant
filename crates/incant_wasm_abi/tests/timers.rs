use incant_wasm_abi::{
    self as abi,
    timers::{Snapshot, encode_id, validate_command},
};

fn frame() -> Vec<u8> {
    let mut bytes = abi::TimerHeader {
        magic: *abi::TIMER_MAGIC,
        tick: 2,
        elapsed_seconds: 0.5,
        count: 1,
        stride: abi::TimerEvent::SIZE as u32,
    }
    .encode()
    .to_vec();
    let (id, id_len) = encode_id("cooldown.ready").unwrap();
    bytes.extend(
        abi::TimerEvent {
            id,
            id_len,
            reserved: 0,
            scheduled_tick: 2,
        }
        .encode(),
    );
    bytes
}
#[test]
fn snapshot_is_exact_bounded_sorted_and_has_no_hidden_tail_or_invalid_clock() {
    let bytes = frame();
    let s = Snapshot::parse(&bytes).unwrap();
    assert_eq!(s.events().collect::<Vec<_>>(), vec![("cooldown.ready", 2)]);
    assert_eq!((s.tick(), s.elapsed_seconds(), s.len()), (2, 0.5, 1));
    for n in 0..bytes.len() {
        assert!(Snapshot::parse(&bytes[..n]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(Snapshot::parse(&trailing).is_err());
    for offset in [
        0,
        8,
        24,
        28,
        abi::TimerHeader::SIZE + 64,
        abi::TimerHeader::SIZE + 68,
        abi::TimerHeader::SIZE + 72,
    ] {
        let mut bad = bytes.clone();
        bad[offset] ^= 0xff;
        assert!(Snapshot::parse(&bad).is_err(), "offset{offset}");
    }
    for f in [f64::NAN, f64::INFINITY, -1., 0.] {
        let mut bad = bytes.clone();
        bad[16..24].copy_from_slice(&f.to_le_bytes());
        assert!(Snapshot::parse(&bad).is_err());
    }
    let mut duplicated = bytes.clone();
    duplicated[24..28].copy_from_slice(&2u32.to_le_bytes());
    duplicated.extend_from_slice(&bytes[abi::TimerHeader::SIZE..]);
    assert!(Snapshot::parse(&duplicated).is_err());
}
#[test]
fn ids_and_commands_validate_original_values_without_coercion_or_truncation() {
    for id in ["", "two words", "é", "a\0b", "slash/name"] {
        assert!(encode_id(id).is_err());
    }
    assert!(encode_id(&"a".repeat(65)).is_err());
    assert!(encode_id(&"Z".repeat(64)).is_ok());
    let (id, id_len) = encode_id("A_.:-09").unwrap();
    let mut c = abi::TimerCommand {
        id,
        id_len,
        kind: 0,
        delay: 1,
        interval: u32::MAX,
    };
    assert!(validate_command(&c).is_ok());
    c.delay = 0;
    assert!(validate_command(&c).is_err());
    c.kind = 1;
    assert!(validate_command(&c).is_err());
    c.interval = 0;
    assert!(validate_command(&c).is_ok());
    c.id[63] = 1;
    assert!(validate_command(&c).is_err());
}

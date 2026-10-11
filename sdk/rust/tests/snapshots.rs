use incant_module::{EntityId, FlipbookSnapshot, InputSnapshot, Key};
use incant_wasm_abi as abi;
fn input(name: &[u8]) -> Vec<u8> {
    let mut bytes = abi::InputHeader {
        magic: *abi::INPUT_MAGIC,
        length: 0,
        focused: 1,
        tick: 9,
    }
    .encode()
    .to_vec();
    bytes.extend(1u32.to_le_bytes());
    bytes.extend(
        abi::Action {
            name_length: name.len() as u8,
            kind: 0,
            flags: 3,
            reserved: 0,
            value: [1., 0.],
        }
        .encode(),
    );
    bytes.extend(name);
    bytes.extend(
        abi::Keyboard {
            held: 1 << 40,
            pressed: 1 << 40,
            released: 0,
        }
        .encode(),
    );
    bytes.extend(
        abi::Mouse {
            buttons: [0; 3],
            has_position: 0,
            position: [0.; 2],
            motion: [0.; 2],
            wheel_pixels: [0.; 2],
            wheel_lines: [0.; 2],
        }
        .encode(),
    );
    bytes.extend([0; 24]); // six counted empty collections
    let len = bytes.len() as u32;
    bytes[8..12].copy_from_slice(&len.to_le_bytes());
    bytes
}
#[test]
fn borrowed_input_accepts_unaligned_storage_and_boundary_length_names() {
    let name = "a".repeat(64);
    let bytes = input(name.as_bytes());
    let mut unaligned = vec![0xa5];
    unaligned.extend(&bytes);
    let snapshot = InputSnapshot::parse(&unaligned[1..]).unwrap();
    assert_eq!(snapshot.tick, 9);
    assert!(snapshot.focused);
    assert!(snapshot.keyboard.held(Key::Space as u8));
    let action = snapshot.action(&name).unwrap();
    assert!(action.active && action.pressed && !action.released);
    assert_eq!(action.value, [1., 0.]);
    assert_eq!(snapshot.actions().count(), 1);
    assert_eq!(snapshot.gamepads().count(), 0);
    assert_eq!(snapshot.gestures().count(), 0);
}
#[test]
fn input_refuses_unknown_layouts_utf8_action_limits_and_truncated_prefixes() {
    let bytes = input(b"jump");
    for n in 0..bytes.len() {
        assert!(InputSnapshot::parse(&bytes[..n]).is_err(), "prefix {n}");
        if n >= abi::InputHeader::SIZE {
            let mut truncated = bytes[..n].to_vec();
            truncated[8..12].copy_from_slice(&(n as u32).to_le_bytes());
            assert!(
                InputSnapshot::parse(&truncated).is_err(),
                "declared prefix {n}"
            );
        }
    }
    for name in [
        vec![],
        vec![b'a'; 65],
        vec![0xff],
        "é".as_bytes().to_vec(),
        b"bad name".to_vec(),
    ] {
        assert!(InputSnapshot::parse(&input(&name)).is_err());
    }
    for (at, value) in [(0, 0), (15, 1), (29, 9), (30, 128), (31, 1)] {
        let mut bad = bytes.clone();
        bad[at] = value;
        assert!(InputSnapshot::parse(&bad).is_err(), "offset {at}");
    }
    let mut bad = bytes.clone();
    bad[24..28].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(InputSnapshot::parse(&bad).is_err());
    let mut bad = bytes.clone();
    bad[32..40].copy_from_slice(&f64::NAN.to_le_bytes());
    assert!(InputSnapshot::parse(&bad).is_err());
    let mut bad = bytes.clone();
    bad.push(0);
    let n = bad.len() as u32;
    bad[8..12].copy_from_slice(&n.to_le_bytes());
    assert!(InputSnapshot::parse(&bad).is_err());
}
fn flipbooks() -> Vec<u8> {
    let mut bytes = abi::FlipbookHeader {
        magic: *abi::FLIPBOOK_MAGIC,
        count: 1,
        stride: 24,
    }
    .encode()
    .to_vec();
    bytes.extend(
        abi::Flipbook {
            id: 99u128.to_be_bytes(),
            frame: 2,
            flags: 1,
        }
        .encode(),
    );
    bytes
}
#[test]
fn playback_readback_validates_whole_buffer_before_exposing_any_rows() {
    let bytes = flipbooks();
    let mut unaligned = vec![0xa5];
    unaligned.extend(&bytes);
    let snapshot = FlipbookSnapshot::parse(&unaligned[1..]).unwrap();
    let row = snapshot.iter().next().unwrap();
    assert_eq!(row.entity, EntityId::from_u128(99));
    assert_eq!(row.frame, 2);
    assert!(row.playing && !row.completed);
    for n in 0..bytes.len() {
        assert!(FlipbookSnapshot::parse(&bytes[..n]).is_err());
    }
    for (at, value) in [(8, u32::MAX), (12, 25), (32, 256), (36, 4)] {
        let mut bad = bytes.clone();
        bad[at..at + 4].copy_from_slice(&value.to_le_bytes());
        assert!(FlipbookSnapshot::parse(&bad).is_err());
    }
    let mut bad = bytes.clone();
    bad.push(0);
    assert!(FlipbookSnapshot::parse(&bad).is_err());
}

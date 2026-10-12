use incant_module::{AnimationSnapshot, EntityId};
use incant_wasm_abi as abi;
fn bytes(row: abi::AnimationRow) -> Vec<u8> {
    let mut bytes = abi::AnimationHeader {
        magic: *abi::ANIMATION_MAGIC,
        count: 1,
        stride: abi::AnimationRow::SIZE as u32,
    }
    .encode()
    .to_vec();
    bytes.extend(row.encode());
    bytes
}
fn row() -> abi::AnimationRow {
    abi::AnimationRow {
        id: [7; 16],
        clip: 255,
        flags: 3,
        seconds: 0.75,
        speed: -16.,
    }
}
#[test]
fn copied_playback_is_borrowed_unaligned_and_preserves_exact_native_values() {
    let mut data = vec![0xff];
    data.extend(bytes(row()));
    let snapshot = AnimationSnapshot::parse(&data[1..]).unwrap();
    assert_eq!(snapshot.len(), 1);
    assert!(!snapshot.is_empty());
    let value = snapshot.iter().next().unwrap();
    assert_eq!(value.entity, EntityId([7; 16]));
    assert_eq!(value.clip, Some(255));
    assert_eq!(value.seconds, 0.75);
    assert_eq!(value.speed, -16.);
    assert!(value.playing && value.looping && !value.completed);
    let empty = abi::AnimationHeader {
        magic: *abi::ANIMATION_MAGIC,
        count: 0,
        stride: 40,
    }
    .encode();
    assert!(AnimationSnapshot::parse(&empty).unwrap().is_empty());
    for (clip, flags, seconds) in [(u32::MAX, 3, 0.), (0, 4, 1.), (0, 0, 0.)] {
        let mut value = row();
        value.clip = clip;
        value.flags = flags;
        value.seconds = seconds;
        assert!(AnimationSnapshot::parse(&bytes(value)).is_ok());
    }
}
#[test]
fn every_truncation_and_invalid_late_row_refuses_the_entire_snapshot() {
    let data = bytes(row());
    for n in 0..data.len() {
        assert!(AnimationSnapshot::parse(&data[..n]).is_err());
    }
    for field in 0..11 {
        let mut bad = row();
        match field {
            0 => bad.clip = 256,
            1 => bad.flags = 8,
            2 => bad.flags = 5,
            10 => bad.flags = 6,
            3 => bad.seconds = -1.,
            4 => bad.seconds = f64::NAN,
            5 => bad.speed = f64::INFINITY,
            6 => bad.speed = 16.001,
            7 => bad.speed = -16.001,
            8 => {
                bad.clip = u32::MAX;
                bad.seconds = 1.;
            }
            _ => {
                bad.clip = u32::MAX;
                bad.flags = 4;
                bad.seconds = 0.;
            }
        }
        let mut data = bytes(row());
        data[8..12].copy_from_slice(&2u32.to_le_bytes());
        data.extend(bad.encode());
        assert!(AnimationSnapshot::parse(&data).is_err(), "field {field}");
    }
    for (offset, value) in [(0, 0), (8, 100001), (12, 41)] {
        let mut bad = data.clone();
        bad[offset..offset + 4].copy_from_slice(&u32::to_le_bytes(value));
        assert!(AnimationSnapshot::parse(&bad).is_err());
    }
    let mut trailing = data;
    trailing.push(0);
    assert!(AnimationSnapshot::parse(&trailing).is_err());
}

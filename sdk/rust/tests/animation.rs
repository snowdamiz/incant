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

fn v2row() -> abi::AnimationRowV2 {
    abi::AnimationRowV2 {
        id: [7; 16],
        clip: 0,
        flags: 1,
        seconds: 0.5,
        speed: -2.,
        source_clip: 255,
        transition_flags: 7,
        source_seconds: 0.75,
        source_speed: 16.,
        elapsed: 0.25,
        duration: 0.5,
    }
}
fn v2bytes(row: abi::AnimationRowV2) -> Vec<u8> {
    let mut bytes = abi::AnimationHeader {
        magic: *abi::ANIMATION_V2_MAGIC,
        count: 1,
        stride: abi::AnimationRowV2::SIZE as u32,
    }
    .encode()
    .to_vec();
    bytes.extend(row.encode());
    bytes
}
#[test]
fn v2_owned_unaligned_transition_keeps_every_captured_field_and_legacy_rows_remain_valid() {
    let mut bytes = vec![99];
    bytes.extend(v2bytes(v2row()));
    let row = AnimationSnapshot::parse(&bytes[1..])
        .unwrap()
        .iter()
        .next()
        .unwrap();
    let t = row.transition.unwrap();
    assert_eq!(
        (
            t.clip, t.seconds, t.speed, t.playing, t.looping, t.elapsed, t.duration
        ),
        (Some(255), 0.75, 16., true, true, 0.25, 0.5)
    );
    assert_eq!((row.clip, row.seconds, row.speed), (Some(0), 0.5, -2.));
    let mut inactive = v2row();
    inactive.transition_flags = 0;
    inactive.source_clip = 0;
    inactive.source_seconds = 0.;
    inactive.source_speed = 0.;
    inactive.elapsed = 0.;
    inactive.duration = 0.;
    assert!(
        AnimationSnapshot::parse(&v2bytes(inactive))
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .transition
            .is_none()
    );
    assert!(
        AnimationSnapshot::parse(&super_legacy())
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .transition
            .is_none()
    );
}
fn super_legacy() -> Vec<u8> {
    bytes(row())
}
#[test]
fn every_v2_truncation_reserved_inactive_suffix_and_bad_late_transition_refuses_all_rows() {
    let data = v2bytes(v2row());
    for n in 0..data.len() {
        assert!(AnimationSnapshot::parse(&data[..n]).is_err());
    }
    for field in 0..17 {
        let mut bad = v2row();
        match field {
            0 => bad.transition_flags = 8,
            1 => bad.transition_flags = 2,
            2 => bad.source_clip = 256,
            3 => bad.source_seconds = -1.,
            4 => bad.source_seconds = f64::NAN,
            5 => bad.source_speed = 16.001,
            6 => bad.source_speed = f64::INFINITY,
            7 => bad.elapsed = -1.,
            8 => bad.elapsed = 0.5,
            9 => bad.elapsed = f64::NAN,
            10 => bad.duration = 0.,
            11 => bad.duration = 60.001,
            12 => bad.duration = f64::INFINITY,
            13 => {
                bad.source_clip = u32::MAX;
                bad.source_seconds = 1.;
            }
            14 => bad.flags = 4,
            15 => {
                bad.transition_flags = 0;
                bad.source_clip = 0;
                bad.source_seconds = 0.;
                bad.source_speed = 0.;
                bad.elapsed = 0.;
                bad.duration = -0.;
            }
            _ => {
                bad.transition_flags = 0;
                bad.source_seconds = 0.;
                bad.source_speed = 0.;
                bad.elapsed = 0.;
                bad.duration = 0.;
            }
        }
        let mut data = v2bytes(v2row());
        data[8..12].copy_from_slice(&2u32.to_le_bytes());
        data.extend(bad.encode());
        assert!(AnimationSnapshot::parse(&data).is_err(), "field {field}");
    }
    let mut trailing = data;
    trailing.push(0);
    assert!(AnimationSnapshot::parse(&trailing).is_err());
    let mut mismatched = v2bytes(v2row());
    mismatched[..8].copy_from_slice(abi::ANIMATION_MAGIC);
    assert!(AnimationSnapshot::parse(&mismatched).is_err());
}

use incant_wasm_abi::{self as abi, physics as wire};
fn q() -> abi::PhysicsQuery {
    abi::PhysicsQuery {
        kind: 0,
        flags: 0,
        entity: [0; 16],
        a: [0.; 3],
        b: [1., 0., 0.],
        c: 10.,
        d: 0.,
        e: 0.,
        memberships: u32::MAX,
        filter: u32::MAX,
        reserved: 0,
    }
}
#[test]
fn original_f64_domains_canonical_unused_fields_and_whole_batch_validation() {
    let mut bytes = vec![0; abi::PHYSICS_MAX_REQUEST_BYTES];
    let n = wire::encode_batch(&[q(); 64], &mut bytes).unwrap();
    assert_eq!(n, bytes.len());
    assert_eq!(wire::decode_batch(&bytes).unwrap().len(), 64);
    for field in 0..9 {
        let mut bad = q();
        match field {
            0 => bad.a[0] = 1_000_000.000001,
            1 => bad.c = -f64::MIN_POSITIVE,
            2 => bad.a[1] = f64::NAN,
            3 => bad.flags = 8,
            4 => bad.entity[15] = 1,
            5 => bad.reserved = 1,
            6 => bad.d = 1.,
            7 => {
                bad.kind = 1;
                bad.b[2] = 1.;
            }
            _ => bad.b = [0.; 3],
        };
        bytes[n - abi::PhysicsQuery::SIZE..].copy_from_slice(&bad.encode());
        assert!(wire::decode_batch(&bytes).is_err());
    }
    for length in [0, 7, 15, 16, 17, abi::PHYSICS_MAX_REQUEST_BYTES - 1] {
        assert!(wire::decode_batch(&bytes[..length]).is_err());
    }
}
#[test]
fn result_frames_validate_all_rows_collision_ranges_and_unaligned_storage() {
    let row = abi::PhysicsRow {
        kind: 2,
        flags: 1,
        entity: [0; 16],
        distance: 0.,
        point: [1., 0., 0.],
        normal: [0.; 3],
        first: 0,
        count: 2,
        reserved: 0,
    };
    let mut bytes = vec![0xa5];
    bytes.extend(
        abi::PhysicsHeader {
            tag: *abi::PHYSICS_RESULT_MAGIC,
            count: 1,
            extra: 2,
        }
        .encode(),
    );
    bytes.extend(row.encode());
    bytes.extend(1u128.to_be_bytes());
    bytes.extend(u128::MAX.to_be_bytes());
    let r = wire::decode_results(&bytes[1..]).unwrap();
    assert_eq!(r.iter().next().unwrap().1[1], u128::MAX.to_be_bytes());
    for field in 0..5 {
        let mut bad = row;
        match field {
            0 => bad.first = u32::MAX,
            1 => bad.count = 3,
            2 => bad.flags = 8,
            3 => bad.normal[0] = 1.,
            _ => bad.point[0] = f64::INFINITY,
        };
        bytes[17..17 + abi::PhysicsRow::SIZE].copy_from_slice(&bad.encode());
        assert!(wire::decode_results(&bytes[1..]).is_err());
    }
    bytes[17..17 + abi::PhysicsRow::SIZE].copy_from_slice(&row.encode());
    let n = bytes.len();
    bytes[n - 16..].copy_from_slice(&0u128.to_be_bytes());
    assert!(wire::decode_results(&bytes[1..]).is_err());
}
#[test]
fn event_frames_are_complete_ordered_and_bounded() {
    let event = abi::PhysicsEvent {
        first: 1u128.to_be_bytes(),
        second: 2u128.to_be_bytes(),
        entered: 1,
        reserved: 0,
    };
    for dimension in [2, 3] {
        let bytes = abi::PhysicsEventHeader {
            tag: *abi::PHYSICS_EVENTS_MAGIC,
            scene: u128::MAX.to_be_bytes(),
            count: 0,
            dimension,
        }
        .encode();
        let empty = wire::decode_events(&bytes).unwrap();
        assert!(empty.is_empty());
        assert_eq!(empty.scene, u128::MAX.to_be_bytes());
        assert_eq!(empty.dimension, dimension);
    }
    let mut bytes = abi::PhysicsEventHeader {
        tag: *abi::PHYSICS_EVENTS_MAGIC,
        scene: u128::MAX.to_be_bytes(),
        count: 1,
        dimension: 2,
    }
    .encode()
    .to_vec();
    bytes.extend(event.encode());
    assert_eq!(
        wire::decode_events(&bytes).unwrap().scene,
        u128::MAX.to_be_bytes()
    );
    let mut unaligned = vec![0xa5];
    unaligned.extend_from_slice(&bytes);
    assert_eq!(
        wire::decode_events(&unaligned[1..]).unwrap().scene,
        u128::MAX.to_be_bytes()
    );
    for length in [0, 15, 16, 31, 32, bytes.len() - 1] {
        assert!(wire::decode_events(&bytes[..length]).is_err());
    }
    assert_eq!(
        wire::decode_events(&bytes).unwrap().iter().next().unwrap(),
        event
    );
    bytes.extend(event.encode());
    bytes[24..28].copy_from_slice(&2u32.to_le_bytes());
    assert!(wire::decode_events(&bytes).is_err());
    assert_eq!(
        abi::PHYSICS_MAX_EVENT_BYTES,
        32 + abi::PHYSICS_MAX_EVENTS * abi::PhysicsEvent::SIZE
    );
    assert_eq!(
        abi::PHYSICS_MAX_RESULT_BYTES,
        16 + abi::PHYSICS_MAX_BATCH * abi::PhysicsRow::SIZE + abi::PHYSICS_MAX_COLLISIONS * 16
    );
}

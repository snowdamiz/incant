use incant_module::{AnimationGraphKey, AnimationGraphSnapshot, EntityId};
use incant_wasm_abi as abi;

fn fixture() -> Vec<u8> {
    let mut bytes = abi::AnimationGraphHeader {
        magic: *abi::ANIMATION_GRAPH_MAGIC,
        count: 1,
        stride: abi::AnimationGraphRow::SIZE as u32,
        parameter_count: 2,
        parameter_stride: abi::AnimationGraphParameter::SIZE as u32,
    }
    .encode()
    .to_vec();
    bytes.extend(
        abi::AnimationGraphRow {
            id: 3u128.to_be_bytes(),
            state: 20u128.to_be_bytes(),
            seconds: 5000.0,
            elapsed: 3600.0,
            flags: 1,
            parameter_offset: 0,
            parameter_count: 2,
            reserved: 0,
        }
        .encode(),
    );
    for (id, value) in [(10u128, -0.5), (11, 1_000_000.0)] {
        bytes.extend(
            abi::AnimationGraphParameter {
                id: id.to_be_bytes(),
                value,
            }
            .encode(),
        );
    }
    bytes
}

#[test]
fn borrowed_unaligned_graph_snapshot_preserves_pause_keys_and_parameter_ownership() {
    let mut bytes = vec![0xaa];
    bytes.extend(fixture());
    let snapshot = AnimationGraphSnapshot::parse(&bytes[1..]).unwrap();
    assert_eq!(snapshot.len(), 1);
    let row = snapshot.iter().next().unwrap();
    assert_eq!(row.entity, EntityId::from_u128(3));
    assert_eq!(row.state, AnimationGraphKey::from_u128(20));
    assert!(row.paused);
    // State elapsed is capped, but a long authored clip may exceed that duration.
    assert_eq!((row.seconds, row.elapsed), (5000.0, 3600.0));
    let parameters: Vec<_> = row.parameters().collect();
    assert_eq!(parameters[0].id, AnimationGraphKey::from_u128(10));
    assert_eq!(parameters[0].value, -0.5);
    assert_eq!(parameters[1].id, AnimationGraphKey::from_u128(11));
    assert_eq!(parameters[1].value, 1_000_000.0);
    let empty = abi::AnimationGraphHeader {
        magic: *abi::ANIMATION_GRAPH_MAGIC,
        count: 0,
        stride: 64,
        parameter_count: 0,
        parameter_stride: 24,
    }
    .encode();
    assert!(AnimationGraphSnapshot::parse(&empty).unwrap().is_empty());
}

#[test]
fn malformed_framing_ranges_reserved_fields_and_late_parameters_refuse_atomically() {
    let bytes = fixture();
    for end in 0..bytes.len() {
        assert!(
            AnimationGraphSnapshot::parse(&bytes[..end]).is_err(),
            "prefix {end}"
        );
    }
    let mut extra = bytes.clone();
    extra.push(0);
    assert!(AnimationGraphSnapshot::parse(&extra).is_err());
    let row = 24;
    for (offset, value) in [
        (0, 0u32),
        (8, 1025),
        (12, 63),
        (16, 16385),
        (20, 25),
        (row + 48, 2),
        (row + 52, 1),
        (row + 56, 17),
        (row + 60, 1),
    ] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(
            AnimationGraphSnapshot::parse(&bad).is_err(),
            "offset {offset}"
        );
    }
    for (offset, value) in [
        (row + 32, -1.0f64),
        (row + 32, f64::NAN),
        (row + 40, 3600.0001),
        (row + 40, -1.0),
        (row + 64 + 24 + 16, f64::INFINITY),
        (row + 64 + 24 + 16, 1_000_000.000_1),
    ] {
        let mut bad = bytes.clone();
        bad[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
        assert!(
            AnimationGraphSnapshot::parse(&bad).is_err(),
            "offset {offset} value {value}"
        );
    }
    let mut duplicate = bytes.clone();
    duplicate[row + 64 + 24..row + 64 + 24 + 16].copy_from_slice(&10u128.to_be_bytes());
    assert!(AnimationGraphSnapshot::parse(&duplicate).is_err());
    let mut unreferenced = bytes;
    unreferenced[row + 56..row + 60].copy_from_slice(&1u32.to_le_bytes());
    assert!(AnimationGraphSnapshot::parse(&unreferenced).is_err());
}

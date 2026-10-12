use incant_module::{AnimationIkSnapshot as Snapshot, EntityId};
use incant_wasm_abi as abi;
fn frame(rows: &[abi::AnimationIkRow]) -> Vec<u8> {
    let mut poses = abi::AnimationHeaderV4 {
        magic: *abi::ANIMATION_V4_MAGIC,
        scene: [9; 16],
        count: 3,
        stride: abi::AnimationRowV4::SIZE as u32,
        sample_count: 3,
        sample_stride: 24,
    }
    .encode()
    .to_vec();
    for (offset, id) in [1, 3, 7].into_iter().enumerate() {
        poses.extend(
            abi::AnimationRowV4 {
                id: [id; 16],
                clip: 0,
                flags: 3,
                seconds: 0.25,
                speed: 1.,
                source_clip: 0,
                transition_flags: 0,
                source_seconds: 0.,
                source_speed: 0.,
                elapsed: 0.,
                duration: 0.,
                pose_kind: 0,
                sample_offset: offset as u32,
                sample_count: 1,
                reserved: 0,
            }
            .encode(),
        );
    }
    for _ in 0..3 {
        poses.extend(
            abi::AnimationPoseSample {
                clip: 0,
                reserved: 0,
                seconds: 0.25,
                weight: 1.,
            }
            .encode(),
        );
    }
    let mut bytes = abi::AnimationIkHeader {
        magic: *abi::ANIMATION_IK_MAGIC,
        pose_bytes: poses.len() as u32,
        count: rows.len() as u32,
        stride: 104,
        reserved: 0,
    }
    .encode()
    .to_vec();
    bytes.extend(poses);
    for row in rows {
        bytes.extend(row.encode());
    }
    bytes
}
fn row(entity: u8, constraint: u8) -> abi::AnimationIkRow {
    abi::AnimationIkRow {
        entity: [entity; 16],
        constraint: [constraint; 16],
        chain: [0, 1, 2],
        reserved: 0,
        target: [1., 2., 3.],
        pole: [0., 1., 0.],
        weight: 0.125,
    }
}
#[test]
fn unaligned_complete_frame_retains_pose_membership_and_native_constraint_order() {
    let rows = [row(1, 9), row(1, 2), row(7, 9)];
    let mut bytes = vec![0xa5];
    bytes.extend(frame(&rows));
    let s = Snapshot::parse(&bytes[1..]).unwrap();
    assert_eq!(s.poses().scene(), EntityId([9; 16]));
    assert_eq!(s.poses().len(), 3);
    for (a, b) in s.constraints().zip(rows) {
        assert_eq!(a.entity.0, b.entity);
        assert_eq!(a.constraint.0, b.constraint);
        assert_eq!(a.target, b.target);
        assert_eq!(a.chain, [0, 1, 2]);
        assert_eq!(a.weight.to_bits(), b.weight.to_bits());
    }
    assert_eq!(Snapshot::parse(&frame(&[])).unwrap().constraints().len(), 0);
}
#[test]
fn invalid_sections_membership_duplicate_ids_chains_and_numeric_data_refuse_before_views() {
    let bytes = frame(&[row(1, 9), row(1, 2), row(7, 9)]);
    for n in 0..bytes.len() {
        assert!(Snapshot::parse(&bytes[..n]).is_err());
    }
    let mut tail = bytes.clone();
    tail.push(0);
    assert!(Snapshot::parse(&tail).is_err());
    for n in 0..17 {
        let mut rows = [row(1, 9), row(1, 2), row(7, 9)];
        match n {
            0 => rows[1].constraint = rows[0].constraint,
            1 => rows[2].entity = [1; 16],
            2 => rows[1].entity = [2; 16],
            3 => rows.swap(0, 2),
            4 => rows[0].reserved = 1,
            5 => rows[0].chain = [0, 0, 2],
            6 => rows[0].chain = [0, 1, 4096],
            7 => rows[0].chain = [0, 1, u32::MAX],
            8 => rows[0].target[0] = f64::NAN,
            9 => rows[0].pole[2] = f64::INFINITY,
            10 => rows[0].weight = f64::NAN,
            11 => rows[0].weight = 1.1,
            12 => rows[0].weight = -0.1,
            13 => rows[0].target[1] = 1e9 + 1.,
            14 => rows[0].pole[1] = -1e9 - 1.,
            15 => rows[0].entity = [0; 16],
            _ => rows[2].entity = [8; 16],
        }
        assert!(Snapshot::parse(&frame(&rows)).is_err(), "mutation {n}");
    }
    for (offset, value) in [(8, u32::MAX), (12, 4097), (16, 103), (20, 1)] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
        assert!(Snapshot::parse(&bad).is_err());
    }
    let mut bad = bytes;
    bad[24] = 0;
    assert!(
        Snapshot::parse(&bad).is_err(),
        "inner frame is also validated"
    );
}
#[test]
fn thirty_two_constraints_are_valid_but_thirty_three_refuse_without_hidden_truncation() {
    let rows: Vec<_> = (0..33).map(|n| row(3, n)).collect();
    assert_eq!(
        Snapshot::parse(&frame(&rows[..32]))
            .unwrap()
            .constraints()
            .len(),
        32
    );
    assert!(Snapshot::parse(&frame(&rows)).is_err());
}

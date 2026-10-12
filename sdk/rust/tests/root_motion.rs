use incant_module::{EntityId, RootMotionClockOwner, RootMotionSnapshot as Snapshot};
use incant_wasm_abi as abi;
fn frame(rows: &[abi::RootMotionRow]) -> Vec<u8> {
    let mut pose = abi::AnimationHeaderV5 {
        magic: *abi::ANIMATION_V5_MAGIC,
        scene: [9; 16],
        count: 1,
        stride: abi::AnimationRowV4::SIZE as u32,
        sample_count: 1,
        sample_stride: 24,
        compound_count: 0,
        compound_stride: 40,
    }
    .encode()
    .to_vec();
    pose.extend(
        abi::AnimationRowV4 {
            id: [1; 16],
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
            sample_offset: 0,
            sample_count: 1,
            reserved: 0,
        }
        .encode(),
    );
    pose.extend(
        abi::AnimationPoseSample {
            clip: 0,
            reserved: 0,
            seconds: 0.25,
            weight: 1.,
        }
        .encode(),
    );
    let mut ik = abi::AnimationIkHeader {
        magic: *abi::ANIMATION_IK_V2_MAGIC,
        pose_bytes: pose.len() as u32,
        count: 1,
        stride: 104,
        reserved: 0,
    }
    .encode()
    .to_vec();
    ik.extend(pose);
    ik.extend(
        abi::AnimationIkRow {
            entity: [1; 16],
            constraint: [8; 16],
            chain: [0, 1, 2],
            reserved: 0,
            target: [1., 2., 3.],
            pole: [0., 1., 0.],
            weight: 0.5,
        }
        .encode(),
    );
    let mut bytes = abi::RootMotionHeader {
        magic: *abi::ROOT_MOTION_MAGIC,
        scene: [9; 16],
        pose_bytes: ik.len() as u32,
        count: rows.len() as u32,
        stride: abi::RootMotionRow::SIZE as u32,
        reserved: 0,
    }
    .encode()
    .to_vec();
    bytes.extend(ik);
    for row in rows {
        bytes.extend(row.encode());
    }
    bytes
}
fn row() -> abi::RootMotionRow {
    abi::RootMotionRow {
        entity: [1; 16],
        fingerprint: [4; 32],
        root: 3,
        policy: 0,
        flags: 3,
        reserved: 0,
        tick: u64::MAX,
        current_kind: 2,
        current_clip: u32::MAX,
        current_state: [11; 16],
        current_cycle: i64::MAX,
        current_seconds: 0.25,
        outgoing_kind: 1,
        outgoing_clip: 7,
        outgoing_state: [0; 16],
        outgoing_cycle: i64::MIN,
        outgoing_seconds: 0.125,
        requested: [1., 2., 3., 0., 0., 0., 1.],
        applied: [4., 5., 6., 0., 0., 0., 1.],
        accumulated_requested: [7., 8., 9., 0., 0., 0., 1.],
        accumulated_applied: [10., 11., 12., 0., 0., 0., 1.],
    }
}
#[test]
fn complete_unaligned_frame_keeps_nested_pose_ik_and_every_exact_motion_field() {
    let r = row();
    let mut bytes = vec![0xa5];
    bytes.extend(frame(&[r]));
    let s = Snapshot::parse(&bytes[1..]).unwrap();
    assert_eq!(s.scene(), EntityId([9; 16]));
    assert_eq!(s.poses().iter().next().unwrap().seconds, 0.25);
    assert_eq!(
        s.constraints().next().unwrap().constraint,
        EntityId([8; 16])
    );
    let a = s.iter().next().unwrap();
    assert_eq!(a.entity.0, r.entity);
    assert_eq!(a.fingerprint, r.fingerprint);
    assert_eq!(a.root, 3);
    assert!(a.applying);
    assert_eq!(a.tick, u64::MAX);
    assert_eq!(
        a.current.owner,
        RootMotionClockOwner::State(EntityId([11; 16]))
    );
    assert_eq!(a.current.cycle, i64::MAX);
    assert_eq!(a.current.seconds, 0.25);
    assert_eq!(a.outgoing.unwrap().owner, RootMotionClockOwner::Clip(7));
    assert_eq!(a.outgoing.unwrap().cycle, i64::MIN);
    assert_eq!(a.outgoing.unwrap().seconds, 0.125);
    for (a, b) in [
        (a.requested, r.requested),
        (a.applied, r.applied),
        (a.accumulated_requested, r.accumulated_requested),
        (a.accumulated_applied, r.accumulated_applied),
    ] {
        assert_eq!(a.translation, &b[..3]);
        assert_eq!(a.rotation, &b[3..]);
    }
    assert!(Snapshot::parse(&frame(&[])).unwrap().is_empty());
}
#[test]
fn invalid_lengths_nested_sections_and_owner_membership_never_expose_partial_rows() {
    let valid = frame(&[row()]);
    for n in 0..valid.len() {
        assert!(Snapshot::parse(&valid[..n]).is_err());
    }
    let mut tail = valid.clone();
    tail.push(0);
    assert!(Snapshot::parse(&tail).is_err());
    for offset in [0, 8, 24, 28, 32, 36, 40, 48] {
        let mut b = valid.clone();
        b[offset] = 0xff;
        assert!(Snapshot::parse(&b).is_err(), "offset{offset}");
    }
    let mut r = row();
    r.entity = [2; 16];
    assert!(Snapshot::parse(&frame(&[r])).is_err());
    assert!(Snapshot::parse(&frame(&[row(), row()])).is_err());
}
#[test]
fn unknown_tags_nonfinite_rotations_ambiguous_clocks_and_reserved_bytes_refuse() {
    for case in 0..25 {
        let mut r = row();
        match case {
            0 => r.root = 4096,
            1 => r.policy = 4,
            2 => r.flags = 4,
            3 => r.reserved = 1,
            4 => r.current_kind = 3,
            5 => r.current_clip = 0,
            6 => r.current_seconds = -1.,
            7 => r.current_seconds = f64::NAN,
            8 => r.current_seconds = 86400.1,
            9 => r.outgoing_clip = 256,
            10 => r.outgoing_state = [1; 16],
            11 => r.outgoing_kind = u32::MAX,
            12 => r.requested[0] = f64::INFINITY,
            13 => r.applied[3] = 1.,
            14 => r.accumulated_requested[6] = 0.,
            15 => r.accumulated_applied[6] = f64::MAX,
            16 => r.flags = 1,
            17 => {
                r.current_kind = 0;
                r.current_cycle = 0;
                r.current_seconds = 0.;
            }
            18 => {
                r.current_kind = 1;
                r.current_clip = 1;
            }
            19 => r.outgoing_seconds = f64::NEG_INFINITY,
            20 => {
                r.flags = 1;
                r.outgoing_kind = 0;
                r.outgoing_clip = 0;
                r.outgoing_state = [0; 16];
                r.outgoing_cycle = 0;
                r.outgoing_seconds = -0.;
            }
            21 => r.policy = 256,
            22 => r.root = u32::MAX,
            23 => r.current_kind = 256,
            _ => r.accumulated_applied[3] = f64::NAN,
        }
        assert!(Snapshot::parse(&frame(&[r])).is_err(), "case{case}");
    }
    let mut r = row();
    r.flags = 0;
    r.outgoing_kind = 0;
    r.outgoing_clip = 0;
    r.outgoing_state = [0; 16];
    r.outgoing_cycle = 0;
    r.outgoing_seconds = 0.;
    assert!(
        Snapshot::parse(&frame(&[r]))
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .outgoing
            .is_none()
    );
}
#[test]
fn all_exact_signed_cycle_extremes_and_rest_clock_are_representable() {
    for cycle in [
        i64::MIN,
        -9_007_199_254_740_993,
        0,
        9_007_199_254_740_993,
        i64::MAX,
    ] {
        let mut r = row();
        r.current_cycle = cycle;
        let bytes = frame(&[r]);
        assert_eq!(
            Snapshot::parse(&bytes)
                .unwrap()
                .iter()
                .next()
                .unwrap()
                .current
                .cycle,
            cycle
        );
    }
    let mut r = row();
    r.current_kind = 0;
    r.current_state = [0; 16];
    r.current_cycle = 0;
    r.current_seconds = 0.;
    assert_eq!(
        Snapshot::parse(&frame(&[r]))
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .current
            .owner,
        RootMotionClockOwner::Rest
    );
}

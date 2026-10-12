use incant_module::{AnimationPoseKind as Kind, AnimationPoseSnapshot as Snapshot, EntityId};
use incant_wasm_abi as abi;

fn row(kind: Kind) -> abi::AnimationRowV4 {
    abi::AnimationRowV4 {
        id: [1; 16],
        clip: 0,
        flags: 3,
        seconds: 0.25,
        speed: 1.0,
        source_clip: 0,
        transition_flags: 0,
        source_seconds: 0.0,
        source_speed: 0.0,
        elapsed: 0.0,
        duration: 0.0,
        pose_kind: kind as u32,
        sample_offset: 0,
        sample_count: 0,
        reserved: 0,
    }
}
fn sample(clip: u32, seconds: f64, weight: f64) -> abi::AnimationPoseSample {
    abi::AnimationPoseSample {
        clip,
        reserved: 0,
        seconds,
        weight,
    }
}
fn frame(mut row: abi::AnimationRowV4, samples: &[abi::AnimationPoseSample]) -> Vec<u8> {
    row.sample_count = samples.len() as u32;
    let mut bytes = abi::AnimationHeaderV4 {
        magic: *abi::ANIMATION_V4_MAGIC,
        scene: [9; 16],
        count: 1,
        stride: abi::AnimationRowV4::SIZE as u32,
        sample_count: samples.len() as u32,
        sample_stride: abi::AnimationPoseSample::SIZE as u32,
    }
    .encode()
    .to_vec();
    bytes.extend(row.encode());
    for sample in samples {
        bytes.extend(sample.encode());
    }
    bytes
}
#[test]
fn sixteen_owned_samples_retain_order_same_clip_phases_and_full_f64_weights() {
    let samples: Vec<_> = (0..16)
        .map(|n| sample(n % 8, if n < 8 { 0.25 } else { 0.75 }, 1.0 / 16.0))
        .collect();
    let mut storage = vec![0xa5];
    storage.extend(frame(row(Kind::WeightedQuaternionMean), &samples));
    let snapshot = Snapshot::parse(&storage[1..]).unwrap();
    assert_eq!(snapshot.scene(), EntityId([9; 16]));
    assert_eq!(snapshot.len(), 1);
    let value = snapshot.iter().next().unwrap();
    assert_eq!(value.kind, Kind::WeightedQuaternionMean);
    assert_eq!(
        (value.clip, value.seconds, value.speed),
        (Some(0), 0.25, 1.0)
    );
    assert!(value.playing && value.looping && !value.completed && value.transition.is_none());
    assert_eq!(value.samples().len(), 16);
    for (native, copy) in samples.iter().zip(value.samples()) {
        assert_eq!(copy.clip, Some(native.clip as u16));
        assert_eq!(copy.seconds.to_bits(), native.seconds.to_bits());
        assert_eq!(copy.weight.to_bits(), native.weight.to_bits());
    }
    let empty = abi::AnimationHeaderV4 {
        magic: *abi::ANIMATION_V4_MAGIC,
        scene: [9; 16],
        count: 0,
        stride: 96,
        sample_count: 0,
        sample_stride: 24,
    }
    .encode();
    assert!(Snapshot::parse(&empty).unwrap().is_empty());
}
#[test]
fn interpolation_kinds_and_completed_rest_have_distinct_valid_contracts() {
    let mut r = row(Kind::Single);
    let bytes = frame(r, &[sample(0, 0.25, 1.0)]);
    assert_eq!(
        Snapshot::parse(&bytes).unwrap().iter().next().unwrap().kind,
        Kind::Single
    );
    r.pose_kind = Kind::SlerpParameter as u32;
    for weight in [0.125, 1.0] {
        let bytes = frame(r, &[sample(1, 0.75, 1.0 - weight), sample(0, 0.25, weight)]);
        assert_eq!(
            Snapshot::parse(&bytes)
                .unwrap()
                .iter()
                .next()
                .unwrap()
                .samples()
                .len(),
            2
        );
    }
    r.pose_kind = Kind::SlerpTransition as u32;
    r.transition_flags = 3;
    r.source_clip = u32::MAX;
    r.source_seconds = 0.0;
    r.source_speed = -2.0;
    r.elapsed = 0.0;
    r.duration = 2.0;
    let bytes = frame(r, &[sample(u32::MAX, 0.0, 1.0), sample(0, 0.25, 0.0)]);
    let pose = Snapshot::parse(&bytes).unwrap().iter().next().unwrap();
    assert_eq!(pose.transition.unwrap().speed, -2.0);
    assert_eq!(pose.kind, Kind::SlerpTransition);
    let mut rest = row(Kind::WeightedQuaternionMean);
    rest.clip = u32::MAX;
    rest.seconds = 0.0;
    rest.flags = 4;
    let bytes = frame(rest, &[sample(u32::MAX, 0.0, 1.0)]);
    assert!(
        Snapshot::parse(&bytes)
            .unwrap()
            .iter()
            .next()
            .unwrap()
            .completed
    );
    rest.pose_kind = Kind::Single as u32;
    assert!(Snapshot::parse(&frame(rest, &[sample(u32::MAX, 0.0, 1.0)])).is_err());
}
#[test]
fn malformed_tables_numbers_identity_order_and_interpolation_downgrades_reject_whole_frame() {
    let samples = [sample(0, 0.25, 0.25), sample(1, 0.5, 0.75)];
    let bytes = frame(row(Kind::WeightedQuaternionMean), &samples);
    for end in 0..bytes.len() {
        assert!(Snapshot::parse(&bytes[..end]).is_err());
    }
    let mut tail = bytes.clone();
    tail.push(0);
    assert!(Snapshot::parse(&tail).is_err());
    for mode in 0..24 {
        let mut r = row(Kind::WeightedQuaternionMean);
        let mut s = samples;
        match mode {
            0 => r.pose_kind = 256,
            1 => r.reserved = 1,
            2 => r.sample_offset = 1,
            3 => r.clip = 256,
            4 => r.seconds = f64::NAN,
            5 => r.speed = f64::INFINITY,
            6 => r.flags = 8,
            7 => r.flags = 5,
            8 => r.clip = 1,
            9 => r.transition_flags = 1,
            10 => r.source_speed = -0.0,
            11 => s[1].clip = 256,
            12 => s[1].seconds = f64::INFINITY,
            13 => s[1].weight = f64::NAN,
            14 => s[1].weight = -0.1,
            15 => s[1].weight = 0.0,
            16 => s[1].weight = 1.01,
            17 => s[1].reserved = 1,
            18 => s[1].seconds = -1.0,
            19 => s[1].clip = u32::MAX,
            20 => s[1] = s[0],
            21 => s[1].weight = 0.74,
            22 => r.pose_kind = Kind::Single as u32,
            _ => r.pose_kind = Kind::SlerpTransition as u32,
        }
        assert!(Snapshot::parse(&frame(r, &s)).is_err(), "case {mode}");
    }
    for (offset, value) in [(24, 100001), (28, 95), (32, 214337), (36, 23)] {
        let mut bad = bytes.clone();
        bad[offset..offset + 4].copy_from_slice(&u32::to_le_bytes(value));
        assert!(Snapshot::parse(&bad).is_err());
    }
    // A valid first entity cannot hide an invalid later row.
    let mut duplicate = bytes[..40].to_vec();
    duplicate[24..28].copy_from_slice(&2u32.to_le_bytes());
    duplicate[32..36].copy_from_slice(&4u32.to_le_bytes());
    duplicate.extend(&bytes[40..136]);
    let mut second = abi::AnimationRowV4::decode(&bytes[40..]).unwrap();
    second.sample_offset = 2;
    duplicate.extend(second.encode());
    duplicate.extend(&bytes[136..]);
    duplicate.extend(&bytes[136..]);
    assert!(Snapshot::parse(&duplicate).is_err());
    second.id = [2; 16];
    second.sample_offset = 3;
    duplicate[136..232].copy_from_slice(&second.encode());
    assert!(Snapshot::parse(&duplicate).is_err());
    second.sample_offset = 2;
    duplicate[136..232].copy_from_slice(&second.encode());
    assert_eq!(Snapshot::parse(&duplicate).unwrap().len(), 2);
}
#[test]
fn slerp_weights_must_match_scalar_destination_and_transition_metadata() {
    let mut r = row(Kind::SlerpTransition);
    r.transition_flags = 1;
    r.source_clip = 1;
    r.source_seconds = 0.5;
    r.source_speed = 1.0;
    r.elapsed = 0.25;
    r.duration = 1.0;
    let valid = [sample(1, 0.5, 0.75), sample(0, 0.25, 0.25)];
    assert!(Snapshot::parse(&frame(r, &valid)).is_ok());
    for mode in 0..6 {
        let mut s = valid;
        let mut bad = r;
        match mode {
            0 => s.swap(0, 1),
            1 => s[0].seconds = 0.6,
            2 => {
                s[0].weight = 0.5;
                s[1].weight = 0.5;
            }
            3 => bad.pose_kind = Kind::WeightedQuaternionMean as u32,
            4 => bad.elapsed = 1.0,
            _ => bad.source_clip = u32::MAX,
        }
        assert!(Snapshot::parse(&frame(bad, &s)).is_err());
    }
}

#[test]
fn maximum_complete_frame_fits_guest_memory_without_truncating_expanded_rows() {
    let mut bytes = abi::AnimationHeaderV4 {
        magic: *abi::ANIMATION_V4_MAGIC,
        scene: [9; 16],
        count: abi::ANIMATION_MAX_ROWS as u32,
        stride: 96,
        sample_count: abi::ANIMATION_POSE_MAX_SAMPLE_ROWS as u32,
        sample_stride: 24,
    }
    .encode()
    .to_vec();
    let mut offset = 0;
    for index in 0..abi::ANIMATION_MAX_ROWS {
        let weighted = index < abi::ANIMATION_GRAPH_MAX_ROWS;
        let mut r = row(if weighted {
            Kind::WeightedQuaternionMean
        } else {
            Kind::SlerpParameter
        });
        r.id = (index as u128).to_be_bytes();
        r.sample_offset = offset;
        r.sample_count = if weighted { 16 } else { 2 };
        offset += r.sample_count;
        bytes.extend(r.encode());
    }
    for index in 0..abi::ANIMATION_MAX_ROWS {
        if index < abi::ANIMATION_GRAPH_MAX_ROWS {
            for clip in 0..16 {
                bytes.extend(sample(clip, 0.25, 1.0 / 16.0).encode());
            }
        } else {
            bytes.extend(sample(1, 0.5, 0.75).encode());
            bytes.extend(sample(0, 0.25, 0.25).encode());
        }
    }
    assert_eq!(bytes.len(), abi::ANIMATION_V4_MAX_BYTES);
    assert!(bytes.len() < abi::MEMORY);
    let snapshot = Snapshot::parse(&bytes).unwrap();
    assert_eq!(snapshot.len(), 100000);
    assert_eq!(snapshot.iter().next().unwrap().samples().len(), 16);
    assert_eq!(snapshot.iter().last().unwrap().samples().len(), 2);
    bytes[32..36].copy_from_slice(&(abi::ANIMATION_POSE_MAX_SAMPLE_ROWS as u32 + 1).to_le_bytes());
    assert!(Snapshot::parse(&bytes).is_err());
}

//! Compare compact decoded fields with untouched SampleX, not invented redundancy.
use super::{Sample, StorageRecord};
use crate::motion::visibility::decode_compact;

#[test]
#[ignore = "coordinated 64-square offscreen numerical three-plane comparison; no window"]
fn output_visibility_compact_two_arm_numerical() {
    let root =
        std::path::PathBuf::from(std::env::var_os("INCANT_VISIBILITY_COMPACT_OUTPUT").unwrap());
    super::capture(&root, super::Capture::Compact);
}

pub(super) fn packing_errors(records: &[StorageRecord]) -> Vec<usize> {
    assert_eq!(records.len(), 64 * 64 * 4, "full compact raw plane");
    records
        .iter()
        .enumerate()
        .filter_map(|(i, r)| {
            let Some((motion, owner)) = decode_compact(r.raw_motion_zw) else {
                return Some(i);
            };
            let exact = r.sample.motion[2].to_bits() == motion[0]
            && r.sample.motion[3].to_bits() == motion[1]
            && r.sample.owner == [owner, owner & 65535, owner >> 16, 0]
            // No standalone owner plane exists; reserved readback words stay zero.
            && r.raw_owner == [0; 4];
            let clear = r.sample.position_depth[2] != 1.
                || (r.raw_motion_zw == [0; 4]
                    && r.sample.motion.map(f32::to_bits) == [0; 4]
                    && r.sample.position_depth[..2]
                        .iter()
                        .all(|v| v.to_bits() == 0));
            (!exact || !clear).then_some(i)
        })
        .collect()
}

pub(super) fn differences(actual: &[Sample], reference: &[Sample]) -> Vec<usize> {
    assert_eq!(actual.len(), 64 * 64 * 4, "full compact semantic plane");
    assert_eq!(
        reference.len(),
        actual.len(),
        "full SampleX reference plane"
    );
    actual
        .iter()
        .zip(reference)
        .enumerate()
        .filter_map(|(i, (a, b))| {
            (a.owner[0] != b.owner[0]
                || a.motion.map(f32::to_bits) != b.motion.map(f32::to_bits)
                || a.position_depth.map(f32::to_bits) != b.position_depth.map(f32::to_bits))
            .then_some(i)
        })
        .collect()
}

#[test]
fn compact_visibility_comparison_detects_owner_motion_depth_position_and_truncation() {
    let raw = vec![
        Sample {
            motion: [0.; 4],
            position_depth: [0.; 4],
            owner: [0; 4]
        };
        64 * 64 * 4
    ];
    assert!(differences(&raw, &raw).is_empty());
    for field in 0..9 {
        let mut changed = raw.clone();
        match field {
            0..4 => changed[48].motion[field] = f32::from_bits(1),
            4..8 => changed[48].position_depth[field - 4] = f32::from_bits(1),
            _ => changed[48].owner[0] = 0x8000_0001,
        }
        assert_eq!(differences(&changed, &raw), [48]);
    }
    let mut signed_zero = raw.clone();
    signed_zero[0].motion[2] = -0.;
    assert_eq!(differences(&signed_zero, &raw), [0]);
    assert!(std::panic::catch_unwind(|| differences(&raw[..1], &raw)).is_err());
    assert!(std::panic::catch_unwind(|| differences(&raw, &raw[..1])).is_err());
}

#[test]
fn compact_visibility_raw_words_reject_corruption_and_invented_companion() {
    let clear = StorageRecord {
        sample: Sample {
            motion: [0.; 4],
            position_depth: [0., 0., 1., 0.],
            owner: [0; 4],
        },
        raw_motion_zw: [0; 4],
        raw_owner: [0; 4],
    };
    let mut raw = vec![clear; 64 * 64 * 4];
    raw[0] = StorageRecord {
        sample: Sample {
            motion: [0., 0., -0., 1.],
            position_depth: [0.125, 0.375, 0.5, 0.],
            owner: [0x8000_0001, 1, 32768, 0],
        },
        raw_motion_zw: [0, 49152, 1, 32768],
        raw_owner: [0; 4],
    };
    assert!(packing_errors(&raw).is_empty());
    for field in 0..4 {
        let mut changed = raw.clone();
        changed[0].raw_motion_zw[field] ^= 1;
        assert_eq!(packing_errors(&changed), [0]);
    }
    raw[0].raw_owner[2] = 1;
    assert_eq!(packing_errors(&raw), [0]);
    raw[0].raw_owner = [0; 4];
    raw[1].raw_motion_zw = raw[0].raw_motion_zw;
    raw[1].sample.motion = raw[0].sample.motion;
    raw[1].sample.owner = raw[0].sample.owner;
    assert_eq!(
        packing_errors(&raw),
        [1],
        "uncovered sample must remain clear"
    );
    assert!(std::panic::catch_unwind(|| packing_errors(&raw[..1])).is_err());
}

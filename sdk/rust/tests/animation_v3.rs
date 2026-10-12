use incant_module::{AnimationBlendKind, AnimationSnapshot};
use incant_wasm_abi as abi;

fn row() -> abi::AnimationRowV3 {
    abi::AnimationRowV3 {
        id: [7; 16],
        clip: 1,
        flags: 3,
        seconds: 0.5,
        speed: 1.,
        source_clip: 0,
        transition_flags: 0,
        source_seconds: 0.,
        source_speed: 0.,
        elapsed: 0.,
        duration: 0.,
        blend_kind: 2,
        blend_source_clip: 0,
        blend_source_seconds: 0.25,
        blend_weight: 0.25,
    }
}
fn bytes(row: abi::AnimationRowV3) -> Vec<u8> {
    let mut b = abi::AnimationHeader {
        magic: *abi::ANIMATION_V3_MAGIC,
        count: 1,
        stride: abi::AnimationRowV3::SIZE as u32,
    }
    .encode()
    .to_vec();
    b.extend(row.encode());
    b
}
#[test]
fn parameter_and_timed_blends_preserve_distinct_pose_and_clock_meanings() {
    let mut unaligned = vec![0xa5];
    unaligned.extend(bytes(row()));
    let value = AnimationSnapshot::parse(&unaligned[1..])
        .unwrap()
        .iter()
        .next()
        .unwrap();
    assert_eq!((value.clip, value.seconds), (Some(1), 0.5));
    assert!(value.transition.is_none());
    let blend = value.pose_blend.unwrap();
    assert_eq!(
        (blend.kind, blend.clip, blend.seconds, blend.weight),
        (AnimationBlendKind::Parameter, Some(0), 0.25, 0.25)
    );
    let mut fade = row();
    fade.blend_kind = 1;
    fade.transition_flags = 3;
    fade.source_seconds = 0.25;
    fade.source_speed = -1.;
    fade.elapsed = 0.5;
    fade.duration = 2.;
    let data = bytes(fade);
    let value = AnimationSnapshot::parse(&data)
        .unwrap()
        .iter()
        .next()
        .unwrap();
    assert_eq!(
        value.pose_blend.unwrap().kind,
        AnimationBlendKind::Transition
    );
    assert_eq!(value.transition.unwrap().speed, -1.);
    assert_eq!(value.pose_blend.unwrap().weight, 0.25);
    // A near-boundary parameter calculation may round to one while retaining
    // the two samples. It is valid; zero instead has a single-pose encoding.
    let mut endpoint = row();
    endpoint.blend_weight = 1.;
    assert!(AnimationSnapshot::parse(&bytes(endpoint)).is_ok());
}
#[test]
fn truncated_mismatched_or_forged_pose_fields_reject_the_whole_frame() {
    let data = bytes(row());
    for end in 0..data.len() {
        assert!(AnimationSnapshot::parse(&data[..end]).is_err());
    }
    for mode in 0..12 {
        let mut r = row();
        match mode {
            0 => r.blend_kind = 3,
            1 => r.blend_source_clip = 256,
            2 => r.blend_source_seconds = -1.,
            3 => r.blend_source_seconds = f64::NAN,
            4 => r.blend_weight = 0.,
            5 => r.blend_weight = 1.001,
            6 => r.blend_weight = f64::INFINITY,
            7 => r.blend_kind = 0,
            8 => r.blend_kind = 1,
            9 => {
                r.blend_source_clip = u32::MAX;
                r.blend_source_seconds = 0.;
            }
            10 => {
                r.blend_kind = 1;
                r.transition_flags = 3;
                r.source_seconds = 0.3;
                r.elapsed = 0.25;
                r.duration = 1.;
            }
            _ => {
                r.blend_kind = 1;
                r.transition_flags = 3;
                r.source_seconds = 0.25;
                r.elapsed = 0.5;
                r.duration = 1.;
            }
        }
        let mut bad = data.clone();
        bad[8..12].copy_from_slice(&2u32.to_le_bytes());
        bad.extend(r.encode());
        assert!(AnimationSnapshot::parse(&bad).is_err(), "mode {mode}");
    }
    let mut bad = data.clone();
    bad.push(0);
    assert!(AnimationSnapshot::parse(&bad).is_err());
    let mut bad = data;
    bad[..8].copy_from_slice(abi::ANIMATION_V2_MAGIC);
    assert!(AnimationSnapshot::parse(&bad).is_err());
}

#[test]
fn parameter_blend_cannot_forge_a_rest_destination() {
    let mut r = row();
    r.clip = u32::MAX;
    r.seconds = 0.;
    assert!(AnimationSnapshot::parse(&bytes(r)).is_err());
}

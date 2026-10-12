#[path = "../../incant_assets/tests/skeletal/alloc.rs"]
mod alloc;
#[global_allocator]
static ALLOC: alloc::CountAlloc = alloc::CountAlloc;

use glam::{DMat4, DQuat, DVec3};
use incant_anim::{
    RootMotionCatalog, RootMotionClip, RootMotionDelta, RootMotionError, RootMotionTime,
    RootMotionTrack,
    track::{Interpolation, Kind, Track},
};

fn track(values: Vec<[f32; 4]>, kind: Kind) -> RootMotionTrack {
    RootMotionTrack::new(vec![0., 2.], values, Interpolation::Linear, kind).unwrap()
}
fn turning_clip() -> RootMotionClip {
    let q = std::f32::consts::FRAC_1_SQRT_2;
    RootMotionClip::new(
        2.,
        Some(track(vec![[0.; 4], [2., 0., 0., 0.]], Kind::Vector)),
        Some(track(
            vec![[0., 0., 0., 1.], [0., 0., q, q]],
            Kind::Rotation,
        )),
    )
    .unwrap()
}
fn catalog(clips: Vec<RootMotionClip>) -> Result<RootMotionCatalog, RootMotionError> {
    RootMotionCatalog::new(
        [7; 32],
        3,
        [0.; 3],
        [0., 0., 0., 1.],
        RootMotionDelta::IDENTITY,
        clips,
    )
}
fn time(cycle: i64, seconds: f64) -> RootMotionTime {
    RootMotionTime { cycle, seconds }
}
fn matrix(delta: RootMotionDelta) -> DMat4 {
    DMat4::from_rotation_translation(
        DQuat::from_array(delta.rotation),
        DVec3::from_array(delta.translation),
    )
}

#[test]
fn catalog_copies_only_checked_root_tracks_and_keeps_each_clip_duration() {
    let catalog = catalog(vec![
        turning_clip(),
        RootMotionClip::new(4., None, None).unwrap(),
        RootMotionClip::new(0., None, None).unwrap(),
    ])
    .unwrap();
    assert_eq!(catalog.fingerprint(), [7; 32]);
    assert_eq!(catalog.root(), 3);
    assert_eq!(
        catalog
            .clips()
            .map(RootMotionClip::duration)
            .collect::<Vec<_>>(),
        [2., 4., 0.]
    );
    assert!(catalog.sampler(3).is_none());
    let root = catalog.sampler(0).unwrap();
    let cycle = DMat4::from_rotation_translation(
        DQuat::from_rotation_z(std::f64::consts::FRAC_PI_2),
        DVec3::new(2., 0., 0.),
    );
    for n in -5i64..=5 {
        let mut expected = DMat4::IDENTITY;
        for _ in 0..n.unsigned_abs() {
            expected *= if n < 0 { cycle.inverse() } else { cycle };
        }
        let got = root
            .loop_delta(time(i64::MAX - 10, 0.), time(i64::MAX - 10 + n, 0.))
            .unwrap();
        assert!(matrix(got).abs_diff_eq(expected, 2e-6));
    }
    assert_eq!(
        catalog
            .sampler(1)
            .unwrap()
            .loop_delta(time(-4, 3.), time(5, 1.))
            .unwrap(),
        RootMotionDelta::IDENTITY
    );
    assert_eq!(
        catalog.sampler(2).unwrap().clamped_delta(-3., 8.).unwrap(),
        RootMotionDelta::IDENTITY
    );
}

#[test]
fn checked_track_ingress_refuses_malformed_channels_before_sampling() {
    let vector = [[0.; 4], [1., 0., 0., 0.]];
    for times in [
        vec![],
        vec![0.],
        vec![0., 0.],
        vec![1., 0.],
        vec![-1., 1.],
        vec![0., f32::NAN],
    ] {
        assert!(Track::new(&times, &vector, Interpolation::Linear, Kind::Vector).is_err());
    }
    for values in [vec![[f32::NAN; 4]; 2], vec![[0., 0., 0., 1.]; 2]] {
        assert!(Track::new(&[0., 1.], &values, Interpolation::Linear, Kind::Vector).is_err());
    }
    assert!(Track::new(&[0., 1.], &vector, Interpolation::Linear, Kind::Rotation).is_err());
    let checked = Track::new(&[0., 1.], &vector, Interpolation::Step, Kind::Vector).unwrap();
    assert_eq!(checked.sample(0.999).unwrap(), [0.; 4]);
    assert_eq!(checked.sample(1.).unwrap(), [1., 0., 0., 0.]);
    assert_eq!(checked.sample(f64::NAN), Err(RootMotionError::Time));
    assert!(RootMotionClip::new(1., Some(track(vector.to_vec(), Kind::Vector)), None).is_err());
    assert!(RootMotionClip::new(2., None, Some(track(vector.to_vec(), Kind::Vector))).is_err());
    assert!(RootMotionClip::new(-0., None, None).is_err());
}

#[test]
fn catalog_admission_counts_aggregate_keys_and_root_identity() {
    assert!(matches!(
        catalog(
            (0..257)
                .map(|_| RootMotionClip::new(1., None, None).unwrap())
                .collect()
        ),
        Err(RootMotionError::Capacity(_))
    ));
    assert!(
        RootMotionCatalog::new(
            [7; 32],
            4096,
            [0.; 3],
            [0., 0., 0., 1.],
            RootMotionDelta::IDENTITY,
            vec![]
        )
        .is_err()
    );
    let mut invalid_reference = RootMotionDelta::IDENTITY;
    invalid_reference.rotation = [0.; 4];
    assert!(
        RootMotionCatalog::new(
            [7; 32],
            0,
            [0.; 3],
            [0., 0., 0., 1.],
            invalid_reference,
            vec![]
        )
        .is_err()
    );
    assert!(
        RootMotionCatalog::new(
            [7; 32],
            0,
            [f32::NAN; 3],
            [0., 0., 0., 1.],
            RootMotionDelta::IDENTITY,
            vec![]
        )
        .is_err()
    );
    let clips = (0..2)
        .map(|_| {
            let count = incant_anim::MAX_ROOT_MOTION_KEYS / 2 + 1;
            let t = RootMotionTrack::new(
                (0..count).map(|n| n as f32 * 0.000001).collect(),
                vec![[0.; 4]; count],
                Interpolation::Linear,
                Kind::Vector,
            )
            .unwrap();
            RootMotionClip::new(1., Some(t), None).unwrap()
        })
        .collect();
    assert_eq!(
        catalog(clips).unwrap_err(),
        RootMotionError::Capacity("total root keys")
    );
}

#[test]
fn catalog_discards_spare_input_capacity_and_reuses_endpoints_without_allocations() {
    let make = |capacity| {
        let mut times = Vec::with_capacity(capacity);
        times.extend([0., 2.]);
        let mut values = Vec::with_capacity(capacity);
        values.extend([[0.; 4], [2., 0., 0., 0.]]);
        let translation =
            RootMotionTrack::new(times, values, Interpolation::Linear, Kind::Vector).unwrap();
        catalog(vec![
            RootMotionClip::new(2., Some(translation), None).unwrap(),
        ])
        .unwrap()
    };
    let compact = make(2);
    let oversized = make(4096);
    assert_eq!(compact, oversized);
    assert_eq!(compact.retained_bytes(), oversized.retained_bytes());
    let turning = catalog(vec![turning_clip()]).unwrap();
    let (_, allocations) = alloc::measured(|| {
        for i in 0..4096 {
            let sampler = turning.sampler(0).unwrap();
            std::hint::black_box(sampler.loop_delta(time(i, 1.9), time(i + 2, 0.1)).unwrap());
            assert_eq!(
                sampler.loop_delta(time(i64::MIN, 0.), time(i64::MAX, 0.)),
                Err(RootMotionError::Cycles)
            );
        }
    });
    assert_eq!(allocations, 0);
}

#[test]
fn canonical_binary_roundtrip_and_malformed_payloads_never_publish_a_catalog() {
    let source = catalog(vec![
        turning_clip(),
        RootMotionClip::new(4., None, None).unwrap(),
    ])
    .unwrap();
    let bytes = source.to_bytes().unwrap();
    assert_eq!(bytes.len(), source.encoded_len().unwrap());
    let loaded = RootMotionCatalog::from_bytes(&bytes).unwrap();
    assert_eq!(loaded, source);
    assert_eq!(loaded.to_bytes().unwrap(), bytes);
    for length in 0..bytes.len() {
        assert!(
            RootMotionCatalog::from_bytes(&bytes[..length]).is_err(),
            "truncation {length}"
        );
    }
    for variant in 0..10 {
        let mut bad = bytes.clone();
        match variant {
            0 => bad[0] ^= 1,
            1 => bad[8..12].copy_from_slice(&2u32.to_le_bytes()),
            2 => bad[46] = 1,
            3 => bad[132..136].copy_from_slice(&u32::MAX.to_le_bytes()),
            4 => bad[140] |= 128,
            5 => bad[144] = 2,
            6 => bad[148..152].copy_from_slice(&u32::MAX.to_le_bytes()),
            7 => bad[172..176].copy_from_slice(&0f32.to_le_bytes()),
            8 => bad[60..76].fill(0),
            _ => bad.push(0),
        }
        assert!(
            RootMotionCatalog::from_bytes(&bad).is_err(),
            "malformed {variant}"
        );
    }
    // A later valid admission is independent of any rejected candidate.
    assert_eq!(RootMotionCatalog::from_bytes(&bytes).unwrap(), source);
}

#[test]
fn rigid_basis_composition_checks_original_f64_and_overflow_without_narrowing() {
    let delta = RootMotionDelta {
        translation: [f64::MAX, 0., 0.],
        ..RootMotionDelta::IDENTITY
    };
    assert_eq!(delta.compose(delta), Err(RootMotionError::NonFinite));
    assert_eq!(
        RootMotionDelta {
            rotation: [0.; 4],
            ..RootMotionDelta::IDENTITY
        }
        .inverse(),
        Err(RootMotionError::NonFinite)
    );
    let parent = RootMotionDelta::from_rest(
        [10., 0., 0.],
        [
            0.,
            0.,
            std::f32::consts::FRAC_1_SQRT_2,
            std::f32::consts::FRAC_1_SQRT_2,
        ],
    )
    .unwrap();
    let child = RootMotionDelta::from_rest([1., 0., 0.], [0., 0., 0., 1.]).unwrap();
    let actual = parent.compose(child).unwrap();
    assert!((actual.translation[0] - 10.).abs() < 1e-12);
    assert!((actual.translation[1] - 1.).abs() < 1e-12);
    assert!(
        matrix(actual.compose(actual.inverse().unwrap()).unwrap())
            .abs_diff_eq(DMat4::IDENTITY, 1e-12)
    );
}

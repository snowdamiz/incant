use super::*;
use incant_assets_runtime::{
    CookedModel,
    animation::{ClipSample, PoseSource, WeightedClipSample},
};

fn model() -> CookedModel {
    let dir = tempfile::tempdir().unwrap();
    fixture::fixture(dir.path());
    let model = cook_gltf(
        dir.path(),
        Path::new("model.gltf"),
        &dir.path().join("cache"),
    )
    .unwrap();
    dir.close().unwrap();
    model
}
fn rotated() -> ClipSample {
    ClipSample {
        clip: Some(1),
        seconds: 2.,
    }
}
fn weighted() -> [WeightedClipSample; 2] {
    [
        WeightedClipSample {
            sample: ClipSample::REST,
            weight: 3.,
        },
        WeightedClipSample {
            sample: rotated(),
            weight: 1.,
        },
    ]
}
fn pair() -> PoseSource<'static> {
    PoseSource::SphericalPair {
        from: ClipSample::REST,
        to: rotated(),
        weight: 0.25,
    }
}
fn accepted(p: &AnimationPlayer) -> Vec<glam::DMat4> {
    p.locals()
        .iter()
        .chain(p.globals())
        .chain(p.joint_palette(0).unwrap())
        .copied()
        .collect()
}
fn sample(p: &mut AnimationPlayer, source: PoseSource<'_>) {
    match source {
        PoseSource::Clip(s) => p.sample(s.clip, s.seconds).unwrap(),
        PoseSource::SphericalPair { from, to, weight } => p.sample_blend(from, to, weight).unwrap(),
        PoseSource::Weighted(s) => p.sample_weighted(s).unwrap(),
    }
}

#[test]
fn grouped_transition_preserves_slerp_and_weighted_mean_instead_of_flattening() {
    let m = model();
    let mut p = AnimationPlayer::new(&m).unwrap();
    let weighted = weighted();
    let weight = 0.4;
    p.sample_pose_crossfade(pair(), PoseSource::Weighted(&weighted), weight)
        .unwrap();
    // Same nominal .75/.25 contribution weights, different rotation rules:
    // pair => 45 degrees; weighted mean => 2*atan(1/3). Outer SLERP lies on
    // that angular arc. Flattening the four samples would lose the first rule.
    let angle = std::f64::consts::FRAC_PI_4 * (1. - weight) + 2. * (1f64 / 3.).atan() * weight;
    let (sin, cos) = angle.sin_cos();
    near(p.locals()[2].x_axis.truncate().to_array(), [cos, sin, 0.]);
    for mesh in [3, 4] {
        near(
            p.skin_position(mesh, 0, 2, [2., 0., 0.]).unwrap(),
            [11. + cos + sin, 1. + sin - cos, 0.],
        );
    }
    let mut flat = AnimationPlayer::new(&m).unwrap();
    flat.sample_weighted(&weighted).unwrap();
    assert!((p.locals()[2].x_axis.x - flat.locals()[2].x_axis.x).abs() > 0.02);
}

#[test]
fn every_source_pair_preserves_exact_endpoints_and_ordinary_pair_compatibility() {
    let m = model();
    let mut p = AnimationPlayer::new(&m).unwrap();
    let mut baseline = AnimationPlayer::new(&m).unwrap();
    let weights = weighted();
    let sources = [
        PoseSource::Clip(ClipSample::REST),
        pair(),
        PoseSource::Weighted(&weights),
    ];
    for from in sources {
        for to in sources {
            for (weight, source) in [(0., from), (1., to)] {
                sample(&mut baseline, source);
                p.sample_pose_crossfade(from, to, weight).unwrap();
                assert_eq!(accepted(&p), accepted(&baseline));
            }
        }
    }
    for weight in [0., 0.001, 0.25, 0.5, 0.999, 1.] {
        baseline
            .sample_blend(ClipSample::REST, rotated(), weight)
            .unwrap();
        p.sample_pose_crossfade(
            PoseSource::Clip(ClipSample::REST),
            PoseSource::Clip(rotated()),
            weight,
        )
        .unwrap();
        assert_eq!(accepted(&p), accepted(&baseline));
    }
}

#[test]
fn grouped_linear_channels_mix_once_and_fixed_matrix_nodes_stay_unchanged() {
    let mut m = model();
    m.animation.as_mut().unwrap().rest[3] = None;
    let mut p = AnimationPlayer::new(&m).unwrap();
    let entries = [
        WeightedClipSample {
            sample: ClipSample {
                clip: Some(0),
                seconds: 2.,
            },
            weight: 1.,
        },
        WeightedClipSample {
            sample: ClipSample {
                clip: Some(2),
                seconds: 2.,
            },
            weight: 1.,
        },
    ];
    p.sample_pose_crossfade(
        PoseSource::Weighted(&entries),
        PoseSource::Clip(ClipSample::REST),
        0.25,
    )
    .unwrap();
    // First group: translation2, scale(0,1.5,1); rest: translation1, scale1.
    near(p.locals()[1].w_axis.truncate().to_array(), [1.75, 0., 0.]);
    near(
        [
            p.locals()[1].x_axis.x,
            p.locals()[1].y_axis.y,
            p.locals()[1].z_axis.z,
        ],
        [0.25, 1.375, 1.],
    );
    near(p.locals()[3].w_axis.truncate().to_array(), [100., 0., 0.]);
    p.sample_pose_crossfade(
        PoseSource::Clip(ClipSample::REST),
        PoseSource::Weighted(&entries),
        0.75,
    )
    .unwrap();
    near(
        p.globals()[2].w_axis.truncate().to_array(),
        [11.75, 1.375, 0.],
    );
}

#[test]
fn inactive_invalid_sources_and_weights_refuse_atomically_and_recover() {
    let m = model();
    let mut p = AnimationPlayer::new(&m).unwrap();
    sample(&mut p, pair());
    let prior = accepted(&p);
    let bad_zero = [
        WeightedClipSample {
            sample: ClipSample {
                clip: Some(999),
                seconds: 0.,
            },
            weight: 0.,
        },
        weighted()[0],
    ];
    let empty = [];
    let oversized = [weighted()[0]; 17];
    let bad_weight = [WeightedClipSample {
        weight: f64::NAN,
        ..weighted()[0]
    }];
    let zero = [WeightedClipSample {
        weight: 0.,
        ..weighted()[0]
    }];
    let invalid = [
        PoseSource::Weighted(&bad_zero),
        PoseSource::Weighted(&empty),
        PoseSource::Weighted(&oversized),
        PoseSource::Weighted(&bad_weight),
        PoseSource::Weighted(&zero),
        PoseSource::SphericalPair {
            from: ClipSample {
                seconds: f64::NAN,
                ..ClipSample::REST
            },
            to: rotated(),
            weight: 1.,
        },
        PoseSource::SphericalPair {
            from: ClipSample::REST,
            to: rotated(),
            weight: 1.01,
        },
    ];
    for bad in invalid {
        for weight in [0., 0.5, 1.] {
            for (from, to) in [(pair(), bad), (bad, pair())] {
                assert!(p.sample_pose_crossfade(from, to, weight).is_err());
                assert_eq!(accepted(&p), prior);
            }
        }
    }
    for weight in [f64::NAN, f64::INFINITY, -0.1, 1.1] {
        assert!(p.sample_pose_crossfade(pair(), pair(), weight).is_err());
        assert_eq!(accepted(&p), prior);
    }
    p.sample_pose_crossfade(pair(), pair(), 0.).unwrap();
    assert_eq!(accepted(&p), prior);
}

#[test]
fn grouped_updates_reuse_retained_storage_during_mixed_sampling() {
    let m = model();
    let mut p = AnimationPlayer::new(&m).unwrap();
    let weighted = weighted();
    let (_, allocations) = alloc::measured(|| {
        for step in 0..1024 {
            p.sample_pose_crossfade(
                pair(),
                PoseSource::Weighted(&weighted),
                (step % 33) as f64 / 32.,
            )
            .unwrap();
            std::hint::black_box(p.skin_position(3, 0, 2, [2., 0., 0.]).unwrap());
            p.sample_weighted(&weighted).unwrap();
        }
    });
    assert_eq!(allocations, 0);
}

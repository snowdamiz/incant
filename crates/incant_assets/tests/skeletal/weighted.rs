use super::*;
use incant_assets_runtime::{
    CookedModel,
    animation::{ClipSample, MAX_POSE_SAMPLES, WeightedClipSample},
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
fn entry(clip: Option<usize>, seconds: f64, weight: f64) -> WeightedClipSample {
    WeightedClipSample {
        sample: ClipSample { clip, seconds },
        weight,
    }
}
fn samples() -> [WeightedClipSample; 4] {
    [
        entry(Some(0), 2., 1.),
        entry(Some(1), 2., 1.),
        entry(Some(2), 2., 1.),
        entry(None, 0., 1.),
    ]
}
pub(super) fn four_pose_oracle(model: &CookedModel) {
    let mut player = AnimationPlayer::new(model).unwrap();
    player.sample_weighted(&samples()).unwrap();
    // Independent local-space oracle: mean translation1.5, scale(.5,1.25,1),
    // child quaternion normalize(0,0,1,3), so sin=.6 and cos=.8. Applying the
    // hierarchy and inverse bind to vertex(2,0) gives(12.2,1). Matrix blending
    // or sequential pairwise spherical blending does not produce this result.
    near(
        player.locals()[1].w_axis.truncate().to_array(),
        [1.5, 0., 0.],
    );
    near(
        player.globals()[2].w_axis.truncate().to_array(),
        [11.5, 1.25, 0.],
    );
    for node in [3, 4] {
        near(
            player.skin_position(node, 0, 2, [2., 0., 0.]).unwrap(),
            [12.2, 1., 0.],
        );
    }
}

#[test]
fn four_pose_mix_uses_rest_for_missing_channels_and_composes_one_hierarchy() {
    let mut model = model();
    model.animation.as_mut().unwrap().rest[3] = None;
    four_pose_oracle(&model);
    let mut player = AnimationPlayer::new(&model).unwrap();
    player.sample_weighted(&samples()).unwrap();
    near(
        player.locals()[3].w_axis.truncate().to_array(),
        [100., 0., 0.],
    );
    player.sample(Some(0), 1.25).unwrap();
    player.sample_weighted(&samples()).unwrap();
    near(
        player.skin_position(3, 0, 2, [2., 0., 0.]).unwrap(),
        [12.2, 1., 0.],
    );
}

#[test]
fn relative_weights_support_extreme_finite_values_and_exact_single_contributions() {
    let model = model();
    let mut player = AnimationPlayer::new(&model).unwrap();
    let mut expected = AnimationPlayer::new(&model).unwrap();
    expected.sample_weighted(&samples()).unwrap();
    for weight in [f64::MAX, f64::MIN_POSITIVE, f64::from_bits(1), 0.125] {
        let set = samples().map(|mut s| {
            s.weight = weight;
            s
        });
        player.sample_weighted(&set).unwrap();
        assert_eq!(player.locals(), expected.locals());
        assert_eq!(
            player.joint_palette(0).unwrap(),
            expected.joint_palette(0).unwrap()
        );
    }
    for sample in [
        ClipSample::REST,
        ClipSample {
            clip: Some(1),
            seconds: -1.,
        },
        ClipSample {
            clip: Some(0),
            seconds: 1.234,
        },
        ClipSample {
            clip: Some(2),
            seconds: 4.,
        },
    ] {
        expected.sample(sample.clip, sample.seconds).unwrap();
        player
            .sample_weighted(&[
                entry(Some(0), 0., 0.),
                WeightedClipSample {
                    sample,
                    weight: f64::from_bits(1),
                },
            ])
            .unwrap();
        assert_eq!(player.locals(), expected.locals());
        assert_eq!(player.globals(), expected.globals());
        assert_eq!(
            player.joint_palette(0).unwrap(),
            expected.joint_palette(0).unwrap()
        );
    }
}

#[test]
fn rotation_mix_is_antipodal_safe_including_orthogonal_hemispheres_and_ties() {
    let original = model();
    let mut signed = model();
    for channel in &mut signed.animation.as_mut().unwrap().clips[1].channels {
        for value in &mut channel.values {
            *value = value.map(|v| -v);
        }
    }
    let mut a = AnimationPlayer::new(&original).unwrap();
    let mut b = AnimationPlayer::new(&signed).unwrap();
    let mut set = samples();
    a.sample_weighted(&set).unwrap();
    for _ in 0..set.len() {
        set.rotate_left(1);
        b.sample_weighted(&set).unwrap();
        for (a, b) in a.globals().iter().zip(b.globals()) {
            assert!(a.abs_diff_eq(*b, 1e-12));
        }
    }
}

#[test]
fn invalid_weight_sets_and_unused_samples_cannot_publish_or_contaminate_next_mix() {
    let mut player = AnimationPlayer::new(&model()).unwrap();
    player.sample(Some(1), 0.5).unwrap();
    let locals = player.locals().to_vec();
    let globals = player.globals().to_vec();
    let joints = player.joint_palette(0).unwrap().to_vec();
    let too_many = [entry(Some(0), 1., 1.); MAX_POSE_SAMPLES + 1];
    let bad = [
        entry(Some(99), 0., 0.),
        entry(None, f64::NAN, 0.),
        entry(Some(0), f64::INFINITY, 0.),
        entry(None, 0., f64::NAN),
        entry(None, 0., f64::INFINITY),
        entry(None, 0., -0.1),
    ];
    for entry in bad {
        assert!(player.sample_weighted(&[entry, samples()[0]]).is_err());
        assert_eq!(player.locals(), locals);
        assert_eq!(player.globals(), globals);
        assert_eq!(player.joint_palette(0).unwrap(), joints);
    }
    for invalid in [&[][..], &too_many[..], &[entry(None, 0., 0.)][..]] {
        assert!(player.sample_weighted(invalid).is_err());
        assert_eq!(player.locals(), locals);
        assert_eq!(player.globals(), globals);
        assert_eq!(player.joint_palette(0).unwrap(), joints);
    }
    player.sample_weighted(&samples()).unwrap();
    near(
        player.skin_position(3, 0, 2, [2., 0., 0.]).unwrap(),
        [12.2, 1., 0.],
    );
}

#[test]
fn maximum_input_mix_and_skinning_allocate_nothing_after_admission() {
    let mut player = AnimationPlayer::new(&model()).unwrap();
    let mut set = [samples()[0]; MAX_POSE_SAMPLES];
    for (i, item) in set.iter_mut().enumerate() {
        *item = samples()[i % 4];
    }
    let mut palette = [glam::DMat4::IDENTITY; 2];
    let (_, count) = alloc::measured(|| {
        for i in 0..1024 {
            set[0].sample.seconds = i as f64 / 512.;
            player.sample_weighted(&set).unwrap();
            player.mesh_local_palette(4, &mut palette).unwrap();
            std::hint::black_box(
                player
                    .skin_pose(3, 0)
                    .unwrap()
                    .deform_vertex(0, [1.; 12])
                    .unwrap(),
            );
        }
    });
    assert_eq!(count, 0);
}

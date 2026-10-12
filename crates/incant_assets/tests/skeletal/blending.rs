use super::*;
use incant_assets_runtime::{CookedModel, animation::ClipSample};

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
fn sample(clip: usize, seconds: f64) -> ClipSample {
    ClipSample {
        clip: Some(clip),
        seconds,
    }
}

#[test]
fn crossfade_evaluates_local_trs_then_hierarchy_and_skinning_without_sources() {
    let mut model = model();
    model.animation.as_mut().unwrap().rest[3] = None;
    let mut player = AnimationPlayer::new(&model).unwrap();
    // Move the first joint by two units, rotate the child through 180 degrees.
    // Halfway is a one-unit translation and a 90 degree child rotation. Mixing
    // final matrices instead would collapse the rotated vertex onto the pivot.
    player
        .sample_blend(sample(0, 2.), sample(1, 2.), 0.5)
        .unwrap();
    near(
        player.skin_position(3, 0, 2, [2., 0., 0.]).unwrap(),
        [13., 2., 0.],
    );
    near(
        player.skin_position(4, 0, 2, [2., 0., 0.]).unwrap(),
        [13., 2., 0.],
    );
    near(
        player.globals()[1].w_axis.truncate().to_array(),
        [12., 0., 0.],
    );
    near(
        player.globals()[2].w_axis.truncate().to_array(),
        [12., 1., 0.],
    );
    // Static mesh instance matrices survive, even though skinning ignores them.
    near(
        player.locals()[3].w_axis.truncate().to_array(),
        [100., 0., 0.],
    );
}

#[test]
fn overflow_during_hierarchy_composition_cannot_publish_a_partial_blend() {
    use incant_assets_runtime::{
        ModelNode,
        animation::{Channel, NodeTrs},
    };
    let mut model = model();
    let first = model.metadata.nodes.len();
    model.metadata.nodes[0].children.push(first);
    let data = model.animation.as_mut().unwrap();
    for i in 0..20 {
        let node = first + i;
        model.metadata.nodes.push(ModelNode {
            name: None,
            children: if i == 19 { vec![] } else { vec![node + 1] },
            meshes: vec![],
            transform: glam::Mat4::IDENTITY.to_cols_array_2d(),
        });
        data.rest.push(Some(NodeTrs {
            translation: [0.; 3],
            rotation: [0., 0., 0., 1.],
            scale: [1.; 3],
        }));
        data.node_skins.push(None);
        data.clips[0].channels.push(Channel {
            node,
            property: Property::Scale,
            interpolation: Interpolation::Linear,
            times: vec![0., 1.],
            values: vec![[1., 1., 1., 0.], [1e30, 1e30, 1e30, 0.]],
        });
    }
    let mut player = AnimationPlayer::new(&model).unwrap();
    player.sample(Some(1), 0.5).unwrap();
    let locals = player.locals().to_vec();
    let globals = player.globals().to_vec();
    let joints = player.joint_palette(0).unwrap().to_vec();
    assert!(
        player
            .sample_blend(sample(0, 1.), ClipSample::REST, 0.5)
            .is_err()
    );
    assert_eq!(player.locals(), locals);
    assert_eq!(player.globals(), globals);
    assert_eq!(player.joint_palette(0).unwrap(), joints);
    use incant_assets_runtime::animation::WeightedClipSample;
    assert!(
        player
            .sample_weighted(&[
                WeightedClipSample {
                    sample: sample(0, 1.),
                    weight: 1.
                },
                WeightedClipSample {
                    sample: ClipSample::REST,
                    weight: 1.
                },
                WeightedClipSample {
                    sample: sample(1, 1.),
                    weight: 1.
                },
            ])
            .is_err()
    );
    assert_eq!(player.locals(), locals);
    assert_eq!(player.globals(), globals);
    assert_eq!(player.joint_palette(0).unwrap(), joints);
    use incant_assets_runtime::animation::PoseSource;
    let overflow = [WeightedClipSample {
        sample: sample(0, 1.),
        weight: 1.,
    }];
    for (from, to) in [
        (
            PoseSource::Weighted(&overflow),
            PoseSource::Clip(ClipSample::REST),
        ),
        (
            PoseSource::Clip(ClipSample::REST),
            PoseSource::Weighted(&overflow),
        ),
    ] {
        assert!(player.sample_pose_crossfade(from, to, 0.5).is_err());
        assert_eq!(player.locals(), locals);
        assert_eq!(player.globals(), globals);
        assert_eq!(player.joint_palette(0).unwrap(), joints);
    }
    player
        .sample_blend(sample(0, 0.), sample(1, 2.), 0.5)
        .unwrap();
    assert_ne!(player.globals(), globals);
}

#[test]
fn endpoints_are_exact_and_missing_channels_use_rest_not_the_previous_pose() {
    let model = model();
    let mut player = AnimationPlayer::new(&model).unwrap();
    let mut expected = AnimationPlayer::new(&model).unwrap();
    for from in [
        ClipSample::REST,
        sample(0, 1.1),
        sample(1, -4.),
        sample(2, 2.),
    ] {
        for to in [
            ClipSample::REST,
            sample(0, 7.),
            sample(1, 0.9),
            sample(2, 1.),
        ] {
            for (weight, endpoint) in [(0., from), (1., to)] {
                expected.sample(endpoint.clip, endpoint.seconds).unwrap();
                player.sample_blend(from, to, weight).unwrap();
                assert_eq!(player.locals(), expected.locals());
                assert_eq!(player.globals(), expected.globals());
                assert_eq!(
                    player.joint_palette(0).unwrap(),
                    expected.joint_palette(0).unwrap()
                );
            }
        }
    }
    // Scale uses STEP; the two clocks must be sampled separately before blending.
    player
        .sample_blend(sample(2, 1.99), sample(2, 2.), 0.25)
        .unwrap();
    near(
        player.skin_position(3, 0, 2, [2., 0., 0.]).unwrap(),
        [11.5, 0., 0.],
    );
    player
        .sample_blend(ClipSample::REST, sample(0, 2.), 0.25)
        .unwrap();
    near(
        player.skin_position(3, 0, 2, [2., 0., 0.]).unwrap(),
        [12.5, 0., 0.],
    );
}

#[test]
fn antipodal_quaternions_take_the_same_short_arc() {
    let mut model = model();
    let animation = model.animation.as_mut().unwrap();
    let positive = [
        0.,
        0.,
        std::f32::consts::FRAC_1_SQRT_2,
        std::f32::consts::FRAC_1_SQRT_2,
    ];
    animation.clips[1].channels[0].values.fill(positive);
    let mut other = animation.clips[1].clone();
    other.channels[0].values.fill(positive.map(|v| -v));
    animation.clips.push(other);
    let mut player = AnimationPlayer::new(&model).unwrap();
    for weight in [0., 0.01, 0.25, 0.5, 0.9, 1.] {
        player
            .sample_blend(sample(1, 0.2), sample(3, 1.7), weight)
            .unwrap();
        near(
            player.skin_position(3, 0, 2, [2., 0., 0.]).unwrap(),
            [12., 2., 0.],
        );
    }
}

#[test]
fn invalid_blends_preserve_all_published_pose_buffers_and_recover() {
    let mut player = AnimationPlayer::new(&model()).unwrap();
    player
        .sample_blend(sample(0, 0.5), sample(1, 1.), 0.5)
        .unwrap();
    let locals = player.locals().to_vec();
    let globals = player.globals().to_vec();
    let joints = player.joint_palette(0).unwrap().to_vec();
    for (from, to, weight) in [
        (sample(99, 0.), sample(0, 0.), 1.),
        (sample(0, 0.), sample(99, 0.), 0.),
        (sample(0, f64::NAN), ClipSample::REST, 1.),
        (ClipSample::REST, sample(0, f64::INFINITY), 0.),
        (ClipSample::REST, ClipSample::REST, f64::NAN),
        (ClipSample::REST, ClipSample::REST, f64::INFINITY),
        (ClipSample::REST, ClipSample::REST, -0.01),
        (ClipSample::REST, ClipSample::REST, 1.01),
    ] {
        assert!(player.sample_blend(from, to, weight).is_err());
        assert_eq!(player.locals(), locals);
        assert_eq!(player.globals(), globals);
        assert_eq!(player.joint_palette(0).unwrap(), joints);
    }
    player
        .sample_blend(sample(1, 1.), ClipSample::REST, 0.5)
        .unwrap();
    assert_ne!(player.globals(), globals);
}

#[test]
fn repeated_crossfades_and_deformation_allocate_nothing() {
    let mut player = AnimationPlayer::new(&model()).unwrap();
    let mut palette = [glam::DMat4::IDENTITY; 2];
    let (_, count) = alloc::measured(|| {
        for i in 0..1024 {
            player
                .sample_blend(
                    sample(0, i as f64 / 128.),
                    sample(1, i as f64 / 256.),
                    (i % 101) as f64 / 100.,
                )
                .unwrap();
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

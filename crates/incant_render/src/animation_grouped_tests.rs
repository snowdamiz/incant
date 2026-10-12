use super::*;
use incant_assets_runtime::animation::{ClipSample, PoseSource, WeightedClipSample};
use incant_types::{AnimationPoseSamples, AnimationSample, WeightedAnimationSample};

#[test]
fn compound_consumer_preserves_inner_interpolation_endpoints_and_inactive_admission() {
    let dir = tempfile::tempdir().unwrap();
    let source = source(dir.path());
    std::fs::remove_file(dir.path().join("model.gltf")).unwrap();
    std::fs::remove_file(dir.path().join("model.bin")).unwrap();
    std::fs::remove_dir_all(dir.path().join("cache")).unwrap();
    let clip = AnimationSample {
        clip: Some(0),
        seconds: 1.0,
    };
    let pair = AnimationPoseSamples::spherical_pair(AnimationSample::REST, clip, 0.25).unwrap();
    let mean = AnimationPoseSamples::new(&[
        WeightedAnimationSample {
            sample: clip,
            weight: 0.75,
        },
        WeightedAnimationSample {
            sample: AnimationSample::REST,
            weight: 0.25,
        },
    ])
    .unwrap();
    let mut binding = node(source.clone(), "compound", 1.0).mesh.unwrap();
    let mut player = AnimationPlayer::new(model(&source)).unwrap();
    let mut oracle = AnimationPlayer::new(model(&source)).unwrap();
    let weighted = [
        WeightedClipSample {
            sample: ClipSample {
                clip: Some(0),
                seconds: 1.0,
            },
            weight: 0.75,
        },
        WeightedClipSample {
            sample: ClipSample::REST,
            weight: 0.25,
        },
    ];
    for weight in [0.0, 0.25, 0.5, 1.0] {
        let pose = AnimationPoseSamples::crossfade(&pair, &mean, weight).unwrap();
        binding.animation_pose = Some(pose);
        binding.animation = Some(pose.representative());
        sample(&mut player, &binding).unwrap();
        oracle
            .sample_pose_crossfade(
                PoseSource::SphericalPair {
                    from: ClipSample::REST,
                    to: weighted[0].sample,
                    weight: 0.25,
                },
                PoseSource::Weighted(&weighted),
                weight,
            )
            .unwrap();
        assert_eq!(player.globals(), oracle.globals());
        let vertex = model(&source).meshes[0].vertices[0];
        let position = player
            .skin_position(3, 0, 0, [vertex[0], vertex[1], vertex[2]])
            .unwrap();
        assert!((position[0] - (11.25 + 0.5 * weight)).abs() < 1e-6);
    }
    let allocations = crate::retained_alloc::measured(|| {
        for _ in 0..128 {
            sample(&mut player, &binding).unwrap();
        }
    });
    assert_eq!(allocations, 0);
    let previous = player.globals().to_vec();
    // A structurally valid descriptor can reference a clip absent from this
    // particular model; inactive outer endpoints must still reject it.
    let invalid = AnimationPoseSamples::clip(AnimationSample {
        clip: Some(250),
        seconds: 0.0,
    })
    .unwrap();
    for (from, to, weight) in [(&invalid, &mean, 1.0), (&pair, &invalid, 0.0)] {
        let pose = AnimationPoseSamples::crossfade(from, to, weight).unwrap();
        binding.animation = Some(pose.representative());
        binding.animation_pose = Some(pose);
        assert!(sample(&mut player, &binding).is_err());
        assert_eq!(player.globals(), previous);
    }
}

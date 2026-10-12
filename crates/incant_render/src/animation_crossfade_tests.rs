use super::*;
use incant_types::{AnimationBlendSample, AnimationSample};

fn blend(input: &mut MeshInput, weight: f64) {
    input.animation = Some(AnimationSample {
        clip: Some(1),
        seconds: 1.,
    });
    input.animation_blend = Some(AnimationBlendSample {
        sample: AnimationSample {
            clip: Some(0),
            seconds: 1.,
        },
        weight,
    });
}

#[test]
fn canonical_blend_matches_independent_trs_skin_oracle_without_source_or_allocations() {
    let dir = tempfile::tempdir().unwrap();
    let source = source(dir.path());
    let mut node = node(Arc::clone(&source), "blend", 0.);
    dir.close().unwrap();
    let model = model(&source);
    let mut player = AnimationPlayer::new(model).unwrap();
    let mut vertices = vec![[0.; 12]; model.meshes[0].vertices.len()];
    for weight in [0., 0.125, 0.5, 0.875, 1.] {
        let input = node.mesh.as_mut().unwrap();
        blend(input, weight);
        let allocations = crate::retained_alloc::measured(|| {
            for _ in 0..64 {
                sample(&mut player, input).unwrap();
                Geometry::fill(&player, 3, 0, &model.meshes[0].vertices, &mut vertices).unwrap();
            }
        });
        assert_eq!(allocations, 0);
        // Move's joint-1 translation blends to rest while rotate's joint-2
        // rotation blends from identity. This analytic oracle does not call
        // the animation sampler, palette or skin_position reference.
        let translation = 1. - weight;
        let (sin, cos) = (std::f64::consts::FRAC_PI_2 * weight).sin_cos();
        for (source, actual) in model.meshes[0].vertices.iter().zip(&vertices) {
            let [x, y] = [f64::from(source[0]), f64::from(source[1])];
            let first = [10. + translation + x, y];
            let second = [
                11. + translation + cos * (x - 1.) - sin * (y - 1.),
                1. + sin * (x - 1.) + cos * (y - 1.),
            ];
            let joint1 = if x == 2. {
                1.
            } else if y == 1. {
                0.75
            } else {
                0.
            };
            for axis in 0..2 {
                let expected = first[axis] * (1. - joint1) + second[axis] * joint1;
                assert!((f64::from(actual[axis]) - expected).abs() < 2e-6);
            }
            assert_eq!(actual[2], 0.);
            assert_eq!(&actual[6..8], &source[6..8]);
        }
        // The second mesh uses the same skin and a reflected mesh transform;
        // glTF model-space skinning ignores that mesh-node transform.
        let mut other = vertices.clone();
        Geometry::fill(&player, 4, 0, &model.meshes[0].vertices, &mut other).unwrap();
        assert_eq!(other, vertices);
    }
    // Fresh canonical resolution and the retained CPU sampler use the same
    // validated input, including a REST destination selected by absent primary.
    node.mesh.as_mut().unwrap().animation = None;
    let input = SceneInput {
        nodes: vec![node],
        ..Default::default()
    };
    let resolved = crate::scene::resolve(&input).unwrap();
    let rest = AnimationPlayer::new(model).unwrap();
    assert_eq!(resolved.animations[0].player.globals(), rest.globals());
}

#[test]
fn both_samples_and_weight_are_checked_at_endpoints_before_pose_publication() {
    let dir = tempfile::tempdir().unwrap();
    let source = source(dir.path());
    let mut node = node(Arc::clone(&source), "blend", 0.);
    let mut player = AnimationPlayer::new(model(&source)).unwrap();
    let input = node.mesh.as_mut().unwrap();
    blend(input, 0.5);
    sample(&mut player, input).unwrap();
    let locals = player.locals().to_vec();
    let globals = player.globals().to_vec();
    let palette = player.joint_palette(0).unwrap().to_vec();
    for invalid in [
        AnimationSample {
            clip: Some(u16::MAX),
            seconds: 0.,
        },
        AnimationSample {
            clip: Some(0),
            seconds: -1.,
        },
        AnimationSample {
            clip: Some(0),
            seconds: f64::NAN,
        },
        AnimationSample {
            clip: Some(0),
            seconds: f64::INFINITY,
        },
        AnimationSample {
            clip: Some(0),
            seconds: 2.00001,
        },
        AnimationSample {
            clip: None,
            seconds: 1.,
        },
    ] {
        for outgoing in [false, true] {
            blend(input, if outgoing { 1. } else { 0. });
            if outgoing {
                input.animation_blend.as_mut().unwrap().sample = invalid;
            } else {
                input.animation = Some(invalid);
            }
            assert!(sample(&mut player, input).is_err());
            assert_eq!(player.locals(), locals);
            assert_eq!(player.globals(), globals);
            assert_eq!(player.joint_palette(0).unwrap(), palette);
        }
    }
    for invalid in [f64::NAN, f64::INFINITY, -0.01, 1.00001] {
        blend(input, invalid);
        assert!(sample(&mut player, input).is_err());
        assert_eq!(player.globals(), globals);
    }
    blend(input, 0.75);
    sample(&mut player, input).unwrap();
    assert_ne!(player.globals(), globals);
}

#[test]
fn outgoing_only_sample_cannot_silently_bind_a_static_model() {
    let dir = tempfile::tempdir().unwrap();
    let asset = crate::test_support::model(dir.path(), 0.);
    let source = Arc::new(
        RuntimeAsset::load_model(
            "static",
            0,
            &dir.path().join(".incant/cache/models"),
            &asset.sha256,
        )
        .unwrap(),
    );
    let mut node = node(source, "static", 0.);
    let input = node.mesh.as_mut().unwrap();
    input.animation = None;
    input.animation_blend = Some(AnimationBlendSample {
        sample: AnimationSample::REST,
        weight: 1.,
    });
    assert!(matches!(
        crate::scene::resolve(&SceneInput {
            nodes: vec![node],
            ..Default::default()
        }),
        Err(SceneError::Input(
            "animation sample requires animated model"
        ))
    ));
}

#[test]
fn reflected_blend_keeps_orientation_policy_and_singular_deformation_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let source = source(dir.path());
    let mut node = node(Arc::clone(&source), "scale", 2.);
    let input = node.mesh.as_mut().unwrap();
    input.animation.as_mut().unwrap().clip = Some(2);
    input.animation_blend = Some(AnimationBlendSample {
        sample: AnimationSample::REST,
        weight: 0.,
    });
    let model = model(&source);
    let mut player = AnimationPlayer::new(model).unwrap();
    let mut vertices = vec![[0.; 12]; model.meshes[0].vertices.len()];
    dir.close().unwrap();
    for weight in [0., 0.25, 0.75, 1.] {
        input.animation_blend.as_mut().unwrap().weight = weight;
        sample(&mut player, input).unwrap();
        let (_, _, mirrored) =
            Geometry::fill(&player, 3, 0, &model.meshes[0].vertices, &mut vertices).unwrap();
        assert_eq!(mirrored, weight > 0.5);
        for (source, actual) in model.meshes[0].vertices.iter().zip(&vertices) {
            let expected = [
                11. + (1. - 2. * weight) * (f64::from(source[0]) - 1.),
                (1. + weight) * f64::from(source[1]),
            ];
            for axis in 0..2 {
                assert!((f64::from(actual[axis]) - expected[axis]).abs() < 1e-6);
            }
        }
        input.animation_blend.as_mut().unwrap().weight = 0.5;
        // Finite singular pose matrices are allowed by the CPU sampler. The
        // canonical vertex-normal boundary refuses this drawable geometry;
        // retained rendering performs both operations in its private staging.
        sample(&mut player, input).unwrap();
        assert!(Geometry::fill(&player, 3, 0, &model.meshes[0].vertices, &mut vertices).is_err());
        input.animation_blend.as_mut().unwrap().weight = weight;
        sample(&mut player, input).unwrap();
        Geometry::fill(&player, 3, 0, &model.meshes[0].vertices, &mut vertices).unwrap();
    }
}

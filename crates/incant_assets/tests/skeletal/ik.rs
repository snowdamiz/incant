use super::*;
use glam::{DMat4, DVec3};
use incant_assets_runtime::{
    CookedModel,
    animation::{ClipSample, MAX_TWO_BONE_IK, TwoBoneIk, TwoBoneIkError, WeightedClipSample},
};

fn model(change: impl FnOnce(&mut serde_json::Value)) -> CookedModel {
    let dir = tempfile::tempdir().unwrap();
    let mut doc = fixture::fixture(dir.path());
    change(&mut doc);
    fixture::write(dir.path(), &doc);
    let cache = dir.path().join("cache");
    let cooked = cook_gltf(dir.path(), Path::new("model.gltf"), &cache).unwrap();
    let key = cooked.metadata.fingerprint.clone();
    drop(cooked);
    fs::remove_file(dir.path().join("model.gltf")).unwrap();
    fs::remove_file(dir.path().join("model.bin")).unwrap();
    incant_assets_runtime::load_model(&cache, &key).unwrap()
}
fn constraint() -> TwoBoneIk {
    TwoBoneIk {
        root: 0,
        middle: 1,
        tip: 2,
        target: [11., 1., 0.],
        pole: [10., 1., 0.],
        weight: 1.,
    }
}
fn pos(p: &AnimationPlayer, node: usize) -> [f64; 3] {
    p.globals()[node].w_axis.truncate().to_array()
}
fn snapshot(p: &AnimationPlayer) -> (Vec<DMat4>, Vec<DMat4>, Vec<DMat4>) {
    (
        p.locals().to_vec(),
        p.globals().to_vec(),
        p.joint_palette(0).unwrap().to_vec(),
    )
}
fn lengths(p: &AnimationPlayer) -> [f64; 2] {
    [
        (DVec3::from_array(pos(p, 1)) - DVec3::from_array(pos(p, 0))).length(),
        (DVec3::from_array(pos(p, 2)) - DVec3::from_array(pos(p, 1))).length(),
    ]
}
#[test]
fn cooked_source_deleted_ik_updates_both_skin_instances_and_preserves_other_locals() {
    let mut p = AnimationPlayer::new(&model(|_| {})).unwrap();
    let initial = snapshot(&p);
    p.apply_two_bone_ik(&[constraint()]).unwrap();
    near(pos(&p, 1), [10., 1., 0.]);
    near(pos(&p, 2), [11., 1., 0.]);
    assert_eq!(&p.locals()[2..], &initial.0[2..]);
    for i in 0..2 {
        assert_eq!(p.locals()[i].w_axis, initial.0[i].w_axis);
    }
    for node in [3, 4] {
        near(
            p.skin_position(node, 0, 2, [2., 0., 0.]).unwrap(),
            [10., 0., 0.],
        );
        let v = p
            .skin_pose(node, 0)
            .unwrap()
            .deform_vertex(2, [2., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 1.])
            .unwrap();
        near([v[0], v[1], v[2]].map(f64::from), [10., 0., 0.]);
        let mut palette = [DMat4::IDENTITY; 2];
        p.mesh_local_palette(node, &mut palette).unwrap();
        for (i, m) in palette.into_iter().enumerate() {
            assert!((p.globals()[node] * m).abs_diff_eq(p.joint_palette(0).unwrap()[i], 1e-9));
        }
    }
    p.sample(None, 0.).unwrap();
    assert_eq!(snapshot(&p), initial);
}
#[test]
fn weights_blend_local_rotations_zero_is_exact_and_half_is_not_tip_lerp() {
    let mut p = AnimationPlayer::new(&model(|_| {})).unwrap();
    let original = snapshot(&p);
    p.apply_two_bone_ik(&[TwoBoneIk {
        weight: 0.,
        ..constraint()
    }])
    .unwrap();
    assert_eq!(snapshot(&p), original);
    p.apply_two_bone_ik(&[TwoBoneIk {
        weight: 0.5,
        ..constraint()
    }])
    .unwrap();
    near(pos(&p, 1), [10. + 0.5f64.sqrt(), 0.5f64.sqrt(), 0.]);
    near(pos(&p, 2), [10., 0., 0.]);
    near([lengths(&p)[0], lengths(&p)[1], 0.], [1., 1., 0.]);
    p.sample(None, 0.).unwrap();
    p.apply_two_bone_ik(&[constraint()]).unwrap();
    near(pos(&p, 2), constraint().target);
}
#[test]
fn unreachable_targets_clamp_both_radii_equal_fold_and_unequal_coincidence() {
    for second in [1f64, 2., 0.5] {
        let mut p = AnimationPlayer::new(&model(|d| {
            d["nodes"][2]["translation"] = serde_json::json!([0., second, 0.])
        }))
        .unwrap();
        for (target, expect) in [
            ([10., 0., 0.], [10. + (1. - second).abs(), 0., 0.]),
            (
                [10.1, 0., 0.],
                [10. + 0.1f64.max((1. - second).abs()), 0., 0.],
            ),
            ([100., 0., 0.], [11. + second, 0., 0.]),
        ] {
            p.sample(None, 0.).unwrap();
            // At coincidence choose current tip direction, including unequal legs.
            let expected = if target == [10., 0., 0.] {
                (DVec3::X * 10. + DVec3::new(1., second, 0.).normalize() * (1. - second).abs())
                    .to_array()
            } else {
                expect
            };
            p.apply_two_bone_ik(&[TwoBoneIk {
                target,
                pole: [10., 0., 1.],
                ..constraint()
            }])
            .unwrap();
            near(pos(&p, 2), expected);
            near([lengths(&p)[0], lengths(&p)[1], 0.], [1., second, 0.]);
        }
    }
}
#[test]
fn coincident_tip_and_collinear_pole_are_deterministic_without_nan() {
    let mut p = AnimationPlayer::new(&model(|d| {
        d["nodes"][2]["translation"] = serde_json::json!([-1., 0., 0.])
    }))
    .unwrap();
    let mut prior = None;
    for _ in 0..16 {
        p.sample(None, 0.).unwrap();
        p.apply_two_bone_ik(&[TwoBoneIk {
            target: [10., 0., 0.],
            pole: [10., 0., 0.],
            ..constraint()
        }])
        .unwrap();
        near(pos(&p, 2), [10., 0., 0.]);
        near(pos(&p, 1), [10., 1., 0.]);
        if let Some(prior) = &prior {
            assert_eq!(&snapshot(&p), prior);
        }
        prior = Some(snapshot(&p));
    }
    // Current-bend fallback takes precedence over a synthetic axis.
    let mut p = AnimationPlayer::new(&model(|_| {})).unwrap();
    p.apply_two_bone_ik(&[TwoBoneIk {
        target: [11., 1., 0.],
        pole: [12., 2., 0.],
        ..constraint()
    }])
    .unwrap();
    near(pos(&p, 1), [11., 0., 0.]);
}
#[test]
fn rotated_uniform_ancestor_and_scaled_chain_preserve_local_offsets() {
    let mut p=AnimationPlayer::new(&model(|d|{
        d["nodes"].as_array_mut().unwrap().push(serde_json::json!({"translation":[5.,-2.,1.],"rotation":[0.,0.,0.70710677,0.70710677],"scale":[2.,2.,2.],"children":[0]}));
        d["scenes"][0]["nodes"]=serde_json::json!([5,3,4]);
    })).unwrap();
    let ancestor = p.globals()[5];
    let target = ancestor
        .transform_point3(DVec3::from_array(constraint().target))
        .to_array();
    let pole = ancestor
        .transform_point3(DVec3::from_array(constraint().pole))
        .to_array();
    let before = snapshot(&p);
    p.apply_two_bone_ik(&[TwoBoneIk {
        target,
        pole,
        ..constraint()
    }])
    .unwrap();
    near(pos(&p, 2), target);
    assert_eq!(p.locals()[5], before.0[5]);
    assert_eq!(p.locals()[0].w_axis, before.0[0].w_axis);
    assert_eq!(p.locals()[1].w_axis, before.0[1].w_axis);
    near([lengths(&p)[0], lengths(&p)[1], 0.], [2., 2., 0.]);
}
#[test]
fn malformed_degenerate_and_unsupported_batches_preserve_all_live_buffers_and_recover() {
    let mut p = AnimationPlayer::new(&model(|_| {})).unwrap();
    let prior = snapshot(&p);
    for bad in [
        TwoBoneIk {
            root: usize::MAX,
            ..constraint()
        },
        TwoBoneIk {
            middle: 2,
            ..constraint()
        },
        TwoBoneIk {
            tip: 1,
            ..constraint()
        },
        TwoBoneIk {
            weight: f64::NAN,
            ..constraint()
        },
        TwoBoneIk {
            weight: -1.,
            ..constraint()
        },
        TwoBoneIk {
            weight: 1.01,
            ..constraint()
        },
        TwoBoneIk {
            target: [f64::INFINITY; 3],
            ..constraint()
        },
        TwoBoneIk {
            pole: [f64::NAN; 3],
            ..constraint()
        },
    ] {
        assert!(p.apply_two_bone_ik(&[constraint(), bad]).is_err());
        assert_eq!(snapshot(&p), prior);
    }
    assert_eq!(
        p.apply_two_bone_ik(&[constraint(); MAX_TWO_BONE_IK + 1]),
        Err(TwoBoneIkError::Limit)
    );
    // The second solve fails geometrically after the first has modified staging.
    let late = TwoBoneIk {
        root: 1,
        middle: 2,
        tip: 5,
        ..constraint()
    };
    let mut p = AnimationPlayer::new(&model(|d| {
        d["nodes"][2]["children"] = serde_json::json!([5]);
        d["nodes"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"translation":[0,0,0]}));
    }))
    .unwrap();
    let prior = snapshot(&p);
    assert_eq!(
        p.apply_two_bone_ik(&[constraint(), late]),
        Err(TwoBoneIkError::Degenerate)
    );
    assert_eq!(snapshot(&p), prior);
    p.apply_two_bone_ik(&[constraint()]).unwrap();
    near(pos(&p, 2), constraint().target);
    for scale in [[2., 1., 1.], [-1., 1., 1.], [0., 0., 0.]] {
        let mut p = AnimationPlayer::new(&model(|d| {
            d["nodes"][0]["scale"] = serde_json::json!(scale)
        }))
        .unwrap();
        let prior = snapshot(&p);
        assert_eq!(
            p.apply_two_bone_ik(&[constraint()]),
            Err(TwoBoneIkError::UnsupportedTransform)
        );
        assert_eq!(snapshot(&p), prior);
    }
}
#[test]
fn existing_weighted_sampling_then_ik_is_ordered_and_has_no_per_frame_allocation() {
    let mut p = AnimationPlayer::new(&model(|_| {})).unwrap();
    let (_, allocations) = alloc::measured(|| {
        for i in 0..512 {
            p.sample_weighted(&[
                WeightedClipSample {
                    sample: ClipSample::REST,
                    weight: 1.,
                },
                WeightedClipSample {
                    sample: ClipSample {
                        clip: Some(0),
                        seconds: 0.5,
                    },
                    weight: 1.,
                },
            ])
            .unwrap();
            let c = TwoBoneIk {
                target: [11., 1., 0.],
                weight: (i % 17) as f64 / 16.,
                ..constraint()
            };
            p.apply_two_bone_ik(&[c, c]).unwrap();
            std::hint::black_box(
                p.skin_pose(3, 0)
                    .unwrap()
                    .deform_vertex(2, [2., 0., 0., 0., 0., 1., 0., 0., 1., 0., 0., 1.])
                    .unwrap(),
            );
            assert_eq!(
                p.apply_two_bone_ik(&[TwoBoneIk {
                    weight: f64::NAN,
                    ..c
                }]),
                Err(TwoBoneIkError::Input)
            );
        }
    });
    assert_eq!(allocations, 0);
    near([lengths(&p)[0], lengths(&p)[1], 0.], [1.25, 1., 0.]);
}

#[test]
fn spatial_target_sweep_preserves_bones_and_clamped_tip_without_stretching() {
    let mut p = AnimationPlayer::new(&model(|_| {})).unwrap();
    for x in -2..=2 {
        for y in -2..=2 {
            for z in -2..=2 {
                let direction = DVec3::new(f64::from(x), f64::from(y), f64::from(z));
                if direction == DVec3::ZERO {
                    continue;
                }
                let direction = direction.normalize();
                for radius in [0.2f64, 1., 1.9, 2., 4.] {
                    p.sample(None, 0.).unwrap();
                    let target = DVec3::new(10., 0., 0.) + direction * radius;
                    p.apply_two_bone_ik(&[TwoBoneIk {
                        target: target.to_array(),
                        pole: [10., 0., 5.],
                        ..constraint()
                    }])
                    .unwrap();
                    near(
                        pos(&p, 2),
                        (DVec3::new(10., 0., 0.) + direction * radius.min(2.)).to_array(),
                    );
                    near([lengths(&p)[0], lengths(&p)[1], 0.], [1., 1., 0.]);
                }
            }
        }
    }
}

#[test]
fn ordered_overlapping_constraints_equal_separate_solves_and_half_turn_ties_are_stable() {
    let m = model(|_| {});
    let mut batch = AnimationPlayer::new(&m).unwrap();
    let mut separate = AnimationPlayer::new(&m).unwrap();
    let a = TwoBoneIk {
        weight: 0.25,
        ..constraint()
    };
    let b = TwoBoneIk {
        target: [10., 0., 1.5],
        pole: [10., 2., 0.],
        weight: 0.75,
        ..constraint()
    };
    batch.apply_two_bone_ik(&[a, b]).unwrap();
    separate.apply_two_bone_ik(&[a]).unwrap();
    separate.apply_two_bone_ik(&[b]).unwrap();
    assert_eq!(snapshot(&batch), snapshot(&separate));
    for delta in [-1e-14, 0., 1e-14] {
        batch.sample(None, 0.).unwrap();
        batch
            .apply_two_bone_ik(&[TwoBoneIk {
                target: [11. + delta, 1., 0.],
                weight: 0.5,
                ..constraint()
            }])
            .unwrap();
        near(pos(&batch, 2), [10., 0., 0.]);
    }
}

#[test]
fn local_uniform_scales_fixed_nodes_and_sheared_ancestors_have_explicit_behavior() {
    let mut p = AnimationPlayer::new(&model(|d| {
        d["nodes"][0]["scale"] = serde_json::json!([2., 2., 2.]);
        d["nodes"][1]["scale"] = serde_json::json!([3., 3., 3.]);
    }))
    .unwrap();
    p.apply_two_bone_ik(&[TwoBoneIk {
        target: [14., 4., 0.],
        ..constraint()
    }])
    .unwrap();
    near(pos(&p, 2), [14., 4., 0.]);
    near([lengths(&p)[0], lengths(&p)[1], 0.], [2., 6., 0.]);
    for (i, scale) in [(0, 2.), (1, 3.)] {
        let m = p.locals()[i];
        near(
            [
                m.x_axis.truncate().length(),
                m.y_axis.truncate().length(),
                m.z_axis.truncate().length(),
            ],
            [scale; 3],
        );
    }
    let mut m = model(|_| {});
    m.animation.as_mut().unwrap().rest[0] = None;
    let mut p = AnimationPlayer::new(&m).unwrap();
    assert_eq!(
        p.apply_two_bone_ik(&[constraint()]),
        Err(TwoBoneIkError::Chain)
    );
    let mut p = AnimationPlayer::new(&model(|d| {
        d["nodes"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"scale":[2,1,1],"children":[0]}));
        d["scenes"][0]["nodes"] = serde_json::json!([5, 3, 4]);
        d["nodes"][0]["rotation"] = serde_json::json!([0., 0., 0.38268343, 0.92387953]);
    }))
    .unwrap();
    let before = snapshot(&p);
    assert_eq!(
        p.apply_two_bone_ik(&[constraint()]),
        Err(TwoBoneIkError::UnsupportedTransform)
    );
    assert_eq!(snapshot(&p), before);
}

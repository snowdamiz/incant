//! Shared committed animation sampling; no second clock or GPU dependency.
use crate::{MeshInput, SceneError, SceneInput};
use incant_assets_runtime::animation::AnimationPlayer;

fn checked_sample(
    player: &AnimationPlayer,
    sample: incant_types::AnimationSample,
) -> std::result::Result<incant_assets_runtime::animation::ClipSample, SceneError> {
    if !sample.seconds.is_finite()
        || sample.seconds < 0.
        || (sample.clip.is_none() && sample.seconds != 0.)
    {
        return Err(SceneError::Input("invalid committed animation time"));
    }
    if let Some(clip) = sample.clip {
        let duration = player
            .clip_duration(usize::from(clip))
            .ok_or(SceneError::Input("animation clip index"))?;
        if sample.seconds > f64::from(duration) {
            return Err(SceneError::Input("animation sample exceeds clip duration"));
        }
    }
    Ok(incant_assets_runtime::animation::ClipSample {
        clip: sample.clip.map(usize::from),
        seconds: sample.seconds,
    })
}
pub fn sample_committed(
    player: &mut AnimationPlayer,
    input: &MeshInput,
) -> std::result::Result<(), SceneError> {
    use incant_assets_runtime::animation::{AnimationPoseError, PoseSource, TwoBoneIk};
    let mut constraints = [TwoBoneIk {
        root: 0,
        middle: 1,
        tip: 2,
        target: [0.; 3],
        pole: [0.; 3],
        weight: 0.,
    }; incant_types::MAX_ANIMATION_IK];
    if let Some(ik) = &input.animation_ik {
        if ik.is_empty() || ik.len() > incant_types::MAX_ANIMATION_IK {
            return Err(SceneError::Input("IK constraint count"));
        }
        for (i, (out, c)) in constraints.iter_mut().zip(ik.iter()).enumerate() {
            c.validate().map_err(SceneError::Input)?;
            if ik[..i].iter().any(|old| old.id == c.id) {
                return Err(SceneError::Input("duplicate IK constraint"));
            }
            *out = TwoBoneIk {
                root: usize::from(c.chain[0]),
                middle: usize::from(c.chain[1]),
                tip: usize::from(c.chain[2]),
                target: c.target,
                pole: c.pole,
                weight: c.weight,
            };
        }
    }
    let constraints = &constraints[..input.animation_ik.as_ref().map_or(0, |ik| ik.len())];
    let pose_error = |error| match error {
        AnimationPoseError::Sample(error) => SceneError::from(error),
        AnimationPoseError::Root => SceneError::Input("unsupported sampled root-removal binding"),
        AnimationPoseError::Ik(_) => SceneError::Input("unsupported or degenerate sampled IK pose"),
    };
    let primary = checked_sample(
        player,
        input
            .animation
            .unwrap_or(incant_types::AnimationSample::REST),
    )?;
    if let Some(pose) = input.animation_pose {
        if input.animation_blend.is_some() || input.animation != Some(pose.representative()) {
            return Err(SceneError::Input("inconsistent grouped animation binding"));
        }
        use incant_assets_runtime::animation::{ClipSample, WeightedClipSample};
        let mut samples = [WeightedClipSample {
            sample: ClipSample {
                clip: None,
                seconds: 0.0,
            },
            weight: 0.0,
        }; incant_types::MAX_ANIMATION_POSE_SAMPLES];
        for (out, entry) in samples.iter_mut().zip(pose.samples()) {
            *out = WeightedClipSample {
                sample: checked_sample(player, entry.sample)?,
                weight: entry.weight,
            };
        }
        use incant_types::PoseInterpolation;
        let first = pose.group(0).expect("retained pose group");
        fn source(kind: PoseInterpolation, samples: &[WeightedClipSample]) -> PoseSource<'_> {
            match kind {
                PoseInterpolation::Clip => PoseSource::Clip(samples[0].sample),
                PoseInterpolation::SphericalPair => PoseSource::SphericalPair {
                    from: samples[0].sample,
                    to: samples[1].sample,
                    weight: samples[1].weight,
                },
                PoseInterpolation::Weighted => PoseSource::Weighted(samples),
            }
        }
        let from = source(first.interpolation, &samples[..first.samples.len()]);
        if let Some(second) = pose.group(1) {
            let to = source(
                second.interpolation,
                &samples[first.samples.len()..pose.samples().len()],
            );
            player
                .sample_pose_pipeline(
                    from,
                    to,
                    pose.outer_weight().expect("two pose groups"),
                    input.animation_root,
                    constraints,
                )
                .map_err(pose_error)?;
        } else {
            player
                .sample_pose_pipeline(from, from, 0.0, input.animation_root, constraints)
                .map_err(pose_error)?;
        }
    } else if let Some(blend) = input.animation_blend {
        // Do not skip invalid outgoing samples at weight one or invalid primary
        // samples at zero. These are complete committed bindings, not hints.
        let outgoing = checked_sample(player, blend.sample)?;
        if !blend.weight.is_finite() || !(0. ..=1.).contains(&blend.weight) {
            return Err(SceneError::Input(
                "invalid committed animation blend weight",
            ));
        }
        player
            .sample_pose_pipeline(
                PoseSource::Clip(outgoing),
                PoseSource::Clip(primary),
                blend.weight,
                input.animation_root,
                constraints,
            )
            .map_err(pose_error)?;
    } else {
        player
            .sample_pose_pipeline(
                PoseSource::Clip(primary),
                PoseSource::Clip(primary),
                0.0,
                input.animation_root,
                constraints,
            )
            .map_err(pose_error)?;
    }

    Ok(())
}
pub fn validate_ik_capacity(input: &SceneInput) -> std::result::Result<(), SceneError> {
    let mut count = 0usize;
    for node in &input.nodes {
        if let Some(ik) = node.mesh.as_ref().and_then(|m| m.animation_ik.as_ref()) {
            if ik.is_empty() || ik.len() > incant_types::MAX_ANIMATION_IK {
                return Err(SceneError::Input("IK constraint count"));
            }
            count = count
                .checked_add(ik.len())
                .ok_or(SceneError::Input("IK aggregate capacity"))?;
            if count > incant_types::MAX_SCENE_ANIMATION_IK {
                return Err(SceneError::Input("IK aggregate capacity"));
            }
        }
    }
    Ok(())
}

/// Fill a retained skin payload and its one admitted primitive orientation.
pub fn skin_primitive(
    player: &AnimationPlayer,
    node: usize,
    primitive: usize,
    source: &[incant_assets_runtime::Vertex],
    out: &mut [incant_assets_runtime::Vertex],
) -> Result<(glam::Vec3, glam::Vec3, bool), SceneError> {
    use glam::Vec3;
    let pose = player.skin_pose(node, primitive)?;
    if pose.vertex_count() != source.len() || out.len() != source.len() {
        return Err(SceneError::Input("skin geometry length changed"));
    }
    let mut minimum = Vec3::splat(f32::INFINITY);
    let mut maximum = Vec3::splat(f32::NEG_INFINITY);
    let mut mirrored = None;
    for (i, (source, out)) in source.iter().zip(out).enumerate() {
        let (vertex, orientation) = pose.deform_vertex_oriented(i, *source)?;
        if mirrored.is_some_and(|prior| prior != orientation) {
            return Err(SceneError::Input(
                "mixed skin orientation within primitive is unsupported",
            ));
        }
        mirrored = Some(orientation);
        *out = vertex;
        let position = Vec3::new(out[0], out[1], out[2]);
        minimum = minimum.min(position);
        maximum = maximum.max(position);
    }
    Ok((minimum, maximum, mirrored.unwrap_or(false)))
}

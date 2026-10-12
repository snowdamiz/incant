//! Complete v5 pose groups: preserve inner SLERP/mean and independent outer fade.
use crate::BoundaryError as Error;
use incant_runtime::{AnimationSample, AnimationSnapshot, StableId};
use incant_wasm_abi::{self as abi, AnimationPoseKindV5 as Kind};

const _: () = {
    assert!(incant_runtime::MAX_MESH_ENTITIES == abi::ANIMATION_MAX_ROWS);
    assert!(incant_runtime::MAX_ANIMATION_GRAPH_INSTANCES == abi::ANIMATION_GRAPH_MAX_ROWS);
    assert!(incant_runtime::MAX_ANIMATION_POSE_SAMPLES == abi::ANIMATION_POSE_MAX_SAMPLES);
    assert!(
        abi::ANIMATION_POSE_MAX_SAMPLE_ROWS
            == 2 * abi::ANIMATION_MAX_ROWS
                + (abi::ANIMATION_POSE_MAX_SAMPLES - 2) * abi::ANIMATION_GRAPH_MAX_ROWS
    );
    assert!(abi::ANIMATION_V5_MAX_BYTES < abi::MEMORY);
};

pub(crate) fn encode(
    scene: StableId,
    rows: &mut [AnimationSnapshot],
    bytes: &mut Vec<u8>,
) -> Result<(), Error> {
    bytes.clear();
    if rows.len() > abi::ANIMATION_MAX_ROWS {
        return Err(Error::Limit("animation pose rows"));
    }
    let mut samples = 0usize;
    let mut weighted = 0usize;
    let mut compounds = 0usize;
    for row in rows.iter() {
        let count = if let Some(pose) = row.pose {
            compounds += usize::from(pose.outer_weight().is_some());

            if row.blend.is_some() || row.state.transition.is_some() {
                return Err(Error::Abi("ambiguous native animation pose"));
            }
            weighted += 1;
            pose.samples().len()
        } else if row.blend.is_some() {
            2
        } else {
            1
        };
        samples = samples
            .checked_add(count)
            .filter(|n| *n <= abi::ANIMATION_POSE_MAX_SAMPLE_ROWS)
            .ok_or(Error::Limit("animation pose samples"))?;
    }
    if weighted > abi::ANIMATION_GRAPH_MAX_ROWS {
        return Err(Error::Limit("animation weighted pose rows"));
    }
    // Only the new v5 caller pays for canonical ordering. Older cached snapshots
    // retain their original copied bytes; no ECS order is exposed by v5.
    rows.sort_unstable_by_key(|r| r.id);
    bytes.extend(
        abi::AnimationHeaderV5 {
            magic: *abi::ANIMATION_V5_MAGIC,
            scene: scene.0,
            count: rows.len() as u32,
            stride: abi::AnimationRowV4::SIZE as u32,
            sample_count: samples as u32,
            sample_stride: abi::AnimationPoseSample::SIZE as u32,
            compound_count: compounds as u32,
            compound_stride: abi::AnimationPoseGroups::SIZE as u32,
        }
        .encode(),
    );
    let mut offset = 0u32;
    for row in rows.iter() {
        let state = row.state;
        let transition = state.transition;
        let (kind, count) = if let Some(pose) = row.pose {
            (
                if pose.outer_weight().is_some() {
                    Kind::Compound
                } else {
                    match pose.group(0).expect("retained group").interpolation {
                        incant_types::PoseInterpolation::Clip => Kind::Single,
                        incant_types::PoseInterpolation::SphericalPair => Kind::SlerpParameter,
                        incant_types::PoseInterpolation::Weighted => Kind::WeightedQuaternionMean,
                    }
                },
                pose.samples().len(),
            )
        } else if row.blend.is_some() {
            (
                if transition.is_some() {
                    Kind::SlerpTransition
                } else {
                    Kind::SlerpParameter
                },
                2,
            )
        } else {
            (Kind::Single, 1)
        };
        bytes.extend(
            abi::AnimationRowV4 {
                id: row.id.0,
                clip: state.sample.clip.map_or(u32::MAX, u32::from),
                flags: u32::from(state.playing)
                    | (u32::from(state.looping) << 1)
                    | (u32::from(state.completed) << 2),
                seconds: state.sample.seconds,
                speed: state.speed,
                source_clip: transition.map_or(0, |t| t.source.clip.map_or(u32::MAX, u32::from)),
                transition_flags: transition.map_or(0, |t| {
                    1 | (u32::from(t.playing) << 1) | (u32::from(t.looping) << 2)
                }),
                source_seconds: transition.map_or(0.0, |t| t.source.seconds),
                source_speed: transition.map_or(0.0, |t| t.speed),
                elapsed: transition.map_or(0.0, |t| t.elapsed),
                duration: transition.map_or(0.0, |t| t.duration),
                pose_kind: kind as u32,
                sample_offset: offset,
                sample_count: count as u32,
                reserved: 0,
            }
            .encode(),
        );
        offset += count as u32;
    }
    for row in rows.iter() {
        if let Some(pose) = row.pose {
            for entry in pose.samples() {
                sample(bytes, entry.sample, entry.weight);
            }
        } else if let Some(blend) = row.blend {
            sample(bytes, blend.sample, 1.0 - blend.weight);
            sample(bytes, row.state.sample, blend.weight);
        } else {
            sample(bytes, row.state.sample, 1.0);
        }
    }
    for row in rows.iter() {
        if let Some(pose) = row.pose.filter(|p| p.outer_weight().is_some()) {
            let from = pose.group(0).expect("retained source");
            let to = pose.group(1).expect("retained destination");
            bytes.extend(
                abi::AnimationPoseGroups {
                    entity: row.id.0,
                    split: from.samples.len() as u32,
                    source_kind: from.interpolation as u32,
                    destination_kind: to.interpolation as u32,
                    reserved: 0,
                    weight: pose.outer_weight().expect("retained fade"),
                }
                .encode(),
            );
        }
    }
    debug_assert!(bytes.len() <= abi::ANIMATION_V5_MAX_BYTES);
    Ok(())
}
fn sample(bytes: &mut Vec<u8>, sample: AnimationSample, weight: f64) {
    bytes.extend(
        abi::AnimationPoseSample {
            clip: sample.clip.map_or(u32::MAX, u32::from),
            reserved: 0,
            seconds: sample.seconds,
            weight,
        }
        .encode(),
    );
}

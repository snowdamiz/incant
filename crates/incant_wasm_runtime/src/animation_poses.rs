//! v4 copies complete native pose contributions without changing their sampling
//! semantics. This is a lazy callback snapshot, never a model/graph evaluator.
use crate::BoundaryError as Error;
use incant_runtime::{AnimationSample, AnimationSnapshot, StableId};
use incant_wasm_abi::{self as abi, AnimationPoseKind as Kind};

const _: () = {
    assert!(incant_runtime::MAX_MESH_ENTITIES == abi::ANIMATION_MAX_ROWS);
    assert!(incant_runtime::MAX_ANIMATION_GRAPH_INSTANCES == abi::ANIMATION_GRAPH_MAX_ROWS);
    assert!(incant_runtime::MAX_ANIMATION_POSE_SAMPLES == abi::ANIMATION_POSE_MAX_SAMPLES);
    assert!(
        abi::ANIMATION_POSE_MAX_SAMPLE_ROWS
            == 2 * abi::ANIMATION_MAX_ROWS
                + (abi::ANIMATION_POSE_MAX_SAMPLES - 2) * abi::ANIMATION_GRAPH_MAX_ROWS
    );
    assert!(abi::ANIMATION_V4_MAX_BYTES < abi::MEMORY);
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
    for row in rows.iter() {
        let count = if let Some(pose) = row.pose {
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
    // Only the new v4 caller pays for canonical ordering. Older cached snapshots
    // retain their original copied bytes; no ECS order is exposed by v4.
    rows.sort_unstable_by_key(|r| r.id);
    bytes.extend(
        abi::AnimationHeaderV4 {
            magic: *abi::ANIMATION_V4_MAGIC,
            scene: scene.0,
            count: rows.len() as u32,
            stride: abi::AnimationRowV4::SIZE as u32,
            sample_count: samples as u32,
            sample_stride: abi::AnimationPoseSample::SIZE as u32,
        }
        .encode(),
    );
    let mut offset = 0u32;
    for row in rows.iter() {
        let state = row.state;
        let transition = state.transition;
        let (kind, count) = if let Some(pose) = row.pose {
            (Kind::WeightedQuaternionMean, pose.samples().len())
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
    debug_assert!(bytes.len() <= abi::ANIMATION_V4_MAX_BYTES);
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn synthetic_sixteen_sample_native_value_encodes_every_contribution() {
        // Capacity coverage only: real tree-state fades are a separate runtime
        // integration requirement, not established by this constructed value.
        let values: Vec<_> = (0..16)
            .map(|n| incant_runtime::WeightedAnimationSample {
                sample: AnimationSample {
                    clip: Some(n % 8),
                    seconds: if n < 8 { 0.25 } else { 0.75 },
                },
                weight: 1.0,
            })
            .collect();
        let pose = incant_runtime::AnimationPoseSamples::new(&values).unwrap();
        let mut rows = [AnimationSnapshot {
            id: StableId([1; 16]),
            blend: None,
            pose: Some(pose),
            state: incant_runtime::AnimationState {
                sample: values[0].sample,
                speed: 1.0,
                playing: true,
                looping: true,
                completed: false,
                transition: None,
            },
        }];
        let mut bytes = Vec::new();
        encode(StableId([9; 16]), &mut rows, &mut bytes).unwrap();
        let snapshot = incant_module::AnimationPoseSnapshot::parse(&bytes).unwrap();
        let row = snapshot.iter().next().unwrap();
        assert_eq!(row.samples().len(), 16);
        for (actual, native) in row.samples().zip(pose.samples()) {
            assert_eq!(actual.clip, native.sample.clip);
            assert_eq!(actual.seconds, native.sample.seconds);
            assert_eq!(actual.weight.to_bits(), native.weight.to_bits());
        }
    }
}

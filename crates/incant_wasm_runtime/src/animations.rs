//! Copied skeletal playback observations; native clocks and assets stay private.
use crate::BoundaryError as Error;
use incant_runtime::{AnimationControl, AnimationSample, AnimationSnapshot, NativeWorld};
use incant_wasm_abi as abi;

const _: () = {
    assert!(incant_runtime::MAX_MESH_ENTITIES == abi::ANIMATION_MAX_ROWS);
    assert!(incant_runtime::MAX_ANIMATION_CLIPS == abi::ANIMATION_MAX_CLIPS);
    assert!(incant_types::MAX_ANIMATION_SPEED == abi::ANIMATION_MAX_SPEED as f64);
    assert!(
        incant_runtime::MAX_ANIMATION_TRANSITION_SECONDS
            == abi::ANIMATION_MAX_TRANSITION_SECONDS as f64
    );
};

pub(crate) fn encode(
    world: &mut NativeWorld,
    rows: &mut Vec<AnimationSnapshot>,
    bytes: &mut Vec<u8>,
    v2: &mut Vec<u8>,
) -> Result<(), Error> {
    world.snapshot_animations(rows)?;
    bytes.clear();
    v2.clear();
    // v2 describes timed transitions, not parameter-driven graph blends. Keep
    // both unavailable until a representable committed snapshot exists; callers
    // which do not request these imports can still use the scene normally.
    if rows
        .iter()
        .any(|row| row.pose.is_some() || (row.blend.is_some() && row.state.transition.is_none()))
    {
        return Ok(());
    }
    v2.extend_from_slice(
        &abi::AnimationHeader {
            magic: *abi::ANIMATION_V2_MAGIC,
            count: rows.len() as u32,
            stride: abi::AnimationRowV2::SIZE as u32,
        }
        .encode(),
    );
    // A legacy row cannot describe an active blend. Do not publish a projection
    // which misrepresents it as a single pose; length/read both refuse below.
    let legacy = rows.iter().all(|row| row.blend.is_none());
    if legacy {
        bytes.extend_from_slice(
            &abi::AnimationHeader {
                magic: *abi::ANIMATION_MAGIC,
                count: rows.len() as u32,
                stride: abi::AnimationRow::SIZE as u32,
            }
            .encode(),
        );
    }
    for row in rows {
        let state = row.state;
        let prefix = abi::AnimationRow {
            id: row.id.0,
            clip: state.sample.clip.map_or(u32::MAX, u32::from),
            flags: u32::from(state.playing)
                | (u32::from(state.looping) << 1)
                | (u32::from(state.completed) << 2),
            seconds: state.sample.seconds,
            speed: state.speed,
        };
        if legacy {
            bytes.extend_from_slice(&prefix.encode());
        }
        let transition = state.transition;
        v2.extend_from_slice(
            &abi::AnimationRowV2 {
                id: prefix.id,
                clip: prefix.clip,
                flags: prefix.flags,
                seconds: prefix.seconds,
                speed: prefix.speed,
                source_clip: transition.map_or(0, |t| t.source.clip.map_or(u32::MAX, u32::from)),
                transition_flags: transition.map_or(0, |t| {
                    1 | (u32::from(t.playing) << 1) | (u32::from(t.looping) << 2)
                }),
                source_seconds: transition.map_or(0., |t| t.source.seconds),
                source_speed: transition.map_or(0., |t| t.speed),
                elapsed: transition.map_or(0., |t| t.elapsed),
                duration: transition.map_or(0., |t| t.duration),
            }
            .encode(),
        );
    }
    debug_assert!(v2.len() <= abi::ANIMATION_V2_MAX_BYTES);
    debug_assert!(bytes.len() <= abi::ANIMATION_MAX_BYTES);
    Ok(())
}

pub(crate) fn command(
    bytes: &[u8; abi::AnimationCommand::SIZE],
) -> Result<AnimationControl, Error> {
    let row = abi::AnimationCommand::decode(bytes).ok_or(Error::Abi("animation command frame"))?;
    if !row.is_finite() {
        return Err(Error::Abi("animation command nonfinite value"));
    }
    let zero_value = row.value.to_bits() == 0;
    let zero_seconds = row.seconds.to_bits() == 0;
    Ok(match row.kind {
        0 if zero_value && row.seconds >= 0. => {
            let clip = match row.clip {
                u32::MAX if row.seconds == 0. => None,
                n if n < abi::ANIMATION_MAX_CLIPS as u32 => Some(n as u16),
                _ => return Err(Error::Abi("animation clip/rest sample")),
            };
            AnimationControl::Select(AnimationSample {
                clip,
                seconds: row.seconds,
            })
        }
        1 if row.clip == 0 && zero_value && row.seconds >= 0. => {
            AnimationControl::Seek(row.seconds)
        }
        2 | 3 if row.clip == 0 && zero_seconds && (row.value == 0. || row.value == 1.) => {
            if row.kind == 2 {
                AnimationControl::SetPlaying(row.value == 1.)
            } else {
                AnimationControl::SetLooping(row.value == 1.)
            }
        }
        4 if row.clip == 0
            && zero_seconds
            && row.value.abs() <= abi::ANIMATION_MAX_SPEED as f64 =>
        {
            AnimationControl::SetSpeed(row.value)
        }
        _ => return Err(Error::Abi("animation command tag/reserved field/value")),
    })
}

/// Encode only on first v3 access in this callback. Legacy-only modules do not
/// allocate a third complete animation byte buffer.
pub(crate) fn encode_v3(rows: &[AnimationSnapshot], bytes: &mut Vec<u8>) {
    bytes.clear();
    // The v3 two-pose frame cannot represent a weighted tree.
    if rows.iter().any(|row| row.pose.is_some()) {
        return;
    }
    bytes.extend(
        abi::AnimationHeader {
            magic: *abi::ANIMATION_V3_MAGIC,
            count: rows.len() as u32,
            stride: abi::AnimationRowV3::SIZE as u32,
        }
        .encode(),
    );
    for row in rows {
        let state = row.state;
        let transition = state.transition;
        let blend = row.blend;
        bytes.extend(
            abi::AnimationRowV3 {
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
                blend_kind: if blend.is_none() {
                    0
                } else if transition.is_some() {
                    1
                } else {
                    2
                },
                blend_source_clip: blend.map_or(0, |b| b.sample.clip.map_or(u32::MAX, u32::from)),
                blend_source_seconds: blend.map_or(0.0, |b| b.sample.seconds),
                blend_weight: blend.map_or(0.0, |b| b.weight),
            }
            .encode(),
        );
    }
    debug_assert!(bytes.len() <= abi::ANIMATION_V3_MAX_BYTES);
}

/// v2 preserves v1 controls and adds a crossfade with a copied target/duration.
pub(crate) fn command_v2(
    bytes: &[u8; abi::AnimationCommand::SIZE],
) -> Result<AnimationControl, Error> {
    let row = abi::AnimationCommand::decode(bytes).ok_or(Error::Abi("animation command frame"))?;
    if row.kind != 5 {
        return command(bytes);
    }
    if !row.is_finite()
        || row.seconds < 0.
        || !(0.0..=abi::ANIMATION_MAX_TRANSITION_SECONDS as f64).contains(&row.value)
    {
        return Err(Error::Abi("animation crossfade time/duration"));
    }
    let clip = match row.clip {
        u32::MAX if row.seconds == 0. => None,
        n if n < abi::ANIMATION_MAX_CLIPS as u32 => Some(n as u16),
        _ => return Err(Error::Abi("animation clip/rest sample")),
    };
    Ok(AnimationControl::Crossfade {
        target: AnimationSample {
            clip,
            seconds: row.seconds,
        },
        duration: row.value,
    })
}

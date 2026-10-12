//! Copied skeletal playback observations; native clocks and assets stay private.
use crate::BoundaryError as Error;
use incant_runtime::{AnimationControl, AnimationSample, AnimationSnapshot, NativeWorld};
use incant_wasm_abi as abi;

const _: () = {
    assert!(incant_runtime::MAX_MESH_ENTITIES == abi::ANIMATION_MAX_ROWS);
    assert!(incant_runtime::MAX_ANIMATION_CLIPS == abi::ANIMATION_MAX_CLIPS);
    assert!(incant_types::MAX_ANIMATION_SPEED == abi::ANIMATION_MAX_SPEED as f64);
};

pub(crate) fn encode(
    world: &mut NativeWorld,
    rows: &mut Vec<AnimationSnapshot>,
    bytes: &mut Vec<u8>,
) -> Result<(), Error> {
    world.snapshot_animations(rows)?;
    bytes.clear();
    bytes.extend_from_slice(
        &abi::AnimationHeader {
            magic: *abi::ANIMATION_MAGIC,
            count: rows.len() as u32,
            stride: abi::AnimationRow::SIZE as u32,
        }
        .encode(),
    );
    for row in rows {
        let state = row.state;
        bytes.extend_from_slice(
            &abi::AnimationRow {
                id: row.id.0,
                clip: state.sample.clip.map_or(u32::MAX, u32::from),
                flags: u32::from(state.playing)
                    | (u32::from(state.looping) << 1)
                    | (u32::from(state.completed) << 2),
                seconds: state.sample.seconds,
                speed: state.speed,
            }
            .encode(),
        );
    }
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

//! Bounded graph snapshots copied from the committed ECS state. No graph
//! definitions, assets or authoring records cross into guest memory.
use crate::BoundaryError as Error;
use incant_runtime::{AnimationGraphControl, AnimationGraphSnapshot, StableId};
use incant_wasm_abi as abi;

const _: () = {
    assert!(incant_runtime::MAX_ANIMATION_GRAPH_INSTANCES == abi::ANIMATION_GRAPH_MAX_ROWS);
    assert!(incant_types::MAX_ANIMATION_GRAPH_PARAMETERS == abi::ANIMATION_GRAPH_MAX_PARAMETERS);
    assert!(incant_types::MAX_ANIMATION_GRAPH_SECONDS == abi::ANIMATION_GRAPH_MAX_SECONDS as f64);
    assert!(incant_types::MAX_ANIMATION_GRAPH_VALUE == abi::ANIMATION_GRAPH_MAX_VALUE as f64);
};

pub(crate) fn encode(rows: &[AnimationGraphSnapshot], bytes: &mut Vec<u8>) {
    bytes.clear();
    let parameters: usize = rows.iter().map(|r| usize::from(r.parameter_count)).sum();
    bytes.extend(
        abi::AnimationGraphHeader {
            magic: *abi::ANIMATION_GRAPH_MAGIC,
            count: rows.len() as u32,
            stride: abi::AnimationGraphRow::SIZE as u32,
            parameter_count: parameters as u32,
            parameter_stride: abi::AnimationGraphParameter::SIZE as u32,
        }
        .encode(),
    );
    let mut offset = 0u32;
    for row in rows {
        bytes.extend(
            abi::AnimationGraphRow {
                id: row.entity.0,
                state: row.state.0,
                seconds: row.seconds,
                elapsed: row.elapsed,
                flags: u32::from(row.paused),
                parameter_offset: offset,
                parameter_count: u32::from(row.parameter_count),
                reserved: 0,
            }
            .encode(),
        );
        offset += u32::from(row.parameter_count);
    }
    for row in rows {
        for (&id, &value) in row.parameter_ids[..usize::from(row.parameter_count)]
            .iter()
            .zip(&row.parameters)
        {
            bytes.extend(abi::AnimationGraphParameter { id: id.0, value }.encode());
        }
    }
    debug_assert!(bytes.len() <= abi::ANIMATION_GRAPH_MAX_BYTES);
}

pub(crate) fn command(
    bytes: &[u8; abi::AnimationGraphCommand::SIZE],
) -> Result<AnimationGraphControl, Error> {
    let row = abi::AnimationGraphCommand::decode(bytes)
        .ok_or(Error::Abi("animation graph command frame"))?;
    if row.reserved != 0 || !row.value.is_finite() {
        return Err(Error::Abi("animation graph command fields"));
    }
    match row.kind {
        0 if row.value.abs() <= abi::ANIMATION_GRAPH_MAX_VALUE as f64 => {
            Ok(AnimationGraphControl::SetParameter {
                parameter: StableId(row.id),
                value: row.value,
            })
        }
        1 if row.value.to_bits() == 0 => Ok(AnimationGraphControl::SelectState(StableId(row.id))),
        _ => Err(Error::Abi("animation graph command kind/value")),
    }
}

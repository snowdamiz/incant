//! Complete copied pose and post-sample IK; no model evaluation in the ABI.
use crate::BoundaryError as Error;
use incant_runtime::{AnimationSnapshot, IkControl, IkSnapshot, StableId, StructuralCommand};
use incant_wasm_abi as abi;

const _: () = {
    assert!(incant_runtime::MAX_SCENE_ANIMATION_IK == abi::ANIMATION_IK_MAX_ROWS);
    assert!(incant_types::MAX_ANIMATION_IK == abi::ANIMATION_IK_MAX_PER_OWNER);
    assert!(incant_types::MAX_IK_MODEL_NODES == abi::ANIMATION_IK_MAX_MODEL_NODES);
    assert!(incant_types::MAX_IK_COORDINATE == abi::ANIMATION_IK_MAX_COORDINATE as f64);
    assert!(abi::ANIMATION_IK_MAX_BYTES < abi::MEMORY);
};

pub(crate) fn encode(
    poses: &[AnimationSnapshot],
    pose_bytes: &[u8],
    rows: &[IkSnapshot],
    output: &mut Vec<u8>,
) -> Result<(), Error> {
    if rows.len() > abi::ANIMATION_IK_MAX_ROWS || pose_bytes.len() > abi::ANIMATION_V4_MAX_BYTES {
        return Err(Error::Limit("animation IK snapshot"));
    }
    // Frozen rows come from the validated native owner. Check association and
    // canonical grouping before reserving or publishing any private frame bytes.
    let mut owner = None;
    let mut count = 0;
    let mut ids = [StableId([0; 16]); abi::ANIMATION_IK_MAX_PER_OWNER];
    for row in rows {
        if owner != Some(row.entity) {
            if owner.is_some_and(|p| p >= row.entity)
                || poses.binary_search_by_key(&row.entity, |p| p.id).is_err()
            {
                return Err(Error::Abi("animation IK pose association"));
            }
            owner = Some(row.entity);
            count = 0;
        }
        if count == ids.len() || ids[..count].contains(&row.sample.id) {
            return Err(Error::Abi("animation IK constraint identity"));
        }
        row.sample
            .validate()
            .map_err(|_| Error::Abi("animation IK sample"))?;
        ids[count] = row.sample.id;
        count += 1;
    }
    output.clear();
    output.extend(
        abi::AnimationIkHeader {
            magic: *abi::ANIMATION_IK_MAGIC,
            pose_bytes: pose_bytes.len() as u32,
            count: rows.len() as u32,
            stride: abi::AnimationIkRow::SIZE as u32,
            reserved: 0,
        }
        .encode(),
    );
    output.extend_from_slice(pose_bytes);
    for row in rows {
        let s = row.sample;
        output.extend(
            abi::AnimationIkRow {
                entity: row.entity.0,
                constraint: s.id.0,
                chain: s.chain.map(u32::from),
                reserved: 0,
                target: s.target,
                pole: s.pole,
                weight: s.weight,
            }
            .encode(),
        );
    }
    debug_assert!(output.len() <= abi::ANIMATION_IK_MAX_BYTES);
    Ok(())
}

pub(crate) fn command(
    bytes: &[u8; abi::AnimationIkCommand::SIZE],
) -> Result<StructuralCommand, Error> {
    let row = abi::AnimationIkCommand::decode(bytes).ok_or(Error::Abi("animation IK command"))?;
    if !row.is_finite()
        || row
            .target
            .iter()
            .chain(&row.pole)
            .any(|n| n.abs() > abi::ANIMATION_IK_MAX_COORDINATE as f64)
        || !(0.0..=1.0).contains(&row.weight)
    {
        return Err(Error::Abi("animation IK control values"));
    }
    Ok(StructuralCommand::ControlIk {
        entity: StableId(row.entity),
        control: IkControl {
            constraint: StableId(row.constraint),
            target: row.target,
            pole: row.pole,
            weight: row.weight,
        },
    })
}

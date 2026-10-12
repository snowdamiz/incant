//! Complete pose/strip/IK observations captured before callback commands.
use crate::BoundaryError as Error;
use incant_runtime::{
    AnimationSnapshot, RootMotionControl, RootMotionSnapshot, StableId, StructuralCommand,
};
use incant_types::RootMotionApplication;
use incant_wasm_abi as abi;

const _: () = assert!(abi::ROOT_MOTION_MAX_BYTES < abi::MEMORY);

pub(crate) fn encode(
    scene: StableId,
    poses: &[AnimationSnapshot],
    pose_bytes: &[u8],
    rows: &[RootMotionSnapshot],
    output: &mut Vec<u8>,
) -> Result<(), Error> {
    if rows.len() > abi::ROOT_MOTION_MAX_ROWS || pose_bytes.len() > abi::ANIMATION_IK_V2_MAX_BYTES {
        return Err(Error::Limit("root-motion snapshot"));
    }
    let mut previous = None;
    for row in rows {
        if previous.is_some_and(|id| id >= row.entity)
            || poses.binary_search_by_key(&row.entity, |p| p.id).is_err()
        {
            return Err(Error::Abi("root-motion pose association"));
        }
        previous = Some(row.entity);
    }
    output.clear();
    output.extend(
        abi::RootMotionHeader {
            magic: *abi::ROOT_MOTION_MAGIC,
            scene: scene.0,
            pose_bytes: pose_bytes.len() as u32,
            count: rows.len() as u32,
            stride: abi::RootMotionRow::SIZE as u32,
            reserved: 0,
        }
        .encode(),
    );
    output.extend_from_slice(pose_bytes);
    let delta = |t: [f64; 3], q: [f64; 4]| [t[0], t[1], t[2], q[0], q[1], q[2], q[3]];
    for row in rows {
        let current = clock(row.current);
        let outgoing = row.outgoing.map(clock).unwrap_or((0, 0, [0; 16], 0, 0.));
        output.extend(
            abi::RootMotionRow {
                entity: row.entity.0,
                fingerprint: row.fingerprint,
                root: u32::from(row.root),
                policy: match row.application {
                    RootMotionApplication::ExtractOnly => 0,
                    RootMotionApplication::Transform => 1,
                    RootMotionApplication::Kinematic3d => 2,
                    RootMotionApplication::Character3d => 3,
                },
                flags: u32::from(row.applying) | (u32::from(row.outgoing.is_some()) << 1),
                reserved: 0,
                tick: row.tick,
                current_kind: current.0,
                current_clip: current.1,
                current_state: current.2,
                current_cycle: current.3,
                current_seconds: current.4,
                outgoing_kind: outgoing.0,
                outgoing_clip: outgoing.1,
                outgoing_state: outgoing.2,
                outgoing_cycle: outgoing.3,
                outgoing_seconds: outgoing.4,
                requested: delta(row.requested.translation, row.requested.rotation),
                applied: delta(row.applied.translation, row.applied.rotation),
                accumulated_requested: delta(
                    row.accumulated_requested.translation,
                    row.accumulated_requested.rotation,
                ),
                accumulated_applied: delta(
                    row.accumulated_applied.translation,
                    row.accumulated_applied.rotation,
                ),
            }
            .encode(),
        );
    }
    debug_assert!(output.len() <= abi::ROOT_MOTION_MAX_BYTES);
    Ok(())
}
fn clock(c: incant_runtime::RootMotionClockSnapshot) -> (u32, u32, [u8; 16], i64, f64) {
    (
        if c.state.is_some() {
            2
        } else if c.clip.is_some() {
            1
        } else {
            0
        },
        c.clip.map_or(u32::MAX, u32::from),
        c.state.map_or([0; 16], |s| s.0),
        c.cycle,
        c.seconds,
    )
}
pub(crate) fn command(
    bytes: &[u8; abi::RootMotionCommand::SIZE],
) -> Result<StructuralCommand, Error> {
    let row = abi::RootMotionCommand::decode(bytes).ok_or(Error::Abi("root-motion command"))?;
    let control = match (row.kind, row.value) {
        (0, value @ 0..=1) => RootMotionControl::SetApplying(value == 1),
        (1, 0) => RootMotionControl::ResetAccumulated,
        _ => return Err(Error::Abi("root-motion control values")),
    };
    Ok(StructuralCommand::ControlRootMotion {
        entity: StableId(row.entity),
        control,
    })
}

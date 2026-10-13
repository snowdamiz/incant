//! V11 numeric rows follow the immutable root catalogs in the embedded scene.
use super::SaveError;
use crate::{
    NativeWorld, StableId,
    animation::Playback,
    animation_step::ClockOwner,
    binary::Reader,
    root_motion::{RootMotionState, state::Cursor},
};
use incant_anim::{RootMotionCatalog, RootMotionDelta};
use std::sync::Arc;
pub(super) const ROW: usize = 304;

pub(super) fn write(bytes: &mut Vec<u8>, id: StableId, state: RootMotionState) {
    bytes.extend(id.0);
    bytes.push(u8::from(state.applying) | (u8::from(state.outgoing.is_some()) << 1));
    bytes.extend([0; 7]);
    bytes.extend(state.tick.to_le_bytes());
    write_cursor(bytes, state.current);
    if let Some(cursor) = state.outgoing {
        write_cursor(bytes, cursor);
    } else {
        bytes.resize(bytes.len() + 24, 0);
    }
    for delta in [
        state.requested,
        state.applied,
        state.accumulated_requested,
        state.accumulated_applied,
    ] {
        for value in delta.translation.into_iter().chain(delta.rotation) {
            bytes.extend(value.to_le_bytes());
        }
    }
}
fn write_cursor(bytes: &mut Vec<u8>, cursor: Cursor) {
    let (kind, index) = match cursor.owner {
        ClockOwner::Clip(clip) => (0, clip.unwrap_or(u16::MAX)),
        ClockOwner::State(state) => (1, u16::from(state)),
    };
    bytes.extend([kind, 0]);
    bytes.extend(index.to_le_bytes());
    bytes.extend([0; 4]);
    bytes.extend(cursor.cycle.to_le_bytes());
    bytes.extend(cursor.seconds.to_le_bytes());
}
fn read_cursor(reader: &mut Reader<'_>) -> Result<Cursor, SaveError> {
    let kind = reader.byte()?;
    let reserved = reader.byte()?;
    let index = u16::from_le_bytes(reader.array()?);
    if reserved != 0 || reader.array::<4>()? != [0; 4] {
        return Err(SaveError::Invalid("root-motion cursor reserved bytes"));
    }
    let owner = match (kind, index) {
        (0, u16::MAX) => ClockOwner::Clip(None),
        (0, index) if usize::from(index) < incant_types::MAX_ANIMATION_CLIPS => {
            ClockOwner::Clip(Some(index))
        }
        (1, index) if usize::from(index) < incant_types::MAX_ANIMATION_GRAPH_STATES => {
            ClockOwner::State(index as u8)
        }
        _ => return Err(SaveError::Invalid("root-motion cursor owner")),
    };
    Ok(Cursor {
        owner,
        cycle: i64::from_le_bytes(reader.array()?),
        seconds: f64::from_le_bytes(reader.array()?),
    })
}
pub(super) fn install(world: &mut NativeWorld, bytes: &[u8], tick: u64) -> Result<(), SaveError> {
    if bytes.len() != world.root_ids.len() * ROW {
        return Err(SaveError::Invalid("root-motion state membership"));
    }
    for (id, row) in world.root_ids.iter().zip(bytes.as_chunks::<ROW>().0) {
        let mut r = Reader::new(row);
        if StableId(r.array()?) != *id {
            return Err(SaveError::Invalid("root-motion owner order"));
        }
        let flags = r.byte()?;
        if flags > 3 || r.array::<7>()? != [0; 7] {
            return Err(SaveError::Invalid("root-motion saved flags"));
        }
        let state_tick = u64::from_le_bytes(r.array()?);
        let current = read_cursor(&mut r)?;
        let outgoing = if flags & 2 != 0 {
            Some(read_cursor(&mut r)?)
        } else {
            if r.array::<24>()? != [0; 24] {
                return Err(SaveError::Invalid("root-motion absent outgoing cursor"));
            }
            None
        };
        let mut delta = || {
            Ok::<_, SaveError>(RootMotionDelta {
                translation: r.floats()?,
                rotation: r.floats()?,
            })
        };
        let state = RootMotionState {
            applying: flags & 1 != 0,
            current,
            outgoing,
            tick: state_tick,
            requested: delta()?,
            applied: delta()?,
            accumulated_requested: delta()?,
            accumulated_applied: delta()?,
        };
        let handle = world.entities[id];
        let playback = world
            .world
            .get::<Playback>(handle)
            .expect("saved root animation");
        state.validate(playback, tick)?;
        *world
            .world
            .get_mut::<RootMotionState>(handle)
            .expect("saved root component") = state;
    }
    Ok(())
}
impl NativeWorld {
    pub(super) fn validate_root_catalog(
        &self,
        catalog: &[Arc<RootMotionCatalog>],
    ) -> Result<(), SaveError> {
        for id in &self.root_ids {
            let root = &self
                .world
                .get::<crate::CookedSpatial>(self.entities[id])
                .expect("root spatial")
                .root_motion
                .as_ref()
                .expect("root binding")
                .catalog;
            if !catalog.iter().any(|c| Arc::ptr_eq(c, root) || c == root) {
                return Err(SaveError::Invalid("root-motion original captured binding"));
            }
        }
        Ok(())
    }
}

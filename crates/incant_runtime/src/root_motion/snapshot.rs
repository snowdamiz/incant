use super::{RootMotionState, state::Cursor};
use crate::{
    NativeWorld, RuntimeError, SceneError, StableId, animation::Playback,
    animation_step::ClockOwner,
};
use incant_anim::{MAX_ROOT_MOTION_INSTANCES, RootMotionDelta};
use incant_types::RootMotionApplication;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootMotionClockSnapshot {
    /// Graph state identity, or None for an ordinary clip clock.
    pub state: Option<StableId>,
    /// Some only for ordinary clip clocks; rest has neither state nor clip.
    pub clip: Option<u16>,
    pub cycle: i64,
    pub seconds: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RootMotionSnapshot {
    pub entity: StableId,
    pub fingerprint: [u8; 32],
    pub root: u16,
    pub application: RootMotionApplication,
    pub applying: bool,
    pub current: RootMotionClockSnapshot,
    pub outgoing: Option<RootMotionClockSnapshot>,
    pub requested: RootMotionDelta,
    pub applied: RootMotionDelta,
    pub accumulated_requested: RootMotionDelta,
    pub accumulated_applied: RootMotionDelta,
    pub tick: u64,
}
impl NativeWorld {
    /// Copy committed numeric observations into reusable caller storage.
    pub fn snapshot_root_motion(
        &self,
        rows: &mut Vec<RootMotionSnapshot>,
    ) -> Result<(), RuntimeError> {
        self.ensure_usable()?;
        rows.clear();
        if self.root_ids.len() > MAX_ROOT_MOTION_INSTANCES {
            return Err(SceneError::Invalid("root-motion snapshot capacity").into());
        }
        for id in &self.root_ids {
            let entity = self.entities[id];
            let state = self
                .world
                .get::<RootMotionState>(entity)
                .expect("retained root state");
            let playback = self
                .world
                .get::<Playback>(entity)
                .expect("retained animation");
            let root = self
                .world
                .get::<crate::CookedSpatial>(entity)
                .and_then(|n| n.root_motion.as_ref())
                .expect("retained root catalog");
            rows.push(RootMotionSnapshot {
                entity: *id,
                fingerprint: root.catalog.fingerprint(),
                root: root.catalog.root(),
                application: root.application,
                applying: state.applying,
                current: clock(state.current, playback),
                outgoing: state.outgoing.map(|c| clock(c, playback)),
                requested: state.requested,
                applied: state.applied,
                accumulated_requested: state.accumulated_requested,
                accumulated_applied: state.accumulated_applied,
                tick: state.tick,
            });
        }
        Ok(())
    }
}
fn clock(cursor: Cursor, playback: &Playback) -> RootMotionClockSnapshot {
    let (state, clip) = match cursor.owner {
        ClockOwner::State(index) => (
            Some(
                playback
                    .definition
                    .graph
                    .as_ref()
                    .expect("retained graph")
                    .states()[usize::from(index)]
                .id,
            ),
            None,
        ),
        ClockOwner::Clip(clip) => (None, clip),
    };
    RootMotionClockSnapshot {
        state,
        clip,
        cycle: cursor.cycle,
        seconds: cursor.seconds,
    }
}

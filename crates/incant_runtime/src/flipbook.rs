//! Cold immutable frames and a small, retained numeric playback cursor.
use crate::{
    CookedSpatial, NativeSpriteRegion, NativeSpriteRenderer, NativeWorld, SceneError, StableId,
};
use bevy_ecs::prelude::{Component, Query};
use incant_2d::FlipbookClock;
pub use incant_2d::flipbook::{MAX_FLIPBOOK_FRAMES, MAX_SCENE_FLIPBOOK_FRAMES};
pub use incant_2d::{FlipbookControl, FlipbookState, FrameRate};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq)]
pub struct NativeFlipbook {
    pub frames: Arc<[NativeSpriteRegion]>,
    pub rate: FrameRate,
    pub looping: bool,
    pub playing: bool,
}
impl NativeFlipbook {
    pub(crate) fn validate(&self, sprite: &NativeSpriteRenderer) -> Result<(), SceneError> {
        self.rate
            .validate()
            .map_err(|_| SceneError::Invalid("flipbook rate"))?;
        if self.frames.is_empty() || self.frames.len() > MAX_FLIPBOOK_FRAMES {
            return Err(SceneError::Invalid("flipbook frame count"));
        }
        let mut candidate = sprite.clone();
        for &region in self.frames.iter() {
            candidate.region = region;
            candidate.validate()?;
        }
        Ok(())
    }
    pub(crate) fn clock(&self, tick_rate: u32) -> FlipbookClock {
        FlipbookClock::new(self.rate, self.frames.len(), self.looping, tick_rate)
            .expect("validated native flipbook")
    }
}
#[derive(Component)]
pub(crate) struct Playback {
    pub clock: FlipbookClock,
    pub state: FlipbookState,
}
impl NativeWorld {
    /// Committed playback only. No mutable escape, text or asset lookup.
    pub fn flipbook_state(&self, id: StableId) -> Option<FlipbookState> {
        if self.physics_failed {
            return None;
        }
        self.world
            .get::<Playback>(*self.entities.get(&id)?)
            .map(|p| p.state)
    }
    pub(crate) fn prepare_flipbook_controls(
        &self,
        commands: &crate::FrameCommands,
        changes: &std::collections::BTreeMap<StableId, Option<CookedSpatial>>,
        exists: impl Fn(StableId) -> bool,
    ) -> Result<std::collections::BTreeMap<StableId, FlipbookState>, crate::RuntimeError> {
        let mut states = std::collections::BTreeMap::new();
        for command in commands.iter() {
            let crate::StructuralCommand::ControlFlipbook { entity, control } = command else {
                continue;
            };
            if !exists(*entity) {
                return Err(crate::RuntimeError::MissingEntity(*entity));
            }
            let old = self
                .entities
                .get(entity)
                .and_then(|h| self.world.get::<CookedSpatial>(*h))
                .and_then(|s| s.flipbook.as_ref());
            let selected = changes
                .get(entity)
                .map_or(old, |s| s.as_ref().and_then(|s| s.flipbook.as_ref()));
            let selected = selected.ok_or(SceneError::Invalid(
                "flipbook control requires final animation",
            ))?;
            let state = states.entry(*entity).or_insert_with(|| {
                if old == Some(selected) {
                    self.flipbook_state(*entity).expect("installed flipbook")
                } else {
                    FlipbookState::new(selected.playing)
                }
            });
            selected
                .clock(self.tick_rate)
                .control(state, *control)
                .map_err(|_| SceneError::Invalid("flipbook seek frame"))?;
        }
        Ok(states)
    }
    pub(crate) fn install_flipbook_states(
        &mut self,
        states: std::collections::BTreeMap<StableId, FlipbookState>,
    ) {
        for (id, state) in states {
            self.world
                .get_mut::<Playback>(self.entities[&id])
                .expect("prepared animation")
                .state = state;
        }
    }
}

// A retained Bevy system/query: no query-state construction on each tick.
pub(crate) fn advance(mut query: Query<&mut Playback>) {
    for mut playback in &mut query {
        playback.clock.advance(&mut playback.state);
    }
}

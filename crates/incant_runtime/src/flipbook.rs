//! Cold immutable frames and a small, retained numeric playback cursor.
use crate::{
    CookedSpatial, NativeSpriteRegion, NativeSpriteRenderer, NativeWorld, SceneError, StableId,
};
use bevy_ecs::prelude::{Component, Query, QueryState};
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
pub(crate) type PlaybackQuery = QueryState<(&'static crate::world::Identity, &'static Playback)>;

/// One owned committed playback row. No asset/configuration or private clock
/// remainder is copied. Row order is unspecified; identify rows by stable ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FlipbookSnapshot {
    pub id: StableId,
    pub frame: u16,
    pub playing: bool,
    pub completed: bool,
}
impl NativeWorld {
    /// Replace reusable caller scratch with committed playback rows, before any
    /// numeric loan. A retained ECS query visits only animation archetypes and
    /// discovers structural changes without rebuilding the query each tick.
    /// At most MAX_SPRITE_ENTITIES rows; no frame lists or asset bindings copied.
    /// Later queued controls do not change these owned observations.
    pub fn snapshot_flipbooks(
        &mut self,
        rows: &mut Vec<FlipbookSnapshot>,
    ) -> Result<(), crate::RuntimeError> {
        self.ensure_usable()?;
        rows.clear();
        for (id, playback) in self.flipbook_query.iter(&self.world) {
            if rows.len() == crate::MAX_SPRITE_ENTITIES {
                rows.clear();
                return Err(SceneError::Invalid("flipbook snapshot quota").into());
            }
            rows.push(FlipbookSnapshot {
                id: id.0,
                frame: playback.state.frame(),
                playing: playback.state.playing(),
                completed: playback.state.completed(),
            });
        }
        Ok(())
    }

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

//! Retained native playback. Only committed fixed steps advance these clocks.
use crate::{CookedSpatial, FrameCommands, NativeWorld, RuntimeError, SceneError, StableId};
use bevy_ecs::prelude::{Component, Query, QueryState, Res};
use incant_types::Animation;
pub use incant_types::{
    AnimationBlendSample, AnimationSample, MAX_ANIMATION_CLIPS, MAX_ANIMATION_SPEED,
};
use std::{collections::BTreeMap, sync::Arc};

pub const MAX_SCENE_ANIMATION_CLIPS: usize = 1_000_000;
pub const MAX_ANIMATION_TRANSITION_SECONDS: f64 = 60.0;

/// Checked cold metadata tied to the same mesh asset as its spatial declaration.
/// Consumers compare the complete duration table with the sealed cooked asset.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeAnimation {
    pub mesh_fingerprint: [u8; 32],
    pub clip_durations: Arc<[f32]>,
    pub initial: Animation,
}
impl NativeAnimation {
    pub fn validate(&self) -> Result<(), SceneError> {
        self.initial.validate().map_err(SceneError::Invalid)?;
        if self.clip_durations.len() > MAX_ANIMATION_CLIPS
            || self
                .clip_durations
                .iter()
                .any(|d| !d.is_finite() || *d < 0.0 || d.to_bits() == (-0.0f32).to_bits())
        {
            return Err(SceneError::Invalid("animation duration table"));
        }
        self.validate_sample(AnimationSample {
            clip: self.initial.clip,
            seconds: self.initial.seconds,
        })
    }
    pub fn validate_sample(&self, sample: AnimationSample) -> Result<(), SceneError> {
        let duration = match sample.clip {
            Some(clip) => f64::from(
                *self
                    .clip_durations
                    .get(usize::from(clip))
                    .ok_or(SceneError::Invalid("animation clip index"))?,
            ),
            None => 0.0,
        };
        if !sample.seconds.is_finite() || sample.seconds < 0.0 || sample.seconds > duration {
            return Err(SceneError::Invalid("animation sample time"));
        }
        Ok(())
    }
    pub(crate) fn validate_state(&self, state: AnimationState) -> Result<(), SceneError> {
        self.validate_sample(state.sample)?;
        if !state.speed.is_finite() || state.speed.abs() > MAX_ANIMATION_SPEED {
            return Err(SceneError::Invalid("animation speed"));
        }
        if let Some(transition) = state.transition {
            self.validate_sample(transition.source)?;
            if !transition.speed.is_finite()
                || transition.speed.abs() > MAX_ANIMATION_SPEED
                || !transition.duration.is_finite()
                || transition.duration <= 0.0
                || transition.duration > MAX_ANIMATION_TRANSITION_SECONDS
                || !transition.elapsed.is_finite()
                || transition.elapsed < 0.0
                || transition.elapsed >= transition.duration
                || state.completed
            {
                return Err(SceneError::Invalid("animation transition state"));
            }
        }
        if state.completed {
            let clip = state
                .sample
                .clip
                .ok_or(SceneError::Invalid("rest pose completion"))?;
            let duration = f64::from(self.clip_durations[usize::from(clip)]);
            if state.playing
                || state.looping
                || (state.sample.seconds != 0.0 && state.sample.seconds != duration)
            {
                return Err(SceneError::Invalid("animation completion state"));
            }
        }
        Ok(())
    }
    pub(crate) fn initial_state(&self) -> AnimationState {
        AnimationState {
            sample: AnimationSample {
                clip: self.initial.clip,
                seconds: self.initial.seconds,
            },
            speed: self.initial.speed,
            playing: self.initial.playing,
            looping: self.initial.looping,
            completed: false,
            transition: None,
        }
    }
    pub(crate) fn control(
        &self,
        state: &mut AnimationState,
        control: AnimationControl,
    ) -> Result<(), SceneError> {
        let mut next = *state;
        match control {
            AnimationControl::Select(sample) => {
                self.validate_sample(sample)?;
                next.sample = sample;
                next.completed = false;
                next.transition = None;
            }
            AnimationControl::Crossfade { target, duration } => {
                self.validate_sample(target)?;
                if !duration.is_finite()
                    || !(0.0..=MAX_ANIMATION_TRANSITION_SECONDS).contains(&duration)
                {
                    return Err(SceneError::Invalid("animation transition duration"));
                }
                if duration == 0.0 {
                    next.transition = None;
                } else {
                    if next.transition.is_some() {
                        return Err(SceneError::Invalid("animation transition already active"));
                    }
                    next.transition = Some(AnimationTransition {
                        source: next.sample,
                        speed: next.speed,
                        playing: next.playing,
                        looping: next.looping,
                        elapsed: 0.0,
                        duration,
                    });
                }
                next.sample = target;
                next.completed = false;
            }
            AnimationControl::Seek(seconds) => {
                let sample = AnimationSample {
                    seconds,
                    ..next.sample
                };
                self.validate_sample(sample)?;
                next.sample = sample;
                next.completed = false;
                next.transition = None;
            }
            AnimationControl::SetPlaying(playing) => {
                next.playing = playing;
                if playing {
                    next.completed = false;
                }
            }
            AnimationControl::SetLooping(looping) => {
                next.looping = looping;
                if looping {
                    next.completed = false;
                }
            }
            AnimationControl::SetSpeed(speed) => {
                if !speed.is_finite() || speed.abs() > MAX_ANIMATION_SPEED {
                    return Err(SceneError::Invalid("animation speed"));
                }
                next.speed = speed;
            }
        }
        *state = next;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationState {
    pub sample: AnimationSample,
    pub speed: f64,
    pub playing: bool,
    pub looping: bool,
    pub completed: bool,
    pub transition: Option<AnimationTransition>,
}
impl AnimationState {
    pub fn blend_sample(self) -> Option<AnimationBlendSample> {
        self.transition.map(|t| AnimationBlendSample {
            sample: t.source,
            weight: t.elapsed / t.duration,
        })
    }
}
/// Outgoing clock captured when a crossfade starts. Master `playing` pauses
/// both clocks and the fade; speed/loop controls thereafter affect only the
/// destination. No model data or evaluated pose is copied per step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationTransition {
    pub source: AnimationSample,
    pub speed: f64,
    pub playing: bool,
    pub looping: bool,
    pub elapsed: f64,
    pub duration: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationControl {
    /// Select a clip/rest pose and seek atomically; preserves speed/play/loop flags.
    Select(AnimationSample),
    /// Blend to a clip/rest sample over 0..=60 seconds; zero selects immediately.
    /// A nonzero transition cannot interrupt another transition. Seek/Select
    /// explicitly cancel it. The master playing flag is preserved.
    Crossfade {
        target: AnimationSample,
        duration: f64,
    },
    Seek(f64),
    SetPlaying(bool),
    SetLooping(bool),
    SetSpeed(f64),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationSnapshot {
    pub id: StableId,
    pub state: AnimationState,
}
#[derive(Component)]
pub(crate) struct Playback {
    pub definition: Arc<NativeAnimation>,
    pub state: AnimationState,
}
pub(crate) type PlaybackQuery = QueryState<(&'static crate::world::Identity, &'static Playback)>;

impl NativeWorld {
    pub fn animation_state(&self, id: StableId) -> Option<AnimationState> {
        if self.physics_failed {
            return None;
        }
        self.world
            .get::<Playback>(*self.entities.get(&id)?)
            .map(|p| p.state)
    }
    /// Owned committed numeric rows in unspecified ECS order. Retain caller
    /// capacity across ticks; immutable asset/configuration data is never copied.
    pub fn snapshot_animations(
        &mut self,
        rows: &mut Vec<AnimationSnapshot>,
    ) -> Result<(), RuntimeError> {
        self.ensure_usable()?;
        rows.clear();
        for (id, playback) in self.animation_query.iter(&self.world) {
            if rows.len() == crate::MAX_MESH_ENTITIES {
                rows.clear();
                return Err(SceneError::Invalid("animation snapshot quota").into());
            }
            rows.push(AnimationSnapshot {
                id: id.0,
                state: playback.state,
            });
        }
        Ok(())
    }
    pub(crate) fn prepare_animation_controls(
        &self,
        commands: &FrameCommands,
        changes: &BTreeMap<StableId, Option<CookedSpatial>>,
        exists: impl Fn(StableId) -> bool,
    ) -> Result<BTreeMap<StableId, AnimationState>, RuntimeError> {
        let mut states = BTreeMap::new();
        for command in commands.iter() {
            let crate::StructuralCommand::ControlAnimation { entity, control } = command else {
                continue;
            };
            if !exists(*entity) {
                return Err(RuntimeError::MissingEntity(*entity));
            }
            let old = self
                .entities
                .get(entity)
                .and_then(|h| self.world.get::<CookedSpatial>(*h))
                .and_then(|s| s.animation.as_ref());
            let selected = changes
                .get(entity)
                .map_or(old, |s| s.as_ref().and_then(|s| s.animation.as_ref()))
                .ok_or(SceneError::Invalid(
                    "animation control requires final animation",
                ))?;
            let state = states.entry(*entity).or_insert_with(|| {
                if old == Some(selected) {
                    self.animation_state(*entity).expect("installed animation")
                } else {
                    selected.initial_state()
                }
            });
            selected.control(state, *control)?;
        }
        Ok(states)
    }
    pub(crate) fn install_animation_states(&mut self, states: BTreeMap<StableId, AnimationState>) {
        for (id, state) in states {
            self.world
                .get_mut::<Playback>(self.entities[&id])
                .expect("prepared animation")
                .state = state;
        }
    }
}

pub(crate) fn advance(step: Res<crate::world::Step>, mut query: Query<&mut Playback>) {
    for mut playback in &mut query {
        if !playback.state.playing {
            continue;
        }
        let Playback { definition, state } = &mut *playback;
        let ended = advance_sample(
            &mut state.sample,
            state.speed,
            state.looping,
            step.0,
            definition,
        );
        if let Some(transition) = &mut state.transition {
            if transition.playing
                && advance_sample(
                    &mut transition.source,
                    transition.speed,
                    transition.looping,
                    step.0,
                    definition,
                )
            {
                transition.playing = false;
            }
            transition.elapsed += step.0;
            // Bound accumulated addition roundoff by step count * epsilon *
            // duration (under one nanosecond at the supported 60s/240Hz limits).
            // An integral-tick fade must not gain an extra frame from rounding.
            let roundoff =
                f64::EPSILON * transition.duration * (transition.duration / step.0 + 1.0);
            if transition.duration - transition.elapsed <= roundoff {
                state.transition = None;
            }
        }
        if ended && state.transition.is_none() {
            state.playing = false;
            state.completed = true;
        }
    }
}

fn advance_sample(
    sample: &mut AnimationSample,
    speed: f64,
    looping: bool,
    seconds: f64,
    definition: &NativeAnimation,
) -> bool {
    let Some(clip) = sample.clip else {
        return false;
    };
    let duration = f64::from(definition.clip_durations[usize::from(clip)]);
    if duration == 0.0 {
        sample.seconds = 0.0;
        return !looping;
    }
    if speed == 0.0 {
        return false;
    }
    let next = sample.seconds + seconds * speed;
    if looping {
        let wrapped = next.rem_euclid(duration);
        sample.seconds = if wrapped >= duration { 0.0 } else { wrapped };
    } else if (speed > 0.0 && next >= duration) || (speed < 0.0 && next <= 0.0) {
        sample.seconds = next.clamp(0.0, duration);
        return true;
    } else {
        sample.seconds = next;
    }
    false
}

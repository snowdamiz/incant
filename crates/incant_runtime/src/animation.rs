//! Retained native playback. Only committed fixed steps advance these clocks.
use crate::{CookedSpatial, FrameCommands, NativeWorld, RuntimeError, SceneError, StableId};
use bevy_ecs::prelude::{Component, Query, QueryState, Res, Without};
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
    pub graph: Option<Arc<crate::NativeAnimationGraph>>,
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
        })?;
        if let Some(graph) = &self.graph {
            graph.validate(&self.clip_durations)?;
            let entry = &graph.states()[usize::from(graph.initial())];
            let clip = match entry.motion {
                crate::NativeAnimationMotion::Clip(clip) => clip,
                crate::NativeAnimationMotion::Blend1d { .. }
                | crate::NativeAnimationMotion::Tree { .. } => None,
            };
            if self.initial.clip != clip
                || self.initial.seconds != 0.0
                || self.initial.looping != entry.looping
            {
                return Err(SceneError::Invalid(
                    "animation graph initial playback must match its entry",
                ));
            }
        }
        Ok(())
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
        if state.completed && self.graph.is_none() {
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
        let mut state = AnimationState {
            sample: AnimationSample {
                clip: self.initial.clip,
                seconds: self.initial.seconds,
            },
            speed: self.initial.speed,
            playing: self.initial.playing,
            looping: self.initial.looping,
            completed: false,
            transition: None,
        };
        if let Some(graph) = &self.graph {
            state.sample = graph
                .sample(&graph.initial_state(self.initial.playing), self)
                .0;
        }
        state
    }
    pub(crate) fn control(
        &self,
        state: &mut AnimationState,
        control: AnimationControl,
    ) -> Result<(), SceneError> {
        if self.graph.is_some()
            && !matches!(
                control,
                AnimationControl::SetPlaying(_) | AnimationControl::SetSpeed(_)
            )
        {
            return Err(SceneError::Invalid(
                "animation graph owns clip, seek, loop and transition controls",
            ));
        }
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
    /// Actual second pose, including a graph blend without a timed transition.
    pub blend: Option<AnimationBlendSample>,
    /// Complete weighted local pose for a nested tree; scalar sample is representative only.
    pub pose: Option<incant_types::AnimationPoseSamples>,
}
#[derive(Component)]
pub(crate) struct Playback {
    pub definition: Arc<NativeAnimation>,
    pub state: AnimationState,
    pub graph: Option<crate::AnimationGraphState>,
}
impl Playback {
    pub(crate) fn pose_sample(&self) -> Option<incant_types::AnimationPoseSamples> {
        self.definition
            .graph
            .as_ref()?
            .pose(self.graph.as_ref()?, &self.definition)
    }
    pub(crate) fn blend_sample(&self) -> Option<AnimationBlendSample> {
        if let (Some(definition), Some(graph)) = (&self.definition.graph, &self.graph) {
            if graph.fade.is_some() {
                return None;
            }
            definition
                .sample(graph, &self.definition)
                .1
                .or_else(|| self.state.blend_sample())
        } else {
            self.state.blend_sample()
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub(crate) struct PreparedAnimationState {
    state: AnimationState,
    graph: Option<crate::AnimationGraphState>,
    root: Option<crate::root_motion::RootMotionState>,
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
                blend: playback.blend_sample(),
                pose: playback.pose_sample(),
            });
        }
        Ok(())
    }
    pub(crate) fn prepare_animation_controls(
        &self,
        commands: &FrameCommands,
        changes: &BTreeMap<StableId, Option<CookedSpatial>>,
        exists: impl Fn(StableId) -> bool,
    ) -> Result<BTreeMap<StableId, PreparedAnimationState>, RuntimeError> {
        let mut states = BTreeMap::new();
        for command in commands.iter() {
            let entity = match command {
                crate::StructuralCommand::ControlAnimation { entity, .. }
                | crate::StructuralCommand::ControlAnimationGraph { entity, .. }
                | crate::StructuralCommand::ControlRootMotion { entity, .. } => entity,
                _ => continue,
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
                    let playback = self
                        .world
                        .get::<Playback>(self.entities[entity])
                        .expect("installed animation");
                    PreparedAnimationState {
                        state: playback.state,
                        graph: playback.graph,
                        root: None,
                    }
                } else {
                    PreparedAnimationState {
                        state: selected.initial_state(),
                        graph: selected
                            .graph
                            .as_ref()
                            .map(|g| g.initial_state(selected.initial.playing)),
                        root: None,
                    }
                }
            });
            let old_root = self
                .entities
                .get(entity)
                .and_then(|h| self.world.get::<CookedSpatial>(*h))
                .and_then(|s| s.root_motion.as_ref());
            let selected_root = changes.get(entity).map_or(old_root, |s| {
                s.as_ref().and_then(|s| s.root_motion.as_ref())
            });
            if state.root.is_none()
                && let Some(root) = selected_root
            {
                state.root = Some(if old == Some(selected) && old_root == Some(root) {
                    *self
                        .world
                        .get::<crate::root_motion::RootMotionState>(self.entities[entity])
                        .expect("installed root state")
                } else {
                    crate::root_motion::RootMotionState::new(
                        root,
                        &Playback {
                            definition: selected.clone(),
                            state: state.state,
                            graph: state.graph,
                        },
                        self.tick,
                    )
                });
            }
            let before = Playback {
                definition: selected.clone(),
                state: state.state,
                graph: state.graph,
            };
            match command {
                crate::StructuralCommand::ControlAnimation { control, .. } => {
                    selected.control(&mut state.state, *control)?;
                    if let (Some(graph), AnimationControl::SetPlaying(playing)) =
                        (&mut state.graph, control)
                    {
                        graph.paused = !playing;
                    }
                }
                crate::StructuralCommand::ControlAnimationGraph { control, .. } => {
                    let graph = selected.graph.as_ref().ok_or(SceneError::Invalid(
                        "animation graph control requires final graph",
                    ))?;
                    graph.control(
                        state.graph.as_mut().expect("prepared graph"),
                        &mut state.state,
                        selected,
                        *control,
                    )?;
                }
                crate::StructuralCommand::ControlRootMotion { control, .. } => {
                    let root = state.root.as_mut().ok_or(SceneError::Invalid(
                        "root-motion control requires final root-motion binding",
                    ))?;
                    match control {
                        crate::RootMotionControl::SetApplying(applying) => {
                            root.applying = *applying
                        }
                        crate::RootMotionControl::ResetAccumulated => {
                            root.requested = incant_anim::RootMotionDelta::IDENTITY;
                            root.applied = incant_anim::RootMotionDelta::IDENTITY;
                            root.accumulated_requested = incant_anim::RootMotionDelta::IDENTITY;
                            root.accumulated_applied = incant_anim::RootMotionDelta::IDENTITY;
                        }
                    }
                }
                _ => unreachable!("selected animation control"),
            }
            if let Some(root) = &mut state.root {
                let after = Playback {
                    definition: selected.clone(),
                    state: state.state,
                    graph: state.graph,
                };
                match command {
                    crate::StructuralCommand::ControlAnimation { control, .. } => {
                        root.animation_control(&before, &after, *control)
                    }
                    crate::StructuralCommand::ControlAnimationGraph { control, .. } => {
                        root.graph_control(&after, *control)
                    }
                    _ => {}
                }
            }
        }
        Ok(states)
    }
    pub(crate) fn install_animation_states(
        &mut self,
        states: BTreeMap<StableId, PreparedAnimationState>,
    ) {
        for (id, state) in states {
            let mut playback = self
                .world
                .get_mut::<Playback>(self.entities[&id])
                .expect("prepared animation");
            playback.state = state.state;
            playback.graph = state.graph;
            if let Some(root) = state.root {
                *self
                    .world
                    .get_mut::<crate::root_motion::RootMotionState>(self.entities[&id])
                    .expect("prepared root") = root;
            }
        }
    }
}

pub(crate) fn advance(
    step: Res<crate::world::Step>,
    mut query: Query<&mut Playback, Without<crate::root_motion::RootMotionState>>,
) {
    for mut playback in &mut query {
        let Playback {
            definition,
            state,
            graph,
        } = &mut *playback;
        if let (Some(definition_graph), Some(graph)) = (&definition.graph, graph) {
            definition_graph.advance(graph, state, definition, step.0);
        } else {
            advance_state(state, definition, step.0);
        }
    }
}

pub(crate) fn advance_state(state: &mut AnimationState, definition: &NativeAnimation, dt: f64) {
    if !state.playing {
        return;
    }
    let ended = advance_sample(
        &mut state.sample,
        state.speed,
        state.looping,
        dt,
        definition,
    );
    if let Some(transition) = &mut state.transition {
        if transition.playing
            && advance_sample(
                &mut transition.source,
                transition.speed,
                transition.looping,
                dt,
                definition,
            )
        {
            transition.playing = false;
        }
        transition.elapsed += dt;
        // Bound accumulated addition roundoff by step count * epsilon *
        // duration (under one nanosecond at the supported 60s/240Hz limits).
        // An integral-tick fade must not gain an extra frame from rounding.
        let roundoff = f64::EPSILON * transition.duration * (transition.duration / dt + 1.0);
        if transition.duration - transition.elapsed <= roundoff {
            state.transition = None;
        }
    }
    if ended && state.transition.is_none() {
        state.playing = false;
        state.completed = true;
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
    advance_clock(&mut sample.seconds, duration, speed, looping, seconds)
}

pub(crate) fn advance_clock(
    time: &mut f64,
    duration: f64,
    speed: f64,
    looping: bool,
    seconds: f64,
) -> bool {
    if duration == 0.0 {
        *time = 0.0;
        return !looping;
    }
    if speed == 0.0 {
        return false;
    }
    let next = *time + seconds * speed;
    if looping {
        let wrapped = next.rem_euclid(duration);
        *time = if wrapped >= duration { 0.0 } else { wrapped };
    } else if (speed > 0.0 && next >= duration) || (speed < 0.0 && next <= 0.0) {
        *time = next.clamp(0.0, duration);
        return true;
    } else {
        *time = next;
    }
    false
}

use super::*;
use crate::{
    AnimationBlendSample, AnimationSample, AnimationState, NativeAnimation, SceneError, StableId,
};
use incant_types::{MAX_ANIMATION_GRAPH_PARAMETERS, MAX_ANIMATION_GRAPH_SECONDS};

/// Retained numeric state only. Configuration and model data are never copied
/// during a step or command. Unused parameter slots are canonical positive zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationGraphState {
    pub(crate) current: u8,
    pub(crate) paused: bool,
    pub(crate) seconds: f64,
    pub(crate) elapsed: f64,
    pub(crate) parameters: [f64; MAX_ANIMATION_GRAPH_PARAMETERS],
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationGraphControl {
    SetParameter {
        parameter: StableId,
        value: f64,
    },
    /// Explicit immediate selection resets state clocks and cancels a fade.
    /// The master play/pause and speed settings remain unchanged.
    SelectState(StableId),
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationGraphSnapshot {
    pub entity: StableId,
    pub state: StableId,
    /// Explicit graph master pause, independent of natural clip completion.
    pub paused: bool,
    pub seconds: f64,
    pub elapsed: f64,
    pub parameters: [f64; MAX_ANIMATION_GRAPH_PARAMETERS],
    pub parameter_count: u8,
    pub parameter_ids: [StableId; MAX_ANIMATION_GRAPH_PARAMETERS],
    pub blend: Option<AnimationBlendSample>,
    pub pose: Option<incant_types::AnimationPoseSamples>,
}
impl NativeAnimationGraph {
    pub(crate) fn initial_state(&self, playing: bool) -> AnimationGraphState {
        let mut parameters = [0.0; MAX_ANIMATION_GRAPH_PARAMETERS];
        for (slot, p) in parameters.iter_mut().zip(self.parameters.iter()) {
            *slot = p.initial;
        }
        AnimationGraphState {
            current: self.initial,
            paused: !playing,
            seconds: 0.0,
            elapsed: 0.0,
            parameters,
        }
    }
    pub(crate) fn pose(
        &self,
        graph: &AnimationGraphState,
    ) -> Option<incant_types::AnimationPoseSamples> {
        let NativeAnimationMotion::Tree {
            cycle_seconds,
            tree,
        } = &self.states[usize::from(graph.current)].motion
        else {
            return None;
        };
        Some(
            tree.sample(
                &graph.parameters[..self.parameters.len()],
                graph.seconds / cycle_seconds,
            )
            .expect("validated graph tree state"),
        )
    }
    pub(crate) fn sample(
        &self,
        graph: &AnimationGraphState,
        definition: &NativeAnimation,
    ) -> (AnimationSample, Option<AnimationBlendSample>) {
        match &self.states[usize::from(graph.current)].motion {
            NativeAnimationMotion::Tree { .. } => (
                self.pose(graph).expect("tree motion").samples()[0].sample,
                None,
            ),
            NativeAnimationMotion::Clip(clip) => (
                AnimationSample {
                    clip: *clip,
                    seconds: graph.seconds,
                },
                None,
            ),
            NativeAnimationMotion::Blend1d {
                parameter,
                cycle_seconds,
                points,
            } => {
                let value = graph.parameters[usize::from(*parameter)];
                let upper = points.partition_point(|point| point.threshold <= value);
                let sample = |point: &NativeAnimationBlendPoint| AnimationSample {
                    clip: Some(point.clip),
                    seconds: (graph.seconds / cycle_seconds)
                        * f64::from(definition.clip_durations[usize::from(point.clip)]),
                };
                if upper == 0 {
                    return (sample(&points[0]), None);
                }
                if upper == points.len() {
                    return (sample(&points[upper - 1]), None);
                }
                let (left, right) = (&points[upper - 1], &points[upper]);
                let weight = (value - left.threshold) / (right.threshold - left.threshold);
                if weight == 0.0 {
                    return (sample(left), None);
                }
                (
                    sample(right),
                    Some(AnimationBlendSample {
                        sample: sample(left),
                        weight,
                    }),
                )
            }
        }
    }
    pub(crate) fn control(
        &self,
        graph: &mut AnimationGraphState,
        playback: &mut AnimationState,
        definition: &NativeAnimation,
        control: AnimationGraphControl,
    ) -> Result<(), SceneError> {
        match control {
            AnimationGraphControl::SetParameter { parameter, value } => {
                let index = self
                    .parameters
                    .binary_search_by_key(&parameter, |p| p.id)
                    .map_err(|_| SceneError::Invalid("animation graph parameter ID"))?;
                let p = &self.parameters[index];
                if !value.is_finite() || value < p.min || value > p.max {
                    return Err(SceneError::Invalid("animation graph parameter value"));
                }
                graph.parameters[index] = value;
                playback.sample = self.sample(graph, definition).0;
            }
            AnimationGraphControl::SelectState(id) => {
                let index = self
                    .states
                    .binary_search_by_key(&id, |state| state.id)
                    .map_err(|_| SceneError::Invalid("animation graph state ID"))?;
                self.enter(graph, playback, definition, index as u8, 0.0);
            }
        }
        Ok(())
    }
    fn enter(
        &self,
        graph: &mut AnimationGraphState,
        playback: &mut AnimationState,
        definition: &NativeAnimation,
        state: u8,
        fade: f64,
    ) {
        let transition = (fade > 0.0).then_some(crate::AnimationTransition {
            source: playback.sample,
            speed: playback.speed,
            playing: playback.playing,
            looping: playback.looping,
            elapsed: 0.0,
            duration: fade,
        });
        graph.current = state;
        graph.seconds = 0.0;
        graph.elapsed = 0.0;
        playback.sample = self.sample(graph, definition).0;
        playback.looping = self.states[usize::from(state)].looping;
        playback.completed = false;
        playback.transition = transition;
        playback.playing = !graph.paused;
    }
    pub(crate) fn advance(
        &self,
        graph: &mut AnimationGraphState,
        playback: &mut AnimationState,
        definition: &NativeAnimation,
        dt: f64,
    ) {
        if graph.paused {
            return;
        }
        let transitioning = playback.transition.is_some();
        match &self.states[usize::from(graph.current)].motion {
            NativeAnimationMotion::Clip(_) => {
                crate::animation::advance_state(playback, definition, dt);
                graph.seconds = playback.sample.seconds;
            }
            NativeAnimationMotion::Blend1d { cycle_seconds, .. }
            | NativeAnimationMotion::Tree { cycle_seconds, .. } => {
                // Identical wrapping/completion rules to the ordinary clip clock;
                // a normalized blend cycle has its own declared duration.
                let ended = playback.playing
                    && crate::animation::advance_clock(
                        &mut graph.seconds,
                        *cycle_seconds,
                        playback.speed,
                        playback.looping,
                        dt,
                    );
                playback.sample = self.sample(graph, definition).0;
                if ended {
                    playback.playing = false;
                    playback.completed = true;
                }
            }
        }
        graph.elapsed = (graph.elapsed + dt).min(MAX_ANIMATION_GRAPH_SECONDS);
        // At most one automatic edge per committed step. An in-flight fade owns
        // its full step, even when it finishes; no interruption or cascade.
        if transitioning {
            return;
        }
        let state = usize::from(graph.current);
        for edge in &self.transitions[usize::from(self.transition_starts[state])
            ..usize::from(self.transition_starts[state + 1])]
        {
            if edge.after_seconds.is_some_and(|after| {
                let roundoff = f64::EPSILON * after * (after / dt + 1.0);
                after - graph.elapsed > roundoff
            }) || edge.conditions.iter().any(|condition| {
                !condition.comparison.accepts(
                    graph.parameters[usize::from(condition.parameter)],
                    condition.value,
                )
            }) {
                continue;
            }
            // A non-looping state reaching its endpoint may still take an exit
            // in the same step. Explicit master pause never enters this function.
            self.enter(graph, playback, definition, edge.to, edge.fade_seconds);
            break;
        }
    }
    pub(crate) fn validate_state(
        &self,
        graph: &AnimationGraphState,
        playback: AnimationState,
        definition: &NativeAnimation,
    ) -> Result<(), SceneError> {
        let state = self
            .states
            .get(usize::from(graph.current))
            .ok_or(SceneError::Invalid("animation graph saved state"))?;
        let duration = match state.motion {
            NativeAnimationMotion::Clip(clip) => clip.map_or(0.0, |c| {
                f64::from(definition.clip_durations[usize::from(c)])
            }),
            NativeAnimationMotion::Blend1d { cycle_seconds, .. }
            | NativeAnimationMotion::Tree { cycle_seconds, .. } => cycle_seconds,
        };
        if !graph.seconds.is_finite()
            || graph.seconds < 0.0
            || graph.seconds > duration
            || !graph.elapsed.is_finite()
            || !(0.0..=MAX_ANIMATION_GRAPH_SECONDS).contains(&graph.elapsed)
            || playback.looping != state.looping
            || (graph.paused && playback.playing)
            || (!graph.paused && !playback.playing && !playback.completed)
            || graph.parameters[..self.parameters.len()]
                .iter()
                .zip(self.parameters.iter())
                .any(|(v, p)| !v.is_finite() || *v < p.min || *v > p.max)
            || graph.parameters[self.parameters.len()..]
                .iter()
                .any(|n| n.to_bits() != 0)
            || (playback.completed && matches!(state.motion, NativeAnimationMotion::Clip(None)))
            || (playback.completed
                && (playback.playing
                    || playback.looping
                    || (graph.seconds != 0.0 && graph.seconds != duration)))
            || (playback.transition.is_some()
                && !matches!(state.motion, NativeAnimationMotion::Clip(_)))
        {
            return Err(SceneError::Invalid("animation graph checkpoint values"));
        }
        if let Some(fade) = playback.transition
            && (graph.elapsed != fade.elapsed
                || !self.transitions.iter().any(|edge| {
                    edge.to == graph.current
                        && edge.fade_seconds == fade.duration
                        && self.states[usize::from(edge.from)].looping == fade.looping
                        && matches!(
                            self.states[usize::from(edge.from)].motion,
                            NativeAnimationMotion::Clip(clip) if clip == fade.source.clip
                        )
                }))
        {
            return Err(SceneError::Invalid("animation graph checkpoint transition"));
        }
        if self.sample(graph, definition).0 != playback.sample {
            return Err(SceneError::Invalid("animation graph checkpoint sample"));
        }
        Ok(())
    }
}

impl crate::animation::Playback {
    fn graph_snapshot(&self, entity: StableId) -> Option<AnimationGraphSnapshot> {
        let state = self.graph?;
        let definition = self.definition.graph.as_ref()?;
        let mut parameter_ids = [StableId([0; 16]); MAX_ANIMATION_GRAPH_PARAMETERS];
        for (slot, parameter) in parameter_ids.iter_mut().zip(definition.parameters.iter()) {
            *slot = parameter.id;
        }
        Some(AnimationGraphSnapshot {
            entity,
            state: definition.states[usize::from(state.current)].id,
            paused: state.paused,
            seconds: state.seconds,
            elapsed: state.elapsed,
            parameters: state.parameters,
            parameter_count: definition.parameters.len() as u8,
            parameter_ids,
            blend: self.blend_sample(),
            pose: self.pose_sample(),
        })
    }
}
impl crate::NativeWorld {
    pub fn animation_graph_state(&self, entity: StableId) -> Option<AnimationGraphSnapshot> {
        if self.physics_failed {
            return None;
        }
        self.world
            .get::<crate::animation::Playback>(*self.entities.get(&entity)?)?
            .graph_snapshot(entity)
    }
    pub fn snapshot_animation_graphs(
        &mut self,
        rows: &mut Vec<AnimationGraphSnapshot>,
    ) -> Result<(), crate::RuntimeError> {
        self.ensure_usable()?;
        rows.clear();
        for (id, playback) in self.animation_query.iter(&self.world) {
            if let Some(row) = playback.graph_snapshot(id.0) {
                if rows.len() == MAX_ANIMATION_GRAPH_INSTANCES {
                    rows.clear();
                    return Err(SceneError::Invalid("animation graph snapshot quota").into());
                }
                rows.push(row);
            }
        }
        Ok(())
    }
}

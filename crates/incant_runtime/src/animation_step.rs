//! Copied candidate preparation for the pre-physics root-motion boundary.
//! The ordinary animation system keeps its existing unchecked admitted path.
mod clock;
#[cfg(test)]
mod tests;
use crate::{
    AnimationGraphState, AnimationSample, AnimationState, NativeAnimation, NativeAnimationMotion,
    SceneError, animation::Playback,
};
use incant_types::AnimationPoseSamples;

/// A state owner is captured before automatic edges run. A legacy clip fade
/// lacks a state ID; root state retains that edge provenance separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClockOwner {
    State(u8),
    Clip(Option<u16>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ClockSpan {
    pub owner: ClockOwner,
    pub before: f64,
    pub after: f64,
    pub duration: f64,
    pub wraps: i64,
    pub looping: bool,
    pub playing: bool,
    pub completed: bool,
    pub before_pose: AnimationPoseSamples,
    pub after_pose: AnimationPoseSamples,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct FadeSpan {
    pub before: f64,
    /// Unclamped interval end: the average includes any time after completion.
    pub after: f64,
    pub duration: f64,
    pub mean_weight: f64,
}
impl FadeSpan {
    fn new(before: f64, duration: f64, playing: bool, dt: f64) -> Result<Self, SceneError> {
        if !before.is_finite()
            || !duration.is_finite()
            || duration <= 0.0
            || before < 0.0
            || before >= duration
        {
            return Err(SceneError::Invalid("root-motion fade interval"));
        }
        let after = before + if playing { dt } else { 0.0 };
        if !after.is_finite() {
            return Err(SceneError::Invalid("root-motion fade overflow"));
        }
        let remaining = duration - before;
        let mean_weight = if !playing {
            before / duration
        } else if dt <= remaining {
            (before + dt * 0.5) / duration
        } else {
            // Integrate the linear segment then the constant-one segment.
            ((before / duration + 1.0) * 0.5 * remaining + (dt - remaining)) / dt
        };
        Ok(Self {
            before,
            after,
            duration,
            mean_weight,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct PreparedAnimationStep {
    pub state: AnimationState,
    pub graph: Option<AnimationGraphState>,
    pub current: ClockSpan,
    pub outgoing: Option<ClockSpan>,
    pub fade: Option<FadeSpan>,
}

impl PreparedAnimationStep {
    /// Prepare from immutable accepted state. Refusal never mutates playback;
    /// full catalog/model validation remains a cold-admission responsibility.
    pub fn new(playback: &Playback, dt: f64) -> Result<Self, SceneError> {
        if !dt.is_finite() || dt <= 0.0 {
            return Err(SceneError::Invalid("root-motion fixed interval"));
        }
        let definition = &playback.definition;
        let state = playback.state;
        let mut next_state = state;
        let mut next_graph = playback.graph;
        let (current, outgoing, fade) =
            if let (Some(graph), Some(g)) = (&definition.graph, playback.graph) {
                let current = graph_span(
                    definition,
                    &g,
                    g.current,
                    g.seconds,
                    state.speed,
                    state.playing && !g.paused,
                    dt,
                )?;
                let (outgoing, fade) = if let Some(f) = g.fade {
                    (
                        Some(graph_span(
                            definition,
                            &g,
                            f.state,
                            f.seconds,
                            f.speed,
                            f.playing && !g.paused,
                            dt,
                        )?),
                        Some(FadeSpan::new(f.elapsed, f.duration, !g.paused, dt)?),
                    )
                } else if let Some(t) = state.transition {
                    (
                        Some(clip_span(
                            definition,
                            t.source,
                            t.speed,
                            t.looping,
                            state.playing && !g.paused && t.playing,
                            dt,
                        )?),
                        Some(FadeSpan::new(
                            t.elapsed,
                            t.duration,
                            state.playing && !g.paused,
                            dt,
                        )?),
                    )
                } else {
                    (None, None)
                };
                // Preserve exact existing edge ordering and completion behavior.
                graph.advance(
                    next_graph.as_mut().expect("copied graph"),
                    &mut next_state,
                    definition,
                    dt,
                );
                (current, outgoing, fade)
            } else {
                let current = clip_span(
                    definition,
                    state.sample,
                    state.speed,
                    state.looping,
                    state.playing,
                    dt,
                )?;
                let (outgoing, fade) = if let Some(t) = state.transition {
                    (
                        Some(clip_span(
                            definition,
                            t.source,
                            t.speed,
                            t.looping,
                            state.playing && t.playing,
                            dt,
                        )?),
                        Some(FadeSpan::new(t.elapsed, t.duration, state.playing, dt)?),
                    )
                } else {
                    (None, None)
                };
                crate::animation::advance_state(&mut next_state, definition, dt);
                (current, outgoing, fade)
            };
        Ok(Self {
            state: next_state,
            graph: next_graph,
            current,
            outgoing,
            fade,
        })
    }
}

fn graph_span(
    definition: &NativeAnimation,
    state: &AnimationGraphState,
    current: u8,
    seconds: f64,
    speed: f64,
    playing: bool,
    dt: f64,
) -> Result<ClockSpan, SceneError> {
    let graph = definition.graph.as_ref().expect("admitted graph");
    let motion = &graph.states()[usize::from(current)];
    let duration = graph.state_duration(current, definition);
    let advance = playing && !matches!(motion.motion, NativeAnimationMotion::Clip(None));
    let (after, wraps, completed) =
        clock::advance(seconds, duration, speed, motion.looping, advance, dt)?;
    Ok(ClockSpan {
        owner: ClockOwner::State(current),
        before: seconds,
        after,
        duration,
        wraps,
        looping: motion.looping,
        playing,
        completed,
        before_pose: graph.state_pose(state, current, seconds, definition),
        after_pose: graph.state_pose(state, current, after, definition),
    })
}
fn clip_span(
    definition: &NativeAnimation,
    sample: AnimationSample,
    speed: f64,
    looping: bool,
    playing: bool,
    dt: f64,
) -> Result<ClockSpan, SceneError> {
    let duration = sample.clip.map_or(0.0, |c| {
        f64::from(definition.clip_durations[usize::from(c)])
    });
    let (after, wraps, completed) = clock::advance(
        sample.seconds,
        duration,
        speed,
        looping,
        playing && sample.clip.is_some(),
        dt,
    )?;
    Ok(ClockSpan {
        owner: ClockOwner::Clip(sample.clip),
        before: sample.seconds,
        after,
        duration,
        wraps,
        looping,
        playing,
        completed,
        before_pose: AnimationPoseSamples::clip(sample).expect("admitted sample"),
        after_pose: AnimationPoseSamples::clip(AnimationSample {
            seconds: after,
            ..sample
        })
        .expect("admitted sample"),
    })
}

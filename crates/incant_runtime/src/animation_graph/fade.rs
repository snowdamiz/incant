//! Independent outgoing state clock for transitions involving compound motions.
use super::*;
use crate::{AnimationState, NativeAnimation, SceneError, StableId};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct AnimationGraphFade {
    pub state: u8,
    pub seconds: f64,
    pub speed: f64,
    pub playing: bool,
    pub elapsed: f64,
    pub duration: f64,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationGraphFadeSnapshot {
    pub state: StableId,
    pub seconds: f64,
    pub speed: f64,
    pub playing: bool,
    pub elapsed: f64,
    pub duration: f64,
}
impl NativeAnimationGraph {
    pub(super) fn advance_fade(
        &self,
        graph: &mut AnimationGraphState,
        definition: &NativeAnimation,
        dt: f64,
    ) {
        let Some(fade) = graph.fade.as_mut() else {
            return;
        };
        let state = &self.states[usize::from(fade.state)];
        if fade.playing
            && !matches!(state.motion, NativeAnimationMotion::Clip(None))
            && crate::animation::advance_clock(
                &mut fade.seconds,
                self.state_duration(fade.state, definition),
                fade.speed,
                state.looping,
                dt,
            )
        {
            fade.playing = false;
        }
        fade.elapsed = (fade.elapsed + dt).min(fade.duration);
        let roundoff = f64::EPSILON * fade.duration * (fade.duration / dt + 1.0);
        if fade.duration - fade.elapsed <= roundoff {
            graph.fade = None;
        }
    }
    pub(super) fn validate_fade(
        &self,
        graph: &AnimationGraphState,
        playback: AnimationState,
        definition: &NativeAnimation,
    ) -> Result<(), SceneError> {
        let Some(fade) = graph.fade else {
            return Ok(());
        };
        let Some(state) = self.states.get(usize::from(fade.state)) else {
            return Err(SceneError::Invalid("animation graph fade source"));
        };
        let duration = self.state_duration(fade.state, definition);
        if playback.transition.is_some()
            || fade.state == graph.current
            || !fade.seconds.is_finite()
            || !(0.0..=duration).contains(&fade.seconds)
            || !fade.speed.is_finite()
            || fade.speed.abs() > incant_types::MAX_ANIMATION_SPEED
            || !fade.duration.is_finite()
            || fade.duration <= 0.0
            || fade.duration > crate::MAX_ANIMATION_TRANSITION_SECONDS
            || !fade.elapsed.is_finite()
            || fade.elapsed < 0.0
            || fade.elapsed >= fade.duration
            || fade.elapsed != graph.elapsed
            || (!fade.playing
                && (state.looping || (fade.seconds != 0.0 && fade.seconds != duration)))
            || (matches!(state.motion, NativeAnimationMotion::Clip(_))
                && matches!(
                    self.states[usize::from(graph.current)].motion,
                    NativeAnimationMotion::Clip(_)
                ))
            || !self.transitions.iter().any(|edge| {
                edge.from == fade.state
                    && edge.to == graph.current
                    && edge.fade_seconds == fade.duration
            })
        {
            return Err(SceneError::Invalid("animation graph fade values"));
        }
        Ok(())
    }
}

//! Exact retained cycle provenance independent of wrapped playback phases.
use super::{NativeRootMotion, interval};
use crate::{
    AnimationControl, AnimationGraphControl, SceneError,
    animation::Playback,
    animation_step::{ClockOwner, ClockSpan, PreparedAnimationStep},
};
use bevy_ecs::prelude::Component;
use incant_anim::RootMotionDelta;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Cursor {
    pub owner: ClockOwner,
    pub cycle: i64,
    pub seconds: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Component)]
pub(crate) struct RootMotionState {
    pub applying: bool,
    pub current: Cursor,
    pub outgoing: Option<Cursor>,
    pub requested: RootMotionDelta,
    pub applied: RootMotionDelta,
    pub accumulated_requested: RootMotionDelta,
    pub accumulated_applied: RootMotionDelta,
    pub tick: u64,
}

impl RootMotionState {
    pub(crate) fn validate(&self, playback: &Playback, tick: u64) -> Result<(), SceneError> {
        let expected = cursor(playback);
        if self.current.owner != expected.owner
            || self.current.seconds != expected.seconds
            || self.tick > tick
        {
            return Err(SceneError::Invalid("root-motion saved current clock"));
        }
        if (!playback.state.looping
            || playback.state.sample.clip.is_none() && playback.graph.is_none())
            && self.current.cycle != 0
        {
            return Err(SceneError::Invalid("root-motion saved nonloop cycle"));
        }
        match (
            self.outgoing,
            playback.graph.and_then(|g| g.fade),
            playback.state.transition,
        ) {
            (Some(out), Some(fade), None) => {
                if out.owner != ClockOwner::State(fade.state)
                    || out.seconds != fade.seconds
                    || (!playback
                        .definition
                        .graph
                        .as_ref()
                        .expect("graph fade")
                        .states()[usize::from(fade.state)]
                    .looping
                        && out.cycle != 0)
                {
                    return Err(SceneError::Invalid(
                        "root-motion saved outgoing graph clock",
                    ));
                }
            }
            (Some(out), None, Some(fade)) => {
                let owner=match (&playback.definition.graph,playback.graph,out.owner) {
                    (None,None,ClockOwner::Clip(clip))=>clip==fade.source.clip,
                    (Some(def),Some(graph),ClockOwner::State(index))=>def.states().get(usize::from(index)).is_some_and(|s|matches!(s.motion,crate::NativeAnimationMotion::Clip(c) if c==fade.source.clip))
                        && def.transitions().iter().any(|e|e.from==index && e.to==graph.current && e.fade_seconds==fade.duration),
                    _=>false,
                };
                if !owner || out.seconds != fade.source.seconds || (!fade.looping && out.cycle != 0)
                {
                    return Err(SceneError::Invalid(
                        "root-motion saved outgoing clip provenance",
                    ));
                }
            }
            (None, None, None) => {}
            _ => return Err(SceneError::Invalid("root-motion saved fade membership")),
        }
        for delta in [
            self.requested,
            self.applied,
            self.accumulated_requested,
            self.accumulated_applied,
        ] {
            delta
                .compose(RootMotionDelta::IDENTITY)
                .map_err(|_| SceneError::Invalid("root-motion saved rigid delta"))?;
        }
        Ok(())
    }
    pub fn new(definition: &NativeRootMotion, playback: &Playback, tick: u64) -> Self {
        Self {
            applying: definition.applying,
            current: cursor(playback),
            outgoing: playback
                .graph
                .and_then(|g| g.fade)
                .map(|f| Cursor {
                    owner: ClockOwner::State(f.state),
                    cycle: 0,
                    seconds: f.seconds,
                })
                .or_else(|| {
                    playback.state.transition.map(|t| Cursor {
                        owner: ClockOwner::Clip(t.source.clip),
                        cycle: 0,
                        seconds: t.source.seconds,
                    })
                }),
            requested: RootMotionDelta::IDENTITY,
            applied: RootMotionDelta::IDENTITY,
            accumulated_requested: RootMotionDelta::IDENTITY,
            accumulated_applied: RootMotionDelta::IDENTITY,
            tick,
        }
    }

    /// Explicit clock-changing controls rebase even if their requested sample
    /// equals the current one. They never apply a delta themselves. A new fade
    /// retains the actual outgoing owner's cycle; no clip-to-state guessing.
    pub fn animation_control(
        &mut self,
        before: &Playback,
        after: &Playback,
        control: AnimationControl,
    ) {
        match control {
            AnimationControl::Crossfade { duration, .. } if duration > 0.0 => {
                self.outgoing = Some(self.current);
                self.current = cursor(after);
            }
            AnimationControl::Select(_)
            | AnimationControl::Seek(_)
            | AnimationControl::Crossfade { .. } => {
                self.current = cursor(after);
                self.outgoing = None;
            }
            AnimationControl::SetLooping(looping) if looping != before.state.looping => {
                self.current = cursor(after);
            }
            _ => {}
        }
    }
    pub fn graph_control(&mut self, after: &Playback, control: AnimationGraphControl) {
        if matches!(control, AnimationGraphControl::SelectState(_)) {
            self.current = cursor(after);
            self.outgoing = None;
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PreparedRootMotion {
    pub animation: PreparedAnimationStep,
    pub root: RootMotionState,
}
impl PreparedRootMotion {
    pub fn new(
        definition: &NativeRootMotion,
        playback: &Playback,
        accepted: RootMotionState,
        dt: f64,
        tick: u64,
    ) -> Result<Self, SceneError> {
        let animation = PreparedAnimationStep::new(playback, dt)?;
        verify_cursor(accepted.current, &animation.current, playback)?;
        let (incoming, cycle) = interval::sample(
            &definition.catalog,
            &animation.current,
            accepted.current.cycle,
        )?;
        let advanced = Cursor {
            cycle,
            seconds: animation.current.after,
            ..accepted.current
        };
        let (delta, outgoing) = if let Some(span) = &animation.outgoing {
            let cursor = accepted.outgoing.ok_or(SceneError::Invalid(
                "root-motion missing outgoing provenance",
            ))?;
            verify_cursor(cursor, span, playback)?;
            let (source, cycle) = interval::sample(&definition.catalog, span, cursor.cycle)?;
            let delta = interval::spherical(
                source,
                incoming,
                animation.fade.expect("outgoing fade").mean_weight,
            )?;
            (
                delta,
                Some(Cursor {
                    cycle,
                    seconds: span.after,
                    ..cursor
                }),
            )
        } else {
            if accepted.outgoing.is_some() {
                return Err(SceneError::Invalid(
                    "root-motion unexpected outgoing provenance",
                ));
            }
            (incoming, None)
        };
        let reference = definition.catalog.model_reference();
        let requested = if delta == RootMotionDelta::IDENTITY {
            delta
        } else {
            reference
                .compose(delta)
                .and_then(|delta| delta.compose(reference.inverse()?))
                .map_err(|_| SceneError::Invalid("root-motion model-space overflow"))?
        };
        let mut root = RootMotionState {
            current: advanced,
            outgoing,
            requested,
            applied: RootMotionDelta::IDENTITY,
            tick,
            ..accepted
        };
        if requested != RootMotionDelta::IDENTITY {
            root.accumulated_requested = root
                .accumulated_requested
                .compose(requested)
                .map_err(|_| SceneError::Invalid("root-motion accumulated request overflow"))?;
        }
        if let (Some(old), Some(new)) = (playback.graph, animation.graph)
            && old.current != new.current
        {
            // An automatic edge is selected only after this interval. Its new
            // incoming state starts from zero; a fade retains pre-edge identity.
            root.current = Cursor {
                owner: ClockOwner::State(new.current),
                cycle: 0,
                seconds: new.seconds,
            };
            root.outgoing =
                (new.fade.is_some() || animation.state.transition.is_some()).then_some(advanced);
        } else if animation.state.transition.is_none()
            && animation.graph.is_none_or(|g| g.fade.is_none())
        {
            root.outgoing = None;
        }
        Ok(Self { animation, root })
    }
}

fn cursor(playback: &Playback) -> Cursor {
    match playback.graph {
        Some(graph) => Cursor {
            owner: ClockOwner::State(graph.current),
            cycle: 0,
            seconds: graph.seconds,
        },
        None => Cursor {
            owner: ClockOwner::Clip(playback.state.sample.clip),
            cycle: 0,
            seconds: playback.state.sample.seconds,
        },
    }
}
fn verify_cursor(cursor: Cursor, span: &ClockSpan, playback: &Playback) -> Result<(), SceneError> {
    let owner_matches = cursor.owner == span.owner || match (cursor.owner, span.owner) {
        (ClockOwner::State(state), ClockOwner::Clip(clip)) => playback
            .definition
            .graph
            .as_ref()
            .and_then(|g| g.states().get(usize::from(state)))
            .is_some_and(
                |state| matches!(state.motion, crate::NativeAnimationMotion::Clip(c) if c == clip),
            ),
        _ => false,
    };
    if !owner_matches || cursor.seconds != span.before {
        return Err(SceneError::Invalid("root-motion clock provenance mismatch"));
    }
    Ok(())
}

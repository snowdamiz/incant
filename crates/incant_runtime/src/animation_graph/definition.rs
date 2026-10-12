use crate::{SceneError, StableId};
use incant_types::{
    AnimationComparison, MAX_ANIMATION_BLEND_POINTS, MAX_ANIMATION_GRAPH_CONDITIONS,
    MAX_ANIMATION_GRAPH_PARAMETERS, MAX_ANIMATION_GRAPH_SECONDS, MAX_ANIMATION_GRAPH_STATES,
    MAX_ANIMATION_GRAPH_TRANSITIONS, MAX_ANIMATION_GRAPH_VALUE,
};
use std::{collections::BTreeSet, sync::Arc};

pub const MAX_ANIMATION_GRAPH_INSTANCES: usize = 1024;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeAnimationParameter {
    pub id: StableId,
    pub initial: f64,
    pub min: f64,
    pub max: f64,
}
#[derive(Debug, Clone, PartialEq)]
pub struct NativeAnimationGraphState {
    pub id: StableId,
    pub motion: NativeAnimationMotion,
    pub looping: bool,
}
#[derive(Debug, Clone, PartialEq)]
pub enum NativeAnimationMotion {
    Clip(Option<u16>),
    Tree {
        cycle_seconds: f64,
        tree: Arc<super::NativeAnimationBlendTree>,
    },
    Blend1d {
        parameter: u8,
        cycle_seconds: f64,
        points: Arc<[NativeAnimationBlendPoint]>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeAnimationBlendPoint {
    pub threshold: f64,
    pub clip: u16,
}
#[derive(Debug, Clone, PartialEq)]
pub struct NativeAnimationGraphTransition {
    pub id: StableId,
    pub from: u8,
    pub to: u8,
    pub priority: i16,
    pub fade_seconds: f64,
    pub after_seconds: Option<f64>,
    pub conditions: Arc<[NativeAnimationCondition]>,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeAnimationCondition {
    pub parameter: u8,
    pub comparison: AnimationComparison,
    pub value: f64,
}
/// Checked immutable numeric tables. The cooker resolves canonical ULIDs into
/// table indices; binary readers repeat all bounds/reference checks before use.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeAnimationGraph {
    pub(super) initial: u8,
    pub(super) parameters: Arc<[NativeAnimationParameter]>,
    pub(super) states: Arc<[NativeAnimationGraphState]>,
    pub(super) transitions: Arc<[NativeAnimationGraphTransition]>,
    pub(super) transition_starts: [u8; MAX_ANIMATION_GRAPH_STATES + 1],
}
impl NativeAnimationGraph {
    pub fn new(
        initial: u8,
        parameters: Vec<NativeAnimationParameter>,
        states: Vec<NativeAnimationGraphState>,
        transitions: Vec<NativeAnimationGraphTransition>,
        clip_durations: &[f32],
    ) -> Result<Self, SceneError> {
        let mut result = Self {
            initial,
            parameters: parameters.into(),
            states: states.into(),
            transitions: transitions.into(),
            transition_starts: [0; MAX_ANIMATION_GRAPH_STATES + 1],
        };
        result.validate(clip_durations)?;
        for state in 0..=result.states.len() {
            result.transition_starts[state] = result
                .transitions
                .partition_point(|transition| usize::from(transition.from) < state)
                as u8;
        }
        Ok(result)
    }
    pub fn parameters(&self) -> &[NativeAnimationParameter] {
        &self.parameters
    }
    pub fn states(&self) -> &[NativeAnimationGraphState] {
        &self.states
    }
    pub fn transitions(&self) -> &[NativeAnimationGraphTransition] {
        &self.transitions
    }
    pub fn initial(&self) -> u8 {
        self.initial
    }
    pub(crate) fn has_compound_fades(&self) -> bool {
        self.transitions.iter().any(|t| {
            t.fade_seconds > 0.0
                && (!matches!(
                    self.states[usize::from(t.from)].motion,
                    NativeAnimationMotion::Clip(_)
                ) || !matches!(
                    self.states[usize::from(t.to)].motion,
                    NativeAnimationMotion::Clip(_)
                ))
        })
    }
    pub fn validate(&self, clip_durations: &[f32]) -> Result<(), SceneError> {
        let error = SceneError::Invalid;
        if clip_durations.len() > crate::MAX_ANIMATION_CLIPS
            || clip_durations
                .iter()
                .any(|d| !d.is_finite() || *d < 0.0 || d.to_bits() == (-0.0f32).to_bits())
            || self.parameters.len() > MAX_ANIMATION_GRAPH_PARAMETERS
            || self.states.is_empty()
            || self.states.len() > MAX_ANIMATION_GRAPH_STATES
            || usize::from(self.initial) >= self.states.len()
            || self.transitions.len() > MAX_ANIMATION_GRAPH_TRANSITIONS
            || self.parameters.windows(2).any(|p| p[0].id >= p[1].id)
            || self.states.windows(2).any(|p| p[0].id >= p[1].id)
            || self
                .transitions
                .windows(2)
                .any(|p| (p[0].from, p[0].priority, p[0].id) >= (p[1].from, p[1].priority, p[1].id))
        {
            return Err(error("animation graph capacity or canonical order"));
        }
        let mut ids = BTreeSet::new();
        for id in self
            .parameters
            .iter()
            .map(|p| p.id)
            .chain(self.states.iter().map(|s| s.id))
            .chain(self.transitions.iter().map(|t| t.id))
        {
            if !ids.insert(id) {
                return Err(error("animation graph duplicate ID"));
            }
        }
        let bounded = |value: f64| value.is_finite() && value.abs() <= MAX_ANIMATION_GRAPH_VALUE;
        let valid_clip = |clip: u16| usize::from(clip) < clip_durations.len();
        for p in self.parameters.iter() {
            if !bounded(p.initial)
                || !bounded(p.min)
                || !bounded(p.max)
                || p.min > p.max
                || p.initial < p.min
                || p.initial > p.max
            {
                return Err(error("animation graph parameter range"));
            }
        }
        for state in self.states.iter() {
            match &state.motion {
                NativeAnimationMotion::Tree {
                    cycle_seconds,
                    tree,
                } => {
                    if !cycle_seconds.is_finite()
                        || !(0.000001..=MAX_ANIMATION_GRAPH_SECONDS).contains(cycle_seconds)
                        || !tree.matches_context(&self.parameters, clip_durations)
                    {
                        return Err(error("animation tree duration or model context"));
                    }
                }
                NativeAnimationMotion::Clip(clip) => {
                    if clip.is_some_and(|clip| !valid_clip(clip)) {
                        return Err(error("animation graph model clip"));
                    }
                }
                NativeAnimationMotion::Blend1d {
                    parameter,
                    cycle_seconds,
                    points,
                } => {
                    let p = self
                        .parameters
                        .get(usize::from(*parameter))
                        .ok_or_else(|| error("animation graph blend parameter"))?;
                    if !cycle_seconds.is_finite()
                        || !(0.000001..=MAX_ANIMATION_GRAPH_SECONDS).contains(cycle_seconds)
                        || !(2..=MAX_ANIMATION_BLEND_POINTS).contains(&points.len())
                        || points.iter().any(|point| {
                            !bounded(point.threshold)
                                || point.threshold < p.min
                                || point.threshold > p.max
                                || !valid_clip(point.clip)
                        })
                        || points.windows(2).any(|p| p[0].threshold >= p[1].threshold)
                    {
                        return Err(error("animation graph blend points or duration"));
                    }
                }
            }
        }
        for t in self.transitions.iter() {
            self.states
                .get(usize::from(t.from))
                .ok_or_else(|| error("animation graph transition source"))?;
            self.states
                .get(usize::from(t.to))
                .ok_or_else(|| error("animation graph transition destination"))?;
            if t.from == t.to
                || !t.fade_seconds.is_finite()
                || !(0.0..=crate::MAX_ANIMATION_TRANSITION_SECONDS).contains(&t.fade_seconds)
                || t.after_seconds.is_some_and(|n| {
                    !n.is_finite() || !(0.0..=MAX_ANIMATION_GRAPH_SECONDS).contains(&n)
                })
                || t.conditions.len() > MAX_ANIMATION_GRAPH_CONDITIONS
            {
                return Err(error("animation graph transition duration or conditions"));
            }
            for condition in t.conditions.iter() {
                let p = self
                    .parameters
                    .get(usize::from(condition.parameter))
                    .ok_or_else(|| error("animation graph condition parameter"))?;
                if !bounded(condition.value) || condition.value < p.min || condition.value > p.max {
                    return Err(error("animation graph condition threshold"));
                }
            }
        }
        Ok(())
    }
}

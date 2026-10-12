//! Bounded authored animation graph values. Compilation resolves every ID and
//! clip once; the simulation consumes numeric native tables, never this schema.
use crate::{Id, MAX_ANIMATION_CLIPS};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_ANIMATION_GRAPH_PARAMETERS: usize = 16;
pub const MAX_ANIMATION_GRAPH_STATES: usize = 32;
pub const MAX_ANIMATION_GRAPH_TRANSITIONS: usize = 128;
pub const MAX_ANIMATION_GRAPH_CONDITIONS: usize = 8;
pub const MAX_ANIMATION_BLEND_POINTS: usize = 8;
pub const MAX_ANIMATION_GRAPH_VALUE: f64 = 1_000_000.0;
pub const MAX_ANIMATION_GRAPH_SECONDS: f64 = 3600.0;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnimationGraph {
    pub initial: Id,
    #[schemars(length(max = 16))]
    pub parameters: BTreeMap<Id, AnimationParameter>,
    #[schemars(length(min = 1, max = 32))]
    pub states: BTreeMap<Id, AnimationGraphState>,
    #[schemars(length(max = 128))]
    pub transitions: BTreeMap<Id, AnimationGraphTransition>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnimationParameter {
    pub initial: f64,
    pub min: f64,
    pub max: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnimationGraphState {
    pub motion: AnimationMotion,
    pub looping: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AnimationMotion {
    Clip {
        clip: Option<u16>,
    },
    /// Adjacent threshold clips share one normalized clock. Values outside the
    /// point interval clamp to its endpoints; no extrapolation or hidden child.
    Blend1d {
        parameter: Id,
        cycle_seconds: f64,
        #[schemars(length(min = 2, max = 8))]
        points: Vec<AnimationBlendPoint>,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnimationBlendPoint {
    pub threshold: f64,
    pub clip: u16,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnimationGraphTransition {
    pub from: Id,
    pub to: Id,
    /// Lower values win, with transition ULID as the stable tie breaker.
    pub priority: i16,
    /// Nonzero fades currently require single-clip source and destination states.
    pub fade_seconds: f64,
    /// Minimum committed time in the current state. None has no time condition.
    pub after_seconds: Option<f64>,
    #[schemars(length(max = 8))]
    pub conditions: Vec<AnimationCondition>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnimationCondition {
    pub parameter: Id,
    pub comparison: AnimationComparison,
    pub value: f64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnimationComparison {
    Less,
    LessEqual,
    Equal,
    GreaterEqual,
    Greater,
}
impl AnimationComparison {
    pub fn accepts(self, actual: f64, threshold: f64) -> bool {
        match self {
            Self::Less => actual < threshold,
            Self::LessEqual => actual <= threshold,
            Self::Equal => actual == threshold,
            Self::GreaterEqual => actual >= threshold,
            Self::Greater => actual > threshold,
        }
    }
}

impl AnimationGraph {
    /// Cold shape/reference validation. Model-specific clip checks happen once
    /// during cooking and again when admitting the native binary definition.
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.parameters.len() > MAX_ANIMATION_GRAPH_PARAMETERS
            || self.states.is_empty()
            || self.states.len() > MAX_ANIMATION_GRAPH_STATES
            || self.transitions.len() > MAX_ANIMATION_GRAPH_TRANSITIONS
            || !self.states.contains_key(&self.initial)
            || self
                .parameters
                .keys()
                .chain(self.states.keys())
                .chain(self.transitions.keys())
                .any(|id| !canonical_id(id))
        {
            return Err("animation graph capacity, canonical ID or initial state");
        }
        let mut ids = BTreeSet::new();
        if self
            .parameters
            .keys()
            .chain(self.states.keys())
            .chain(self.transitions.keys())
            .any(|id| !ids.insert(id))
        {
            return Err("animation graph duplicate ID");
        }
        for p in self.parameters.values() {
            if !value(p.min)
                || !value(p.max)
                || !value(p.initial)
                || p.min > p.max
                || p.initial < p.min
                || p.initial > p.max
            {
                return Err("animation graph parameter range");
            }
        }
        for state in self.states.values() {
            match &state.motion {
                AnimationMotion::Clip { clip } => {
                    if clip.is_some_and(|c| usize::from(c) >= MAX_ANIMATION_CLIPS) {
                        return Err("animation graph clip index");
                    }
                }
                AnimationMotion::Blend1d {
                    parameter,
                    cycle_seconds,
                    points,
                } => {
                    let p = self
                        .parameters
                        .get(parameter)
                        .ok_or("animation blend parameter")?;
                    if !cycle_seconds.is_finite()
                        || !(0.000001..=MAX_ANIMATION_GRAPH_SECONDS).contains(cycle_seconds)
                        || !(2..=MAX_ANIMATION_BLEND_POINTS).contains(&points.len())
                        || points.iter().any(|point| {
                            !value(point.threshold)
                                || point.threshold < p.min
                                || point.threshold > p.max
                                || usize::from(point.clip) >= MAX_ANIMATION_CLIPS
                        })
                        || points
                            .windows(2)
                            .any(|pair| pair[0].threshold >= pair[1].threshold)
                    {
                        return Err("animation blend points or cycle duration");
                    }
                }
            }
        }
        for transition in self.transitions.values() {
            let from = self
                .states
                .get(&transition.from)
                .ok_or("animation transition source")?;
            let to = self
                .states
                .get(&transition.to)
                .ok_or("animation transition destination")?;
            if transition.from == transition.to
                || !transition.fade_seconds.is_finite()
                || !(0.0..=60.0).contains(&transition.fade_seconds)
                || transition.after_seconds.is_some_and(|n| {
                    !n.is_finite() || !(0.0..=MAX_ANIMATION_GRAPH_SECONDS).contains(&n)
                })
                || transition.conditions.len() > MAX_ANIMATION_GRAPH_CONDITIONS
            {
                return Err("animation transition duration, conditions or self edge");
            }
            if transition.fade_seconds != 0.0
                && (!matches!(from.motion, AnimationMotion::Clip { .. })
                    || !matches!(to.motion, AnimationMotion::Clip { .. }))
            {
                return Err("fades involving blend-tree states are unsupported");
            }
            for condition in &transition.conditions {
                let parameter = self
                    .parameters
                    .get(&condition.parameter)
                    .ok_or("animation condition parameter")?;
                if !value(condition.value)
                    || condition.value < parameter.min
                    || condition.value > parameter.max
                {
                    return Err("animation condition threshold");
                }
            }
        }
        Ok(())
    }
}
fn value(n: f64) -> bool {
    n.is_finite() && n.abs() <= MAX_ANIMATION_GRAPH_VALUE
}
fn canonical_id(id: &str) -> bool {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
    id.len() == 26 && id.as_bytes()[0] <= b'7' && id.bytes().all(|b| ALPHABET.contains(&b))
}

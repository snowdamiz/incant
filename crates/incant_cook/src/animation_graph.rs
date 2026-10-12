//! Resolve authored stable IDs into immutable native indices at cook time.
use crate::{CookError, stable_id};
use incant_doc::{AnimationGraph, AnimationMotion};
use incant_runtime::{
    NativeAnimationBlendPoint, NativeAnimationCondition, NativeAnimationGraph,
    NativeAnimationGraphState, NativeAnimationGraphTransition, NativeAnimationMotion,
    NativeAnimationParameter,
};
use std::{collections::BTreeMap, sync::Arc};

pub(crate) fn compile(
    graph: &AnimationGraph,
    durations: &[f32],
) -> Result<Arc<NativeAnimationGraph>, CookError> {
    graph
        .validate()
        .map_err(incant_runtime::SceneError::Invalid)?;
    let parameters: BTreeMap<_, _> = graph
        .parameters
        .keys()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i as u8))
        .collect();
    let states: BTreeMap<_, _> = graph
        .states
        .keys()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i as u8))
        .collect();
    let initial = states[graph.initial.as_str()];
    let native_parameters = graph
        .parameters
        .iter()
        .map(|(id, p)| {
            Ok(NativeAnimationParameter {
                id: stable_id(id)?,
                initial: p.initial,
                min: p.min,
                max: p.max,
            })
        })
        .collect::<Result<Vec<_>, CookError>>()?;
    let native_states = graph
        .states
        .iter()
        .map(|(id, state)| {
            Ok(NativeAnimationGraphState {
                id: stable_id(id)?,
                looping: state.looping,
                motion: match &state.motion {
                    AnimationMotion::Clip { clip } => NativeAnimationMotion::Clip(*clip),
                    AnimationMotion::Blend1d {
                        parameter,
                        cycle_seconds,
                        points,
                    } => NativeAnimationMotion::Blend1d {
                        parameter: parameters[parameter.as_str()],
                        cycle_seconds: *cycle_seconds,
                        points: points
                            .iter()
                            .map(|p| NativeAnimationBlendPoint {
                                threshold: p.threshold,
                                clip: p.clip,
                            })
                            .collect(),
                    },
                },
            })
        })
        .collect::<Result<Vec<_>, CookError>>()?;
    let mut native_transitions = graph
        .transitions
        .iter()
        .map(|(id, t)| {
            Ok(NativeAnimationGraphTransition {
                id: stable_id(id)?,
                from: states[t.from.as_str()],
                to: states[t.to.as_str()],
                priority: t.priority,
                fade_seconds: t.fade_seconds,
                after_seconds: t.after_seconds,
                conditions: t
                    .conditions
                    .iter()
                    .map(|c| NativeAnimationCondition {
                        parameter: parameters[c.parameter.as_str()],
                        comparison: c.comparison,
                        value: c.value,
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, CookError>>()?;
    native_transitions.sort_by_key(|t| (t.from, t.priority, t.id));
    Ok(Arc::new(NativeAnimationGraph::new(
        initial,
        native_parameters,
        native_states,
        native_transitions,
        durations,
    )?))
}

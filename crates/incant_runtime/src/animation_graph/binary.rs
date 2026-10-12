//! Bounded v14 graph suffix. The v11 animation records remain byte-identical.
use super::*;
use crate::{CookedSpatial, SceneError, StableId, binary::Reader};
use incant_types::{
    AnimationComparison, MAX_ANIMATION_BLEND_POINTS, MAX_ANIMATION_GRAPH_CONDITIONS,
    MAX_ANIMATION_GRAPH_PARAMETERS, MAX_ANIMATION_GRAPH_STATES, MAX_ANIMATION_GRAPH_TRANSITIONS,
};
use std::sync::Arc;

const MIN_RECORD: usize = 40; // entity + table counts + one clip state
pub(crate) fn encoded_size(graph: &NativeAnimationGraph) -> usize {
    20 + graph.parameters.len() * 40
        + graph
            .states
            .iter()
            .map(|state| {
                18 + match &state.motion {
                    NativeAnimationMotion::Clip(_) => 2,
                    NativeAnimationMotion::Tree { tree, .. } => 8 + super::tree::binary::size(tree),
                    NativeAnimationMotion::Blend1d { points, .. } => 10 + points.len() * 10,
                }
            })
            .sum::<usize>()
        + graph
            .transitions
            .iter()
            .map(|edge| 37 + edge.conditions.len() * 10)
            .sum::<usize>()
}
pub(crate) fn write(bytes: &mut Vec<u8>, nodes: &[CookedSpatial]) {
    let records = nodes
        .iter()
        .filter_map(|n| n.animation.as_ref()?.graph.as_ref().map(|g| (n.entity, g)));
    bytes.extend((records.clone().count() as u32).to_le_bytes());
    for (entity, graph) in records {
        bytes.extend(entity.0);
        bytes.extend([
            graph.initial,
            graph.parameters.len() as u8,
            graph.states.len() as u8,
            graph.transitions.len() as u8,
        ]);
        for p in graph.parameters.iter() {
            bytes.extend(p.id.0);
            for n in [p.initial, p.min, p.max] {
                bytes.extend(n.to_le_bytes());
            }
        }
        for state in graph.states.iter() {
            bytes.extend(state.id.0);
            bytes.push(u8::from(state.looping));
            match &state.motion {
                NativeAnimationMotion::Tree {
                    cycle_seconds,
                    tree,
                } => {
                    bytes.push(2);
                    bytes.extend(cycle_seconds.to_le_bytes());
                    super::tree::binary::write(bytes, tree);
                }
                NativeAnimationMotion::Clip(clip) => {
                    bytes.push(0);
                    bytes.extend(clip.unwrap_or(u16::MAX).to_le_bytes());
                }
                NativeAnimationMotion::Blend1d {
                    parameter,
                    cycle_seconds,
                    points,
                } => {
                    bytes.extend([1, *parameter, points.len() as u8]);
                    bytes.extend(cycle_seconds.to_le_bytes());
                    for p in points.iter() {
                        bytes.extend(p.threshold.to_le_bytes());
                        bytes.extend(p.clip.to_le_bytes());
                    }
                }
            }
        }
        for edge in graph.transitions.iter() {
            bytes.extend(edge.id.0);
            bytes.extend([edge.from, edge.to]);
            bytes.extend(edge.priority.to_le_bytes());
            bytes.extend(edge.fade_seconds.to_le_bytes());
            bytes.extend(edge.after_seconds.unwrap_or(-1.0).to_le_bytes());
            bytes.push(edge.conditions.len() as u8);
            for c in edge.conditions.iter() {
                bytes.extend([
                    c.parameter,
                    match c.comparison {
                        AnimationComparison::Less => 0,
                        AnimationComparison::LessEqual => 1,
                        AnimationComparison::Equal => 2,
                        AnimationComparison::GreaterEqual => 3,
                        AnimationComparison::Greater => 4,
                    },
                ]);
                bytes.extend(c.value.to_le_bytes());
            }
        }
    }
}
pub(crate) fn read(
    reader: &mut Reader<'_>,
    nodes: &mut [CookedSpatial],
    version: u32,
) -> Result<(), SceneError> {
    let count = reader.u32()? as usize;
    if count == 0
        || count > MAX_ANIMATION_GRAPH_INSTANCES
        || count > nodes.len()
        || count > reader.remaining_len() / MIN_RECORD
    {
        return Err(SceneError::Invalid("animation graph record count"));
    }
    let mut previous = None;
    for _ in 0..count {
        let id = StableId(reader.array()?);
        if previous.is_some_and(|p| p >= id) {
            return Err(SceneError::Invalid("animation graph entity order"));
        }
        previous = Some(id);
        let index = nodes
            .binary_search_by_key(&id, |n| n.entity)
            .map_err(|_| SceneError::Invalid("animation graph entity"))?;
        let animation = nodes[index]
            .animation
            .as_mut()
            .ok_or(SceneError::Invalid("animation graph requires playback"))?;
        let graph = read_graph(reader, &animation.clip_durations, version)?;
        Arc::make_mut(animation).graph = Some(Arc::new(graph));
        animation.validate()?;
    }
    Ok(())
}
fn read_graph(
    r: &mut Reader<'_>,
    durations: &[f32],
    version: u32,
) -> Result<NativeAnimationGraph, SceneError> {
    let [initial, np, ns, nt] = r.array()?;
    let (np, ns, nt) = (usize::from(np), usize::from(ns), usize::from(nt));
    if np > MAX_ANIMATION_GRAPH_PARAMETERS
        || !(1..=MAX_ANIMATION_GRAPH_STATES).contains(&ns)
        || nt > MAX_ANIMATION_GRAPH_TRANSITIONS
        || np * 40 + ns * 20 + nt * 37 > r.remaining_len()
    {
        return Err(SceneError::Invalid("animation graph table counts"));
    }
    let mut parameters = Vec::with_capacity(np);
    for _ in 0..np {
        let id = StableId(r.array()?);
        let [initial, min, max] = r.floats()?;
        parameters.push(NativeAnimationParameter {
            id,
            initial,
            min,
            max,
        });
    }
    let mut states = Vec::with_capacity(ns);
    for _ in 0..ns {
        let id = StableId(r.array()?);
        let looping = match r.byte()? {
            0 => false,
            1 => true,
            _ => return Err(SceneError::Invalid("animation graph looping")),
        };
        let motion = match r.byte()? {
            2 if version >= 15 => {
                let [cycle_seconds] = r.floats()?;
                NativeAnimationMotion::Tree {
                    cycle_seconds,
                    tree: Arc::new(super::tree::binary::read(r, &parameters, durations)?),
                }
            }
            0 => {
                let clip = u16::from_le_bytes(r.array()?);
                NativeAnimationMotion::Clip((clip != u16::MAX).then_some(clip))
            }
            1 => {
                let parameter = r.byte()?;
                let count = usize::from(r.byte()?);
                let [cycle_seconds] = r.floats()?;
                if !(2..=MAX_ANIMATION_BLEND_POINTS).contains(&count)
                    || count > r.remaining_len() / 10
                {
                    return Err(SceneError::Invalid("animation graph blend point count"));
                }
                let mut points = Vec::with_capacity(count);
                for _ in 0..count {
                    let [threshold] = r.floats()?;
                    points.push(NativeAnimationBlendPoint {
                        threshold,
                        clip: u16::from_le_bytes(r.array()?),
                    });
                }
                NativeAnimationMotion::Blend1d {
                    parameter,
                    cycle_seconds,
                    points: points.into(),
                }
            }
            _ => return Err(SceneError::Invalid("animation graph motion kind")),
        };
        states.push(NativeAnimationGraphState {
            id,
            looping,
            motion,
        });
    }
    let mut transitions = Vec::with_capacity(nt);
    for _ in 0..nt {
        let id = StableId(r.array()?);
        let [from, to] = r.array()?;
        let priority = i16::from_le_bytes(r.array()?);
        let [fade_seconds, after] = r.floats()?;
        let count = usize::from(r.byte()?);
        if count > MAX_ANIMATION_GRAPH_CONDITIONS
            || count > r.remaining_len() / 10
            || (after < 0.0 && after != -1.0)
        {
            return Err(SceneError::Invalid(
                "animation graph condition count or exit time",
            ));
        }
        let mut conditions = Vec::with_capacity(count);
        for _ in 0..count {
            let parameter = r.byte()?;
            let comparison = match r.byte()? {
                0 => AnimationComparison::Less,
                1 => AnimationComparison::LessEqual,
                2 => AnimationComparison::Equal,
                3 => AnimationComparison::GreaterEqual,
                4 => AnimationComparison::Greater,
                _ => return Err(SceneError::Invalid("animation graph comparison")),
            };
            let [value] = r.floats()?;
            conditions.push(NativeAnimationCondition {
                parameter,
                comparison,
                value,
            });
        }
        transitions.push(NativeAnimationGraphTransition {
            id,
            from,
            to,
            priority,
            fade_seconds,
            after_seconds: (after != -1.0).then_some(after),
            conditions: conditions.into(),
        });
    }
    NativeAnimationGraph::new(initial, parameters, states, transitions, durations)
}

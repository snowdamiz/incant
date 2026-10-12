//! Evaluate each state using its authored interpolation rule.
use super::*;
use crate::{AnimationBlendSample, AnimationSample, NativeAnimation};
use incant_types::AnimationPoseSamples;
impl NativeAnimationGraph {
    pub(crate) fn pose(
        &self,
        graph: &AnimationGraphState,
        definition: &NativeAnimation,
    ) -> Option<AnimationPoseSamples> {
        if let Some(fade) = graph.fade {
            let from = self.state_pose(graph, fade.state, fade.seconds, definition);
            let to = self.state_pose(graph, graph.current, graph.seconds, definition);
            return Some(
                AnimationPoseSamples::crossfade(&from, &to, fade.elapsed / fade.duration)
                    .expect("validated graph pose groups"),
            );
        }
        matches!(
            self.states[usize::from(graph.current)].motion,
            NativeAnimationMotion::Tree { .. }
        )
        .then(|| self.state_pose(graph, graph.current, graph.seconds, definition))
    }
    pub(crate) fn sample(
        &self,
        graph: &AnimationGraphState,
        definition: &NativeAnimation,
    ) -> (AnimationSample, Option<AnimationBlendSample>) {
        self.state_sample(graph, graph.current, graph.seconds, definition)
    }
    pub(crate) fn state_pose(
        &self,
        graph: &AnimationGraphState,
        current: u8,
        seconds: f64,
        definition: &NativeAnimation,
    ) -> AnimationPoseSamples {
        if let NativeAnimationMotion::Tree {
            cycle_seconds,
            tree,
        } = &self.states[usize::from(current)].motion
        {
            return tree
                .sample(
                    &graph.parameters[..self.parameters.len()],
                    seconds / cycle_seconds,
                )
                .expect("validated graph tree state");
        }
        let (sample, blend) = self.state_sample(graph, current, seconds, definition);
        match blend {
            Some(blend) => AnimationPoseSamples::spherical_pair(blend.sample, sample, blend.weight),
            None => AnimationPoseSamples::clip(sample),
        }
        .expect("validated graph state pose")
    }
    pub(crate) fn state_duration(&self, current: u8, definition: &NativeAnimation) -> f64 {
        match self.states[usize::from(current)].motion {
            NativeAnimationMotion::Clip(clip) => clip.map_or(0.0, |c| {
                f64::from(definition.clip_durations[usize::from(c)])
            }),
            NativeAnimationMotion::Blend1d { cycle_seconds, .. }
            | NativeAnimationMotion::Tree { cycle_seconds, .. } => cycle_seconds,
        }
    }
    fn state_sample(
        &self,
        graph: &AnimationGraphState,
        current: u8,
        seconds: f64,
        definition: &NativeAnimation,
    ) -> (AnimationSample, Option<AnimationBlendSample>) {
        match &self.states[usize::from(current)].motion {
            NativeAnimationMotion::Tree { .. } => (
                self.state_pose(graph, current, seconds, definition)
                    .representative(),
                None,
            ),
            NativeAnimationMotion::Clip(clip) => (
                AnimationSample {
                    clip: *clip,
                    seconds,
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
                    seconds: (seconds / cycle_seconds)
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
}

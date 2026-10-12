use super::*;
use crate::AnimationSample;

impl NativeAnimationBlendTree {
    /// All leaves share normalized phase, but retain their own clip durations.
    /// Bind pose and zero-duration clips remain at zero. Parameter values outside
    /// their declared ranges are refused; values outside a grid clamp to it.
    pub fn sample(
        &self,
        parameters: &[f64],
        phase: f64,
    ) -> Result<AnimationPoseSamples, SceneError> {
        if !phase.is_finite()
            || !(0.0..=1.0).contains(&phase)
            || parameters.len() != self.parameters.len()
            || parameters
                .iter()
                .zip(self.parameters.iter())
                .any(|(v, p)| !v.is_finite() || *v < p.min || *v > p.max)
        {
            return Err(SceneError::Invalid("animation blend tree sample inputs"));
        }
        let phase = if phase == 0.0 { 0.0 } else { phase };
        let mut weights = [0.0; MAX_BLEND_TREE_NODES];
        weights[self.nodes.len() - 1] = 1.0;
        let mut result = [WeightedAnimationSample {
            sample: AnimationSample::REST,
            weight: 0.0,
        }; MAX_BLEND_TREE_POSES];
        let mut count = 0;
        for (index, node) in self.nodes.iter().enumerate().rev() {
            let weight = weights[index];
            if weight == 0.0 {
                continue;
            }
            match node {
                NativeBlendTreeNode::Clip(clip) => {
                    result[count] = WeightedAnimationSample {
                        sample: AnimationSample {
                            clip: *clip,
                            seconds: clip.map_or(0.0, |clip| {
                                phase * f64::from(self.clip_durations[usize::from(clip)])
                            }),
                        },
                        weight,
                    };
                    count += 1;
                }
                NativeBlendTreeNode::Blend1d { parameter, points } => {
                    let (lo, hi, t) = bracket(
                        points.len(),
                        |i| points[i].threshold,
                        parameters[usize::from(*parameter)],
                    );
                    weights[usize::from(points[lo].child)] += weight * (1.0 - t);
                    weights[usize::from(points[hi].child)] += weight * t;
                }
                NativeBlendTreeNode::Blend2d {
                    parameters: axes,
                    x,
                    y,
                    children,
                } => {
                    let (x0, x1, u) = bracket(x.len(), |i| x[i], parameters[usize::from(axes[0])]);
                    let (y0, y1, v) = bracket(y.len(), |i| y[i], parameters[usize::from(axes[1])]);
                    for (col, wx) in [(x0, 1.0 - u), (x1, u)] {
                        for (row, wy) in [(y0, 1.0 - v), (y1, v)] {
                            weights[usize::from(children[row * x.len() + col])] += weight * wx * wy;
                        }
                    }
                }
            }
        }
        AnimationPoseSamples::new(&result[..count]).map_err(SceneError::Invalid)
    }
}

fn bracket(len: usize, at: impl Fn(usize) -> f64, value: f64) -> (usize, usize, f64) {
    let mut lo = 0;
    let mut hi = len;
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if at(mid) <= value {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        return (0, 0, 0.0);
    }
    if lo == len {
        return (len - 1, len - 1, 0.0);
    }
    (lo - 1, lo, (value - at(lo - 1)) / (at(lo) - at(lo - 1)))
}

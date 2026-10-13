//! Test-only scalar coverage oracle. No renderer path calls this module.
use super::{TemporalConfig, oracle::clean};

pub(super) const SIZE: [usize; 2] = [12, 8];
pub(super) const RECT: [usize; 4] = [1, 1, 10, 6];
pub(super) const NEAR: f64 = 0.1;
pub(super) const FAR: f64 = 100.;
pub(super) const CONFIG: TemporalConfig = TemporalConfig {
    history_weight: 0.9,
    relative_depth: 0.001,
    absolute_depth: 0.001,
    reactive_threshold: 4.,
    reactive_strength: 0.,
};
#[derive(Clone, Copy, Debug)]
pub(super) enum Case {
    DepthEdge,
    ClearEdge,
    MaskGrille,
    Moving,
    MalformedClear,
}
pub(super) const CASES: [Case; 5] = [
    Case::DepthEdge,
    Case::ClearEdge,
    Case::MaskGrille,
    Case::Moving,
    Case::MalformedClear,
];
pub(super) struct Input {
    pub color: Vec<[f32; 4]>,
    pub motion: Vec<[f32; 4]>,
    pub depth: Vec<f32>,
}
pub(super) fn encode(z: f64, ortho: bool) -> f32 {
    (if ortho {
        (z - NEAR) / (FAR - NEAR)
    } else {
        (1. / NEAR - 1. / z) / (1. / NEAR - 1. / FAR)
    }) as f32
}
fn view(d: f64, ortho: bool) -> f64 {
    if ortho {
        (1. - d) * NEAR + d * FAR
    } else {
        1. / ((1. - d) / NEAR + d / FAR)
    }
}
pub(super) fn fixture(case: Case, tick: usize, ortho: bool) -> Input {
    let mut input = Input {
        color: vec![[0.; 4]; SIZE[0] * SIZE[1]],
        motion: vec![[0.; 4]; SIZE[0] * SIZE[1]],
        depth: vec![1.; SIZE[0] * SIZE[1]],
    };
    for y in 0..SIZE[1] {
        for x in 0..SIZE[0] {
            let foreground = match case {
                Case::DepthEdge | Case::ClearEdge => x < 4 + tick % 2,
                Case::MaskGrille => x % 2 == tick % 2,
                Case::Moving => (2 + tick..4 + tick).contains(&x),
                Case::MalformedClear => x < 4,
            };
            let clear = !foreground
                && matches!(
                    case,
                    Case::ClearEdge | Case::MaskGrille | Case::MalformedClear
                );
            let i = y * SIZE[0] + x;
            input.color[i] = if foreground {
                [1., 0.5, 0.25, 1.]
            } else if clear {
                if matches!(case, Case::MalformedClear) {
                    [4., 3., 2., 0.]
                } else {
                    [0.; 4]
                }
            } else {
                [0.1, 0.2, 0.3, 1.]
            };
            input.depth[i] = if clear {
                1.
            } else {
                encode(if foreground { 2. } else { 10. }, ortho)
            };
            if !clear {
                input.motion[i] = [
                    if foreground && matches!(case, Case::Moving) {
                        -1. / RECT[2] as f32
                    } else {
                        0.
                    },
                    0.,
                    input.depth[i],
                    1.,
                ];
            }
        }
    }
    input
}
fn valid(m: [f32; 4]) -> bool {
    m.iter().all(|v| v.is_finite()) && m[3] >= 0.5 && (0. ..=1.).contains(&m[2])
}
/// A neighborhood witness in view space plus the selected normalized motion.
/// Stable center priority on equal depth prevents gratuitous interior reprojection.
pub(super) fn support(
    input: &Input,
    x: usize,
    y: usize,
    ortho: bool,
) -> Option<([f64; 2], [f32; 2])> {
    let i = y * SIZE[0] + x;
    let c = clean(input.color[i]);
    if !input.color[i].iter().all(|v| v.is_finite()) || (c[3] > 0. && !valid(input.motion[i])) {
        return None;
    }
    let mut motion = if c[3] > 0. {
        input.motion[i]
    } else {
        [0., 0., 1., 1.]
    };
    let mut closest = if c[3] > 0. { motion[2] } else { 2. };
    let mut zs = Vec::with_capacity(9);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let px = (x as isize + dx).clamp(RECT[0] as isize, (RECT[0] + RECT[2] - 1) as isize)
                as usize;
            let py = (y as isize + dy).clamp(RECT[1] as isize, (RECT[1] + RECT[3] - 1) as isize)
                as usize;
            let j = py * SIZE[0] + px;
            if !input.color[j].iter().all(|v| v.is_finite()) {
                continue;
            }
            if clean(input.color[j])[3] == 0. {
                zs.push(FAR);
            } else if valid(input.motion[j]) {
                zs.push(view(f64::from(input.motion[j][2]), ortho));
                if input.motion[j][2] < closest {
                    motion = input.motion[j];
                    closest = motion[2];
                }
            }
        }
    }
    let near = zs.iter().copied().reduce(f64::min)?;
    let far = zs.iter().copied().reduce(f64::max)?;
    Some((
        [
            near - f64::from(CONFIG.absolute_depth).max(f64::from(CONFIG.relative_depth) * near),
            far + f64::from(CONFIG.absolute_depth).max(f64::from(CONFIG.relative_depth) * far),
        ],
        [motion[0], motion[1]],
    ))
}
/// Independent f64 reference; each old sample is admitted before renormalization.
pub(super) fn resolve(
    input: &Input,
    old: &[[f32; 4]],
    depth: &[f32],
    ortho: bool,
) -> Vec<[f32; 4]> {
    let colors: Vec<_> = input.color.iter().copied().map(clean).collect();
    let mut out = colors.clone();
    for y in RECT[1]..RECT[1] + RECT[3] {
        for x in RECT[0]..RECT[0] + RECT[2] {
            let Some((range, motion)) = support(input, x, y, ortho) else {
                continue;
            };
            let p = [
                x as f64 + 0.5 + f64::from(motion[0]) * RECT[2] as f64,
                y as f64 + 0.5 + f64::from(motion[1]) * RECT[3] as f64,
            ];
            if (0..2).any(|j| p[j] < RECT[j] as f64 || p[j] >= (RECT[j] + RECT[j + 2]) as f64) {
                continue;
            }
            let q = p.map(|v| v - 0.5);
            let base = q.map(|v| v.floor() as isize);
            let f = q.map(|v| v - v.floor());
            let mut sum = [0.; 4];
            let mut mass = 0.;
            for dy in 0..2 {
                for dx in 0..2 {
                    let w = (if dx == 0 { 1. - f[0] } else { f[0] })
                        * (if dy == 0 { 1. - f[1] } else { f[1] });
                    let xy = [base[0] + dx as isize, base[1] + dy as isize];
                    if w <= 0.
                        || (0..2).any(|j| {
                            xy[j] < RECT[j] as isize || xy[j] >= (RECT[j] + RECT[j + 2]) as isize
                        })
                    {
                        continue;
                    }
                    let j = xy[1] as usize * SIZE[0] + xy[0] as usize;
                    if !(0. ..=1.).contains(&depth[j])
                        || old[j].iter().any(|v| !v.is_finite() || *v < 0.)
                        || old[j][3] > 1.
                    {
                        continue;
                    }
                    let z = view(f64::from(depth[j]), ortho);
                    if z < range[0] || z > range[1] {
                        continue;
                    }
                    mass += w;
                    for (s, v) in sum.iter_mut().zip(clean(old[j])) {
                        *s += f64::from(v.min(65504.)) * w;
                    }
                }
            }
            if mass == 0. {
                continue;
            }
            let i = y * SIZE[0] + x;
            let mut lo = colors[i];
            let mut hi = lo;
            for dy in -1..=1 {
                for dx in -1..=1 {
                    let px = (x as isize + dx)
                        .clamp(RECT[0] as isize, (RECT[0] + RECT[2] - 1) as isize)
                        as usize;
                    let py = (y as isize + dy)
                        .clamp(RECT[1] as isize, (RECT[1] + RECT[3] - 1) as isize)
                        as usize;
                    for k in 0..4 {
                        lo[k] = lo[k].min(colors[py * SIZE[0] + px][k]);
                        hi[k] = hi[k].max(colors[py * SIZE[0] + px][k]);
                    }
                }
            }
            out[i] = std::array::from_fn(|k| {
                ((1. - f64::from(CONFIG.history_weight)) * f64::from(colors[i][k])
                    + f64::from(CONFIG.history_weight)
                        * (sum[k] / mass).clamp(f64::from(lo[k]), f64::from(hi[k])))
                    as f32
            });
        }
    }
    out
}

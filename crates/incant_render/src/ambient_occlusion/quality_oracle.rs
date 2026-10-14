//! Independent f64 proposal model with exact f32 direction/rotation inputs.
//! Camera geometry and nominal bias use f64; this is not a
//! GPU rounding/error-bound oracle and does not change production admission.
use super::quality_geometry::Grid;
use glam::DVec3;

pub const BAYER: [[usize; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
pub const COEFFICIENTS: [f64; 5] = [0.5, 1., 1., 1., 0.5];
pub fn directions() -> [DVec3; 16] {
    std::array::from_fn(|i| {
        let u = (i as f32 + 0.5) / 16.;
        let angle = std::f64::consts::TAU * (i as u32).reverse_bits() as f64 / 4_294_967_296.;
        DVec3::new(
            f64::from((f64::from(u.sqrt()) * angle.cos()) as f32),
            f64::from((f64::from(u.sqrt()) * angle.sin()) as f32),
            f64::from((1. - u).sqrt()),
        )
    })
}
pub fn rotations() -> [[f32; 4]; 16] {
    std::array::from_fn(|b| {
        let alpha = std::f64::consts::TAU / 16. * (b as f64 + 0.5) / 16.;
        [alpha.cos() as f32, alpha.sin() as f32, 0., 0.]
    })
}
#[derive(Clone, Copy)]
pub struct Config {
    pub rotate: bool,
    pub jitter: bool,
    pub geometric: bool,
    pub radius: f64,
    pub strength: f64,
}
pub const REPAIRED: Config = Config {
    rotate: true,
    jitter: true,
    geometric: false,
    radius: 2.,
    strength: 1.,
};
pub const OLD: Config = Config {
    rotate: false,
    jitter: false,
    geometric: false,
    ..REPAIRED
};
pub const BIAS: f64 = 0.01;
pub fn sampling_phase(x: usize, y: usize, ray: usize) -> f64 {
    let base = (BAYER[y % 4][x % 4] as u32).reverse_bits() >> 28;
    // Generate the digital coordinate independently of the shader's lookup table.
    let sobol = [8u32, 12, 10, 15]
        .into_iter()
        .enumerate()
        .filter(|(bit, _)| ray & (1 << bit) != 0)
        .fold(0, |value, (_, direction)| value ^ direction);
    (f64::from(base ^ sobol) + 0.5) / 16.
}
pub fn raw(grid: &Grid, cfg: Config) -> Vec<f64> {
    let kernel = directions();
    let rotation = rotations();
    grid.hits
        .iter()
        .enumerate()
        .map(|(index, own)| {
            let Some(hit) = own else {
                return 1.;
            };
            let n = if cfg.geometric {
                hit.geometric
            } else {
                hit.shading
            };
            if !n.is_finite() || n.length_squared() < 0.5 {
                return 1.;
            }
            let n = n.normalize();
            let axis = if n.z.abs() > 0.9 { DVec3::Y } else { DVec3::Z };
            let tangent = axis.cross(n).normalize();
            let bitangent = n.cross(tangent);
            let x = index % grid.size[0];
            let y = index / grid.size[0];
            let [cos, sin, _, _] = if cfg.rotate {
                rotation[BAYER[y % 4][x % 4]]
            } else {
                [1., 0., 0., 0.]
            };
            let mut count = 0;
            for (ray, k) in kernel.into_iter().enumerate() {
                let t = if cfg.jitter {
                    sampling_phase(x, y, ray)
                } else {
                    1.
                };
                let kx = k.x * f64::from(cos) - k.y * f64::from(sin);
                let ky = k.x * f64::from(sin) + k.y * f64::from(cos);
                let d = tangent * kx + bitangent * ky + n * k.z;
                for j in 0..4 {
                    let q = hit.p + d * cfg.radius * (j as f64 + t) / 4.;
                    let Some(at) = grid.camera.project(q) else {
                        continue;
                    };
                    let Some((_, surface)) = grid.at(at.x.floor() as isize, at.y.floor() as isize)
                    else {
                        continue;
                    };
                    let delta = surface.p - hit.p;
                    if surface.p.z >= q.z + BIAS
                        && delta.dot(n) > BIAS
                        && delta.length_squared() <= cfg.radius.powi(2)
                    {
                        count += 1;
                        break;
                    }
                }
            }
            1. - cfg.strength * f64::from(count) / 16.
        })
        .collect()
}
pub fn r8(value: f64) -> f64 {
    assert!(value.is_finite() && (0.0..=1.0).contains(&value));
    (value * 255.).round() / 255.
}
#[derive(Clone, Copy)]
pub struct Filter {
    pub plane: bool,
    pub symmetric: bool,
    pub quantized: bool,
}
pub const FILTER: Filter = Filter {
    plane: true,
    symmetric: true,
    quantized: true,
};
pub fn pass(grid: &Grid, input: &[f64], horizontal: bool, cfg: Filter) -> Vec<f64> {
    assert_eq!(input.len(), grid.hits.len());
    grid.hits
        .iter()
        .enumerate()
        .map(|(index, own)| {
            let Some(hit) = own else {
                return 1.;
            };
            if hit.shading.length_squared() < 0.5 || !hit.shading.is_finite() {
                return 1.;
            }
            let n = hit.shading.normalize();
            let x = (index % grid.size[0]) as isize;
            let y = (index / grid.size[0]) as isize;
            let mut total = 0.;
            let mut mass = 0.;
            for (tap, coefficient) in COEFFICIENTS.iter().copied().enumerate() {
                if !cfg.symmetric && tap == 4 {
                    continue;
                }
                let offset = tap as isize - 2;
                let Some((i, q)) = grid.at(
                    x + if horizontal { offset } else { 0 },
                    y + if horizontal { 0 } else { offset },
                ) else {
                    continue;
                };
                let plane = if cfg.plane {
                    (1. - (q.p - hit.p).dot(n).abs() / BIAS.max(-hit.p.z * 0.01)).clamp(0., 1.)
                } else {
                    1.
                };
                let w = if cfg.symmetric { coefficient } else { 1. } * plane;
                mass += w;
                total += w * input[i];
            }
            assert!(mass >= 1. && mass.is_finite());
            let value = (total / mass).clamp(0., 1.);
            if cfg.quantized { r8(value) } else { value }
        })
        .collect()
}
pub fn filtered(grid: &Grid, input: &[f64], cfg: Filter) -> Vec<f64> {
    let input: Vec<_> = input
        .iter()
        .copied()
        .map(|v| if cfg.quantized { r8(v) } else { v })
        .collect();
    pass(grid, &pass(grid, &input, true, cfg), false, cfg)
}

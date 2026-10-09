//! Single-scattering split-sum GGX, using the same correlated Smith visibility
//! as the material shader. Equations: Filament sections 5.3 and 9.2–9.5.
//! https://google.github.io/filament/main/filament.html
use super::cube::{Cube, sample_lod};
use glam::{Vec2, Vec3};
use std::f32::consts::{PI, TAU};

pub(super) const SPECULAR_SIZE: u32 = 128;
pub(super) const DIFFUSE_SIZE: u32 = 16;
pub(super) const LUT_SIZE: u32 = 64;
const FILTER_SAMPLES: u32 = 256;
const BRDF_SAMPLES: u32 = 1024;

fn sequence(i: u32, count: u32) -> Vec2 {
    Vec2::new(
        (i as f32 + 0.5) / count as f32,
        i.reverse_bits() as f32 * (1.0 / 4294967296.0),
    )
}
fn half_vector(xi: Vec2, roughness: f32) -> Vec3 {
    let a2 = roughness.powi(4);
    let z = ((1. - xi.y) / (1. + (a2 - 1.) * xi.y)).sqrt();
    let r = (1. - z * z).max(0.).sqrt();
    let (sin, cos) = (TAU * xi.x).sin_cos();
    Vec3::new(r * cos, r * sin, z)
}
fn basis(n: Vec3) -> (Vec3, Vec3) {
    let up = if n.z.abs() < 0.999 { Vec3::Z } else { Vec3::X };
    let tangent = up.cross(n).normalize();
    (tangent, n.cross(tangent))
}
pub(super) struct Filtered {
    pub specular: Vec<Cube>,
    pub diffuse: Cube,
}
pub(super) fn filter(base: Cube) -> Filtered {
    let mut source = vec![base];
    while source.last().unwrap().size > 1 {
        source.push(source.last().unwrap().downsample());
    }
    let texel_angle = 4. * PI / (6. * source[0].size.pow(2) as f32);
    let mut specular = Vec::new();
    for level in 0..source.len() {
        let roughness = level as f32 / (source.len() - 1) as f32;
        let samples: Vec<_> = (0..FILTER_SAMPLES)
            .filter_map(|i| {
                let h = half_vector(sequence(i, FILTER_SAMPLES), roughness);
                let l = 2. * h.z * h - Vec3::Z;
                if l.z <= 0. {
                    return None;
                }
                let a2 = roughness.powi(4);
                let d = a2 / (PI * (h.z * h.z * (a2 - 1.) + 1.).powi(2));
                // N = V, so D * N.H / (4 * V.H) = D/4.
                let pdf = d * 0.25;
                let lod = 0.5 * (1. / (FILTER_SAMPLES as f32 * pdf * texel_angle)).log2();
                Some((l, lod))
            })
            .collect();
        specular.push(Cube::from_fn(source[level].size, |n| {
            if level == 0 {
                return source[0].sample(n);
            }
            let (t, b) = basis(n);
            let mut sum = Vec3::ZERO;
            let mut weight = 0.;
            for &(l, lod) in &samples {
                sum += sample_lod(&source, t * l.x + b * l.y + n * l.z, lod) * l.z;
                weight += l.z;
            }
            sum / weight
        }));
    }
    let diffuse_samples: Vec<_> = (0..FILTER_SAMPLES)
        .map(|i| {
            let xi = sequence(i, FILTER_SAMPLES);
            let r = xi.y.sqrt();
            let (sin, cos) = (TAU * xi.x).sin_cos();
            let l = Vec3::new(r * cos, r * sin, (1. - xi.y).sqrt());
            let lod = 0.5 * (PI / (FILTER_SAMPLES as f32 * l.z * texel_angle)).log2();
            (l, lod)
        })
        .collect();
    // Cosine-weighted samples bake irradiance / PI (Lambert's denominator).
    let diffuse = Cube::from_fn(DIFFUSE_SIZE, |n| {
        let (t, b) = basis(n);
        diffuse_samples
            .iter()
            .map(|&(l, lod)| sample_lod(&source, t * l.x + b * l.y + n * l.z, lod))
            .sum::<Vec3>()
            / FILTER_SAMPLES as f32
    });
    Filtered { specular, diffuse }
}
pub(super) fn integrate_brdf(nv: f32, roughness: f32, count: u32) -> Vec2 {
    let v = Vec3::new((1. - nv * nv).sqrt(), 0., nv);
    let a2 = roughness.powi(4);
    let mut result = Vec2::ZERO;
    for i in 0..count {
        let h = half_vector(sequence(i, count), roughness);
        let vh = v.dot(h).max(0.);
        let l = 2. * vh * h - v;
        if l.z <= 0. {
            continue;
        }
        let visibility = 0.5
            / (l.z * (nv * nv * (1. - a2) + a2).sqrt() + nv * (l.z * l.z * (1. - a2) + a2).sqrt())
                .max(1e-7);
        // BRDF / pdf, including cosine and the reflection Jacobian.
        let weight = 4. * visibility * l.z * vh / h.z.max(1e-7);
        let fresnel = (1. - vh).powi(5);
        result += Vec2::new(1. - fresnel, fresnel) * weight;
    }
    result / count as f32
}
pub(super) fn brdf_lut() -> Vec<Vec2> {
    (0..LUT_SIZE)
        .flat_map(|y| {
            (0..LUT_SIZE).map(move |x| {
                integrate_brdf(
                    (x as f32 + 0.5) / LUT_SIZE as f32,
                    ((y as f32 + 0.5) / LUT_SIZE as f32).max(0.045),
                    BRDF_SAMPLES,
                )
            })
        })
        .collect()
}

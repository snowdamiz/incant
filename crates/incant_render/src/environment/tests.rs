use super::{
    cube::{Cube, direction},
    filter,
};
use glam::Vec3;
use std::f32::consts::{PI, TAU};

fn source(format: incant_assets::TextureFormat, color: [f32; 4]) -> incant_assets::CookedTexture {
    let texel: Vec<u8> = if format == incant_assets::TextureFormat::Rgba32Float {
        color.into_iter().flat_map(f32::to_le_bytes).collect()
    } else {
        color.into_iter().map(|v| v as u8).collect()
    };
    incant_assets::CookedTexture {
        metadata: incant_assets::TextureMetadata {
            version: 1,
            fingerprint: "ab".repeat(32),
            dependency: incant_assets::Dependency {
                path: "environment.exr".into(),
                sha256: "cd".repeat(32),
            },
            usage: incant_assets::TextureUsage::Linear,
            content_sha256: "ef".repeat(32),
        },
        texture: incant_assets::Texture {
            width: 2,
            height: 1,
            format,
            levels: vec![texel.repeat(2), texel],
        },
        cache_hit: true,
    }
}

#[test]
fn environment_sources_decode_linear_srgb_hdr_and_reject_invalid_radiance() {
    use incant_assets::TextureFormat::*;
    for (format, color, expected) in [
        (Rgba32Float, [12., 0.5, 0.1, 0.], Vec3::new(12., 0.5, 0.1)),
        (
            Rgba8Linear,
            [128., 64., 32., 0.],
            Vec3::new(128., 64., 32.) / 255.,
        ),
        (
            Rgba8Srgb,
            [128., 64., 32., 255.],
            Vec3::new(0.215861, 0.051269, 0.014444),
        ),
    ] {
        let decoded = super::source::decode(&source(format, color)).unwrap();
        assert!((decoded.sample(Vec3::Z) - expected).abs().max_element() < 0.000002);
    }
    for value in [-0.1, 65505., f32::NAN, f32::INFINITY] {
        assert!(super::source::decode(&source(Rgba32Float, [value, 0., 0., 1.])).is_err());
    }
    let mut normal = source(Rgba8Linear, [128., 128., 255., 255.]);
    normal.metadata.usage = incant_assets::TextureUsage::Normal;
    assert!(super::source::decode(&normal).is_err());
}

#[test]
fn large_environment_uses_cooked_low_pass_mips_before_cube_resampling() {
    let pixels: Vec<u8> = (0..512)
        .flat_map(|y| {
            (0..1024).flat_map(move |x| {
                let c = if (x + y) % 2 == 0 { 0 } else { 255 };
                [c, c, c, 255]
            })
        })
        .collect();
    let mut png_bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut png_bytes, 1024, 512);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .unwrap()
            .write_image_data(&pixels)
            .unwrap();
    }
    let mut image = source(incant_assets::TextureFormat::Rgba8Linear, [0.; 4]);
    image.texture =
        incant_assets::import_image(&png_bytes, incant_assets::TextureUsage::Linear).unwrap();
    let cube = super::source::decode(&image).unwrap();
    for pixel in cube.pixels {
        assert!((pixel - Vec3::splat(128. / 255.)).abs().max_element() < 0.00001);
    }
}

#[test]
fn cubemap_axes_edges_and_constant_radiance_are_preserved() {
    let cube = Cube::from_fn(32, |d| d * 0.5 + Vec3::splat(0.5));
    for axis in [Vec3::X, -Vec3::X, Vec3::Y, -Vec3::Y, Vec3::Z, -Vec3::Z] {
        assert!(
            (cube.sample(axis) - (axis * 0.5 + Vec3::splat(0.5)))
                .abs()
                .max_element()
                < 0.002
        );
    }
    for face in 0..6 {
        for u in [-1.001, -1., -0.999, 0., 0.999, 1., 1.001] {
            for v in [-1.001, -1., -0.999, 0., 0.999, 1., 1.001] {
                let d = direction(face, u, v);
                assert!(
                    (cube.sample(d) - (d * 0.5 + Vec3::splat(0.5)))
                        .abs()
                        .max_element()
                        < 0.01
                );
            }
        }
    }
    let color = Vec3::new(0.1, 2., 12.);
    let filtered = filter::filter(Cube::from_fn(32, |_| color));
    for mip in filtered
        .specular
        .iter()
        .chain(std::iter::once(&filtered.diffuse))
    {
        for &value in &mip.pixels {
            assert!((value - color).abs().max_element() < 0.00005);
        }
    }
}

// Independent integration over uniform solid angle, with neither a GGX sample
// distribution nor the split-sum prefilter. It checks the DFG's actual integral.
fn quadrature(nv: f32, roughness: f32) -> glam::Vec2 {
    let v = Vec3::new((1. - nv * nv).sqrt(), 0., nv);
    let mut sum = glam::Vec2::ZERO;
    let (rings, sectors) = (384, 768);
    let a2 = roughness.powi(4);
    for y in 0..rings {
        let nl = (y as f32 + 0.5) / rings as f32;
        let r = (1. - nl * nl).sqrt();
        for x in 0..sectors {
            let (s, c) = (TAU * (x as f32 + 0.5) / sectors as f32).sin_cos();
            let l = Vec3::new(r * c, r * s, nl);
            let h = (v + l).normalize();
            let d = a2 / (PI * (h.z * h.z * (a2 - 1.) + 1.).powi(2));
            let vis = 0.5
                / (nl * (nv * nv * (1. - a2) + a2).sqrt() + nv * (nl * nl * (1. - a2) + a2).sqrt());
            let f = (1. - v.dot(h)).powi(5);
            sum += glam::Vec2::new(1. - f, f) * (d * vis * nl);
        }
    }
    sum * (TAU / (rings * sectors) as f32)
}
#[test]
fn brdf_lookup_matches_independent_hemisphere_integrals() {
    for value in filter::brdf_lut() {
        assert!(
            value.is_finite() && value.min_element() >= 0. && value.element_sum() <= 1.01,
            "nonconserving lookup: {value}"
        );
    }
    for nv in [0.1, 0.5, 1.] {
        for roughness in [0.3, 0.6, 1.] {
            let integrated = filter::integrate_brdf(nv, roughness, 16384);
            let reference = quadrature(nv, roughness);
            assert!(
                (integrated - reference).abs().max_element() < 0.004,
                "nv={nv}, r={roughness}: {integrated} vs {reference}"
            );
            assert!(integrated.min_element() >= 0. && integrated.element_sum() <= 1.005);
        }
    }
    let smooth = filter::integrate_brdf(1., 0.045, 4096);
    assert!((smooth.x - 1.).abs() < 0.002 && smooth.y < 0.00001);
}

#[test]
fn prefilter_broadens_a_bright_direction_without_negative_radiance() {
    let base = Cube::from_fn(64, |d| Vec3::splat(if d.z > 0.95 { 4. } else { 0. }));
    let filtered = filter::filter(base);
    assert!(filtered.specular[0].sample(Vec3::Z).x > 3.9);
    let last = filtered.specular.last().unwrap();
    assert!(last.sample(Vec3::Z).x < 1.);
    assert!(last.sample(Vec3::new(0.7, 0., 0.7).normalize()).x > 0.05);
    for mip in &filtered.specular {
        assert!(
            mip.pixels
                .iter()
                .all(|p| p.is_finite() && p.min_element() >= 0. && p.max_element() <= 4.001)
        );
    }
}

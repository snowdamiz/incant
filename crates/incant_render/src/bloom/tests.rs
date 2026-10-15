//! Numerical/reference and shader validation only; device efficacy is separate.
use super::{Bloom, BloomError, Plan, config, output_source};
mod device;
mod output_space;
mod reference;
use reference::*;

fn config() -> Bloom {
    Bloom {
        threshold: 1.,
        intensity: 0.5,
        levels: 6,
    }
}

#[test]
fn typed_admission_requires_complete_cooked_bloom_before_device_use() {
    assert!(config::admission(config(), false, false).is_ok());
    assert!(config::admission(config(), true, true).is_ok());
    assert_eq!(
        config::admission(config(), true, false),
        Err(BloomError::CookedOutput)
    );
    for bad in [f32::NAN, f32::INFINITY, -0.1, 65505.] {
        assert_eq!(
            Bloom {
                threshold: bad,
                ..config()
            }
            .validate(),
            Err(BloomError::Configuration)
        );
    }
    for bad in [f32::NAN, f32::NEG_INFINITY, -0.1, 8.01] {
        assert_eq!(
            Bloom {
                intensity: bad,
                ..config()
            }
            .validate(),
            Err(BloomError::Configuration)
        );
    }
    for levels in [0, 7, u32::MAX] {
        assert_eq!(
            Bloom { levels, ..config() }.validate(),
            Err(BloomError::Configuration)
        );
    }
}

#[test]
fn filter_uniform_layout_matches_the_fixed_cooker_abi() {
    assert_eq!(size_of::<super::Settings>(), 48);
    assert_eq!(std::mem::offset_of!(super::Settings, rect), 0);
    assert_eq!(std::mem::offset_of!(super::Settings, sizes), 16);
    assert_eq!(std::mem::offset_of!(super::Settings, params), 32);
}

#[test]
fn bounded_plan_accounts_every_retained_texture_before_allocation() {
    for source in [[1, 1], [3, 5], [333, 197], [2048, 2048], [8192, 512]] {
        for levels in 1..=6 {
            let plan = Plan::new(source, levels, 8192).unwrap();
            let expected: u64 = plan
                .sizes
                .iter()
                .enumerate()
                .map(|(i, s)| {
                    u64::from(s[0])
                        * u64::from(s[1])
                        * 8
                        * if i + 1 == plan.sizes.len() { 1 } else { 2 }
                })
                .sum();
            assert_eq!(plan.bytes, expected);
            assert!(plan.bytes <= 32 * 1024 * 1024);
            assert!(plan.sizes.len() <= 6);
        }
    }
    assert_eq!(Plan::new([1, 1], 6, 8192).unwrap().bytes, 8);
    assert_eq!(
        Plan::new([3, 5], 6, 8192).unwrap().sizes,
        vec![[2, 3], [1, 2], [1, 1]]
    );
    for size in [
        [0, 1],
        [1, 0],
        [u32::MAX, u32::MAX],
        [2049, 2048],
        [8193, 1],
    ] {
        assert_eq!(Plan::new(size, 6, 8192), Err(BloomError::TargetSize));
    }
    assert_eq!(Plan::new([65, 64], 1, 64), Err(BloomError::TargetSize));
}

#[test]
fn area_filter_preserves_odd_constant_fields_and_integrated_mean() {
    for size in [[1, 1], [3, 5], [33, 19], [1, 37]] {
        let n = (size[0] * size[1]) as usize;
        let input = Field {
            size,
            data: vec![[2., 4., 8.]; n],
        };
        for levels in 1..=6 {
            let bloom = pyramid(input.clone(), levels);
            for value in bloom.data {
                near(value, [2., 4., 8.]);
            }
        }
        let input = Field {
            size,
            data: (0..n).map(|i| [i as f64, 1., 0.]).collect(),
        };
        let reduced = input.reduce();
        let mean = |f: &Field| f.data.iter().map(|v| v[0]).sum::<f64>() / f.data.len() as f64;
        assert!((mean(&input) - mean(&reduced)).abs() < 1e-10);
    }
}

#[test]
fn source_coverage_controls_extraction_and_zero_alpha_residual_is_zero() {
    near(extract([65504., 42., 1., 0.], 0.), [0.; 3]);
    near(extract([0.; 4], 0.), [0.; 3]);
    near(extract([1., 1., 1., 1.], 1.), [0.; 3]);
    for alpha in [0.125, 0.5, 1.] {
        near(
            extract([4. * alpha, 4. * alpha, 4. * alpha, alpha], 1.),
            [3. * alpha; 3],
        );
    }
    let red = extract([4., 0., 0., 1.], 1.);
    near(red, [0.; 3]); // Rec.709 luminance < threshold, not max-channel threshold.
}

#[test]
fn normalized_pyramid_spreads_an_emitter_without_creating_negative_light() {
    let mut input = Field {
        size: [17, 13],
        data: vec![[0.; 3]; 17 * 13],
    };
    input.data[6 * 17 + 8] = [64., 32., 16.];
    let output = pyramid(input, 4);
    assert!(
        output
            .data
            .iter()
            .all(|v| v.iter().all(|v| v.is_finite() && *v >= 0. && *v <= 64.))
    );
    assert!(output.data.iter().filter(|v| v[0] > 0.).count() > 1);
}

#[test]
fn offset_fractional_viewport_samples_are_half_open_and_exclude_external_sources() {
    let rect = [2.5, 1.5, 3., 4.];
    Plan::viewport([9, 7], rect).unwrap();
    let inside = |x: u32, y: u32| {
        let p = [x as f64 + 0.5, y as f64 + 0.5];
        p[0] >= 2.5 && p[0] < 5.5 && p[1] >= 1.5 && p[1] < 5.5
    };
    let data: Vec<_> = (0..7)
        .flat_map(|y| {
            (0..9).map(move |x| {
                if inside(x, y) {
                    extract([2., 2., 2., 1.], 1.)
                } else {
                    [0.; 3]
                }
            })
        })
        .collect();
    assert_eq!(data.iter().filter(|v| v[0] > 0.).count(), 12);
    let out = Field { size: [9, 7], data }.reduce();
    assert!(
        (out.data.iter().map(|v| v[0]).sum::<f64>() / out.data.len() as f64 - 12. / 63.).abs()
            < 1e-12
    );
    for rect in [
        [-1., 0., 1., 1.],
        [0., 0., 10., 7.],
        [0., 0., 0., 1.],
        [f32::NAN, 0., 1., 1.],
    ] {
        assert_eq!(Plan::viewport([9, 7], rect), Err(BloomError::Viewport));
    }
}

#[test]
fn coverage_composition_has_neutral_zero_and_exact_opaque_transparent_limits() {
    assert_eq!(tone([0.; 3]), [0.; 3]);
    let backdrop = [0.1, 0.2, 0.3];
    let r = [0.2, 0.4, 1.];
    let l = [0.1, 0.2, 0.3];
    for a in [0., 0.125, 0.5, 1.] {
        near(compose(r, [0.; 3], a, backdrop), mix(backdrop, tone(r), a));
    }
    near(compose(r, l, 1., backdrop), tone([0.3, 0.6, 1.3]));
    near(compose(r, l, 0., backdrop), [0.2, 0.4, 0.6]);
    near(
        compose(r, l, 0.5, backdrop),
        mix(
            compose(r, l, 0., backdrop),
            compose(r, l, 1., backdrop),
            0.5,
        ),
    );
    for a in [0., 0.5, 1.] {
        let out = compose([65504.; 3], [65504. * 8.; 3], a, backdrop);
        assert!(
            out.into_iter()
                .all(|x| x.is_finite() && (0.0..=1.).contains(&x))
        );
    }
}

#[test]
fn fixed_wgsl_variants_validate_and_preserve_original_tone_chrome_transfer() {
    for source in [include_str!("filter.wgsl").to_owned(), output_source()] {
        let module = naga::front::wgsl::parse_str(&source).unwrap();
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap();
    }
    let original = include_str!("../output.wgsl");
    let source = output_source();
    assert!(source.contains(include_str!("../tone_map.wgsl")));
    assert!(source.starts_with(&original.replace(
        "var color = mix(display.backdrop.rgb,tone_map(radiance),coverage);",
        "var color = display.backdrop.rgb; if inside { color = bloom_color(radiance,coverage,position.xy); }"
    )));
    assert_eq!(std::mem::size_of::<super::Settings>(), 48);
}

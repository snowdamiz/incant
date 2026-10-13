use super::{quality_geometry::*, quality_oracle::*};
use glam::{DVec2, DVec3};

#[test]
fn quality_exact_uniform_inputs_bayer_strata_and_resource_cost() {
    let native = super::math::directions();
    assert_eq!(super::math::rotations(), rotations());
    for (a, b) in native.iter().zip(directions()) {
        assert_eq!(
            DVec3::new(f64::from(a[0]), f64::from(a[1]), f64::from(a[2])),
            b
        );
    }
    let mut values: Vec<_> = BAYER.into_iter().flatten().collect();
    values.sort_unstable();
    assert_eq!(values, (0..16).collect::<Vec<_>>());
    let mut angles = Vec::new();
    for r in rotations() {
        assert!((f64::from(r[0]).hypot(f64::from(r[1])) - 1.).abs() < 4e-8);
        assert_eq!(&r[2..], &[0., 0.]);
        for k in directions() {
            let d = DVec2::new(
                k.x * f64::from(r[0]) - k.y * f64::from(r[1]),
                k.x * f64::from(r[1]) + k.y * f64::from(r[0]),
            );
            angles.push(d.y.atan2(d.x).rem_euclid(std::f64::consts::TAU));
        }
    }
    angles.sort_by(f64::total_cmp);
    assert_eq!(angles.len(), 256);
    for i in 0..256 {
        let gap = (angles[(i + 1) % 256] - angles[i]).rem_euclid(std::f64::consts::TAU);
        assert!((gap - std::f64::consts::TAU / 256.).abs() < 2e-7);
    }
    // Every texel distributes its maximum 64 probes over all radial strata.
    // The previous shared phase revisited only four distinct radii per texel.
    for y in 0..4 {
        for x in 0..4 {
            let mut steps: Vec<_> = (0..16)
                .flat_map(|ray| (0..4).map(move |j| (j as f64 + sampling_phase(x, y, ray)) / 4.))
                .collect();
            steps.sort_by(f64::total_cmp);
            assert_eq!(
                steps,
                (0..64).map(|i| (i as f64 + 0.5) / 64.).collect::<Vec<_>>()
            );
        }
    }
    assert_eq!(size_of::<super::math::Uniform>(), 736);
    assert_eq!(super::math::MAX_PIXELS * 14, 56 * 1024 * 1024);
    assert_eq!(super::math::BYTES_PER_PIXEL, 14);
    assert_eq!(2 * 14 * super::math::MAX_PIXELS, 112 * 1024 * 1024);
    // Old maximum + candidate maximum; two previously queued maxima + current
    // candidate are larger again. Logical idle-cache cap is not total GPU cap.
    assert_eq!((13 + 14) * super::math::MAX_PIXELS, 108 * 1024 * 1024);
    assert_eq!(3 * 14 * super::math::MAX_PIXELS, 168 * 1024 * 1024);
}

#[test]
fn quality_shading_normal_sign_is_preserved_and_withdrawn_geometric_variant_differs() {
    for ortho in [false, true] {
        let normal = fixture("white_slab", ortho);
        let reversed = fixture("reversed_normal", ortho);
        let candidate = raw(&normal, REPAIRED);
        assert!(candidate.iter().any(|&a| a < 1.));
        assert!(
            filtered(&reversed, &raw(&reversed, REPAIRED), FILTER)
                .iter()
                .all(|&a| a == 1.)
        );
        assert!(raw(&reversed, OLD).iter().all(|&a| a == 1.));
        // The superseded draft's viewer-facing geometric variant violates N2.
        assert_eq!(
            candidate,
            raw(
                &reversed,
                Config {
                    geometric: true,
                    ..REPAIRED
                }
            )
        );
    }
}

#[test]
fn quality_flat_far_radius_strength_background_and_degenerate_controls() {
    for ortho in [false, true] {
        for name in ["flat", "far_slab"] {
            let grid = fixture(name, ortho);
            let result = filtered(&grid, &raw(&grid, REPAIRED), FILTER);
            assert!(result.iter().all(|&a| a == 1.), "{name} {ortho}");
        }
        let mut grid = fixture("white_slab", ortho);
        for cfg in [
            Config {
                radius: 0.5,
                ..REPAIRED
            },
            Config {
                strength: 0.,
                ..REPAIRED
            },
        ] {
            assert!(
                filtered(&grid, &raw(&grid, cfg), FILTER)
                    .iter()
                    .all(|&a| a == 1.)
            );
        }
        for h in grid.hits.iter_mut().flatten() {
            h.shading = DVec3::ZERO;
        }
        assert!(
            filtered(&grid, &raw(&grid, REPAIRED), FILTER)
                .iter()
                .all(|&a| a == 1.)
        );
    }
}

#[test]
fn quality_filter_is_symmetric_normalized_and_preserves_fractional_boundaries() {
    let camera = Camera {
        orthographic: false,
        rect: [1.25, 2.75, 8.5, 7.25],
    };
    let grid = Grid::new([12, 12], camera, |p| {
        let (o, d) = camera.ray(p);
        Some(Hit {
            p: o + 4. * d,
            geometric: DVec3::Z,
            shading: DVec3::Z,
            owner: 0,
        })
    });
    let constant = vec![0.4; grid.hits.len()];
    let output = filtered(&grid, &constant, FILTER);
    for (hit, a) in grid.hits.iter().zip(output) {
        assert_eq!(a, if hit.is_some() { 0.4 } else { 1. });
    }
    let residues: Vec<_> = (0..4)
        .map(|r| {
            COEFFICIENTS
                .iter()
                .enumerate()
                .filter(|(i, _)| (*i as isize - 2).rem_euclid(4) == r)
                .map(|(_, w)| w)
                .sum::<f64>()
        })
        .collect();
    assert_eq!(residues, vec![1.; 4]);
    assert_eq!(
        COEFFICIENTS
            .iter()
            .enumerate()
            .map(|(i, w)| (i as f64 - 2.) * w)
            .sum::<f64>(),
        0.
    );
    let gradient: Vec<_> = (0..grid.hits.len())
        .map(|i| 0.1 + (i % 12) as f64 * 0.05)
        .collect();
    let ideal = filtered(
        &grid,
        &gradient,
        Filter {
            quantized: false,
            ..FILTER
        },
    );
    let quantized = filtered(&grid, &gradient, FILTER);
    assert!(
        ideal
            .iter()
            .zip(quantized)
            .all(|(a, b)| (a - b).abs() <= 1.5 / 255. + 1e-15)
    );
    assert_ne!(
        ideal,
        filtered(
            &grid,
            &gradient,
            Filter {
                symmetric: false,
                quantized: false,
                ..FILTER
            }
        )
    );
}

#[test]
fn quality_plane_filter_rejects_silhouette_bleed_and_keeps_grazing_plane() {
    let grid = fixture("white_slab", false);
    let input = raw(&grid, REPAIRED);
    let output = filtered(&grid, &input, FILTER);
    let mutation = filtered(
        &grid,
        &input,
        Filter {
            plane: false,
            ..FILTER
        },
    );
    let mut leaked = 0;
    for (i, hit) in grid.hits.iter().enumerate() {
        if hit.is_some_and(|h| h.owner == 1) {
            assert_eq!(input[i], 1.);
            assert_eq!(output[i], 1.);
            leaked += usize::from(mutation[i] < 1.);
        }
    }
    assert!(leaked > 0, "plane-weight mutation must actually leak");
    let camera = Camera {
        orthographic: false,
        rect: [0., 0., 8., 8.],
    };
    let normal = DVec3::new(0.9, 0., 0.1).normalize();
    let grid = Grid::new([8, 8], camera, |pixel| {
        let p = DVec3::new(pixel.x * 0.01, pixel.y * 0.01, -4. - pixel.x * 0.09);
        Some(Hit {
            p,
            geometric: normal,
            shading: normal,
            owner: 0,
        })
    });
    let mut impulse = vec![0.; 64];
    impulse[4 * 8 + 4] = 1.;
    let result = pass(
        &grid,
        &impulse,
        true,
        Filter {
            quantized: false,
            ..FILTER
        },
    );
    assert!((result[4 * 8 + 4] - 0.25).abs() < 1e-13);
    assert!((result[4 * 8 + 2] - 0.125).abs() < 1e-13);
}

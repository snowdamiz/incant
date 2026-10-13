use super::{
    UpscaleConfig,
    confidence_oracle::{History, cap},
    kernel_oracle::{Evidence, Kernel},
    oracle,
};
fn config() -> UpscaleConfig {
    UpscaleConfig {
        history_weight: 0.9,
        relative_depth: 0.05,
        absolute_depth: 0.02,
        reactive_threshold: 4.,
        reactive_strength: 0.5,
    }
}
fn kernel(n: u8, phase: usize) -> Kernel {
    Kernel::new([128, 128], [0., 0., 128., 128.], n, phase).unwrap()
}
#[test]
fn true_sample_positions_invert_mapping_for_every_scale_and_phase() {
    for rect in [[0., 0., 65., 33.], [3.25, 5.5, 65., 33.]] {
        for n in 4..=16 {
            for phase in 0..8 {
                let k = Kernel::new([71, 41], rect, n, phase).unwrap();
                for point in [[10.5, 10.5], [33.5, 20.5], [64.5, 32.5]] {
                    for t in k.taps(point).unwrap().into_iter().flatten() {
                        let back = oracle::source(t.position, k.output, k.source, k.jitter);
                        for (a, index) in back.into_iter().zip(t.texel) {
                            assert!((a - (f64::from(index) + 0.5)).abs() < 1e-9);
                        }
                    }
                }
            }
        }
    }
    for (size, rect, n) in [
        ([0, 1], [0., 0., 1., 1.], 16),
        ([8193, 1], [0., 0., 8193., 1.], 16),
        ([65, 33], [0., 0., 65., 33.], 3),
        ([65, 33], [0., 0., f32::NAN, 33.], 8),
    ] {
        assert!(Kernel::new(size, rect, n, 0).is_none());
    }
    assert!(kernel(8, 0).taps([f64::NAN, 1.5]).is_none());
    assert!(kernel(8, 0).taps([-0.5, 1.5]).is_none());
}
#[test]
fn cycle_mass_reports_approximation_failure_and_preserves_actual_interior_bound() {
    let mut strict_prose_failed = false;
    for n in 4..=16 {
        let mut min = f64::INFINITY;
        let mut max: f64 = 0.;
        let mut furthest: f64 = 0.;
        for y in 32..48 {
            for x in 32..48 {
                let p = [x as f64 + 0.5, y as f64 + 0.5];
                let mut mass = 0.;
                let mut distance = f64::INFINITY;
                for phase in 0..8 {
                    for t in kernel(n, phase).taps(p).unwrap().into_iter().flatten() {
                        assert!(t.weight > 0. && t.weight.is_finite());
                        mass += t.weight;
                        distance = distance.min(
                            (0..2)
                                .map(|i| (p[i] - t.position[i]).powi(2))
                                .sum::<f64>()
                                .sqrt(),
                        );
                    }
                }
                min = min.min(mass);
                max = max.max(mass);
                furthest = furthest.max(distance);
            }
        }
        assert!(min >= 7.9 && max <= 8.1, "n={n}: {min}..{max}");
        assert!(furthest <= if n >= 8 { 0.40 } else { 1.0 } + 1e-12);
        let prose = min >= 7.93 && max <= 8.07;
        strict_prose_failed |= !prose;
        println!(
            "kernel_mass n={n} min={min:.12} max={max:.12} nearest_max={furthest:.12} prose_8_plus_minus_007={prose}"
        );
    }
    assert!(
        strict_prose_failed,
        "Do not relabel the design's rounded prose as an exact bound"
    );
}
#[test]
fn clipped_fractional_views_skip_unique_samples_and_renormalize_constants() {
    let rect = [3.25, 5.5, 65., 33.];
    for n in 4..=16 {
        for phase in 0..8 {
            let k = Kernel::new([71, 41], rect, n, phase).unwrap();
            for p in [[3.5, 5.5], [68.5 - 1., 37.5], [33.5, 20.5]] {
                let taps = k.taps(p).unwrap();
                let active: Vec<_> = taps.into_iter().flatten().collect();
                assert!(!active.is_empty() && active.len() <= 9);
                for (i, a) in active.iter().enumerate() {
                    assert!(!active[..i].iter().any(|b| b.texel == a.texel));
                }
                let evidence = k.evidence(p, |_| [2., 0.25, 0., 0.5]).unwrap();
                for (got, want) in evidence
                    .sum
                    .map(|v| v / evidence.mass)
                    .into_iter()
                    .zip([2., 0.25, 0., 0.5])
                {
                    assert!((got - want).abs() < 1e-12);
                }
                if p == [3.5, 5.5] {
                    assert!(active.len() < 9);
                }
            }
        }
    }
    // Clipped mass is not normalized to an invented full frame's confidence.
    let k = Kernel::new([71, 41], rect, 8, 0).unwrap();
    let edge = k.evidence([3.5, 5.5], |_| [1.; 4]).unwrap();
    let complete = kernel(8, 0).evidence([3.5, 5.5], |_| [1.; 4]).unwrap();
    assert!(edge.mass < complete.mass);
}
#[test]
fn confidence_uses_evidence_mass_and_preserves_premultiplied_clear() {
    for n in 4..=16 {
        let k = kernel(n, 0);
        let e = k
            .evidence([40.5, 40.5], |p| {
                if p[0] < 40.5 {
                    [2., 1., 0., 0.5]
                } else {
                    [99., 99., 99., 0.]
                }
            })
            .unwrap();
        let h = History::default().step(e, k, config(), true).unwrap();
        assert!(h.color.iter().all(|v| v.is_finite()));
        assert!((h.color[0] - 4. * h.color[3]).abs() < 1e-12);
        let clear = k.evidence([40.5, 40.5], |_| [99., 99., 99., 0.]).unwrap();
        assert!(clear.mass > 0.);
        assert_eq!(clear.sum, [0.; 4]);
        assert_eq!(
            History::default()
                .step(clear, k, config(), false)
                .unwrap()
                .color,
            [0.; 4]
        );
        for color in [[0.; 4], [2., 0.5, 65504., 1.]] {
            let e = k.evidence([40.5, 40.5], |_| color).unwrap();
            let h = History::default().step(e, k, config(), true).unwrap();
            for (a, b) in h.color.into_iter().zip(color) {
                assert!((a - b).abs() < 1e-9);
            }
        }
    }
    for h in [-1., 1., f64::NAN] {
        assert!(cap(h, 2.).is_none());
    }
    for r in [0., 4.01, f64::INFINITY] {
        assert!(cap(0.9, r).is_none());
    }
    let invalid = Evidence {
        sum: [0.; 4],
        mass: 0.,
    };
    assert!(
        History::default()
            .step(invalid, kernel(8, 0), config(), false)
            .is_none()
    );
    assert!((cap(0.9, 1.).unwrap() - 9.).abs() < 1e-12);
    assert!((cap(0.9, 4. / 3.).unwrap() - 16.).abs() < 1e-12);
    assert!((cap(0.9, 2.).unwrap() - 36.).abs() < 1e-12);
}
#[test]
fn cuts_repeat_first_frame_and_scale_increase_clamps_before_density_change() {
    let mut h = History::default();
    for phase in 0..64 {
        let k = kernel(4, phase);
        h = h
            .step(
                k.evidence([40.5, 40.5], |_| [1.; 4]).unwrap(),
                k,
                config(),
                false,
            )
            .unwrap();
    }
    let k = kernel(16, 0);
    let black = k.evidence([40.5, 40.5], |_| [0., 0., 0., 1.]).unwrap();
    let cap = cap(f64::from(config().history_weight), 1.).unwrap();
    let weight = h.mass.min(cap) / 16.;
    let changed = h.step(black, k, config(), false).unwrap();
    assert!((changed.color[0] - weight / (weight + black.mass)).abs() < 1e-12);
    let first = History::default().step(black, k, config(), false).unwrap();
    assert_eq!(h.step(black, k, config(), true).unwrap(), first);
    let small = kernel(4, 0);
    let white = small.evidence([40.5, 40.5], |_| [1.; 4]).unwrap();
    let downward = changed.step(white, small, config(), false).unwrap();
    assert!(
        (downward.color[0]
            - (changed.color[0] * changed.mass + white.sum[0]) / (changed.mass + white.mass))
            .abs()
            < 1e-12
    );
}

#[test]
fn bounded_confidence_refuses_nonfinite_or_excess_evidence_but_reset_discards_old_mass() {
    let k = kernel(8, 0);
    let e = k.evidence([40.5, 40.5], |_| [1.; 4]).unwrap();
    for bad in [
        Evidence {
            mass: f64::NAN,
            ..e
        },
        Evidence {
            sum: [f64::INFINITY; 4],
            ..e
        },
        Evidence {
            sum: [1e100; 4],
            ..e
        },
        Evidence { mass: 1000., ..e },
    ] {
        assert!(History::default().step(bad, k, config(), false).is_none());
    }
    for mass in [f64::NAN, f64::INFINITY, -1., 1000.] {
        let h = History {
            mass,
            ..History::default()
        };
        assert!(h.step(e, k, config(), false).is_none());
        assert_eq!(
            h.step(e, k, config(), true),
            History::default().step(e, k, config(), true)
        );
    }
}

#[test]
fn omitted_kernel_peak_bound_uses_actual_pixel_jitter_lattice_not_arbitrary_points() {
    let mut worst: f64 = 0.;
    for n in 4..=16 {
        for phase in 0..8 {
            let k = kernel(n, phase);
            for y in 32..48 {
                for x in 32..48 {
                    let point = [x as f64 + 0.5, y as f64 + 0.5];
                    let base = oracle::source(point, k.output, k.source, k.jitter)
                        .map(|v| v.floor() as i32);
                    // The first omitted ring contains the largest omitted tap; further
                    // lattice rings have strictly greater distance in at least one axis.
                    for dy in -2_i32..=2 {
                        for dx in -2_i32..=2 {
                            if dx.abs() <= 1 && dy.abs() <= 1 {
                                continue;
                            }
                            let position = k.position([base[0] + dx, base[1] + dy]);
                            worst = worst.max(k.weight(point, position) / k.norm);
                        }
                    }
                }
            }
        }
    }
    assert!(worst < 1e-4);
    let unrestricted = (-1.5_f64.powi(2) / (2. * 0.35_f64.powi(2))).exp();
    assert!(unrestricted > 1e-4);
    println!(
        "kernel_omission actual_lattice_peak_ratio={worst:.12} arbitrary_point_bound={unrestricted:.12}"
    );
}

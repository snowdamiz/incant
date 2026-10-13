//! Static scalar texture experiment. Analytic bilinear texels; no material LOD,
//! dynamic history admission, camera motion, tonemap or appearance inference.
use super::{UpscaleConfig, confidence_oracle::History, kernel_oracle::Kernel};
fn sample(p: [f64; 2], width: i32) -> [f64; 4] {
    let q = p.map(|x| x - 0.5);
    let b = q.map(|x| x.floor() as i32);
    let f = q.map(|x| x - x.floor());
    let mut value = 0.;
    for y in 0..2 {
        for x in 0..2 {
            let ix = b[0] + x;
            let iy = b[1] + y;
            let white = if width == 1 {
                ix.rem_euclid(2) == 0
            } else {
                (ix.div_euclid(width) + iy.div_euclid(width)).rem_euclid(2) == 0
            };
            value += f64::from(u8::from(white))
                * (if x == 0 { 1. - f[0] } else { f[0] })
                * (if y == 0 { 1. - f[1] } else { f[1] });
        }
    }
    [value, value, value, 1.]
}
#[test]
fn static_periodic_inputs_reach_phase_periodic_state_and_beat_bilinear_detail_control() {
    let c = UpscaleConfig {
        history_weight: 0.9,
        relative_depth: 0.05,
        absolute_depth: 0.02,
        reactive_threshold: 4.,
        reactive_strength: 0.5,
    };
    for n in 4..=16 {
        for width in [1, 2, 4] {
            let mut new_contrast = 0.;
            let mut old_contrast = 0.;
            let mut activity: f64 = 0.;
            let mut old_activity: f64 = 0.;
            let mut cycle_residual: f64 = 0.;
            for y in 32..48 {
                for x in 32..48 {
                    let p = [x as f64 + 0.5, y as f64 + 0.5];
                    let sign = if sample(p, width)[0] == 1. { 1. } else { -1. };
                    let mut h = History::default();
                    let mut old = 0.;
                    let mut previous_cycle = [0.; 8];
                    for tick in 0..48 {
                        let k = Kernel::new([128, 128], [0., 0., 128., 128.], n, tick).unwrap();
                        let f = |p| sample(p, width);
                        let before = h.color[0];
                        h = h.step(k.evidence(p, f).unwrap(), k, c, false).unwrap();
                        let bilinear = k.bilinear(p, f).unwrap()[0];
                        let before_old = old;
                        old = if tick == 0 {
                            bilinear
                        } else {
                            0.9 * old + 0.1 * bilinear
                        };
                        if tick >= 40 {
                            new_contrast += sign * h.color[0] / (128. * 8.);
                            old_contrast += sign * old / (128. * 8.);
                            activity = activity.max((h.color[0] - before).abs());
                            old_activity = old_activity.max((old - before_old).abs());
                            cycle_residual =
                                cycle_residual.max((h.color[0] - previous_cycle[tick % 8]).abs());
                        }
                        previous_cycle[tick % 8] = h.color[0];
                    }
                }
            }
            assert!(new_contrast.is_finite() && old_contrast.is_finite() && activity.is_finite());
            if n == 8 {
                assert!(new_contrast > old_contrast + 0.1);
                if width == 1 {
                    assert!(activity <= 0.05);
                }
            }
            println!(
                "kernel_pattern n={n} texels={width} contrast={new_contrast:.12} bilinear_control={old_contrast:.12} activity_max={activity:.12} bilinear_activity_max={old_activity:.12} last_cycle_residual={cycle_residual:.12}"
            );
        }
    }
}

#[test]
fn capped_static_cycle_is_an_affine_contraction_with_an_exact_periodic_limit() {
    use super::confidence_oracle::cap;
    let c = UpscaleConfig {
        history_weight: 0.9,
        relative_depth: 0.05,
        absolute_depth: 0.02,
        reactive_threshold: 4.,
        reactive_strength: 0.5,
    };
    for n in 4..=16 {
        for width in [1, 2, 4] {
            for point in [[40.5, 40.5], [41.5, 42.5]] {
                let mut a = 1.;
                let mut b = 0.;
                let max = cap(f64::from(c.history_weight), 16. / f64::from(n)).unwrap();
                for phase in 0..8 {
                    let k = Kernel::new([128, 128], [0., 0., 128., 128.], n, phase).unwrap();
                    let e = k.evidence(point, |p| sample(p, width)).unwrap();
                    let factor = max / (max + e.mass);
                    a *= factor;
                    b = factor * b + e.sum[0] / (max + e.mass);
                }
                assert!(a > 0. && a < 1.);
                let limit = b / (1. - a);
                assert!((0. ..=1.).contains(&limit));
                let mut h = History {
                    color: [limit, limit, limit, 1.],
                    mass: max,
                    ..History::default()
                };
                let mut perturbed = History {
                    color: [limit / 2., limit / 2., limit / 2., 1.],
                    mass: max,
                    ..History::default()
                };
                for phase in 0..8 {
                    let k = Kernel::new([128, 128], [0., 0., 128., 128.], n, phase).unwrap();
                    let e = k.evidence(point, |p| sample(p, width)).unwrap();
                    h = h.step(e, k, c, false).unwrap();
                    perturbed = perturbed.step(e, k, c, false).unwrap();
                }
                assert!((h.color[0] - limit).abs() < 1e-12);
                assert!((h.color[0] - perturbed.color[0] - a * limit / 2.).abs() < 1e-12);
            }
        }
    }
}

#[test]
fn static_texture_scale_changes_report_recovery_without_resetting_color() {
    let c = UpscaleConfig {
        history_weight: 0.9,
        relative_depth: 0.05,
        absolute_depth: 0.02,
        reactive_threshold: 4.,
        reactive_strength: 0.5,
    };
    for (from, to) in [(4, 16), (12, 4)] {
        for width in [1, 2, 4] {
            let mut contrast = [0.; 9];
            let mut unscaled = [0.; 9];
            for y in 32..48 {
                for x in 32..48 {
                    let p = [x as f64 + 0.5, y as f64 + 0.5];
                    let sign = if sample(p, width)[0] == 1. { 1. } else { -1. };
                    let mut h = History::default();
                    for tick in 0..48 {
                        let k = Kernel::new([128, 128], [0., 0., 128., 128.], from, tick).unwrap();
                        h = h
                            .step(k.evidence(p, |p| sample(p, width)).unwrap(), k, c, false)
                            .unwrap();
                    }
                    contrast[0] += sign * h.color[0] / 128.;
                    unscaled[0] += sign * h.color[0] / 128.;
                    let mut control = h;
                    for tick in 0..8 {
                        let k = Kernel::new([128, 128], [0., 0., 128., 128.], to, tick).unwrap();
                        let e = k.evidence(p, |p| sample(p, width)).unwrap();
                        // Explicit negative control omits only density reweighting.
                        control.source_width = k.source[2];
                        control = control.step(e, k, c, false).unwrap();
                        h = h.step(e, k, c, false).unwrap();
                        contrast[tick + 1] += sign * h.color[0] / 128.;
                        unscaled[tick + 1] += sign * control.color[0] / 128.;
                    }
                }
            }
            assert!(contrast.iter().chain(&unscaled).all(|v| v.is_finite()));
            if from < to {
                assert!(contrast[1] > unscaled[1]);
            } else {
                assert_eq!(contrast, unscaled);
            }
            println!(
                "kernel_scale from={from} to={to} texels={width} contrast={contrast:?} no_density_rescale={unscaled:?}"
            );
        }
    }
}

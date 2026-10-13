use super::coverage::*;

#[test]
fn depth_and_clear_silhouettes_and_mask_holes_accumulate_bounded_coverage() {
    for ortho in [false, true] {
        for case in [Case::DepthEdge, Case::ClearEdge, Case::MaskGrille] {
            let first = fixture(case, 0, ortho);
            let mut history = first.color;
            let mut depth = first.depth;
            let at = 3 * SIZE[0] + 4;
            for tick in 1..16 {
                let input = fixture(case, tick, ortho);
                let next = resolve(&input, &history, &depth, ortho);
                if tick >= 8 {
                    let a = next[at];
                    let b = history[at];
                    let background: [f32; 4] = if matches!(case, Case::DepthEdge) {
                        [0.1, 0.2, 0.3, 1.]
                    } else {
                        [0.; 4]
                    };
                    let foreground: [f32; 4] = [1., 0.5, 0.25, 1.];
                    for k in 0..4 {
                        let contrast = (foreground[k] - background[k]).abs();
                        assert!(
                            (a[k] - b[k]).abs() <= (1. - CONFIG.history_weight) * contrast + 1e-6,
                            "{case:?}/{tick}/{k}: {a:?} {b:?}"
                        );
                        if contrast > 0. {
                            assert!(
                                a[k] > foreground[k].min(background[k])
                                    && a[k] < foreground[k].max(background[k])
                            );
                        }
                    }
                    if !matches!(case, Case::DepthEdge) {
                        assert!((a[0] - a[3]).abs() < 1e-6);
                        assert!((a[1] - 0.5 * a[3]).abs() < 1e-6);
                    }
                }
                history = next;
                depth = input.depth;
            }
        }
    }
}
#[test]
fn moving_foreground_dilates_once_but_cannot_leave_a_two_pixel_trail() {
    for ortho in [false, true] {
        let first = fixture(Case::Moving, 0, ortho);
        let mut history = first.color;
        let mut depth = first.depth;
        for tick in 1..8 {
            let input = fixture(Case::Moving, tick, ortho);
            let next = resolve(&input, &history, &depth, ortho);
            // Background directly beside foreground borrows its valid motion.
            let (_, motion) = support(&input, 1 + tick, 3, ortho).unwrap();
            assert_eq!(motion, [-1. / RECT[2] as f32, 0.]);
            for x in RECT[0]..=(tick.min(RECT[0] + RECT[2] - 1)) {
                let at = 3 * SIZE[0] + x;
                assert_eq!(next[at], input.color[at], "trailing pixel {x}, tick {tick}");
            }
            history = next;
            depth = input.depth;
        }
    }
}
#[test]
fn clear_far_plane_invalid_owner_and_outside_viewport_remain_distinct() {
    for ortho in [false, true] {
        let mut input = fixture(Case::ClearEdge, 0, ortho);
        let at = 3 * SIZE[0] + 4;
        let (range, m) = support(&input, 4, 3, ortho).unwrap();
        assert!(range[0] < 2. && range[1] > FAR);
        assert_eq!(m, [0.; 2]);
        // A neighborhood two pixels beyond the edge contains only clear support.
        let (far, _) = support(&input, 6, 3, ortho).unwrap();
        assert!(far[0] > 90.);
        let invalid_history = vec![[1., 0.5, 0.25, 1.]; SIZE[0] * SIZE[1]];
        let depth = vec![encode(2., ortho); invalid_history.len()];
        let result = resolve(&input, &invalid_history, &depth, ortho);
        assert_eq!(result[3 * SIZE[0] + 6], [0.; 4]);
        assert_eq!(result[0], input.color[0]);
        input.color[at] = [0.25, 0.25, 0.25, 1.];
        for motion in [[0.; 4], [0., 0., f32::NAN, 1.], [0., 0., 1.1, 1.]] {
            input.motion[at] = motion;
            assert!(support(&input, 4, 3, ortho).is_none());
            assert_eq!(
                resolve(&input, &invalid_history, &depth, ortho)[at],
                input.color[at]
            );
        }
    }
}
#[test]
fn nearest_motion_is_stable_on_ties_and_depth_range_rejects_interior_disocclusion() {
    let mut input = fixture(Case::DepthEdge, 0, false);
    let at = 3 * SIZE[0] + 6;
    input.motion[at][0] = 0.1;
    assert_eq!(support(&input, 6, 3, false).unwrap().1, [0.1, 0.]);
    input.motion[at - 1][2] = encode(1., false);
    input.motion[at - 1][0] = -0.2;
    assert_eq!(support(&input, 6, 3, false).unwrap().1, [-0.2, 0.]);
    input.motion[at - 1][2] = f32::NAN;
    assert_eq!(support(&input, 6, 3, false).unwrap().1, [0.1, 0.]);
}

#[test]
fn malformed_zero_alpha_is_canonical_in_current_history_and_neighbor_clamp() {
    use super::oracle::clean;
    for a in [0., -0., -1.] {
        assert_eq!(clean([4., 3., 2., a]), [0.; 4]);
    }
    for ortho in [false, true] {
        let input = fixture(Case::MalformedClear, 0, ortho);
        let at = 3 * SIZE[0] + 4;
        let old = input.color.clone();
        let resolved = resolve(&input, &old, &input.depth, ortho);
        assert_eq!(resolved[at], [0.; 4]); // malformed current AND far history
        assert_eq!(resolved[at + 2], [0.; 4]); // entirely clear neighborhood
        let mut canonical = fixture(Case::ClearEdge, 0, ortho);
        let history_only = resolve(&canonical, &old, &input.depth, ortho);
        assert_eq!(history_only[at], [0.; 4]);
        canonical.motion[at] = [0.; 4];
        let mut dark = old;
        for c in &mut dark {
            if c[3] == 1. {
                *c = [0.25, 0.125, 0.0625, 1.];
            }
        }
        let neighbors = resolve(&input, &dark, &input.depth, ortho);
        assert!((neighbors[at - 1][0] - 0.325).abs() < 1e-6);
        assert_eq!(
            resolve(&canonical, &dark, &input.depth, ortho)[at - 1],
            neighbors[at - 1]
        );
        let mut invalid_current = fixture(Case::MalformedClear, 0, ortho);
        invalid_current.motion[at] = [f32::NAN; 4];
        assert_eq!(
            resolve(&invalid_current, &dark, &input.depth, ortho)[at],
            [0.; 4]
        );
    }
}

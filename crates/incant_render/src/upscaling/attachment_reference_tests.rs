use super::*;

#[test]
fn signed_zero_is_numeric_zero_at_both_attachment_boundaries() {
    // Reproduce the old bit-order trap without invoking the old panicking helper.
    assert_eq!(f16::from_f64((-0_f64).clamp(0., 1.)).to_bits(), 0x8000);
    let zero = representable(-0., 0., 1.);
    assert_eq!(zero, Interval { first: 0, last: 0 });
    assert!(zero.contains(-0.) && zero.contains(0.));
    assert!(!zero.contains(-2_f64.powi(-24)));
    let result = composite([-0., 0., -0., 1.], [0.; 4]);
    assert!(result[..3].iter().all(|interval| interval.contains(-0.)));
}

#[test]
fn inward_half_endpoints_preserve_ties_and_reject_adjacent_values() {
    // Half spacing at 1 is 2^-10. The midpoint uses ties-to-even to locate a
    // representable endpoint, then inward rounding excludes it when necessary.
    let gap = 2_f64.powi(-10);
    let got = representable(1. + gap / 2., 1. + 2.5 * gap, 65504.);
    assert_eq!(got.values(), [1. + gap, 1. + 2. * gap]);
    assert!(got.contains(1. + gap) && got.contains(1. + 2. * gap));
    assert!(!got.contains(1.) && !got.contains(1. + 3. * gap));
    assert!(!got.contains(1. + 1.5 * gap));
    assert!(!got.contains(f64::NAN) && !got.contains(f64::INFINITY));
    let subnormal = 2_f64.powi(-24);
    assert_eq!(
        representable(subnormal / 2., 2.5 * subnormal, 1.).values(),
        [subnormal, 2. * subnormal]
    );
}

#[test]
fn two_attachment_intervals_include_first_store_before_mixed_channel_composition() {
    // Analytic reconstruction value, independent of any device observation.
    let h = 0.350_797_854_141_216_45;
    let over = [f64::from(0.1_f32), f64::from(0.05_f32), 0., 0.5];
    let first = stored(h, 65504.);
    assert_eq!((first.first, first.last), (13724, 13725));
    let got = composite([h, h, h, 1.], over);
    assert_eq!((got[0].first, got[0].last), (13415, 13417));
    assert_eq!((got[1].first, got[1].last), (13109, 13111));
    assert_eq!((got[2].first, got[2].last), (12699, 12702));
    for interval in got {
        for value in interval.values() {
            assert!(interval.contains(value));
        }
        assert!(!interval.contains(f16::from_bits(interval.first - 1).to_f64()));
        assert!(!interval.contains(f16::from_bits(interval.last + 1).to_f64()));
    }
    // A point RNE model loses the independently admitted first store value.
    let point = f16::from_f64(over[1] + f16::from_f64(h).to_f64() * 0.5).to_f64();
    assert_eq!(point, 0.225_463_867_187_5);
    assert!(point - got[1].values()[0] > super::super::half_bound(point));
}

#[test]
fn sanitation_alpha_and_identity_do_not_create_history_or_exceed_domains() {
    let history = [0.25, 0.125, 0.0625, 0.5];
    let first = std::array::from_fn(|i| stored(history[i], if i == 3 { 1. } else { 65504. }));
    assert_eq!(composite(history, [99., 88., 77., 0.]), first);
    assert_eq!(composite(history, [f64::NAN, 0., 0., 1.]), first);
    let opaque = composite(history, [-1., 70000., 0.25, 2.]);
    assert_eq!(opaque, composite([0.; 4], [0., 65504., 0.25, 1.]));
    for (i, interval) in opaque.into_iter().enumerate() {
        let [low, high] = interval.values();
        assert!(low >= 0. && high <= if i == 3 { 1. } else { 65504. });
    }
    let clear = composite([9., 8., 7., 0.], [0.; 4]);
    for interval in clear {
        assert_eq!(interval.values(), [0., 0.]);
        assert!(!interval.contains(2_f64.powi(-24)));
    }
    let tiny = composite([0.25, 0.125, 0.0625, 2_f64.powi(-25)], [0.; 4]);
    for interval in tiny {
        assert!(interval.contains(0.));
    }
    assert!(tiny[0].contains(0.25) && tiny[3].contains(2_f64.powi(-24)));
}

#[test]
fn binade_crossings_use_the_maximum_final_allowance_monotonically() {
    for h in [2_f64.powi(-24), 0.4998, 0.5, 0.9996, 1., 65504.] {
        let history = [h, h, h, 1.];
        let overlay = [0.25, 0.125, 0.0625, 0.5];
        let bounds = composite(history, overlay);
        let first = stored(h, 65504.);
        for bits in first.first..=first.last {
            let loaded = f16::from_bits(bits).to_f64();
            for i in 0..3 {
                let value = (overlay[i] + loaded * 0.5).clamp(0., 65504.);
                let last = stored(value, 65504.);
                assert!(bounds[i].contains(last.values()[0]));
                assert!(bounds[i].contains(last.values()[1]));
            }
        }
    }
}

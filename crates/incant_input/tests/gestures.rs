use incant_input::*;
const DT: f64 = 1. / 60.;
fn touch(id: u32, phase: TouchPhase, position: [f64; 2]) -> InputEvent {
    InputEvent::Touch {
        id,
        phase,
        position,
    }
}

#[test]
fn taps_drags_swipes_and_cancellations_remain_distinct() {
    let mut input = InputRuntime::default();
    input
        .advance(&[touch(1, TouchPhase::Down, [1., 2.])], DT)
        .unwrap();
    let f = input
        .advance(&[touch(1, TouchPhase::Up, [2., 3.])], DT)
        .unwrap();
    assert_eq!(
        f.gestures,
        [Gesture::Tap {
            id: 1,
            position: [2., 3.]
        }]
    );
    assert!(f.touches.is_empty());
    input
        .advance(
            &[
                touch(1, TouchPhase::Down, [0., 0.]),
                touch(1, TouchPhase::Move, [20., 0.]),
            ],
            DT,
        )
        .unwrap();
    let f = input
        .advance(&[touch(1, TouchPhase::Up, [0., 0.])], DT)
        .unwrap();
    assert!(
        f.gestures.is_empty(),
        "moving away then returning is not a tap"
    );
    input
        .advance(&[touch(1, TouchPhase::Down, [0., 0.])], DT)
        .unwrap();
    let f = input
        .advance(
            &[
                touch(1, TouchPhase::Move, [30., 40.]),
                touch(1, TouchPhase::Move, [60., 80.]),
            ],
            DT,
        )
        .unwrap();
    assert_eq!(f.touches[&1].delta, [60., 80.]);
    let f = input
        .advance(&[touch(1, TouchPhase::Up, [60., 80.])], DT)
        .unwrap();
    assert!(
        matches!(&f.gestures[..],[Gesture::Swipe{direction:[x,y],distance,..}] if *x==0.6 && *y==0.8 && *distance==100.)
    );
    input
        .advance(&[touch(1, TouchPhase::Down, [0., 0.])], DT)
        .unwrap();
    let f = input
        .advance(&[touch(1, TouchPhase::Cancel, [100., 0.])], DT)
        .unwrap();
    assert!(f.gestures.is_empty());
}

#[test]
fn long_press_is_once_only_and_independent_of_supported_tick_rate() {
    for rate in [30, 60, 120, 240] {
        let dt = 1. / f64::from(rate);
        let mut input = InputRuntime::default();
        input
            .advance(&[touch(0, TouchPhase::Down, [3., 4.])], dt)
            .unwrap();
        for tick in 1..=rate / 2 {
            let f = input.advance(&[], dt).unwrap();
            if tick == rate / 2 {
                assert_eq!(
                    f.gestures,
                    [Gesture::LongPress {
                        id: 0,
                        position: [3., 4.]
                    }]
                );
            } else {
                assert!(f.gestures.is_empty());
            }
        }
        for _ in 0..rate {
            assert!(input.advance(&[], dt).unwrap().gestures.is_empty());
        }
        assert!(
            input
                .advance(&[touch(0, TouchPhase::Up, [3., 4.])], dt)
                .unwrap()
                .gestures
                .is_empty()
        );
    }
}

#[test]
fn pinch_uses_stable_contact_ids_and_does_not_turn_into_a_tap_or_swipe() {
    let mut input = InputRuntime::default();
    input
        .advance(
            &[
                touch(9, TouchPhase::Down, [10., 0.]),
                touch(3, TouchPhase::Down, [0., 0.]),
            ],
            DT,
        )
        .unwrap();
    let f = input
        .advance(&[touch(9, TouchPhase::Move, [0., 20.])], DT)
        .unwrap();
    assert!(
        matches!(&f.gestures[..],[Gesture::Pinch{ids:[3,9],center:[0.,10.],scale_delta:2.,rotation_delta:r}] if (*r-std::f64::consts::FRAC_PI_2).abs()<1e-12)
    );
    let f = input
        .advance(
            &[
                touch(9, TouchPhase::Up, [0., 20.]),
                touch(3, TouchPhase::Up, [100., 0.]),
            ],
            DT,
        )
        .unwrap();
    assert!(f.gestures.is_empty());
    input
        .advance(
            &[
                touch(3, TouchPhase::Down, [0., 0.]),
                touch(9, TouchPhase::Down, [0., 0.]),
            ],
            DT,
        )
        .unwrap();
    assert!(
        input
            .advance(&[touch(9, TouchPhase::Move, [30., 0.])], DT)
            .unwrap()
            .gestures
            .is_empty()
    );
    let f = input
        .advance(
            &[
                touch(9, TouchPhase::Up, [30., 0.]),
                touch(9, TouchPhase::Down, [300., 0.]),
            ],
            DT,
        )
        .unwrap();
    assert!(
        f.gestures.is_empty(),
        "reused IDs must not bridge different contact lifetimes"
    );
}

#[test]
fn rejected_packets_do_not_age_active_gestures() {
    let mut input = InputRuntime::default();
    input
        .advance(&[touch(0, TouchPhase::Down, [0., 0.])], DT)
        .unwrap();
    for _ in 0..60 {
        assert!(
            input
                .advance(&[touch(1, TouchPhase::Move, [0., 0.])], DT)
                .is_err()
        );
    }
    let f = input
        .advance(&[touch(0, TouchPhase::Up, [0., 0.])], DT)
        .unwrap();
    assert_eq!(
        f.gestures,
        [Gesture::Tap {
            id: 0,
            position: [0., 0.]
        }]
    );
}

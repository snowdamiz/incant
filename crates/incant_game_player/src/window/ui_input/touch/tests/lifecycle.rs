use super::*;

#[test]
fn presses_during_surface_wait_cannot_start_when_a_frame_eventually_presents() {
    let ui = runtime(true);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    input.defer_surface(Some(&ui), &mut physical).unwrap();
    for (id, p) in [(1, [10., 10.]), (2, [350., 80.])] {
        deliver(
            &mut input,
            &mut physical,
            Some(&ui),
            id,
            NativePhase::Started,
            p,
        );
    }
    input.presented(Some(&ui), &mut physical).unwrap();
    input.resume(&mut physical, true);
    for id in [1, 2] {
        deliver(
            &mut input,
            &mut physical,
            Some(&ui),
            id,
            NativePhase::Moved,
            [350., 80.],
        );
        deliver(
            &mut input,
            &mut physical,
            Some(&ui),
            id,
            NativePhase::Ended,
            [10., 10.],
        );
    }
    assert!(physical.events().is_empty() && input.receipts().is_empty());
    deliver(
        &mut input,
        &mut physical,
        Some(&ui),
        1,
        NativePhase::Started,
        [350., 80.],
    );
    assert!(matches!(
        physical.events(),
        [InputEvent::Touch {
            id: 3,
            phase: TouchPhase::Down,
            ..
        }]
    ));
}

#[test]
fn lifecycle_boundaries_cancel_contacts_and_ignore_late_native_continuations() {
    for boundary in 0..4 {
        let mut ui = runtime(true);
        let mut input = UiInput::new();
        let mut physical = PendingInput::new();
        let mut state = InputRuntime::default();
        deliver(
            &mut input,
            &mut physical,
            Some(&ui),
            1,
            NativePhase::Started,
            [350., 80.],
        );
        deliver(
            &mut input,
            &mut physical,
            Some(&ui),
            2,
            NativePhase::Started,
            [10., 10.],
        );
        state.advance(physical.events(), 1. / 60.).unwrap();
        physical.clear();
        accept(&mut ui, &mut input);
        match boundary {
            0 => {
                physical.reset_focus(false);
                input.reset_focus();
            }
            1 => input.layout_pending(Some(&ui), &mut physical).unwrap(),
            2 => input.defer_surface(Some(&ui), &mut physical).unwrap(),
            3 => {
                input.replace_session().unwrap();
                physical.reset_focus(true);
                ui = runtime(true);
                state = InputRuntime::default();
            }
            _ => unreachable!(),
        }
        let cancelled = physical.events().to_vec();
        deliver(
            &mut input,
            &mut physical,
            Some(&ui),
            1,
            NativePhase::Moved,
            [10., 10.],
        );
        deliver(
            &mut input,
            &mut physical,
            Some(&ui),
            2,
            NativePhase::Ended,
            [10., 10.],
        );
        assert_eq!(physical.events(), cancelled);
        state.advance(physical.events(), 1. / 60.).unwrap();
        assert!(state.frame().touches.is_empty() && state.frame().gestures.is_empty());
        if boundary == 0 {
            assert!(!state.frame().focused); // No native End is needed to release it.
        }
        let mut router = UiRouter::default();
        let next = router
            .prepare(
                &ui,
                physical.events(),
                input.receipts(),
                1. / 60.,
                [400., 100.],
                &mut Measure,
            )
            .unwrap();
        router.commit(&mut ui, next).unwrap();
        assert!(ui.actions().is_empty());
        physical.clear();
        input.accepted();
        input.presented(Some(&ui), &mut physical).unwrap();
        input.resume(&mut physical, true);
        deliver(
            &mut input,
            &mut physical,
            Some(&ui),
            1,
            NativePhase::Started,
            [350., 80.],
        );
        state.advance(physical.events(), 1. / 60.).unwrap();
        assert_eq!(
            state.frame().touches.keys().copied().collect::<Vec<_>>(),
            [3]
        );
    }
}

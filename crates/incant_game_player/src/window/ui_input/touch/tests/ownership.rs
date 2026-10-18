use super::*;

#[test]
fn ui_primary_and_secondary_contacts_never_leak_or_retarget() {
    let mut ui = runtime(true);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    deliver(
        &mut input,
        &mut physical,
        Some(&ui),
        1,
        NativePhase::Started,
        [10., 10.],
    );
    deliver(
        &mut input,
        &mut physical,
        Some(&ui),
        2,
        NativePhase::Started,
        [15., 10.],
    );
    accept(&mut ui, &mut input);
    deliver(
        &mut input,
        &mut physical,
        Some(&ui),
        2,
        NativePhase::Moved,
        [350., 80.],
    );
    deliver(
        &mut input,
        &mut physical,
        Some(&ui),
        2,
        NativePhase::Ended,
        [350., 80.],
    );
    deliver(
        &mut input,
        &mut physical,
        Some(&ui),
        1,
        NativePhase::Ended,
        [10., 10.],
    );
    assert!(physical.events().is_empty());
    accept(&mut ui, &mut input);
    assert_eq!(ui.actions(), [UiAction::Activated(id(2))]);
    deliver(
        &mut input,
        &mut physical,
        Some(&ui),
        1,
        NativePhase::Ended,
        [10., 10.],
    );
    accept(&mut ui, &mut input);
    assert!(ui.actions().is_empty());
    for end in [NativePhase::Ended, NativePhase::Cancelled] {
        deliver(
            &mut input,
            &mut physical,
            Some(&ui),
            3,
            NativePhase::Started,
            [10., 10.],
        );
        deliver(&mut input, &mut physical, Some(&ui), 3, end, [350., 80.]);
        accept(&mut ui, &mut input);
        assert!(ui.actions().is_empty());
    }
    assert!(physical.events().is_empty());
}

#[test]
fn mouse_and_touch_cannot_release_each_others_ui_capture() {
    for mouse_first in [true, false] {
        let mut ui = runtime(true);
        let mut input = UiInput::new();
        let mut physical = PendingInput::new();
        input
            .pointer(
                Some(&ui),
                &mut physical,
                PhysicalPosition::new(10., 10.),
                1.,
            )
            .unwrap();
        let touch = |input: &mut UiInput, physical: &mut PendingInput, phase| {
            deliver(input, physical, Some(&ui), 1, phase, [10., 10.])
        };
        if mouse_first {
            input.left(Some(&ui), &mut physical, true).unwrap();
            touch(&mut input, &mut physical, NativePhase::Started);
            touch(&mut input, &mut physical, NativePhase::Ended);
        } else {
            touch(&mut input, &mut physical, NativePhase::Started);
            input.left(Some(&ui), &mut physical, true).unwrap();
            input.left(Some(&ui), &mut physical, false).unwrap();
        }
        accept(&mut ui, &mut input);
        assert!(ui.actions().is_empty());
        if mouse_first {
            input.left(Some(&ui), &mut physical, false).unwrap();
        } else {
            deliver(
                &mut input,
                &mut physical,
                Some(&ui),
                1,
                NativePhase::Ended,
                [10., 10.],
            );
        }
        accept(&mut ui, &mut input);
        assert_eq!(ui.actions(), [UiAction::Activated(id(2))]);
        assert!(physical.events().is_empty());
    }
}

#[test]
fn gameplay_contact_crossing_ui_cancels_without_tap_or_ui_activation() {
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
    state.advance(physical.events(), 1. / 60.).unwrap();
    physical.clear();
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
        1,
        NativePhase::Ended,
        [10., 10.],
    );
    assert!(matches!(
        physical.events(),
        [InputEvent::Touch {
            phase: TouchPhase::Cancel,
            ..
        }]
    ));
    state.advance(physical.events(), 1. / 60.).unwrap();
    assert!(state.frame().touches.is_empty() && state.frame().gestures.is_empty());
    accept(&mut ui, &mut input);
    assert!(ui.actions().is_empty());
}

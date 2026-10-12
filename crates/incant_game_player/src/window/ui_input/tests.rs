use super::*;
use incant_ui::*;
use std::sync::Arc;
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
struct Measure;
impl TextMeasure for Measure {
    fn measure(
        &mut self,
        _: &Widget,
        text: &str,
        known: [Option<f32>; 2],
        _: [MeasureSpace; 2],
    ) -> Result<[f32; 2], UiError> {
        Ok([
            known[0].unwrap_or(text.len() as f32 * 8.),
            known[1].unwrap_or(20.),
        ])
    }
}
fn widgets(interactive: bool) -> Vec<Widget> {
    let text = UiText {
        content: "Name".into(),
        fonts: vec![id(9)],
        font_size: 16.,
        line_height: 20.,
        wrap: incant_types::TextWrap::Word,
    };
    [
        UiKind::Root,
        if interactive {
            UiKind::Button { text: text.clone() }
        } else {
            UiKind::Label { text: text.clone() }
        },
        UiKind::TextInput { text },
    ]
    .into_iter()
    .take(if interactive { 3 } else { 2 })
    .enumerate()
    .map(|(i, kind)| Widget {
        id: id(i as u128 + 1),
        incarnation: 1,
        parent: (i != 0).then(|| id(1)),
        node: Arc::new(UiNode {
            kind,
            disabled: false,
            order: 0,
            style: UiStyle {
                direction: UiDirection::Row,
                size: if i == 0 {
                    [UiLength::Auto; 2]
                } else {
                    [UiLength::Pixels(100.), UiLength::Pixels(30.)]
                },
                ..Default::default()
            },
        }),
    })
    .collect()
}
fn runtime(interactive: bool) -> UiRuntime {
    let mut ui = UiRuntime::new(widgets(interactive)).unwrap();
    let next = ui.prepare(&[], [400., 100.], &mut Measure).unwrap();
    ui.commit(next).unwrap();
    ui
}
fn accept(ui: &mut UiRuntime, input: &mut UiInput) {
    let viewport = ui.layout().unwrap().viewport;
    let next = ui
        .prepare(input.receipts(), viewport, &mut Measure)
        .unwrap();
    ui.commit(next).unwrap();
    input.accepted();
}
fn move_to(
    input: &mut UiInput,
    ui: &UiRuntime,
    physical: &mut PendingInput,
    p: [f64; 2],
    scale: f64,
) {
    input
        .pointer(Some(ui), physical, PhysicalPosition::new(p[0], p[1]), scale)
        .unwrap();
}
#[test]
fn ui_pointer_sequence_is_consumed_and_activates_only_once() {
    let mut ui = runtime(true);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    move_to(&mut input, &ui, &mut physical, [20., 20.], 2.);
    input.left(Some(&ui), &mut physical, true).unwrap();
    assert!(physical.events().is_empty());
    accept(&mut ui, &mut input);
    assert_eq!(ui.focus().unwrap().widget(), id(2));
    assert!(ui.actions().is_empty());
    input.left(Some(&ui), &mut physical, false).unwrap();
    accept(&mut ui, &mut input);
    assert_eq!(ui.actions(), [UiAction::Activated(id(2))]);
    input.left(Some(&ui), &mut physical, false).unwrap();
    accept(&mut ui, &mut input);
    assert!(ui.actions().is_empty());
    assert!(physical.events().is_empty());
}
#[test]
fn noninteractive_hud_passes_gameplay_and_capture_handoff_releases_once() {
    let hud = runtime(false);
    let interactive = runtime(true);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    move_to(&mut input, &hud, &mut physical, [10., 10.], 1.);
    input.left(Some(&hud), &mut physical, true).unwrap();
    let mut gameplay = incant_input::InputRuntime::default();
    gameplay.advance(physical.events(), 1. / 60.).unwrap();
    assert!(
        gameplay
            .frame()
            .mouse
            .buttons
            .held
            .contains(&MouseButton::Left)
    );
    physical.clear();
    input
        .synchronize(Some(&interactive), &mut physical)
        .unwrap();
    assert_eq!(
        physical.events(),
        [InputEvent::MouseButton {
            button: MouseButton::Left,
            down: false
        }]
    );
    gameplay.advance(physical.events(), 1. / 60.).unwrap();
    assert!(
        !gameplay
            .frame()
            .mouse
            .buttons
            .held
            .contains(&MouseButton::Left)
    );
    physical.clear();
    input
        .left(Some(&interactive), &mut physical, false)
        .unwrap();
    assert!(physical.events().is_empty());
    assert!(input.receipts().is_empty());
}
#[test]
fn resize_cancels_captured_press_without_activation_and_retains_text_focus() {
    let mut ui = runtime(true);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    move_to(&mut input, &ui, &mut physical, [110., 10.], 1.);
    input.left(Some(&ui), &mut physical, true).unwrap();
    accept(&mut ui, &mut input);
    let focus = ui.text_focus().unwrap();
    input.commit(Some(focus.clone()), "中").unwrap();
    let next = ui.prepare_refresh([500., 100.], &mut Measure).unwrap();
    ui.commit(next).unwrap();
    input.invalidate_pointer(Some(&ui), &mut physical).unwrap();
    assert_eq!(ui.text_focus(), Some(focus));
    move_to(&mut input, &ui, &mut physical, [110., 10.], 1.);
    input.left(Some(&ui), &mut physical, false).unwrap();
    accept(&mut ui, &mut input);
    assert_eq!(ui.text(id(3)).unwrap().text(), "Name中");
    assert!(
        !ui.actions()
            .iter()
            .any(|a| matches!(a, UiAction::Submitted { .. } | UiAction::Activated(_)))
    );
    assert!(physical.events().is_empty());
}
#[test]
fn wrong_world_receipts_never_match_even_when_all_numeric_ids_match() {
    let mut first = runtime(true);
    let mut second = runtime(true);
    for ui in [&mut first, &mut second] {
        let next = ui
            .prepare(&[UiEvent::Focus(id(3))], [400., 100.], &mut Measure)
            .unwrap();
        ui.commit(next).unwrap();
    }
    assert_ne!(first.pointer_token(), second.pointer_token());
    assert_ne!(first.focus(), second.focus());
    let events = [
        UiEvent::CancelPointer {
            target: first.pointer_token().unwrap(),
        },
        UiEvent::Commit {
            target: first.focus().unwrap(),
            text: "late".into(),
        },
    ];
    let next = second.prepare(&events, [400., 100.], &mut Measure).unwrap();
    second.commit(next).unwrap();
    assert_eq!(second.discarded_events(), 2);
    assert_eq!(second.text(id(3)).unwrap().text(), "Name");
}
#[test]
fn cancelled_release_after_owner_removal_never_becomes_gameplay_press_or_release() {
    let mut ui = runtime(true);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    move_to(&mut input, &ui, &mut physical, [10., 10.], 1.);
    input.left(Some(&ui), &mut physical, true).unwrap();
    accept(&mut ui, &mut input);
    input.synchronize(None, &mut physical).unwrap();
    input.left(None, &mut physical, false).unwrap();
    assert!(physical.events().is_empty());
    assert!(input.receipts().is_empty());
}
#[test]
fn text_receipts_are_bounded_before_copy_and_queue_survives_candidate_refusal() {
    let mut ui = runtime(true);
    let next = ui
        .prepare(&[UiEvent::Focus(id(3))], [400., 100.], &mut Measure)
        .unwrap();
    ui.commit(next).unwrap();
    let mut input = UiInput::new();
    input.commit(ui.text_focus(), "é中").unwrap();
    assert!(
        ui.prepare(input.receipts(), [f32::NAN, 100.], &mut Measure)
            .is_err()
    );
    assert_eq!(input.receipts().len(), 1);
    assert_eq!(ui.text(id(3)).unwrap().text(), "Name");
    accept(&mut ui, &mut input);
    assert_eq!(ui.text(id(3)).unwrap().text(), "Nameé中");
    let too_large = "x".repeat(incant_types::MAX_TEXT_BYTES + 1);
    assert!(matches!(
        input.commit(ui.text_focus(), &too_large),
        Err(Error::UiCapacity)
    ));
    assert!(input.receipts().is_empty());
    let chunk = "x".repeat(incant_types::MAX_TEXT_BYTES);
    for _ in 0..MAX_UI_PACKET_BYTES / incant_types::MAX_TEXT_BYTES {
        input.commit(ui.text_focus(), &chunk).unwrap();
    }
    let before = input.receipts().len();
    assert!(input.commit(ui.text_focus(), "x").is_err());
    assert_eq!(input.receipts().len(), before);
    input.accepted();
    for _ in 0..MAX_UI_EVENTS {
        input.commit(ui.text_focus(), "x").unwrap();
    }
    assert!(input.commit(ui.text_focus(), "x").is_err());
    assert_eq!(input.receipts().len(), MAX_UI_EVENTS);
}
#[test]
fn blur_drops_pending_text_and_neutralizes_capture_with_global_focus_loss() {
    let mut ui = runtime(true);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    let next = ui
        .prepare(&[UiEvent::Focus(id(3))], [400., 100.], &mut Measure)
        .unwrap();
    ui.commit(next).unwrap();
    input.commit(ui.text_focus(), "late").unwrap();
    input.reset_focus();
    physical.reset_focus(false);
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
    assert!(ui.focus().is_none());
    assert_eq!(ui.text(id(3)).unwrap().text(), "Name");
}
#[test]
fn invalid_coordinates_and_scale_leave_receipts_and_physical_queue_unchanged() {
    let ui = runtime(true);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    for (p, scale) in [([f64::NAN, 0.], 1.), ([1_000_001., 0.], 1.), ([1., 1.], 0.)] {
        assert!(
            input
                .pointer(
                    Some(&ui),
                    &mut physical,
                    PhysicalPosition::new(p[0], p[1]),
                    scale
                )
                .is_err()
        );
        assert!(input.receipts().is_empty());
        assert!(physical.events().is_empty());
        assert!(input.position.is_none());
    }
}

#[test]
fn cursor_departure_cancels_runtime_capture_without_changing_text_preedit() {
    let mut ui = runtime(true);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    move_to(&mut input, &ui, &mut physical, [110., 10.], 1.);
    input.left(Some(&ui), &mut physical, true).unwrap();
    accept(&mut ui, &mut input);
    let token = ui.text_focus().unwrap();
    let next = ui
        .prepare(
            &[UiEvent::Preedit {
                target: token.clone(),
                text: "中文".into(),
                cursor: Some(3..6),
            }],
            [400., 100.],
            &mut Measure,
        )
        .unwrap();
    ui.commit(next).unwrap();
    input.invalidate_pointer(Some(&ui), &mut physical).unwrap();
    accept(&mut ui, &mut input);
    assert_eq!(ui.text_focus(), Some(token));
    assert_eq!(ui.text(id(3)).unwrap().preedit().unwrap().text, "中文");
    // Clear preedit, then a synthetic release hit would submit if cancellation
    // had left the runtime capture alive. It must not do so.
    let events = [
        UiEvent::Cancel {
            target: ui.text_focus().unwrap(),
        },
        UiEvent::Pointer {
            position: [110., 10.],
            pressed: false,
            target: ui.pointer_token().unwrap(),
        },
    ];
    let next = ui.prepare(&events, [400., 100.], &mut Measure).unwrap();
    ui.commit(next).unwrap();
    assert!(ui.actions().is_empty());
    assert!(physical.events().is_empty());
}

#[test]
fn pending_presentation_fences_hits_until_consumption_and_unknown_cursor_never_leaks() {
    let mut ui = runtime(true);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    move_to(&mut input, &ui, &mut physical, [10., 10.], 1.);
    input.layout_pending(Some(&ui), &mut physical).unwrap();
    // Reserve may fail repeatedly or no tick may be due: still the same fence.
    for _ in 0..3 {
        input.synchronize(Some(&ui), &mut physical).unwrap();
        input.left(Some(&ui), &mut physical, true).unwrap();
        input.left(Some(&ui), &mut physical, false).unwrap();
    }
    assert!(physical.events().is_empty());
    assert!(
        !input
            .receipts()
            .iter()
            .any(|e| matches!(e, UiEvent::Pointer { .. }))
    );
    let next = ui.prepare_refresh([500., 100.], &mut Measure).unwrap();
    ui.commit(next).unwrap();
    input.invalidate_pointer(Some(&ui), &mut physical).unwrap();
    input.presented(Some(&ui), &mut physical).unwrap();
    // No cursor callback after resize: a stationary click has no trusted hit.
    input.left(Some(&ui), &mut physical, true).unwrap();
    input.left(Some(&ui), &mut physical, false).unwrap();
    assert!(physical.events().is_empty());
    move_to(&mut input, &ui, &mut physical, [10., 10.], 1.);
    input.left(Some(&ui), &mut physical, true).unwrap();
    input.left(Some(&ui), &mut physical, false).unwrap();
    accept(&mut ui, &mut input);
    assert_eq!(ui.actions(), [UiAction::Activated(id(2))]);
}

#[test]
fn resize_acquisition_retry_preserves_delivered_text_and_focus_but_blur_does_not() {
    let mut ui = runtime(true);
    let next = ui
        .prepare(&[UiEvent::Focus(id(3))], [400., 100.], &mut Measure)
        .unwrap();
    ui.commit(next).unwrap();
    let focus = ui.text_focus().unwrap();
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    input.commit(Some(focus.clone()), "x").unwrap();
    input.layout_pending(Some(&ui), &mut physical).unwrap();
    input.defer_surface();
    assert!(input.awaiting_surface());
    input.commit(Some(focus.clone()), "中").unwrap();
    let next = ui.prepare_refresh([500., 100.], &mut Measure).unwrap();
    ui.commit(next).unwrap();
    input.invalidate_pointer(Some(&ui), &mut physical).unwrap();
    input.presented(Some(&ui), &mut physical).unwrap();
    input.resume(&mut physical, true);
    assert!(!input.awaiting_surface());
    assert!(physical.events().is_empty()); // No fabricated global blur/resume.
    accept(&mut ui, &mut input);
    assert_eq!(ui.text_focus(), Some(focus));
    assert_eq!(ui.text(id(3)).unwrap().text(), "Namex中");
    input.commit(ui.text_focus(), "discard").unwrap();
    input.defer_surface();
    input.reset_focus(); // Actual blur/suspend/nonpresentable transition.
    physical.reset_focus(false);
    assert!(!input.awaiting_surface());
    assert!(input.receipts().is_empty());
    input.resume(&mut physical, true);
    assert_eq!(
        physical.events(),
        [
            InputEvent::Focus { focused: false },
            InputEvent::Focus { focused: true }
        ]
    );
}

#[test]
fn gamepad_release_during_preserved_surface_wait_cannot_leave_a_held_button() {
    use incant_input::{
        GamepadButton,
        gamepad::{DeviceEvent, GamepadAdapter},
    };
    let mut adapter = GamepadAdapter::default();
    let mut gameplay = incant_input::InputRuntime::default();
    let mut physical = PendingInput::new();
    let mut input = UiInput::new();
    let connection = adapter
        .event(0u8, DeviceEvent::Connected, true)
        .unwrap()
        .unwrap();
    let press = adapter
        .event(
            0u8,
            DeviceEvent::Button {
                button: GamepadButton::South,
                value: 1.,
            },
            true,
        )
        .unwrap()
        .unwrap();
    gameplay.advance(&[connection, press], 1. / 60.).unwrap();
    adapter.commit();
    assert!(
        gameplay.frame().gamepads[&0]
            .buttons
            .held
            .contains(&GamepadButton::South)
    );
    input.defer_surface();
    let release = adapter
        .event(
            0u8,
            DeviceEvent::Button {
                button: GamepadButton::South,
                value: 0.,
            },
            input.accepts_delivery(true, true),
        )
        .unwrap()
        .unwrap();
    physical.push(release).unwrap();
    input.resume(&mut physical, true);
    gameplay.advance(physical.events(), 1. / 60.).unwrap();
    adapter.commit();
    assert!(
        !gameplay.frame().gamepads[&0]
            .buttons
            .held
            .contains(&GamepadButton::South)
    );
    assert!(
        gameplay.frame().gamepads[&0]
            .buttons
            .released
            .contains(&GamepadButton::South)
    );
    input.defer_surface();
    assert!(!input.accepts_delivery(false, true));
    input.reset_focus();
    assert!(!input.accepts_delivery(true, true));
}

fn focus_text(ui: &mut UiRuntime) {
    let next = ui
        .prepare(&[UiEvent::Focus(id(3))], [400., 100.], &mut Measure)
        .unwrap();
    ui.commit(next).unwrap();
}
#[test]
fn managed_preedit_commit_and_cancel_use_the_exact_accepted_text_owner() {
    use tao::text_input::TextInputEvent as Native;
    let mut ui = runtime(true);
    focus_text(&mut ui);
    let mut input = UiInput::new();
    let (serial, enabled) = input.text_session(Some(&ui), true).unwrap().unwrap();
    assert!(enabled);
    input
        .native_text(
            Some(&ui),
            serial,
            &Native::Preedit {
                text: "a𝄞中".into(),
                cursor: Some(1..5),
            },
        )
        .unwrap();
    accept(&mut ui, &mut input);
    let buffer = ui.text(id(3)).unwrap();
    assert_eq!(buffer.text(), "Name");
    assert_eq!(buffer.preedit().unwrap().text, "a𝄞中");
    assert_eq!(input.text_session(Some(&ui), true).unwrap(), None);
    input
        .native_text(Some(&ui), serial, &Native::Commit("日本".into()))
        .unwrap();
    accept(&mut ui, &mut input);
    assert!(ui.text(id(3)).unwrap().text().contains("日本"));
    assert!(ui.text(id(3)).unwrap().preedit().is_none());
    input
        .native_text(
            Some(&ui),
            serial,
            &Native::Preedit {
                text: "x".into(),
                cursor: Some(1..1),
            },
        )
        .unwrap();
    accept(&mut ui, &mut input);
    input
        .native_text(Some(&ui), serial, &Native::Cancel)
        .unwrap();
    accept(&mut ui, &mut input);
    assert!(ui.text(id(3)).unwrap().preedit().is_none());
}
#[test]
fn native_callback_from_blurred_replaced_or_other_world_owner_is_never_retargeted() {
    use tao::text_input::TextInputEvent as Native;
    let mut ui = runtime(true);
    focus_text(&mut ui);
    let mut input = UiInput::new();
    let (first, _) = input.text_session(Some(&ui), true).unwrap().unwrap();
    let (disabled, on) = input.text_session(Some(&ui), false).unwrap().unwrap();
    assert!(!on && disabled > first);
    input
        .native_text(Some(&ui), first, &Native::Commit("old".into()))
        .unwrap();
    let (second, _) = input.text_session(Some(&ui), true).unwrap().unwrap();
    assert!(second > disabled);
    input
        .native_text(
            Some(&ui),
            first,
            &Native::Preedit {
                text: "old".into(),
                cursor: None,
            },
        )
        .unwrap();
    let mut other = runtime(true);
    focus_text(&mut other);
    input
        .native_text(Some(&other), second, &Native::Commit("other".into()))
        .unwrap();
    let (third, _) = input.text_session(Some(&other), true).unwrap().unwrap();
    input
        .native_text(Some(&other), second, &Native::Commit("replaced".into()))
        .unwrap();
    assert!(input.receipts().is_empty());
    assert_eq!(input.stale_text_events(), 4);
    input
        .native_text(Some(&other), third, &Native::Commit("new".into()))
        .unwrap();
    accept(&mut other, &mut input);
    assert!(other.text(id(3)).unwrap().text().contains("new"));
    assert_eq!(ui.text(id(3)).unwrap().text(), "Name");
}
#[test]
fn managed_text_refusal_does_not_publish_partial_queue_and_surface_wait_keeps_session() {
    use tao::text_input::{TextInputError, TextInputEvent as Native};
    let mut ui = runtime(true);
    focus_text(&mut ui);
    let mut input = UiInput::new();
    let mut physical = PendingInput::new();
    let (serial, _) = input.text_session(Some(&ui), true).unwrap().unwrap();
    input
        .native_text(
            Some(&ui),
            serial,
            &Native::Preedit {
                text: "中".into(),
                cursor: Some(3..3),
            },
        )
        .unwrap();
    for bad in [
        Native::Preedit {
            text: "中".into(),
            cursor: Some(1..2),
        },
        Native::Commit("x".repeat(incant_types::MAX_TEXT_BYTES + 1)),
        Native::Refused(TextInputError::Replacement),
    ] {
        assert!(input.native_text(Some(&ui), serial, &bad).is_err());
        assert_eq!(input.receipts().len(), 1);
        assert_eq!(ui.text(id(3)).unwrap().text(), "Name");
    }
    input.layout_pending(Some(&ui), &mut physical).unwrap();
    input.defer_surface();
    assert_eq!(
        input
            .text_session(Some(&ui), input.accepts_delivery(true, true))
            .unwrap(),
        None
    );
    input.resume(&mut physical, true);
    assert_eq!(input.text_session(Some(&ui), true).unwrap(), None);
    accept(&mut ui, &mut input);
    assert_eq!(ui.text(id(3)).unwrap().preedit().unwrap().text, "中");
}

#[test]
fn ime_consumed_enter_never_submits_and_releases_prior_gameplay_hold() {
    use incant_input::{InputRuntime, KeyCode};
    let mut ui = runtime(true);
    focus_text(&mut ui);
    let input = UiInput::new();
    let mut pending = PendingInput::new();
    let mut physical = InputRuntime::default();
    physical
        .advance(
            &[
                InputEvent::Focus { focused: true },
                InputEvent::Key {
                    code: KeyCode::Enter,
                    down: true,
                },
            ],
            1. / 60.,
        )
        .unwrap();
    input
        .consumed_key(&mut pending, tao::keyboard::KeyCode::Enter, false)
        .unwrap();
    input
        .consumed_key(&mut pending, tao::keyboard::KeyCode::Enter, true)
        .unwrap();
    assert_eq!(pending.events().len(), 1);
    physical.advance(pending.events(), 1. / 60.).unwrap();
    assert!(!physical.frame().keyboard.held.contains(&KeyCode::Enter));
    // A consumed control generates no UI Activate or committed content receipt.
    assert!(input.receipts().is_empty());
    assert!(ui.actions().is_empty());
}

#[test]
fn retired_native_payloads_are_discarded_before_semantic_validation() {
    use tao::text_input::{TextInputError, TextInputEvent as Native};
    let mut ui = runtime(true);
    focus_text(&mut ui);
    let mut input = UiInput::new();
    let (old, _) = input.text_session(Some(&ui), true).unwrap().unwrap();
    input.text_session(Some(&ui), false).unwrap();
    let (current, _) = input.text_session(Some(&ui), true).unwrap().unwrap();
    let bad = [
        Native::Commit("x".repeat(incant_types::MAX_TEXT_BYTES + 1)),
        Native::Commit("\n".into()),
        Native::Preedit {
            text: "中".into(),
            cursor: Some(1..2),
        },
        Native::Preedit {
            text: "x".into(),
            cursor: Some(usize::MAX..usize::MAX),
        },
        Native::Refused(TextInputError::Replacement),
    ];
    for event in &bad {
        input.native_text(Some(&ui), old, event).unwrap();
        assert!(input.receipts().is_empty());
        assert_eq!(ui.text(id(3)).unwrap().text(), "Name");
    }
    assert_eq!(input.stale_text_events(), bad.len() as u64);
    for event in &bad {
        assert!(input.native_text(Some(&ui), current, event).is_err());
        assert!(input.receipts().is_empty());
    }
    input
        .native_text(Some(&ui), current, &Native::Commit("accepted".into()))
        .unwrap();
    accept(&mut ui, &mut input);
    assert!(ui.text(id(3)).unwrap().text().ends_with("accepted"));
}

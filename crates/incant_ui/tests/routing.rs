use incant_input::{GamepadButton, InputEvent, InputRuntime, KeyCode};
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
            known[0].unwrap_or(text.len() as f32 * 8.0),
            known[1].unwrap_or(20.0),
        ])
    }
}
fn widgets() -> Vec<Widget> {
    let text = UiText {
        content: "A👩‍💻B".into(),
        fonts: vec![id(9)],
        font_size: 16.0,
        line_height: 20.0,
        wrap: incant_types::TextWrap::Word,
    };
    [
        UiKind::Root,
        UiKind::Button { text: text.clone() },
        UiKind::TextInput { text: text.clone() },
        UiKind::Button { text },
    ]
    .into_iter()
    .enumerate()
    .map(|(i, kind)| Widget {
        incarnation: 1,
        id: id((i + 1) as u128),
        parent: (i != 0).then(|| id(1)),
        node: Arc::new(UiNode {
            kind,
            style: if i == 0 {
                UiStyle {
                    direction: UiDirection::Row,
                    gap: [10.0; 2],
                    ..Default::default()
                }
            } else {
                UiStyle {
                    size: [UiLength::Pixels(90.0), UiLength::Pixels(30.0)],
                    ..Default::default()
                }
            },
            order: 0,
            disabled: false,
        }),
    })
    .collect()
}
fn runtime() -> UiRuntime {
    let mut ui = UiRuntime::new(widgets()).unwrap();
    let candidate = ui.prepare(&[], [400.0, 100.0], &mut Measure).unwrap();
    ui.commit(candidate).unwrap();
    ui
}
fn key(code: KeyCode, down: bool) -> InputEvent {
    InputEvent::Key { code, down }
}
fn route(
    router: &mut UiRouter,
    ui: &mut UiRuntime,
    game: &mut InputRuntime,
    events: &[InputEvent],
    receipts: &[UiEvent],
) -> incant_input::UiSnapshot {
    let pending = router
        .prepare(ui, events, receipts, 1.0 / 60.0, [400., 100.], &mut Measure)
        .unwrap();
    game.advance(pending.gameplay(), 1.0 / 60.0).unwrap();
    let snapshot = pending.snapshot().clone();
    router.commit(ui, pending).unwrap();
    snapshot
}
#[test]
fn keyboard_focus_captures_pairs_releases_prior_gameplay_and_never_invents_text() {
    let mut ui = runtime();
    let mut router = UiRouter::default();
    let mut game = InputRuntime::default();
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[key(KeyCode::KeyA, true)],
        &[],
    );
    assert!(game.frame().keyboard.held.contains(&KeyCode::KeyA));
    let focused = route(
        &mut router,
        &mut ui,
        &mut game,
        &[
            key(KeyCode::Tab, true),
            key(KeyCode::Tab, false),
            key(KeyCode::Tab, true),
        ],
        &[],
    );
    assert!(focused.text_input);
    assert_eq!(ui.focus().unwrap().widget(), id(3));
    assert!(game.frame().keyboard.held.is_empty());
    assert_eq!(
        game.frame()
            .keyboard
            .released
            .iter()
            .copied()
            .collect::<Vec<_>>(),
        [KeyCode::KeyA]
    );
    assert!(!game.frame().keyboard.pressed.contains(&KeyCode::Tab));
    assert_eq!(ui.text(id(3)).unwrap().text(), "A👩‍💻B");
    // Leaving the field does not transfer held A back to gameplay. Even a
    // repeated physical key-down remains suppressed until its matching release.
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[
            key(KeyCode::Tab, false),
            key(KeyCode::Tab, true),
            key(KeyCode::KeyA, true),
        ],
        &[],
    );
    assert_eq!(ui.focus().unwrap().widget(), id(4));
    assert!(game.frame().keyboard.held.is_empty());
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[key(KeyCode::KeyA, false), key(KeyCode::KeyA, true)],
        &[],
    );
    assert!(game.frame().keyboard.pressed.contains(&KeyCode::KeyA));
}
#[test]
fn gamepad_navigation_threshold_and_activation_are_ui_only_and_validate_devices() {
    let mut ui = runtime();
    let mut router = UiRouter::default();
    let mut game = InputRuntime::default();
    let pad = |button, value| InputEvent::GamepadButton {
        id: 7,
        button,
        value,
    };
    assert!(
        router
            .prepare(
                &ui,
                &[pad(GamepadButton::South, 1.)],
                &[],
                1. / 60.,
                [400., 100.],
                &mut Measure
            )
            .is_err()
    );
    let frame = route(
        &mut router,
        &mut ui,
        &mut game,
        &[
            InputEvent::GamepadConnected { id: 7 },
            pad(GamepadButton::DpadRight, 0.5),
            pad(GamepadButton::South, 0.5),
            pad(GamepadButton::South, 1.),
        ],
        &[],
    );
    assert_eq!(frame.events.len(), 1);
    assert_eq!(ui.actions(), [UiAction::Activated(id(2))]);
    assert!(game.frame().gamepads[&7].buttons.held.is_empty());
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[
            InputEvent::Focus { focused: false },
            pad(GamepadButton::South, 1.),
            InputEvent::Focus { focused: true },
            pad(GamepadButton::South, 1.),
        ],
        &[],
    );
    assert!(ui.actions().is_empty()); // global focus loss cleared the widget focus
}
#[test]
fn stale_receipts_do_not_swallow_blur_and_bad_packets_preserve_router_state() {
    let mut ui = runtime();
    let mut router = UiRouter::default();
    let mut game = InputRuntime::default();
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[],
        &[UiEvent::Focus(id(3))],
    );
    let token = ui.focus().unwrap();
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[],
        &[UiEvent::Focus(id(2))],
    );
    let frame = route(
        &mut router,
        &mut ui,
        &mut game,
        &[InputEvent::Focus { focused: false }],
        &[UiEvent::Commit {
            target: token,
            text: "late".into(),
        }],
    );
    assert_eq!(frame.discarded_events, 1);
    assert!(frame.focused.is_none());
    assert!(!game.frame().focused);
    assert_eq!(ui.text(id(3)).unwrap().text(), "A👩‍💻B");
    let bad = [
        InputEvent::GamepadConnected { id: 9 },
        InputEvent::GamepadAxis {
            id: 9,
            axis: incant_input::GamepadAxis::LeftX,
            value: 2.,
        },
    ];
    assert!(
        router
            .prepare(&ui, &bad, &[], 1. / 60., [400., 100.], &mut Measure)
            .is_err()
    );
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[
            InputEvent::GamepadConnected { id: 9 },
            InputEvent::Focus { focused: true },
        ],
        &[],
    );
    assert!(game.frame().gamepads.contains_key(&9));
}
#[test]
fn dropped_stale_and_over_limit_candidates_do_not_publish_text_or_raw_edges() {
    let mut ui = runtime();
    let mut router = UiRouter::default();
    let mut game = InputRuntime::default();
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[],
        &[UiEvent::Focus(id(3))],
    );
    let old = ui.focus().unwrap();
    let candidate = router
        .prepare(
            &ui,
            &[key(KeyCode::Backspace, true)],
            &[],
            1. / 60.,
            [400., 100.],
            &mut Measure,
        )
        .unwrap();
    drop(candidate);
    assert_eq!(ui.text(id(3)).unwrap().text(), "A👩‍💻B");
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[key(KeyCode::End, true), key(KeyCode::Backspace, true)],
        &[],
    );
    assert_eq!(ui.text(id(3)).unwrap().text(), "A👩‍💻");
    let candidate = router
        .prepare(
            &ui,
            &[],
            &[UiEvent::Commit {
                target: old,
                text: "X".into(),
            }],
            1. / 60.,
            [400., 100.],
            &mut Measure,
        )
        .unwrap();
    route(&mut router, &mut ui, &mut game, &[], &[]);
    assert!(router.commit(&mut ui, candidate).is_err());
    assert_eq!(ui.text(id(3)).unwrap().text(), "A👩‍💻");
    let many: Vec<_> = (0..129)
        .flat_map(|_| [key(KeyCode::Tab, true), key(KeyCode::Tab, false)])
        .collect();
    // 129 presses fit; >256 widget operations must refuse before publication.
    let many: Vec<_> = many.iter().cloned().cycle().take(514).collect();
    assert!(
        router
            .prepare(&ui, &many, &[], 1. / 60., [400., 100.], &mut Measure)
            .is_err()
    );
    assert_eq!(ui.text(id(3)).unwrap().text(), "A👩‍💻");
}
fn replace(ui: &mut UiRuntime, widgets: Vec<Widget>) {
    let pending = ui.prepare_refresh([400., 100.], &mut Measure).unwrap();
    let pending = ui.reconcile(pending, widgets, &mut Measure).unwrap();
    ui.commit(pending).unwrap();
}
#[test]
fn noninteractive_hud_never_acquires_gamepad_or_tab_controls() {
    let mut ui = runtime();
    let mut hud = widgets();
    hud.truncate(2);
    let text = hud[1].node.kind.text().unwrap().clone();
    Arc::make_mut(&mut hud[1].node).kind = UiKind::Label { text };
    replace(&mut ui, hud);
    let mut router = UiRouter::default();
    let mut game = InputRuntime::default();
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[
            InputEvent::GamepadConnected { id: 1 },
            InputEvent::GamepadButton {
                id: 1,
                button: GamepadButton::South,
                value: 1.,
            },
            InputEvent::GamepadButton {
                id: 1,
                button: GamepadButton::DpadRight,
                value: 1.,
            },
            key(KeyCode::Tab, true),
        ],
        &[],
    );
    assert!(
        game.frame().gamepads[&1]
            .buttons
            .held
            .contains(&GamepadButton::South)
    );
    assert!(
        game.frame().gamepads[&1]
            .buttons
            .held
            .contains(&GamepadButton::DpadRight)
    );
    assert!(game.frame().keyboard.held.contains(&KeyCode::Tab));
    assert!(ui.focus().is_none());
    assert!(ui.actions().is_empty());
}
#[test]
fn gamepad_transfer_neutralizes_gameplay_and_survives_ui_removal_without_false_press() {
    let mut ui = runtime();
    replace(&mut ui, vec![]);
    let mut router = UiRouter::default();
    let mut game = InputRuntime::default();
    let pad = |button, value| InputEvent::GamepadButton {
        id: 1,
        button,
        value,
    };
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[
            InputEvent::GamepadConnected { id: 1 },
            pad(GamepadButton::South, 1.),
            pad(GamepadButton::DpadRight, 1.),
        ],
        &[],
    );
    assert_eq!(game.frame().gamepads[&1].buttons.held.len(), 2);
    replace(&mut ui, widgets());
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[
            pad(GamepadButton::South, 0.),
            pad(GamepadButton::DpadRight, 0.),
        ],
        &[UiEvent::Focus(id(2))],
    );
    assert!(game.frame().gamepads[&1].buttons.held.is_empty());
    assert_eq!(game.frame().gamepads[&1].buttons.released.len(), 2);
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[pad(GamepadButton::South, 1.)],
        &[],
    );
    assert_eq!(ui.actions(), [UiAction::Activated(id(2))]);
    replace(&mut ui, vec![]);
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[pad(GamepadButton::South, 1.)],
        &[],
    );
    assert!(game.frame().gamepads[&1].buttons.held.is_empty());
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[pad(GamepadButton::South, 0.), pad(GamepadButton::South, 1.)],
        &[],
    );
    assert!(
        game.frame().gamepads[&1]
            .buttons
            .pressed
            .contains(&GamepadButton::South)
    );
}
#[test]
fn gamepad_ownership_clears_on_disconnect_and_global_blur_including_analog_values() {
    let mut ui = runtime();
    let mut router = UiRouter::default();
    let mut game = InputRuntime::default();
    let pad = |value| InputEvent::GamepadButton {
        id: 7,
        button: GamepadButton::South,
        value,
    };
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[InputEvent::GamepadConnected { id: 7 }, pad(0.2)],
        &[],
    );
    assert_eq!(game.frame().gamepads[&7].values[&GamepadButton::South], 0.2);
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[],
        &[UiEvent::Focus(id(2))],
    );
    assert_eq!(game.frame().gamepads[&7].values[&GamepadButton::South], 0.);
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[
            InputEvent::GamepadDisconnected { id: 7 },
            InputEvent::GamepadConnected { id: 7 },
            pad(1.),
        ],
        &[],
    );
    assert_eq!(ui.actions(), [UiAction::Activated(id(2))]);
    route(
        &mut router,
        &mut ui,
        &mut game,
        &[
            InputEvent::Focus { focused: false },
            InputEvent::Focus { focused: true },
            pad(1.),
        ],
        &[],
    );
    assert!(
        game.frame().gamepads[&7]
            .buttons
            .held
            .contains(&GamepadButton::South)
    );
}
#[test]
fn unchanged_maximum_hud_reuses_layout_and_accepts_full_physical_burst_without_measurement() {
    let mut all = widgets();
    all.truncate(1);
    let label = UiKind::Label {
        text: UiText {
            content: "HUD".into(),
            fonts: vec![id(9)],
            font_size: 16.,
            line_height: 20.,
            wrap: incant_types::TextWrap::None,
        },
    };
    for n in 2..=MAX_UI_NODES {
        all.push(Widget {
            id: id(n as u128),
            incarnation: 1,
            parent: Some(id(1)),
            node: Arc::new(UiNode {
                kind: label.clone(),
                style: UiStyle {
                    size: [UiLength::Pixels(1.), UiLength::Pixels(1.)],
                    ..Default::default()
                },
                order: 0,
                disabled: false,
            }),
        });
    }
    let mut ui = UiRuntime::new(all).unwrap();
    let pending = ui.prepare(&[], [400., 100.], &mut Measure).unwrap();
    ui.commit(pending).unwrap();
    let frame = Arc::clone(ui.layout().unwrap());
    assert!(!frame.has_focusable());
    struct NoMeasure;
    impl TextMeasure for NoMeasure {
        fn measure(
            &mut self,
            _: &Widget,
            _: &str,
            _: [Option<f32>; 2],
            _: [MeasureSpace; 2],
        ) -> Result<[f32; 2], UiError> {
            panic!("unchanged HUD must not reshape or layout")
        }
    }
    let router = UiRouter::default();
    let events: Vec<_> = (0..512)
        .flat_map(|_| [key(KeyCode::Tab, true), key(KeyCode::Tab, false)])
        .collect();
    let next = router
        .prepare(&ui, &events, &[], 1. / 60., [400., 100.], &mut NoMeasure)
        .unwrap();
    assert_eq!(next.gameplay(), events);
    assert!(Arc::ptr_eq(&frame, next.pending().layout()));
    assert!(next.snapshot().is_empty());
}

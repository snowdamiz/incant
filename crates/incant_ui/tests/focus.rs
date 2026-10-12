use incant_text::editing::{Movement, Selection};
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
fn accept(ui: &mut UiRuntime, events: &[UiEvent]) {
    let next = ui.prepare(events, [400.0, 100.0], &mut Measure).unwrap();
    ui.commit(next).unwrap();
}
#[test]
fn logical_keyboard_and_directional_gamepad_focus_activate_once() {
    let mut ui = runtime();
    accept(
        &mut ui,
        &[UiEvent::Traverse { reverse: false }, UiEvent::Activate],
    );
    assert_eq!(ui.focus().unwrap().widget(), id(2));
    assert_eq!(ui.actions(), [UiAction::Activated(id(2))]);
    accept(&mut ui, &[UiEvent::Navigate(Direction::Right)]);
    assert_eq!(ui.focus().unwrap().widget(), id(3));
    assert!(ui.actions().is_empty());
    accept(
        &mut ui,
        &[
            UiEvent::Navigate(Direction::Right),
            UiEvent::Traverse { reverse: false },
        ],
    );
    assert_eq!(ui.focus().unwrap().widget(), id(2));
    accept(&mut ui, &[UiEvent::Traverse { reverse: true }]);
    assert_eq!(ui.focus().unwrap().widget(), id(4));
}

#[test]
fn full_navigation_packet_uses_canonical_order_with_shuffled_widget_storage() {
    let seed = widgets();
    let mut widgets = vec![seed[0].clone()];
    Arc::make_mut(&mut widgets[0].node).style.direction = UiDirection::Column;
    Arc::make_mut(&mut widgets[0].node).style.gap = [0.; 2];
    for n in (2..=1024u128).rev() {
        let mut button = seed[1].clone();
        button.id = id(n);
        Arc::make_mut(&mut button.node).style.size = [UiLength::Pixels(10.), UiLength::Pixels(1.)];
        Arc::make_mut(&mut button.node).style.shrink = 0.;
        widgets.push(button);
    }
    let mut ui = UiRuntime::new(widgets).unwrap();
    let first = ui.prepare(&[], [100., 2048.], &mut Measure).unwrap();
    ui.commit(first).unwrap();
    let frame = Arc::clone(ui.layout().unwrap());
    let commands: Vec<_> = (0..256)
        .map(|_| UiEvent::Traverse { reverse: false })
        .collect();
    let next = ui.prepare(&commands, [100., 2048.], &mut Measure).unwrap();
    ui.commit(next).unwrap();
    assert_eq!(ui.focus().unwrap().widget(), id(257));
    let commands: Vec<_> = (0..256)
        .map(|_| UiEvent::Navigate(Direction::Down))
        .collect();
    let next = ui.prepare(&commands, [100., 2048.], &mut Measure).unwrap();
    ui.commit(next).unwrap();
    assert_eq!(ui.focus().unwrap().widget(), id(513));
    assert!(Arc::ptr_eq(&frame, ui.layout().unwrap()));
    let target = ui.pointer_token().unwrap();
    let next = ui
        .prepare(
            &[UiEvent::Pointer {
                target,
                position: [5., 100.5],
                pressed: true,
            }],
            [100., 2048.],
            &mut Measure,
        )
        .unwrap();
    ui.commit(next).unwrap();
    assert_eq!(ui.focus().unwrap().widget(), id(102));
}
#[test]
fn composition_commit_selection_and_rejected_batch_are_transactional() {
    let mut ui = runtime();
    accept(&mut ui, &[UiEvent::Focus(id(3))]);
    let target = ui.focus().unwrap();
    accept(
        &mut ui,
        &[
            UiEvent::Select {
                target: target.clone(),
                selection: Selection {
                    anchor: 1,
                    focus: 12,
                },
            },
            UiEvent::Preedit {
                target: target.clone(),
                text: "日本".into(),
                cursor: Some(3..6),
            },
        ],
    );
    assert_eq!(ui.text(id(3)).unwrap().text(), "A👩‍💻B");
    assert_eq!(ui.text(id(3)).unwrap().display_text(), "A日本B");
    assert!(ui.actions().is_empty());
    let before = Arc::clone(ui.layout().unwrap());
    let invalid = [
        UiEvent::Commit {
            target: target.clone(),
            text: "ok".into(),
        },
        UiEvent::Select {
            target: target.clone(),
            selection: Selection::caret(999),
        },
    ];
    assert!(ui.prepare(&invalid, [400.0, 100.0], &mut Measure).is_err());
    assert_eq!(ui.text(id(3)).unwrap().display_text(), "A日本B");
    assert!(Arc::ptr_eq(&before, ui.layout().unwrap()));
    let pending = ui
        .prepare(
            &[UiEvent::Commit {
                target: target.clone(),
                text: "界".into(),
            }],
            [400.0, 100.0],
            &mut Measure,
        )
        .unwrap();
    assert_eq!(ui.text(id(3)).unwrap().display_text(), "A日本B");
    ui.commit(pending).unwrap();
    assert_eq!(ui.text(id(3)).unwrap().text(), "A界B");
    assert_eq!(
        ui.actions(),
        [UiAction::ValueChanged {
            widget: id(3),
            value: Arc::from("A界B")
        }]
    );
    accept(
        &mut ui,
        &[
            UiEvent::Move {
                target: target.clone(),
                movement: Movement::Start,
                extend: false,
            },
            UiEvent::Delete { target },
        ],
    );
    assert_eq!(ui.text(id(3)).unwrap().text(), "界B");
}
#[test]
fn stale_ime_never_cancels_global_focus_loss_or_targets_new_owner() {
    let mut ui = runtime();
    accept(&mut ui, &[UiEvent::Focus(id(3))]);
    let old = ui.focus().unwrap();
    accept(
        &mut ui,
        &[UiEvent::Preedit {
            target: old.clone(),
            text: "X".into(),
            cursor: None,
        }],
    );
    accept(
        &mut ui,
        &[
            UiEvent::WindowFocus(false),
            UiEvent::Commit {
                target: old.clone(),
                text: "late".into(),
            },
        ],
    );
    assert!(ui.focus().is_none());
    assert!(ui.text(id(3)).unwrap().preedit().is_none());
    assert_eq!(ui.text(id(3)).unwrap().text(), "A👩‍💻B");
    assert_eq!(ui.discarded_events(), 1);
    accept(
        &mut ui,
        &[
            UiEvent::WindowFocus(true),
            UiEvent::Focus(id(3)),
            UiEvent::Commit {
                target: old.clone(),
                text: "late2".into(),
            },
        ],
    );
    assert_eq!(ui.discarded_events(), 1);
    assert_eq!(ui.text(id(3)).unwrap().text(), "A👩‍💻B");
    let mut replacement = runtime();
    accept(
        &mut replacement,
        &[
            UiEvent::Focus(id(3)),
            UiEvent::Commit {
                target: old,
                text: "wrong owner".into(),
            },
        ],
    );
    assert_eq!(replacement.discarded_events(), 1);
    assert_eq!(replacement.text(id(3)).unwrap().text(), "A👩‍💻B");
}
#[test]
fn pointer_release_needs_same_capture_and_current_displayed_layout() {
    let mut ui = runtime();
    let target = ui.pointer_token().unwrap();
    accept(
        &mut ui,
        &[
            UiEvent::Pointer {
                position: [10.0, 10.0],
                pressed: true,
                target: target.clone(),
            },
            UiEvent::Pointer {
                position: [10.0, 10.0],
                pressed: false,
                target: target.clone(),
            },
        ],
    );
    assert_eq!(ui.actions(), [UiAction::Activated(id(2))]);
    let next = ui
        .prepare(
            &[
                UiEvent::Pointer {
                    position: [10.0, 10.0],
                    pressed: true,
                    target: target.clone(),
                },
                UiEvent::WindowFocus(false),
            ],
            [500.0, 100.0],
            &mut Measure,
        )
        .unwrap();
    ui.commit(next).unwrap();
    assert!(ui.focus().is_none());
    assert!(ui.actions().is_empty());
    assert_eq!(ui.discarded_events(), 1);
    assert_eq!(ui.layout().unwrap().viewport, [500.0, 100.0]);
}
#[test]
fn malformed_stale_packet_is_refused_before_any_focus_change() {
    let mut ui = runtime();
    accept(&mut ui, &[UiEvent::Focus(id(3))]);
    let target = ui.focus().unwrap();
    let events = [
        UiEvent::WindowFocus(false),
        UiEvent::Preedit {
            target,
            text: "界".into(),
            cursor: Some(1..2),
        },
    ];
    assert!(ui.prepare(&events, [400.0, 100.0], &mut Measure).is_err());
    assert_eq!(ui.focus().unwrap().widget(), id(3));
}
#[test]
fn failed_measure_and_dropped_candidate_do_not_publish_text_or_actions() {
    struct Fails;
    impl TextMeasure for Fails {
        fn measure(
            &mut self,
            _: &Widget,
            _: &str,
            _: [Option<f32>; 2],
            _: [MeasureSpace; 2],
        ) -> Result<[f32; 2], UiError> {
            Err(UiError::Limit("fixture"))
        }
    }
    let mut ui = runtime();
    accept(&mut ui, &[UiEvent::Focus(id(3))]);
    let target = ui.focus().unwrap();
    assert!(
        ui.prepare(
            &[UiEvent::Commit {
                target: target.clone(),
                text: "X".into()
            }],
            [400.0, 100.0],
            &mut Fails
        )
        .is_err()
    );
    assert_eq!(ui.text(id(3)).unwrap().text(), "A👩‍💻B");
    assert!(ui.actions().is_empty());
    let pending = ui
        .prepare(
            &[UiEvent::Commit {
                target,
                text: "X".into(),
            }],
            [400.0, 100.0],
            &mut Measure,
        )
        .unwrap();
    drop(pending);
    assert_eq!(ui.text(id(3)).unwrap().text(), "A👩‍💻B");
    assert!(ui.actions().is_empty());
}

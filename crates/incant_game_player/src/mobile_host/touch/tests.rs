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

fn accept(ui: &mut UiRuntime, input: &mut NativeTouchInput) {
    let viewport = ui.layout().unwrap().viewport;
    let next = ui
        .prepare(input.receipts(), viewport, &mut Measure)
        .unwrap();
    ui.commit(next).unwrap();
    input.accepted();
    input.presented(Some(ui)).unwrap();
}
#[test]
fn ui_gesture_activates_once_and_simultaneous_second_finger_never_leaks() {
    let mut ui = runtime(true);
    let mut input = NativeTouchInput::default();
    input.presented(Some(&ui)).unwrap();
    input
        .touch(Some(&ui), 0, TouchPhase::Down, [10., 10.])
        .unwrap();
    input
        .touch(Some(&ui), 1, TouchPhase::Down, [15., 10.])
        .unwrap();
    accept(&mut ui, &mut input);
    assert!(ui.actions().is_empty());
    input
        .touch(Some(&ui), 1, TouchPhase::Up, [300., 10.])
        .unwrap();
    input
        .touch(Some(&ui), 0, TouchPhase::Up, [10., 10.])
        .unwrap();
    accept(&mut ui, &mut input);
    assert_eq!(ui.actions(), [UiAction::Activated(id(2))]);
    assert!(input.physical().is_empty());
    assert_eq!(
        input.touch(Some(&ui), 0, TouchPhase::Up, [10., 10.]),
        Err(TouchError::Invalid)
    );
}
#[test]
fn replacement_cancels_ui_and_gameplay_captures_without_retargeting_releases() {
    let mut first = runtime(true);
    let mut second = runtime(true);
    let mut input = NativeTouchInput::default();
    input.presented(Some(&first)).unwrap();
    input
        .touch(Some(&first), 0, TouchPhase::Down, [10., 10.])
        .unwrap();
    input
        .touch(Some(&first), 1, TouchPhase::Down, [350., 80.])
        .unwrap();
    let mut physical = incant_input::InputRuntime::default();
    physical.advance(input.physical(), 1. / 60.).unwrap();
    accept(&mut first, &mut input);
    input.presented(Some(&second)).unwrap();
    assert!(matches!(
        input.physical(),
        [InputEvent::Touch {
            id: 1,
            phase: TouchPhase::Cancel,
            ..
        }]
    ));
    physical.advance(input.physical(), 1. / 60.).unwrap();
    input
        .touch(Some(&second), 0, TouchPhase::Up, [10., 10.])
        .unwrap();
    input
        .touch(Some(&second), 1, TouchPhase::Up, [10., 10.])
        .unwrap();
    accept(&mut second, &mut input);
    assert!(second.actions().is_empty());
    assert!(input.physical().is_empty());
    // The old ids are reusable only for new complete gestures.
    input
        .touch(Some(&second), 0, TouchPhase::Down, [10., 10.])
        .unwrap();
    input
        .touch(Some(&second), 0, TouchPhase::Up, [10., 10.])
        .unwrap();
    accept(&mut second, &mut input);
    assert_eq!(second.actions(), [UiAction::Activated(id(2))]);
}
#[test]
fn noninteractive_hud_passes_ordered_touch_and_cancel_without_ui_events() {
    let ui = runtime(false);
    let mut input = NativeTouchInput::default();
    input.presented(Some(&ui)).unwrap();
    for (phase, position) in [
        (TouchPhase::Down, [10., 10.]),
        (TouchPhase::Move, [20., 30.]),
        (TouchPhase::Cancel, [25., 35.]),
    ] {
        input.touch(Some(&ui), 0, phase, position).unwrap();
    }
    assert_eq!(input.physical().len(), 3);
    assert!(input.receipts().is_empty());
    let mut runtime = incant_input::InputRuntime::default();
    runtime.advance(input.physical(), 1. / 60.).unwrap();
    input.accepted();
    assert_eq!(
        input.touch(Some(&ui), 0, TouchPhase::Move, [0., 0.]),
        Err(TouchError::Invalid)
    );
}
#[test]
fn capacity_and_invalid_packets_preserve_capture_for_retry() {
    let mut input = NativeTouchInput::default();
    assert_eq!(
        input.touch(None, 99, TouchPhase::Down, [0., 0.]),
        Err(TouchError::Invalid)
    );
    assert_eq!(
        input.touch(None, 0, TouchPhase::Down, [f32::NAN, 0.]),
        Err(TouchError::Invalid)
    );
    input.touch(None, 0, TouchPhase::Down, [0., 0.]).unwrap();
    for _ in 1..MAX_EVENTS_PER_TICK - 2 {
        input.touch(None, 0, TouchPhase::Move, [1., 1.]).unwrap();
    }
    assert_eq!(
        input.touch(None, 0, TouchPhase::Up, [2., 2.]),
        Err(TouchError::Capacity)
    );
    assert_eq!(input.physical().len(), MAX_EVENTS_PER_TICK - 2);
    input.accepted();
    input.touch(None, 0, TouchPhase::Up, [2., 2.]).unwrap();
    assert!(matches!(
        input.physical(),
        [InputEvent::Touch {
            phase: TouchPhase::Up,
            ..
        }]
    ));
    input.reset();
    assert_eq!(
        input.touch(None, 0, TouchPhase::Up, [2., 2.]),
        Err(TouchError::Invalid)
    );
}
#[test]
fn unpublished_ui_replacement_is_rejected_before_capture_mutation() {
    let ui = runtime(true);
    let mut input = NativeTouchInput::default();
    assert_eq!(
        input.touch(Some(&ui), 0, TouchPhase::Down, [10., 10.]),
        Err(TouchError::Invalid)
    );
    assert!(input.physical().is_empty() && input.receipts().is_empty());
    input.presented(Some(&ui)).unwrap();
    input
        .touch(Some(&ui), 0, TouchPhase::Down, [10., 10.])
        .unwrap();
    input
        .touch(Some(&ui), 0, TouchPhase::Cancel, [10., 10.])
        .unwrap();
    let mut ui = ui;
    accept(&mut ui, &mut input);
    assert!(ui.actions().is_empty());
}

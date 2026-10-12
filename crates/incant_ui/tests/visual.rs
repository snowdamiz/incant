use incant_text::editing::Selection;
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
fn widget(n: u128, kind: UiKind<StableId>, parent: Option<u128>) -> Widget {
    Widget {
        incarnation: 1,
        id: id(n),
        parent: parent.map(id),
        node: Arc::new(UiNode {
            kind,
            style: if parent.is_none() {
                UiStyle {
                    direction: UiDirection::Row,
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
    }
}
fn text(content: &str) -> UiText<StableId> {
    UiText {
        content: content.into(),
        fonts: vec![id(9)],
        font_size: 16.0,
        line_height: 20.0,
        wrap: incant_types::TextWrap::Word,
    }
}
fn widgets() -> Vec<Widget> {
    vec![
        widget(1, UiKind::Root, None),
        widget(2, UiKind::Button { text: text("Go") }, Some(1)),
        widget(3, UiKind::TextInput { text: text("ab") }, Some(1)),
    ]
}
fn runtime() -> UiRuntime {
    let mut ui = UiRuntime::new(widgets()).unwrap();
    let first = ui.prepare(&[], [400.0, 100.0], &mut Measure).unwrap();
    ui.commit(first).unwrap();
    ui
}
fn state(visual: &UiVisual<'_>) -> Vec<(StableId, bool, bool, Option<String>)> {
    (0..visual.len())
        .map(|i| {
            let node = visual.node(i).unwrap();
            (
                node.widget.id,
                node.focused,
                node.pressed,
                node.text.map(|t| t.display_text().to_owned()),
            )
        })
        .collect()
}

#[test]
fn candidate_visual_state_is_the_prepared_frame_not_the_accepted_one() {
    let mut ui = runtime();
    let accepted = ui.visual().unwrap();
    assert_eq!(accepted.len(), 3);
    assert!(accepted.node(3).is_none());
    assert!(state(&accepted).iter().all(|(_, f, p, _)| !f && !p));
    let pending = ui
        .prepare(&[UiEvent::Focus(id(3))], [400.0, 100.0], &mut Measure)
        .unwrap();
    let candidate = pending.visual(&ui).unwrap();
    assert!(std::ptr::eq(candidate.layout, &**pending.layout()));
    assert_eq!(
        state(&candidate)[2],
        (id(3), true, false, Some("ab".into()))
    );
    // The accepted owner is unchanged until commit.
    assert!(!state(&ui.visual().unwrap())[2].1);
    ui.commit(pending).unwrap();
    let target = ui.focus().unwrap();
    let pending = ui
        .prepare(
            &[
                UiEvent::Select {
                    target: target.clone(),
                    selection: Selection {
                        anchor: 0,
                        focus: 1,
                    },
                },
                UiEvent::Preedit {
                    target,
                    text: "中".into(),
                    cursor: Some(3..3),
                },
            ],
            [400.0, 100.0],
            &mut Measure,
        )
        .unwrap();
    let visual = pending.visual(&ui).unwrap();
    let field = visual.node(2).unwrap().text.unwrap();
    assert_eq!(field.display_text(), "中b");
    assert_eq!(field.preedit().unwrap().display_range, 0..3);
    assert_eq!(field.display_selection(), Some(Selection::caret(3)));
    assert_eq!(ui.text(id(3)).unwrap().display_text(), "ab");
}

#[test]
fn pointer_capture_is_visible_until_release_and_stale_candidates_refuse() {
    let mut ui = runtime();
    let target = ui.pointer_token().unwrap();
    let press = ui
        .prepare(
            &[UiEvent::Pointer {
                position: [5.0, 5.0],
                pressed: true,
                target: target.clone(),
            }],
            [400.0, 100.0],
            &mut Measure,
        )
        .unwrap();
    assert_eq!(
        state(&press.visual(&ui).unwrap())[1],
        (id(2), true, true, None)
    );
    ui.commit(press).unwrap();
    assert!(state(&ui.visual().unwrap())[1].2);
    let release = ui
        .prepare(
            &[UiEvent::Pointer {
                position: [5.0, 5.0],
                pressed: false,
                target,
            }],
            [400.0, 100.0],
            &mut Measure,
        )
        .unwrap();
    // A candidate prepared against an older accepted revision is refused.
    let stale = ui.prepare(&[], [400.0, 100.0], &mut Measure).unwrap();
    ui.commit(release).unwrap();
    assert!(matches!(stale.visual(&ui), Err(UiError::Stale)));
    assert_eq!(ui.actions(), [UiAction::Activated(id(2))]);
    assert_eq!(state(&ui.visual().unwrap())[1], (id(2), true, false, None));
}

#[test]
fn structural_replacement_exposes_its_own_widgets() {
    let ui = runtime();
    let pending = ui.prepare(&[], [400.0, 100.0], &mut Measure).unwrap();
    let mut next = widgets();
    next.push(widget(4, UiKind::Label { text: text("new") }, Some(1)));
    let pending = ui.reconcile(pending, next, &mut Measure).unwrap();
    let visual = pending.visual(&ui).unwrap();
    assert_eq!(visual.len(), 4);
    assert_eq!(visual.node(3).unwrap().widget.id, id(4));
    assert_eq!(ui.visual().unwrap().len(), 3);
}

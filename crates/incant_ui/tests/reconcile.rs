use incant_text::editing::Selection;
use incant_ui::*;
use std::sync::Arc;
fn id(n: u8) -> StableId {
    let mut b = [0; 16];
    b[15] = n;
    StableId(b)
}
fn nodes() -> Vec<Widget> {
    let text = UiText {
        content: "initial".into(),
        fonts: vec![id(9)],
        font_size: 10.0,
        line_height: 12.0,
        wrap: incant_types::TextWrap::Word,
    };
    vec![
        Widget {
            id: id(1),
            incarnation: 1,
            parent: None,
            node: Arc::new(UiNode {
                kind: UiKind::Root,
                style: Default::default(),
                order: 0,
                disabled: false,
            }),
        },
        Widget {
            id: id(2),
            incarnation: 2,
            parent: Some(id(1)),
            node: Arc::new(UiNode {
                kind: UiKind::Container,
                style: Default::default(),
                order: 0,
                disabled: false,
            }),
        },
        Widget {
            id: id(3),
            incarnation: 3,
            parent: Some(id(1)),
            node: Arc::new(UiNode {
                kind: UiKind::TextInput { text },
                style: Default::default(),
                order: 1,
                disabled: false,
            }),
        },
    ]
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
            known[0].unwrap_or(text.len() as f32 * 6.0),
            known[1].unwrap_or(12.0),
        ])
    }
}
fn ready() -> UiRuntime {
    let mut ui = UiRuntime::new(nodes()).unwrap();
    let p = ui.prepare(&[], [200.0; 2], &mut Measure).unwrap();
    ui.commit(p).unwrap();
    let p = ui
        .prepare(&[UiEvent::Focus(id(3))], [200.0; 2], &mut Measure)
        .unwrap();
    ui.commit(p).unwrap();
    ui
}
#[test]
fn same_tick_edits_survive_reparent_but_old_composition_and_pointer_receipts_expire() {
    let mut ui = ready();
    let token = ui.focus().unwrap();
    let pointer = ui.pointer_token().unwrap();
    let p = ui
        .prepare(
            &[
                UiEvent::Select {
                    target: token.clone(),
                    selection: Selection {
                        anchor: 0,
                        focus: 7,
                    },
                },
                UiEvent::Commit {
                    target: token.clone(),
                    text: "edited".into(),
                },
                UiEvent::Preedit {
                    target: token.clone(),
                    text: "draft".into(),
                    cursor: None,
                },
            ],
            [200.0; 2],
            &mut Measure,
        )
        .unwrap();
    let mut next = nodes();
    next[2].parent = Some(id(2));
    let p = ui.reconcile(p, next, &mut Measure).unwrap();
    assert_eq!(ui.text(id(3)).unwrap().text(), "initial");
    ui.commit(p).unwrap();
    assert_eq!(ui.text(id(3)).unwrap().text(), "edited");
    assert!(ui.text(id(3)).unwrap().preedit().is_none());
    assert_eq!(ui.focus().unwrap().widget(), id(3));
    let p = ui
        .prepare(
            &[
                UiEvent::Commit {
                    target: token,
                    text: "late".into(),
                },
                UiEvent::Pointer {
                    target: pointer,
                    position: [1.0; 2],
                    pressed: true,
                },
                UiEvent::WindowFocus(false),
            ],
            [200.0; 2],
            &mut Measure,
        )
        .unwrap();
    assert_eq!(p.discarded_events(), 2);
    ui.commit(p).unwrap();
    assert!(ui.focus().is_none());
    assert_eq!(ui.text(id(3)).unwrap().text(), "edited");
}
#[test]
fn replacement_and_explicit_content_change_reset_values_and_focus_safely() {
    let mut ui = ready();
    let token = ui.focus().unwrap();
    let p = ui
        .prepare(
            &[UiEvent::Commit {
                target: token.clone(),
                text: "+".into(),
            }],
            [200.0; 2],
            &mut Measure,
        )
        .unwrap();
    ui.commit(p).unwrap();
    let mut next = nodes();
    next[2].incarnation = 4;
    let p = ui.prepare(&[], [200.0; 2], &mut Measure).unwrap();
    let p = ui.reconcile(p, next, &mut Measure).unwrap();
    ui.commit(p).unwrap();
    assert!(ui.focus().is_none());
    assert_eq!(ui.text(id(3)).unwrap().text(), "initial");
    let p = ui
        .prepare(
            &[
                UiEvent::Focus(id(3)),
                UiEvent::Commit {
                    target: token,
                    text: "late".into(),
                },
            ],
            [200.0; 2],
            &mut Measure,
        )
        .unwrap();
    assert_eq!(p.discarded_events(), 1);
    ui.commit(p).unwrap();
    let mut next = ui.widgets().to_vec();
    let UiKind::TextInput { text } = &mut Arc::make_mut(&mut next[2].node).kind else {
        panic!()
    };
    text.content = "script reset".into();
    let p = ui.prepare(&[], [200.0; 2], &mut Measure).unwrap();
    let p = ui.reconcile(p, next, &mut Measure).unwrap();
    ui.commit(p).unwrap();
    assert_eq!(ui.text(id(3)).unwrap().text(), "script reset");
}
#[test]
fn rejected_final_tree_or_stale_candidate_cannot_publish_and_blur_revokes_pointer_delivery() {
    let mut ui = ready();
    let original = Arc::clone(ui.layout().unwrap());
    let target = ui.pointer_token().unwrap();
    let mut next = nodes();
    next.remove(0);
    let p = ui.prepare(&[], [200.0; 2], &mut Measure).unwrap();
    assert!(ui.reconcile(p, next, &mut Measure).is_err());
    assert!(Arc::ptr_eq(&original, ui.layout().unwrap()));
    let p = ui
        .prepare(
            &[
                UiEvent::WindowFocus(false),
                UiEvent::WindowFocus(true),
                UiEvent::Pointer {
                    target,
                    position: [1.0; 2],
                    pressed: true,
                },
            ],
            [200.0; 2],
            &mut Measure,
        )
        .unwrap();
    assert_eq!(p.discarded_events(), 1);
    ui.commit(p).unwrap();
    assert!(ui.focus().is_none());
    let old = ui.prepare(&[], [200.0; 2], &mut Measure).unwrap();
    let p = ui.prepare(&[], [200.0; 2], &mut Measure).unwrap();
    ui.commit(p).unwrap();
    assert!(matches!(
        ui.reconcile(old, nodes(), &mut Measure),
        Err(UiError::Stale)
    ));
}

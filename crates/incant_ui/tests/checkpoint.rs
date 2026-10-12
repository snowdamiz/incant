use incant_text::editing::Selection;
use incant_ui::*;
use sha2::{Digest, Sha256};
use std::sync::Arc;

fn id(value: u128) -> StableId {
    StableId(value.to_be_bytes())
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
            known[0].unwrap_or((text.chars().count() as f32 * 8.).min(MAX_UI_EXTENT)),
            known[1].unwrap_or(20.),
        ])
    }
}
fn widgets() -> Vec<Widget> {
    let text = UiText {
        content: "A👩‍💻界".into(),
        fonts: vec![id(99)],
        font_size: 16.,
        line_height: 20.,
        wrap: incant_types::TextWrap::Word,
    };
    [
        UiKind::Root,
        UiKind::Label { text: text.clone() },
        UiKind::TextInput { text: text.clone() },
        UiKind::Button { text: text.clone() },
        UiKind::TextInput { text },
    ]
    .into_iter()
    .enumerate()
    .map(|(i, kind)| Widget {
        id: id(i as u128 + 1),
        incarnation: 1,
        parent: (i != 0).then(|| id(1)),
        node: Arc::new(UiNode {
            kind,
            order: 0,
            disabled: false,
            style: if i == 0 {
                UiStyle::default()
            } else {
                UiStyle {
                    size: [UiLength::Pixels(180.), UiLength::Pixels(30.)],
                    ..Default::default()
                }
            },
        }),
    })
    .collect()
}
fn runtime(widgets: Vec<Widget>) -> UiRuntime {
    let mut ui = UiRuntime::new(widgets).unwrap();
    let next = ui.prepare(&[], [400., 400.], &mut Measure).unwrap();
    ui.commit(next).unwrap();
    ui
}
fn accept(ui: &mut UiRuntime, events: &[UiEvent]) {
    let next = ui.prepare(events, [400., 400.], &mut Measure).unwrap();
    ui.commit(next).unwrap();
}
fn restored(bytes: &[u8]) -> Result<UiRuntime, UiCheckpointError> {
    UiRuntime::from_checkpoint_bytes(widgets(), bytes, [400., 400.], &mut Measure)
}
fn focus(ui: &mut UiRuntime, widget: u128) -> FocusToken {
    accept(ui, &[UiEvent::Focus(id(widget))]);
    ui.focus().unwrap()
}
fn edit(ui: &mut UiRuntime) {
    let target = focus(ui, 3);
    accept(
        ui,
        &[
            UiEvent::Select {
                target: target.clone(),
                selection: Selection {
                    anchor: 1,
                    focus: 12,
                },
            },
            UiEvent::Commit {
                target: target.clone(),
                text: "e\u{301}日本".into(),
            },
            UiEvent::Select {
                target,
                selection: Selection {
                    anchor: 10,
                    focus: 1,
                },
            },
        ],
    );
}
fn seal(bytes: &mut [u8]) {
    let mut hash = Sha256::new();
    hash.update(&bytes[..48]);
    hash.update(&bytes[80..]);
    bytes[48..80].copy_from_slice(&hash.finalize());
}
fn row(bytes: &[u8], index: usize) -> usize {
    let mut at = 80;
    for _ in 0..index {
        at += 48 + u32::from_le_bytes(bytes[at + 44..at + 48].try_into().unwrap()) as usize;
    }
    at
}

#[test]
fn round_trip_preserves_committed_multilingual_text_selection_and_focus() {
    let mut ui = runtime(widgets());
    edit(&mut ui);
    let bytes = ui.checkpoint_bytes().unwrap();
    let loaded = restored(&bytes).unwrap();
    assert_eq!(loaded.text(id(3)).unwrap().text(), "Ae\u{301}日本界");
    assert_eq!(
        loaded.text(id(3)).unwrap().selection(),
        Selection {
            anchor: 10,
            focus: 1
        }
    );
    assert!(loaded.text(id(3)).unwrap().focused());
    assert!(!loaded.text(id(5)).unwrap().focused());
    assert_eq!(loaded.focus().unwrap().widget(), id(3));
    assert!(loaded.actions().is_empty());
    assert_eq!(loaded.discarded_events(), 0);
    assert_eq!(loaded.checkpoint_bytes().unwrap(), bytes);
}

#[test]
fn encoding_is_canonical_across_storage_order_native_incarnation_and_viewport() {
    let mut ui = runtime(widgets());
    edit(&mut ui);
    let bytes = ui.checkpoint_bytes().unwrap();
    let mut reordered = widgets();
    reordered.reverse();
    for widget in &mut reordered {
        widget.incarnation = 987;
    }
    let loaded =
        UiRuntime::from_checkpoint_bytes(reordered, &bytes, [900., 700.], &mut Measure).unwrap();
    assert_eq!(loaded.checkpoint_bytes().unwrap(), bytes);
    assert_eq!(loaded.layout().unwrap().viewport, [900., 700.]);
}

#[test]
fn old_text_pointer_and_prepared_layout_receipts_cannot_address_loaded_owner() {
    let mut ui = runtime(widgets());
    edit(&mut ui);
    let target = ui.focus().unwrap();
    let pointer = ui.pointer_token().unwrap();
    let old_pending = ui.prepare(&[], [400., 400.], &mut Measure).unwrap();
    let bytes = ui.checkpoint_bytes().unwrap();
    let mut loaded = restored(&bytes).unwrap();
    assert!(matches!(loaded.commit(old_pending), Err(UiError::Stale)));
    accept(
        &mut loaded,
        &[
            UiEvent::Commit {
                target: target.clone(),
                text: "obsolete".into(),
            },
            UiEvent::Preedit {
                target,
                text: "obsolete".into(),
                cursor: None,
            },
            UiEvent::Pointer {
                target: pointer,
                position: [10., 65.],
                pressed: true,
            },
        ],
    );
    assert_eq!(loaded.discarded_events(), 3);
    assert_eq!(loaded.checkpoint_bytes().unwrap(), bytes);
    let new_target = loaded.focus().unwrap();
    accept(
        &mut loaded,
        &[UiEvent::Commit {
            target: new_target,
            text: "new".into(),
        }],
    );
    assert_eq!(loaded.text(id(3)).unwrap().text(), "Anew界");
}

#[test]
fn active_composition_is_refused_without_implicit_commit_or_cancel() {
    let mut ui = runtime(widgets());
    let target = focus(&mut ui, 3);
    accept(
        &mut ui,
        &[UiEvent::Preedit {
            target: target.clone(),
            text: "日本".into(),
            cursor: Some(3..6),
        }],
    );
    let display = ui.text(id(3)).unwrap().display_text().to_owned();
    let frame = Arc::clone(ui.layout().unwrap());
    assert!(matches!(
        ui.checkpoint_bytes(),
        Err(UiCheckpointError::CompositionActive)
    ));
    assert_eq!(ui.text(id(3)).unwrap().display_text(), display);
    assert!(ui.text(id(3)).unwrap().preedit().is_some());
    assert!(Arc::ptr_eq(&frame, ui.layout().unwrap()));
    accept(&mut ui, &[UiEvent::Cancel { target }]);
    assert!(restored(&ui.checkpoint_bytes().unwrap()).is_ok());
}

#[test]
fn unaccepted_candidate_does_not_leak_into_snapshot() {
    let mut ui = runtime(widgets());
    let target = focus(&mut ui, 3);
    let before = ui.checkpoint_bytes().unwrap();
    let pending = ui
        .prepare(
            &[UiEvent::Commit {
                target,
                text: "pending".into(),
            }],
            [400., 400.],
            &mut Measure,
        )
        .unwrap();
    assert_eq!(ui.checkpoint_bytes().unwrap(), before);
    drop(pending);
    assert_eq!(
        restored(&before).unwrap().checkpoint_bytes().unwrap(),
        before
    );
}

#[test]
fn notifications_pointer_capture_and_physical_held_state_are_not_replayed() {
    let mut ui = runtime(widgets());
    let target = ui.pointer_token().unwrap();
    // Button occupies y=60..90; no activation until its matching release.
    accept(
        &mut ui,
        &[UiEvent::Pointer {
            target,
            position: [10., 65.],
            pressed: true,
        }],
    );
    assert_eq!(ui.focus().unwrap().widget(), id(4));
    let bytes = ui.checkpoint_bytes().unwrap();
    let mut loaded = restored(&bytes).unwrap();
    let target = loaded.pointer_token().unwrap();
    accept(
        &mut loaded,
        &[UiEvent::Pointer {
            target,
            position: [10., 65.],
            pressed: false,
        }],
    );
    assert!(loaded.actions().is_empty());
    accept(&mut loaded, &[UiEvent::Activate]);
    assert_eq!(loaded.actions(), [UiAction::Activated(id(4))]);
    let mut loaded = restored(&loaded.checkpoint_bytes().unwrap()).unwrap();
    assert!(loaded.actions().is_empty());
    let mut router = UiRouter::default();
    let routed = router
        .prepare(
            &loaded,
            &[incant_input::InputEvent::Key {
                code: incant_input::KeyCode::Enter,
                down: false,
            }],
            &[],
            1. / 60.,
            [400., 400.],
            &mut Measure,
        )
        .unwrap();
    router.commit(&mut loaded, routed).unwrap();
    assert!(loaded.actions().is_empty());
}

#[test]
fn changed_membership_parent_or_kind_is_refused() {
    let bytes = runtime(widgets()).checkpoint_bytes().unwrap();
    for change in 0..3 {
        let mut w = widgets();
        match change {
            0 => {
                w.pop();
            }
            1 => {
                w[2].id = id(50);
            }
            _ => {
                let text = w[2].node.kind.text().unwrap().clone();
                Arc::make_mut(&mut w[2].node).kind = UiKind::Label { text };
            }
        }
        assert!(UiRuntime::from_checkpoint_bytes(w, &bytes, [400., 400.], &mut Measure).is_err());
    }
    let mut bytes = bytes;
    let at = row(&bytes, 2);
    bytes[at + 16..at + 32].copy_from_slice(&id(4).0);
    seal(&mut bytes);
    assert!(matches!(
        restored(&bytes),
        Err(UiCheckpointError::Invalid("widget identity or kind"))
    ));
}

#[test]
fn saved_focus_must_still_be_present_enabled_and_visible() {
    let mut ui = runtime(widgets());
    focus(&mut ui, 3);
    let bytes = ui.checkpoint_bytes().unwrap();
    for n in 0..3 {
        let mut w = widgets();
        match n {
            0 => Arc::make_mut(&mut w[0].node).disabled = true,
            1 => Arc::make_mut(&mut w[2].node).disabled = true,
            _ => Arc::make_mut(&mut w[2].node).style.size = [UiLength::Pixels(0.); 2],
        }
        assert!(UiRuntime::from_checkpoint_bytes(w, &bytes, [400., 400.], &mut Measure).is_err());
    }
    for target in [id(1), id(99)] {
        let mut bad = bytes.clone();
        bad[24..40].copy_from_slice(&target.0);
        seal(&mut bad);
        assert!(restored(&bad).is_err());
    }
}

#[test]
fn every_truncation_and_single_byte_corruption_is_refused() {
    let bytes = runtime(widgets()).checkpoint_bytes().unwrap();
    for end in 0..bytes.len() {
        assert!(restored(&bytes[..end]).is_err(), "end={end}");
    }
    for at in 0..bytes.len() {
        let mut bad = bytes.clone();
        bad[at] ^= 1;
        assert!(restored(&bad).is_err(), "at={at}");
    }
    let mut too_large = vec![0; MAX_UI_CHECKPOINT_BYTES + 1];
    too_large[..8].copy_from_slice(b"INCUI001");
    assert!(restored(&too_large).is_err());
}

#[test]
fn checksummed_malformed_fields_are_refused_before_publication() {
    let bytes = runtime(widgets()).checkpoint_bytes().unwrap();
    let input = row(&bytes, 2);
    let button = row(&bytes, 3);
    for (at, value) in [
        (8, 2),
        (12, 2),
        (16, 255),
        (20, 2),
        (24, 1),
        (40, 0),
        (44, 1),
        (80 + 16, 1),
        (80 + 33, 2),
        (80 + 34, 1),
        (input + 32, 9),
        (input + 44, 255),
        (input + 48, 255),
        (button + 36, 1),
    ] {
        let mut bad = bytes.clone();
        bad[at] = value;
        seal(&mut bad);
        assert!(restored(&bad).is_err(), "field={at}");
    }
    let mut duplicate = bytes.clone();
    duplicate[128..144].copy_from_slice(&id(1).0);
    seal(&mut duplicate);
    assert!(restored(&duplicate).is_err());
    let mut trailing = bytes.clone();
    trailing.push(0);
    let len = (trailing.len() - 80) as u32;
    trailing[40..44].copy_from_slice(&len.to_le_bytes());
    seal(&mut trailing);
    assert!(restored(&trailing).is_err());
}

#[test]
fn invalid_grapheme_selection_is_refused_even_at_utf8_scalar_boundary() {
    let bytes = runtime(widgets()).checkpoint_bytes().unwrap();
    let at = row(&bytes, 2);
    // byte 5 is a scalar boundary inside the woman-technologist grapheme.
    for selection in [2u32, 5, 999, u32::MAX] {
        let mut bad = bytes.clone();
        bad[at + 36..at + 40].copy_from_slice(&selection.to_le_bytes());
        seal(&mut bad);
        assert!(restored(&bad).is_err());
    }
}

#[test]
fn failed_layout_restore_preserves_existing_owner_and_its_receipts() {
    struct Fails;
    impl TextMeasure for Fails {
        fn measure(
            &mut self,
            _: &Widget,
            _: &str,
            _: [Option<f32>; 2],
            _: [MeasureSpace; 2],
        ) -> Result<[f32; 2], UiError> {
            Err(UiError::Layout)
        }
    }
    let mut ui = runtime(widgets());
    let target = focus(&mut ui, 3);
    let bytes = ui.checkpoint_bytes().unwrap();
    let mut w = widgets();
    Arc::make_mut(&mut w[2].node).style.size = [UiLength::Auto; 2];
    assert!(UiRuntime::from_checkpoint_bytes(w, &bytes, [400., 400.], &mut Fails).is_err());
    assert_eq!(ui.checkpoint_bytes().unwrap(), bytes);
    accept(
        &mut ui,
        &[UiEvent::Commit {
            target,
            text: "still valid".into(),
        }],
    );
    assert!(ui.text(id(3)).unwrap().text().ends_with("still valid"));
}

#[test]
fn combined_restored_edit_and_authored_label_text_is_bounded() {
    let seed = widgets();
    let mut w = vec![seed[0].clone(), seed[1].clone()];
    Arc::make_mut(&mut w[1].node).kind = UiKind::Label {
        text: UiText {
            content: String::new(),
            ..seed[1].node.kind.text().unwrap().clone()
        },
    };
    for i in 0..64 {
        let mut widget = seed[2].clone();
        widget.id = id(10 + i);
        Arc::make_mut(&mut widget.node).kind = UiKind::TextInput {
            text: UiText {
                content: "x".repeat(incant_text::MAX_TEXT_BYTES),
                ..seed[2].node.kind.text().unwrap().clone()
            },
        };
        w.push(widget);
    }
    let ui = runtime(w.clone());
    let bytes = ui.checkpoint_bytes().unwrap();
    assert!(bytes.len() <= MAX_UI_CHECKPOINT_BYTES);
    for widget in &mut w[2..] {
        if let UiKind::TextInput { text } = &mut Arc::make_mut(&mut widget.node).kind {
            text.content.clear();
        }
    }
    assert!(
        UiRuntime::from_checkpoint_bytes(w.clone(), &bytes, [400., 400.], &mut Measure).is_ok()
    );
    if let UiKind::Label { text } = &mut Arc::make_mut(&mut w[1].node).kind {
        text.content.push('x');
    }
    assert!(matches!(
        UiRuntime::from_checkpoint_bytes(w, &bytes, [400., 400.], &mut Measure),
        Err(UiCheckpointError::Invalid("combined widget text capacity"))
    ));
}

#[test]
fn empty_owner_and_button_focus_round_trip_without_inventing_text() {
    let empty = runtime(Vec::new());
    let bytes = empty.checkpoint_bytes().unwrap();
    let loaded =
        UiRuntime::from_checkpoint_bytes(Vec::new(), &bytes, [1., 1.], &mut Measure).unwrap();
    assert!(loaded.widgets().is_empty());
    assert!(loaded.focus().is_none());
    let mut ui = runtime(widgets());
    focus(&mut ui, 4);
    let loaded = restored(&ui.checkpoint_bytes().unwrap()).unwrap();
    assert_eq!(loaded.focus().unwrap().widget(), id(4));
    assert!(loaded.text(id(4)).is_none());
    assert!(loaded.actions().is_empty());
    assert!(matches!(
        UiRuntime::new(widgets()).unwrap().checkpoint_bytes(),
        Err(UiCheckpointError::Invalid("no committed layout"))
    ));
}

#[cfg(feature = "native")]
#[path = "checkpoint/native.rs"]
mod native;

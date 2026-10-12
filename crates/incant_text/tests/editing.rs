use incant_text::{
    FontSource, MAX_TEXT_BYTES, TextEngine, TextStyle, Wrap,
    editing::{Movement, Selection, TextBuffer},
};
use std::{ops::Range, sync::Arc};
use ulid::Ulid;

fn editing(text: &str) -> TextBuffer {
    let mut edit = TextBuffer::new(text).unwrap();
    edit.set_focused(true);
    edit
}

#[derive(Debug, PartialEq, Eq)]
struct Snapshot {
    text: String,
    display: String,
    selection: Selection,
    display_selection: Option<Selection>,
    focused: bool,
    composition: Option<PreeditSnapshot>,
}

#[derive(Debug, PartialEq, Eq)]
struct PreeditSnapshot {
    text: String,
    cursor: Option<Range<usize>>,
    replacement: Range<usize>,
    display_range: Range<usize>,
}

fn snapshot(edit: &TextBuffer) -> Snapshot {
    Snapshot {
        text: edit.text().into(),
        display: edit.display_text().into(),
        selection: edit.selection(),
        display_selection: edit.display_selection(),
        focused: edit.focused(),
        composition: edit.preedit().map(|p| PreeditSnapshot {
            text: p.text.into(),
            cursor: p.cursor,
            replacement: p.replacement,
            display_range: p.display_range,
        }),
    }
}

#[test]
fn each_backspace_and_forward_delete_removes_one_extended_grapheme() {
    let clusters = ["a", "e\u{301}", "👩🏽‍💻", "🇺🇦", "\r\n", "日"];
    let text = clusters.concat();
    let mut edit = editing(&text);
    for remaining in (0..clusters.len()).rev() {
        edit.backspace().unwrap();
        assert_eq!(edit.text(), clusters[..remaining].concat());
        assert_eq!(edit.selection(), Selection::caret(edit.text().len()));
    }
    edit.backspace().unwrap();
    assert_eq!(edit.text(), "");
    edit.set_text(&text).unwrap();
    edit.select(Selection::caret(0)).unwrap();
    for removed in 1..=clusters.len() {
        edit.delete_forward().unwrap();
        assert_eq!(edit.text(), clusters[removed..].concat());
        assert_eq!(edit.selection(), Selection::caret(0));
    }
    edit.delete_forward().unwrap();
    assert_eq!(edit.text(), "");
}

#[test]
fn selection_is_directional_and_navigation_is_logical_not_visual() {
    let mut edit = editing("aאבz");
    edit.move_selection(Movement::PreviousGrapheme, true)
        .unwrap();
    assert_eq!(
        edit.selection(),
        Selection {
            anchor: 6,
            focus: 5
        }
    );
    edit.move_selection(Movement::PreviousGrapheme, true)
        .unwrap();
    assert_eq!(
        edit.selection(),
        Selection {
            anchor: 6,
            focus: 3
        }
    );
    edit.move_selection(Movement::NextGrapheme, false).unwrap();
    assert_eq!(edit.selection(), Selection::caret(6));
    edit.move_selection(Movement::Start, false).unwrap();
    edit.move_selection(Movement::NextGrapheme, true).unwrap();
    edit.move_selection(Movement::NextGrapheme, true).unwrap();
    assert_eq!(
        edit.selection(),
        Selection {
            anchor: 0,
            focus: 3
        }
    );
    edit.move_selection(Movement::PreviousGrapheme, false)
        .unwrap();
    assert_eq!(edit.selection(), Selection::caret(0));
    edit.move_selection(Movement::End, true).unwrap();
    edit.commit("日本").unwrap();
    assert_eq!(edit.text(), "日本");
    assert_eq!(edit.selection(), Selection::caret(6));
}

#[test]
fn invalid_grapheme_selection_preserves_an_active_composition() {
    let mut edit = editing("e\u{301}日");
    edit.set_preedit("本", Some(0..3)).unwrap();
    let before = snapshot(&edit);
    for byte in [1, 2, 4, 5, 7, usize::MAX] {
        for selection in [
            Selection::caret(byte),
            Selection {
                anchor: byte,
                focus: 0,
            },
            Selection {
                anchor: 0,
                focus: byte,
            },
        ] {
            assert!(edit.select(selection).is_err());
            assert_eq!(snapshot(&edit), before);
        }
    }
}

#[test]
fn repeated_preedits_replace_the_original_selection_then_commit_once() {
    let mut edit = editing("AoldZ");
    let selected = Selection {
        anchor: 4,
        focus: 1,
    };
    edit.select(selected).unwrap();
    for (text, cursor) in [("に", Some(3..3)), ("日本", Some(0..6)), ("日本語", None)] {
        edit.set_preedit(text, cursor.clone()).unwrap();
        assert_eq!(edit.text(), "AoldZ");
        assert_eq!(edit.display_text(), format!("A{text}Z"));
        assert_eq!(edit.selection(), selected);
        let preedit = edit.preedit().unwrap();
        assert_eq!(preedit.replacement, 1..4);
        assert_eq!(preedit.display_range, 1..1 + text.len());
        assert_eq!(preedit.cursor, cursor);
        assert_eq!(
            edit.display_selection(),
            cursor.map(|c| Selection {
                anchor: 1 + c.start,
                focus: 1 + c.end,
            })
        );
    }
    edit.commit("日本").unwrap();
    assert_eq!(edit.text(), "A日本Z");
    assert_eq!(edit.display_text(), edit.text());
    assert_eq!(edit.selection(), Selection::caret(7));
    assert!(edit.preedit().is_none());
}

#[test]
fn preedit_clear_commit_and_cancel_have_distinct_semantics() {
    let mut edit = editing("old");
    edit.select(Selection {
        anchor: 0,
        focus: 3,
    })
    .unwrap();
    edit.set_preedit("日本", None).unwrap();
    edit.set_preedit("", None).unwrap();
    assert_eq!(edit.text(), "old");
    assert_eq!(edit.selection().range(), 0..3);
    // Desktop adapters can deliver empty Preedit immediately before Commit.
    edit.commit("日").unwrap();
    assert_eq!(edit.text(), "日");
    edit.select(Selection {
        anchor: 0,
        focus: 3,
    })
    .unwrap();
    edit.set_preedit("本", None).unwrap();
    edit.cancel_preedit();
    assert_eq!(edit.text(), "日");
    assert_eq!(edit.selection().range(), 0..3);
    edit.commit("").unwrap();
    assert_eq!(edit.text(), "");
}

#[test]
fn focus_loss_cancels_and_refuses_late_events_without_committing() {
    let mut edit = editing("kept");
    edit.select(Selection {
        anchor: 0,
        focus: 4,
    })
    .unwrap();
    edit.set_preedit("draft", Some(5..5)).unwrap();
    edit.set_focused(false);
    let before = snapshot(&edit);
    assert!(edit.preedit().is_none());
    assert_eq!(edit.display_text(), "kept");
    assert!(edit.commit("late").is_err());
    assert!(edit.set_preedit("late", None).is_err());
    assert!(edit.backspace().is_err());
    assert!(edit.delete_forward().is_err());
    assert!(edit.move_selection(Movement::Start, false).is_err());
    assert_eq!(snapshot(&edit), before);
    edit.set_focused(true);
    edit.commit("new").unwrap();
    assert_eq!(edit.text(), "new");
}

#[test]
fn ordinary_navigation_and_deletion_cannot_consume_ime_owned_keys() {
    let mut edit = editing("old");
    edit.set_preedit("日本", Some(0..6)).unwrap();
    let before = snapshot(&edit);
    assert!(edit.backspace().is_err());
    assert!(edit.delete_forward().is_err());
    for movement in [
        Movement::Start,
        Movement::End,
        Movement::NextGrapheme,
        Movement::PreviousGrapheme,
    ] {
        assert!(edit.move_selection(movement, false).is_err());
        assert!(edit.move_selection(movement, true).is_err());
    }
    assert_eq!(snapshot(&edit), before);
    edit.select(Selection::caret(0)).unwrap();
    assert!(edit.preedit().is_none());
    edit.delete_forward().unwrap();
    assert_eq!(edit.text(), "ld");
}

#[test]
fn preedit_cursor_uses_scalar_boundaries_and_refuses_invalid_ranges_atomically() {
    let mut edit = editing("x");
    edit.set_preedit("e\u{301}", Some(1..3)).unwrap();
    // An IME may select only the combining scalar within one grapheme.
    assert_eq!(
        edit.display_selection(),
        Some(Selection {
            anchor: 2,
            focus: 4
        })
    );
    let before = snapshot(&edit);
    for cursor in [
        2..2,
        Range { start: 3, end: 1 },
        0..4,
        usize::MAX..usize::MAX,
    ] {
        assert!(edit.set_preedit("e\u{301}", Some(cursor)).is_err());
        assert_eq!(snapshot(&edit), before);
    }
    assert!(edit.set_preedit("", Some(0..1)).is_err());
    assert_eq!(snapshot(&edit), before);
}

#[test]
fn insertion_and_deletion_that_join_graphemes_keep_a_valid_caret() {
    let mut edit = editing("👩💻");
    edit.select(Selection::caret("👩".len())).unwrap();
    edit.commit("\u{200d}").unwrap();
    assert_eq!(edit.text(), "👩‍💻");
    assert_eq!(edit.selection(), Selection::caret(edit.text().len()));
    edit.backspace().unwrap();
    assert_eq!(edit.text(), "");
    edit.set_text("🇺x🇦").unwrap();
    edit.select(Selection {
        anchor: 4,
        focus: 5,
    })
    .unwrap();
    edit.delete_forward().unwrap();
    assert_eq!(edit.text(), "🇺🇦");
    assert_eq!(edit.selection(), Selection::caret(8));
    edit.backspace().unwrap();
    assert_eq!(edit.text(), "");
}

#[test]
fn byte_limits_admit_replacement_but_refuse_overflow_without_losing_preedit() {
    let full = "x".repeat(MAX_TEXT_BYTES);
    assert!(TextBuffer::new(&(full.clone() + "x")).is_err());
    let mut edit = editing(&full);
    edit.select(Selection {
        anchor: 0,
        focus: 3,
    })
    .unwrap();
    edit.set_preedit("日", Some(3..3)).unwrap();
    let before = snapshot(&edit);
    assert_eq!(edit.display_text().len(), MAX_TEXT_BYTES);
    assert!(edit.set_preedit("日本", None).is_err());
    assert!(edit.commit("日本").is_err());
    assert!(edit.set_text(&(full.clone() + "x")).is_err());
    assert_eq!(snapshot(&edit), before);
    edit.commit("日").unwrap();
    assert_eq!(edit.text().len(), MAX_TEXT_BYTES);
    edit.select(Selection {
        anchor: 0,
        focus: MAX_TEXT_BYTES,
    })
    .unwrap();
    edit.commit("short").unwrap();
    assert_eq!(edit.text(), "short");
}

#[test]
fn replacement_preserves_exact_unicode_and_line_endings() {
    let mut edit = TextBuffer::new("").unwrap();
    assert!(!edit.focused());
    edit.set_text("e\u{301}\r\n日\n").unwrap();
    assert_eq!(edit.text(), "e\u{301}\r\n日\n");
    assert_eq!(edit.selection(), Selection::caret(edit.text().len()));
    edit.select(Selection::caret(0)).unwrap();
    edit.set_focused(true);
    edit.set_preedit("temporary", None).unwrap();
    edit.set_text("é").unwrap();
    assert!(edit.preedit().is_none());
    assert_eq!(edit.text(), "é");
    assert_eq!(edit.selection(), Selection::caret(2));
}

#[test]
fn composed_display_feeds_the_real_shaper_without_changing_committed_source() {
    let mut engine = TextEngine::new(vec![
        FontSource {
            id: Ulid::from(1_u128),
            bytes: include_bytes!("fonts/NotoSans-Regular.ttf").to_vec(),
        },
        FontSource {
            id: Ulid::from(2_u128),
            bytes: include_bytes!("fonts/IncantCJKFixture.ttf").to_vec(),
        },
    ])
    .unwrap();
    let style = TextStyle {
        font_size: 20.,
        line_height: 30.,
        width: 300.,
        wrap: Wrap::Word,
    };
    let mut edit = editing("AoldZ");
    edit.select(Selection {
        anchor: 1,
        focus: 4,
    })
    .unwrap();
    edit.set_preedit("日本", Some(3..6)).unwrap();
    let shaped = engine.shape(edit.display_text(), style).unwrap();
    assert_eq!(shaped.source(), "A日本Z");
    assert!(shaped.missing_clusters().is_empty());
    assert_eq!(
        shaped
            .glyphs()
            .iter()
            .filter(|g| g.font == Ulid::from(2_u128))
            .count(),
        2
    );
    assert!(shaped.glyphs().iter().any(|g| g.source == (1..4)));
    assert!(shaped.glyphs().iter().any(|g| g.source == (4..7)));
    let display_ptr = edit.display_text().as_ptr();
    for _ in 0..64 {
        edit.set_preedit("日本", Some(3..6)).unwrap();
        assert_eq!(edit.display_text().as_ptr(), display_ptr);
        assert!(Arc::ptr_eq(
            &shaped,
            &engine.shape(edit.display_text(), style).unwrap()
        ));
    }
    assert_eq!(edit.text(), "AoldZ");
    edit.commit("日本").unwrap();
    assert!(Arc::ptr_eq(
        &shaped,
        &engine.shape(edit.display_text(), style).unwrap()
    ));
    assert_eq!(edit.text(), "A日本Z");
}

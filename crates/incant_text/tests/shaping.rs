use incant_text::{FontSource, MAX_FONTS, MAX_TEXT_BYTES, TextEngine, TextError, TextStyle, Wrap};
use std::sync::Arc;
use ulid::Ulid;

const LATIN: &[u8] = include_bytes!("fonts/NotoSans-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("fonts/NotoSansArabic.ttf");
const HEBREW: &[u8] = include_bytes!("fonts/NotoSansHebrew.ttf");

fn face(id: u128, bytes: &[u8]) -> FontSource {
    FontSource {
        id: Ulid::from(id),
        bytes: bytes.to_vec(),
    }
}
fn engine() -> TextEngine {
    TextEngine::new(vec![face(1, LATIN), face(2, ARABIC), face(3, HEBREW)]).unwrap()
}
fn style() -> TextStyle {
    TextStyle {
        font_size: 20.0,
        line_height: 30.0,
        width: 800.0,
        wrap: Wrap::WordOrGlyph,
    }
}

fn table(bytes: &[u8], tag: &[u8; 4]) -> std::ops::Range<usize> {
    let count = u16::from_be_bytes(bytes[4..6].try_into().unwrap()) as usize;
    let record = bytes[12..12 + 16 * count]
        .as_chunks::<16>()
        .0
        .iter()
        .find(|record| &record[..4] == tag)
        .unwrap();
    let start = u32::from_be_bytes(record[8..12].try_into().unwrap()) as usize;
    let length = u32::from_be_bytes(record[12..16].try_into().unwrap()) as usize;
    start..start + length
}

#[test]
fn metadata_without_a_loadable_shaping_face_is_refused_at_construction() {
    // fontdb accepts this named sfnt, but rustybuzz requires head/hhea/maxp.
    // Build the malformed input in memory; no fixture bytes are modified.
    let names = &LATIN[table(LATIN, b"name")];
    let mut metadata_only = vec![0; 28];
    metadata_only[..4].copy_from_slice(b"\0\x01\0\0");
    metadata_only[4..6].copy_from_slice(&1_u16.to_be_bytes());
    metadata_only[12..16].copy_from_slice(b"name");
    metadata_only[20..24].copy_from_slice(&28_u32.to_be_bytes());
    metadata_only[24..28].copy_from_slice(&(names.len() as u32).to_be_bytes());
    metadata_only.extend_from_slice(names);
    for sources in [
        vec![face(8, &metadata_only)],
        vec![face(1, LATIN), face(8, &metadata_only)],
    ] {
        assert!(
            matches!(TextEngine::new(sources), Err(TextError::Font(id)) if id == Ulid::from(8))
        );
    }
}

#[test]
fn concrete_asset_order_is_independent_of_style_weight_and_stretch_metadata() {
    for (offset, value) in [(62, 1_u16), (4, 700), (6, 3)] {
        let mut styled = LATIN.to_vec();
        let start = table(&styled, b"OS/2").start + offset;
        styled[start..start + 2].copy_from_slice(&value.to_be_bytes());
        for sources in [
            vec![face(8, &styled)],
            vec![face(8, &styled), face(1, LATIN)],
        ] {
            let shaped = TextEngine::new(sources)
                .unwrap()
                .shape("Asset", style())
                .unwrap();
            assert!(shaped.missing_clusters().is_empty());
            assert!(
                shaped
                    .glyphs()
                    .iter()
                    .all(|glyph| glyph.font == Ulid::from(8))
            );
        }
    }
    let mut bold_hebrew = HEBREW.to_vec();
    let weight = table(&bold_hebrew, b"OS/2").start + 4;
    bold_hebrew[weight..weight + 2].copy_from_slice(&700_u16.to_be_bytes());
    let shaped = TextEngine::new(vec![face(1, LATIN), face(8, &bold_hebrew), face(3, HEBREW)])
        .unwrap()
        .shape("שלום", style())
        .unwrap();
    assert!(shaped.missing_clusters().is_empty());
    assert!(
        shaped
            .glyphs()
            .iter()
            .all(|glyph| glyph.font == Ulid::from(8))
    );
}

#[test]
fn shaping_does_not_require_or_decode_font_family_names() {
    let mut nameless = LATIN.to_vec();
    let count = u16::from_be_bytes(nameless[4..6].try_into().unwrap()) as usize;
    let record = nameless[12..12 + 16 * count]
        .as_chunks_mut::<16>()
        .0
        .iter_mut()
        .find(|record| &record[..4] == b"name")
        .unwrap();
    record[..4].copy_from_slice(b"NONE");
    let original = TextEngine::new(vec![face(1, LATIN)])
        .unwrap()
        .shape("Named ffi", style())
        .unwrap();
    let without_names = TextEngine::new(vec![face(1, &nameless)])
        .unwrap()
        .shape("Named ffi", style())
        .unwrap();
    assert_eq!(original.glyphs(), without_names.glyphs());
    assert_eq!(original.lines(), without_names.lines());
}

#[test]
fn latin_ligatures_and_combining_marks_preserve_utf8_clusters() {
    let mut engine = engine();
    let combining = engine.shape("e\u{301}", style()).unwrap();
    assert!(combining.missing_clusters().is_empty());
    assert_eq!(combining.glyphs().len(), 1);
    assert_eq!(combining.glyphs()[0].source, 0..3);
    let ligature = engine.shape("ffi", style()).unwrap();
    assert!(ligature.missing_clusters().is_empty());
    assert_eq!(ligature.glyphs().len(), 1);
    assert_eq!(ligature.glyphs()[0].source, 0..3);
    assert!(ligature.glyphs()[0].advance > 0.0);
}

#[test]
fn packaged_fallback_and_rtl_preserve_absolute_source_positions() {
    let text = "abc\r\nשלום\nسلام";
    let shaped = engine().shape(text, style()).unwrap();
    assert_eq!(shaped.lines().len(), 3);
    assert!(shaped.missing_clusters().is_empty());
    let fonts = [1, 3, 2];
    for (index, line) in shaped.lines().iter().enumerate() {
        assert_eq!(line.rtl, index != 0);
        assert_eq!(line.paragraph, index);
        let glyphs = &shaped.glyphs()[line.glyphs.clone()];
        assert!(!glyphs.is_empty());
        assert!(
            glyphs
                .iter()
                .all(|glyph| glyph.font == Ulid::from(fonts[index]))
        );
        for glyph in glyphs {
            assert!(!text[glyph.source.clone()].is_empty());
            assert_eq!(glyph.bidi_level % 2, u8::from(index != 0));
            assert!(glyph.position.iter().all(|p| p.is_finite()));
        }
        if line.rtl {
            // Draw by the returned positions: the shaper is not required to
            // enumerate glyphs left-to-right. Earlier logical text is rightmost.
            let first = glyphs
                .iter()
                .min_by_key(|glyph| glyph.source.start)
                .unwrap();
            let last = glyphs
                .iter()
                .max_by_key(|glyph| glyph.source.start)
                .unwrap();
            assert!(first.position[0] > last.position[0]);
        }
    }
}

#[test]
fn supplied_chain_order_wins_even_for_same_named_font_files() {
    let mut engine = TextEngine::new(vec![face(7, LATIN), face(8, LATIN)]).unwrap();
    let shaped = engine.shape("Chosen font", style()).unwrap();
    assert!(
        shaped
            .glyphs()
            .iter()
            .all(|glyph| glyph.font == Ulid::from(7))
    );
    let mut engine = TextEngine::new(vec![face(8, LATIN), face(7, LATIN)]).unwrap();
    let shaped = engine.shape("Chosen font", style()).unwrap();
    assert!(
        shaped
            .glyphs()
            .iter()
            .all(|glyph| glyph.font == Ulid::from(8))
    );
}

#[test]
fn fallback_order_is_explicit_for_two_faces_covering_the_same_missing_script() {
    let mut engine =
        TextEngine::new(vec![face(1, LATIN), face(9, HEBREW), face(3, HEBREW)]).unwrap();
    let shaped = engine.shape("שלום", style()).unwrap();
    assert!(shaped.missing_clusters().is_empty());
    assert!(
        shaped
            .glyphs()
            .iter()
            .all(|glyph| glyph.font == Ulid::from(9))
    );
}

#[test]
fn missing_glyphs_are_reported_without_host_font_fallback() {
    let mut engine = TextEngine::new(vec![face(1, LATIN)]).unwrap();
    let text = "A\u{10ffff}";
    let shaped = engine.shape(text, style()).unwrap();
    assert_eq!(shaped.missing_clusters().len(), 1);
    assert_eq!(shaped.missing_clusters()[0], 1..5);
    assert!(
        shaped
            .glyphs()
            .iter()
            .all(|glyph| glyph.font == Ulid::from(1))
    );
}

#[test]
fn wrapping_and_resize_remeasure_but_unchanged_layout_is_retained() {
    let mut engine = engine();
    let text = "One two three four five six seven eight";
    let wide = engine.shape(text, style()).unwrap();
    assert_eq!(wide.lines().len(), 1);
    assert!(Arc::ptr_eq(&wide, &engine.shape(text, style()).unwrap()));
    let small_style = TextStyle {
        width: 100.0,
        ..style()
    };
    let narrow = engine.shape(text, small_style).unwrap();
    assert!(narrow.lines().len() > wide.lines().len());
    assert!(narrow.lines().iter().all(|line| line.width <= 100.01));
    assert!(narrow.size()[1] > wide.size()[1]);
    assert!(!Arc::ptr_eq(&wide, &narrow));
    assert!(Arc::ptr_eq(
        &narrow,
        &engine.shape(text, small_style).unwrap()
    ));
    assert_eq!(wide.source(), text);
}

#[test]
fn no_wrap_allows_overflow_and_word_or_glyph_breaks_long_tokens() {
    let mut engine = engine();
    let text = "ABCDEFGHIJKLMNOPQRSTUVWXYZ";
    let nowrap = engine
        .shape(
            text,
            TextStyle {
                width: 40.0,
                wrap: Wrap::None,
                ..style()
            },
        )
        .unwrap();
    assert_eq!(nowrap.lines().len(), 1);
    assert!(nowrap.size()[0] > 40.0);
    let wrap = engine
        .shape(
            text,
            TextStyle {
                width: 40.0,
                ..style()
            },
        )
        .unwrap();
    assert!(wrap.lines().len() > 1);
    assert!(wrap.lines().iter().all(|line| line.width <= 40.01));
}

#[test]
fn font_size_scales_shaped_metrics() {
    let mut engine = engine();
    let small = engine.shape("Advance", style()).unwrap();
    let large = engine
        .shape(
            "Advance",
            TextStyle {
                font_size: 40.0,
                line_height: 60.0,
                width: 1600.0,
                ..style()
            },
        )
        .unwrap();
    assert_eq!(small.glyphs().len(), large.glyphs().len());
    assert!((large.size()[0] - 2.0 * small.size()[0]).abs() < 0.01);
    assert_eq!(large.size()[1], 2.0 * small.size()[1]);
}

#[test]
fn empty_and_trailing_line_breaks_have_measurable_caret_lines() {
    let mut engine = engine();
    for (text, lines) in [("", 1), ("\n", 2), ("A\r\n", 2), ("A\n\n", 3)] {
        let shaped = engine.shape(text, style()).unwrap();
        assert_eq!(shaped.lines().len(), lines, "{text:?}");
        assert_eq!(shaped.size()[1], lines as f32 * 30.0);
        assert!(shaped.lines().last().unwrap().glyphs.is_empty());
    }
}

#[test]
fn invalid_update_does_not_replace_the_last_valid_layout() {
    let mut engine = engine();
    let valid = engine.shape("Keep", style()).unwrap();
    for invalid in [f32::NAN, f32::INFINITY, -1.0, 0.0, 65537.0] {
        assert!(
            engine
                .shape(
                    "Replace",
                    TextStyle {
                        width: invalid,
                        ..style()
                    }
                )
                .is_err()
        );
    }
    assert!(
        engine
            .shape(
                "Replace",
                TextStyle {
                    font_size: 0.0,
                    ..style()
                }
            )
            .is_err()
    );
    assert!(
        engine
            .shape(
                "Replace",
                TextStyle {
                    line_height: f32::NAN,
                    ..style()
                }
            )
            .is_err()
    );
    assert!(matches!(
        engine.shape(&"a".repeat(MAX_TEXT_BYTES + 1), style()),
        Err(TextError::Limit("text bytes"))
    ));
    assert!(Arc::ptr_eq(&valid, &engine.shape("Keep", style()).unwrap()));
}

#[test]
fn bad_fonts_and_collections_refuse_without_silent_system_substitution() {
    assert!(matches!(
        TextEngine::new(vec![]),
        Err(TextError::Limit("font count"))
    ));
    assert!(matches!(
        TextEngine::new(vec![face(1, LATIN), face(1, ARABIC)]),
        Err(TextError::Invalid("duplicate font asset"))
    ));
    assert!(matches!(
        TextEngine::new(vec![face(1, b"garbage")]),
        Err(TextError::Font(_))
    ));
    assert!(matches!(
        TextEngine::new(vec![face(1, b"ttcf\0\x01\0\0\xff\xff\xff\xff")]),
        Err(TextError::Font(_))
    ));
    assert!(matches!(
        TextEngine::new(vec![face(1, b"\0\x01\0\0")]),
        Err(TextError::Font(_))
    ));
    assert!(matches!(
        TextEngine::new((0..=MAX_FONTS).map(|i| face(i as u128, &[])).collect()),
        Err(TextError::Limit("font count"))
    ));
}

#[test]
fn independent_engines_produce_identical_geometry_from_packaged_fonts() {
    let text = "abc שלום سلام e\u{301}";
    let a = engine().shape(text, style()).unwrap();
    let b = engine().shape(text, style()).unwrap();
    assert_eq!(a.glyphs(), b.glyphs());
    assert_eq!(a.lines(), b.lines());
    assert_eq!(a.missing_clusters(), b.missing_clusters());
}

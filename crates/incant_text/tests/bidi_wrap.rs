use incant_text::{FontSource, ShapedText, TextEngine, TextStyle, Wrap};

fn content(shaped: &ShapedText) -> Vec<(usize, usize, ulid::Ulid, u16, u8)> {
    let mut result: Vec<_> = shaped
        .glyphs()
        .iter()
        .filter(|g| {
            !shaped.source()[g.source.clone()]
                .chars()
                .all(char::is_whitespace)
        })
        .map(|g| (g.source.start, g.source.end, g.font, g.id, g.bidi_level))
        .collect();
    result.sort();
    result
}

fn engine() -> TextEngine {
    TextEngine::new(
        [
            include_bytes!("fonts/NotoSans-Regular.ttf").as_slice(),
            include_bytes!("fonts/NotoSansArabic.ttf").as_slice(),
            include_bytes!("fonts/IncantCJKFixture.ttf").as_slice(),
        ]
        .into_iter()
        .enumerate()
        .map(|(i, bytes)| FontSource {
            id: (i as u128 + 1).into(),
            bytes: bytes.to_vec(),
        })
        .collect(),
    )
    .unwrap()
}

#[test]
fn mixed_direction_wrapping_keeps_interior_spaces_and_respects_width() {
    let mut engine = engine();
    for text in [
        "Wrapped ffi سلام 中文 ש and spaces",
        "Wrapped ffi سلام 中文 ש and",
        "breakfast, إفطار, lunch (غداء) and dinner - عشاء",
        "سلام hello world سلام hello again",
        "Hello سلام  again שלום  world",
        "سلام abc سلام def سلام ghi",
    ] {
        let unwrapped = engine
            .shape(
                text,
                TextStyle {
                    font_size: 28.,
                    line_height: 36.,
                    width: 600.,
                    wrap: Wrap::None,
                },
            )
            .unwrap();
        for wrap in [Wrap::Word, Wrap::WordOrGlyph] {
            for width in [180., 250., 300., 350., 600.] {
                let shaped = engine
                    .shape(
                        text,
                        TextStyle {
                            font_size: 28.,
                            line_height: 36.,
                            width,
                            wrap,
                        },
                    )
                    .unwrap();
                assert_eq!(
                    content(&shaped),
                    content(&unwrapped),
                    "{text:?}, {wrap:?}, {width}"
                );
                for line in shaped.lines() {
                    assert!(
                        line.width <= width + 0.01,
                        "{text:?}, {wrap:?}, width {width}: {}",
                        line.width
                    );
                    let glyphs = &shaped.glyphs()[line.glyphs.clone()];
                    let first = glyphs.iter().map(|g| g.source.start).min().unwrap();
                    let last = glyphs.iter().map(|g| g.source.end).max().unwrap();
                    for (i, ch) in text.char_indices() {
                        if ch == ' ' && first < i && i < last {
                            assert!(
                                glyphs.iter().any(|g| g.source.contains(&i)),
                                "interior space {i} missing in {text:?}, {wrap:?}, width {width}"
                            );
                        }
                    }
                }
                for (i, ch) in text.char_indices().filter(|(_, c)| !c.is_whitespace()) {
                    assert!(
                        shaped.glyphs().iter().any(|g| g.source.contains(&i)),
                        "non-whitespace {ch} at {i} lost"
                    );
                }
            }
        }
    }
}

#[test]
fn mixed_direction_glyph_fallback_keeps_clusters_at_narrow_widths() {
    let mut engine = engine();
    for text in [
        "A שלום ABCDEFGHIJKLMNOPQRSTUVWXYZ سلام xyz",
        "سلام ABCDEFGHIJKLMNOPQRSTUVWXYZ שלום xyz",
        "ffi سلام\r\n中文 ש ABCDEFGHIJKLMNOPQRSTUVWXYZ\n",
    ] {
        let base = TextStyle {
            font_size: 28.,
            line_height: 36.,
            width: 600.,
            wrap: Wrap::None,
        };
        let original = engine.shape(text, base).unwrap();
        let widest_glyph = original
            .glyphs()
            .iter()
            .map(|g| g.advance)
            .fold(0_f32, f32::max);
        for width in [1., 15., 32., 60., 100., 180., 250., 350.] {
            let shaped = engine
                .shape(
                    text,
                    TextStyle {
                        width,
                        wrap: Wrap::WordOrGlyph,
                        ..base
                    },
                )
                .unwrap();
            assert_eq!(
                content(&shaped),
                content(&original),
                "{text:?}, width {width}"
            );
            for line in shaped.lines() {
                // One indivisible shaped glyph can exceed a narrower line.
                assert!(
                    line.width <= width.max(widest_glyph) + 0.01,
                    "{text:?}, width {width}, widest {widest_glyph}: {} {:?}",
                    line.width,
                    &shaped.glyphs()[line.glyphs.clone()]
                );
            }
        }
        let word = engine
            .shape(
                text,
                TextStyle {
                    width: 60.,
                    wrap: Wrap::Word,
                    ..base
                },
            )
            .unwrap();
        assert!(word.lines().iter().any(|line| line.width > 60.));
        assert_eq!(content(&word), content(&original));
    }
}

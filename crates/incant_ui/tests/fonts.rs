use incant_text::SharedFontSource;
use incant_ui::*;
use std::sync::Arc;
#[path = "fonts/rebind.rs"]
mod rebind;
const LATIN: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSans-Regular.ttf");
const HEBREW: &[u8] = include_bytes!("../../incant_text/tests/fonts/NotoSansHebrew.ttf");
const CJK: &[u8] = include_bytes!("../../incant_text/tests/fonts/IncantCJKFixture.ttf");
/// The label halo rule the native host passes (incant_render_data::UI_LABEL_HALO).
const HALO: HaloRule = HaloRule {
    em: 1. / 16.,
    min: 1.,
    max: 2.5,
};
fn id(n: u128) -> StableId {
    StableId(n.to_be_bytes())
}
fn source(n: u128, bytes: &[u8]) -> SharedFontSource {
    SharedFontSource {
        id: n.into(),
        bytes: Arc::new(bytes.to_vec()),
    }
}
fn widgets() -> Vec<Widget> {
    let text = UiText {
        content: "Latin שלום 中文".into(),
        fonts: vec![id(10), id(11), id(12)],
        font_size: 20.0,
        line_height: 25.0,
        wrap: incant_types::TextWrap::WordOrGlyph,
    };
    vec![
        Widget {
            incarnation: 1,
            id: id(1),
            parent: None,
            node: Arc::new(UiNode {
                kind: UiKind::Root,
                style: UiStyle::default(),
                order: 0,
                disabled: false,
            }),
        },
        Widget {
            incarnation: 1,
            id: id(2),
            parent: Some(id(1)),
            node: Arc::new(UiNode {
                kind: UiKind::TextInput { text },
                style: UiStyle {
                    shrink: 0.0,
                    ..Default::default()
                },
                order: 0,
                disabled: false,
            }),
        },
    ]
}
#[test]
fn actual_packaged_multilingual_fonts_measure_layout_and_composed_buffer() {
    let widgets = widgets();
    let sources = vec![source(10, LATIN), source(11, HEBREW), source(12, CJK)];
    let original = Arc::clone(&sources[0].bytes);
    let mut fonts = UiFonts::new(sources, &widgets).unwrap();
    assert!(Arc::ptr_eq(
        &original,
        &fonts.sources().next().unwrap().bytes
    ));
    let mut ui = UiRuntime::new(widgets.clone()).unwrap();
    let first = ui.prepare(&[], [250.0, 250.0], &mut fonts).unwrap();
    ui.commit(first).unwrap();
    let rect = ui
        .layout()
        .unwrap()
        .nodes
        .iter()
        .find(|n| n.id == id(2))
        .unwrap();
    assert_eq!(rect.content.size[0], 250.0);
    assert_eq!(rect.content.size[1], 25.0);
    let shaped = fonts
        .shape(
            &widgets[1],
            ui.text(id(2)).unwrap().display_text(),
            rect.content.size[0],
        )
        .unwrap();
    assert!(shaped.glyphs().iter().any(|g| g.bidi_level % 2 == 1));
    assert!(
        shaped
            .glyphs()
            .iter()
            .any(|g| g.font.to_bytes() == id(12).0)
    );
    assert!(shaped.missing_clusters().is_empty());
    let focus = ui
        .prepare(&[UiEvent::Focus(id(2))], [250.0, 250.0], &mut fonts)
        .unwrap();
    ui.commit(focus).unwrap();
    let target = ui.focus().unwrap();
    let preedit = ui
        .prepare(
            &[UiEvent::Preedit {
                target: target.clone(),
                text: " 日本語".into(),
                cursor: None,
            }],
            [80.0, 250.0],
            &mut fonts,
        )
        .unwrap();
    assert!(
        preedit
            .layout()
            .nodes
            .iter()
            .find(|n| n.id == id(2))
            .unwrap()
            .content
            .size[1]
            > 25.0
    );
    ui.commit(preedit).unwrap();
    assert!(ui.text(id(2)).unwrap().display_text().ends_with(" 日本語"));
    assert!(!ui.text(id(2)).unwrap().text().ends_with(" 日本語"));
    let cancel = ui
        .prepare(&[UiEvent::Cancel { target }], [80.0, 250.0], &mut fonts)
        .unwrap();
    ui.commit(cancel).unwrap();
    assert_eq!(ui.text(id(2)).unwrap().display_text(), "Latin שלום 中文");
}
#[test]
fn missing_invalid_or_duplicate_packaged_fonts_refuse_before_layout() {
    let widgets = widgets();
    assert!(UiFonts::new(vec![source(10, LATIN)], &widgets).is_err());
    assert!(UiFonts::new(vec![source(10, LATIN), source(10, LATIN)], &widgets).is_err());
    assert!(UiFonts::new(vec![source(10, b"not a font")], &widgets).is_err());
}

#[test]
fn retained_paint_refuses_exhaustion_and_never_rebinds_old_atlas_handles() {
    let widgets = widgets();
    let mut fonts = UiFonts::new(
        vec![source(10, LATIN), source(11, HEBREW), source(12, CJK)],
        &widgets,
    )
    .unwrap();
    let mut ui = UiRuntime::new(widgets).unwrap();
    let config = AtlasConfig {
        width: 256,
        height: 256,
        max_glyphs: 128,
    };
    let pending = ui.prepare(&[], [250., 250.], &mut fonts).unwrap();
    let old = UiPaint::prepare(&ui, &pending, &mut fonts, config, HALO, None).unwrap();
    let handle = old.texts[0].glyphs[0].clone();
    assert!(old.atlas.get(&handle).is_some());
    assert!(old.atlas.pixels().iter().any(|p| *p != 0));
    ui.commit(pending).unwrap();
    let next = ui.prepare_refresh([250., 250.], &mut fonts).unwrap();
    assert!(Arc::ptr_eq(
        &old,
        &UiPaint::prepare(&ui, &next, &mut fonts, config, HALO, Some(&old)).unwrap()
    ));
    for bad in [
        AtlasConfig {
            width: 1,
            height: 1,
            ..config
        },
        AtlasConfig {
            max_glyphs: 1,
            ..config
        },
    ] {
        assert!(UiPaint::prepare(&ui, &next, &mut fonts, bad, HALO, Some(&old)).is_err());
        assert!(old.atlas.get(&handle).is_some());
        assert!(Arc::ptr_eq(&old.layout, ui.layout().unwrap()));
    }
    let resized = ui.prepare_refresh([80., 250.], &mut fonts).unwrap();
    let new = UiPaint::prepare(&ui, &resized, &mut fonts, config, HALO, Some(&old)).unwrap();
    assert!(new.atlas.get(&handle).is_none());
    assert!(old.atlas.get(&handle).is_some());
    ui.commit(resized).unwrap();
    assert!(Arc::ptr_eq(&new.layout, ui.layout().unwrap()));
}

#[test]
fn combined_glyph_limit_refuses_before_candidate_atlas_rasterization() {
    let mut widgets = widgets();
    widgets.truncate(1);
    for n in 2..=6 {
        widgets.push(Widget {
            incarnation: 1,
            id: id(n),
            parent: Some(id(1)),
            node: Arc::new(UiNode {
                kind: UiKind::Label {
                    text: UiText {
                        content: "A".repeat(16_384),
                        fonts: vec![id(10)],
                        font_size: 16.,
                        line_height: 20.,
                        wrap: incant_types::TextWrap::WordOrGlyph,
                    },
                },
                style: UiStyle::default(),
                order: 0,
                disabled: false,
            }),
        });
    }
    let mut fonts = UiFonts::new(vec![source(10, LATIN)], &widgets).unwrap();
    let ui = UiRuntime::new(widgets).unwrap();
    let pending = ui.prepare(&[], [65_536., 65_536.], &mut fonts).unwrap();
    // A one-pixel atlas could not fit a glyph. Aggregate refusal must win
    // before creating/rasterizing that candidate, not after partial publication.
    assert!(matches!(
        UiPaint::prepare(
            &ui,
            &pending,
            &mut fonts,
            AtlasConfig {
                width: 1,
                height: 1,
                max_glyphs: 1
            },
            HALO,
            None
        ),
        Err(UiError::Limit("UI glyph placements"))
    ));
    assert!(ui.layout().is_none());
}

#[test]
fn only_labels_get_prepared_halos_and_the_rule_is_part_of_the_paint() {
    let mut widgets = widgets();
    let text = widgets[1].node.kind.text().unwrap().clone();
    widgets.push(Widget {
        incarnation: 1,
        id: id(3),
        parent: Some(id(1)),
        node: Arc::new(UiNode {
            kind: UiKind::Label { text },
            style: UiStyle::default(),
            order: 1,
            disabled: false,
        }),
    });
    let mut fonts = UiFonts::new(
        vec![source(10, LATIN), source(11, HEBREW), source(12, CJK)],
        &widgets,
    )
    .unwrap();
    let mut ui = UiRuntime::new(widgets).unwrap();
    let config = AtlasConfig {
        width: 512,
        height: 512,
        max_glyphs: 256,
    };
    let pending = ui.prepare(&[], [250., 250.], &mut fonts).unwrap();
    let paint = UiPaint::prepare(&ui, &pending, &mut fonts, config, HALO, None).unwrap();
    // Canonical order: the text field (order 0), then the label (order 1).
    let (field, label) = (&paint.texts[0], &paint.texts[1]);
    assert!(field.halos.is_none());
    let halos = label.halos.as_ref().unwrap();
    assert_eq!(halos.len(), label.glyphs.len());
    let radius = HALO.radius(20.).unwrap();
    for (glyph, halo) in label.glyphs.iter().zip(halos.iter()) {
        let (glyph, halo) = (
            paint.atlas.get(glyph).unwrap(),
            paint.atlas.get(halo).unwrap(),
        );
        assert_eq!(glyph.key.halo_radius_bits, 0);
        assert_eq!(halo.key.halo_radius_bits, radius.to_bits());
        assert_eq!(halo.rect.is_some(), glyph.rect.is_some());
    }
    // The field's glyph entries are the label's: halos never replace coverage.
    for (a, b) in field.glyphs.iter().zip(label.glyphs.iter()) {
        assert_eq!(paint.atlas.get(a), paint.atlas.get(b));
    }
    // A different rule is a different paint; an invalid one is refused.
    ui.commit(pending).unwrap();
    let next = ui.prepare_refresh([250., 250.], &mut fonts).unwrap();
    let wider = HaloRule { min: 2., ..HALO };
    let rebuilt = UiPaint::prepare(&ui, &next, &mut fonts, config, wider, Some(&paint)).unwrap();
    assert!(!Arc::ptr_eq(&paint, &rebuilt));
    let excessive = HaloRule { max: 9., ..HALO };
    assert!(UiPaint::prepare(&ui, &next, &mut fonts, config, excessive, Some(&paint)).is_err());
}

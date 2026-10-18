use super::*;

fn fonts(widgets: &[Widget]) -> UiFonts {
    UiFonts::new(
        vec![source(10, LATIN), source(11, HEBREW), source(12, CJK)],
        widgets,
    )
    .unwrap()
}

fn text(widget: &mut Widget) -> &mut UiText<StableId> {
    match &mut Arc::make_mut(&mut widget.node).kind {
        UiKind::TextInput { text } => text,
        _ => panic!("text fixture"),
    }
}

#[test]
fn compatible_rebind_retains_shaper_and_matches_fresh_multilingual_layouts() {
    let mut widgets = widgets();
    let mut retained = fonts(&widgets);
    let original = retained.shape(&widgets[1], "Latin שלום 中文", 80.).unwrap();
    Arc::make_mut(&mut widgets[1].node).disabled = true;
    assert!(retained.prepare_rebind(&widgets).unwrap().is_none());
    assert!(Arc::ptr_eq(
        &original,
        &retained.shape(&widgets[1], "Latin שלום 中文", 80.).unwrap()
    ));
    for (content, width) in [("Health 9 שלום 中文", 80.), ("中文 שלום 123", 250.)] {
        text(&mut widgets[1]).content = content.into();
        assert!(retained.prepare_rebind(&widgets).unwrap().is_none());
        let actual = retained.shape(&widgets[1], content, width).unwrap();
        let expected = fonts(&widgets).shape(&widgets[1], content, width).unwrap();
        assert_eq!(actual.glyphs(), expected.glyphs());
        assert_eq!(actual.lines(), expected.lines());
        assert_eq!(actual.size(), expected.size());
        assert_eq!(actual.missing_clusters(), expected.missing_clusters());
    }
    assert_eq!(original.source(), "Latin שלום 中文");
    assert!(original.missing_clusters().is_empty());
}

#[test]
fn new_font_chain_is_a_candidate_and_refusal_preserves_accepted_shaper() {
    let widgets = widgets();
    let mut retained = fonts(&widgets);
    let original = retained.shape(&widgets[1], "Latin שלום 中文", 80.).unwrap();
    let mut changed = widgets.clone();
    text(&mut changed[1]).fonts.swap(0, 1);
    let mut replacement = retained.prepare_rebind(&changed).unwrap().unwrap();
    let actual = replacement
        .shape(&changed[1], "שלום 中文 Latin", 80.)
        .unwrap();
    let expected = fonts(&changed)
        .shape(&changed[1], "שלום 中文 Latin", 80.)
        .unwrap();
    assert_eq!(actual.glyphs(), expected.glyphs());
    assert_eq!(actual.lines(), expected.lines());
    assert!(replacement.prepare_rebind(&changed).unwrap().is_none());
    // Preparing a different chain never replaces the accepted font owner.
    assert!(retained.prepare_rebind(&widgets).unwrap().is_none());
    assert!(Arc::ptr_eq(
        &original,
        &retained.shape(&widgets[1], "Latin שלום 中文", 80.).unwrap()
    ));
    text(&mut changed[1]).fonts = vec![id(999)];
    assert!(retained.prepare_rebind(&changed).is_err());
    let mut invalid = widgets.clone();
    text(&mut invalid[1]).font_size = f32::NAN;
    assert!(retained.prepare_rebind(&invalid).is_err());
    let oversized = vec![widgets[0].clone(); MAX_UI_NODES + 1];
    assert!(retained.prepare_rebind(&oversized).is_err());
    assert!(Arc::ptr_eq(
        &original,
        &retained.shape(&widgets[1], "Latin שלום 中文", 80.).unwrap()
    ));
}

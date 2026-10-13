//! CPU checks of the UI layer's planning, caret and selection geometry over
//! real shaped text. GPU draws live in ui_draw_tests.rs. No appearance approval.
use super::geometry::{KIND_FILL, KIND_GLYPH, KIND_STROKE, UiQuad, halo_clip, plan};
use super::*;
use crate::text::tests::engine;
use incant_render_data::{UiEdit, UiRect, UiRole, UiWidget};
use incant_text::{
    AtlasConfig, GlyphHandle, ShapedText, TextEngine, TextStyle, Wrap, editing::Selection,
};
use incant_types::StableId;

pub(crate) fn id(n: u8) -> StableId {
    StableId([n; 16])
}
/// Real shaping and rasterization with the packaged test fonts into one atlas.
pub(crate) struct Fixture {
    pub engine: TextEngine,
    pub atlas: GlyphAtlas,
}
impl Fixture {
    pub fn new() -> Self {
        Self {
            engine: engine(),
            atlas: GlyphAtlas::new(AtlasConfig {
                width: 512,
                height: 512,
                max_glyphs: 1024,
            })
            .unwrap(),
        }
    }
    pub fn shape(
        &mut self,
        text: &str,
        font_size: f32,
        width: f32,
        wrap: Wrap,
    ) -> (Arc<ShapedText>, Arc<[GlyphHandle]>) {
        let layout = self
            .engine
            .shape(
                text,
                TextStyle {
                    font_size,
                    line_height: (font_size * 1.25).round(),
                    width,
                    wrap,
                },
            )
            .unwrap();
        let glyphs: Vec<_> = (0..layout.glyphs().len())
            .map(|i| {
                self.engine
                    .cache_glyph(&layout, i, &mut self.atlas)
                    .unwrap()
            })
            .collect();
        (layout, glyphs.into())
    }
    /// One prepared halo per glyph of `layout` at the theme's label rule, as
    /// the UI owner prepares them.
    pub fn halos(&mut self, layout: &ShapedText) -> Arc<[GlyphHandle]> {
        let halos: Vec<_> = (0..layout.glyphs().len())
            .map(|i| {
                let radius = theme::LABEL_HALO_RULE
                    .radius(layout.glyphs()[i].font_size)
                    .unwrap();
                self.engine
                    .cache_halo(layout, i, &mut self.atlas, radius)
                    .unwrap()
            })
            .collect();
        halos.into()
    }
    /// A widget whose content box is `bounds` inset by `padding` on every side;
    /// a label also gets its prepared halos.
    pub fn widget(
        &mut self,
        n: u8,
        role: UiRole,
        bounds: UiRect,
        padding: f32,
        content: &str,
        font_size: f32,
    ) -> UiWidget {
        let clip = [
            bounds[0] + padding,
            bounds[1] + padding,
            bounds[2] - padding,
            bounds[3] - padding,
        ];
        let (layout, glyphs) = self.shape(content, font_size, clip[2] - clip[0], Wrap::Word);
        let halos = (role == UiRole::Label).then(|| self.halos(&layout));
        UiWidget {
            id: id(n),
            role,
            disabled: false,
            focused: false,
            pressed: false,
            bounds,
            visible: bounds,
            origin: [clip[0], clip[1]],
            clip,
            layout,
            glyphs,
            halos,
            edit: (role == UiRole::TextInput).then(|| UiEdit {
                selection: Some(Selection::caret(content.len())),
                preedit: None,
            }),
        }
    }
    pub fn finish(self, viewport: [f32; 2], widgets: Vec<UiWidget>) -> Arc<UiInput> {
        Arc::new(UiInput {
            viewport,
            atlas: Arc::new(self.atlas),
            widgets,
        })
    }
}
fn shaped(text: &str, width: f32, wrap: Wrap) -> Arc<ShapedText> {
    Fixture::new().shape(text, 20., width, wrap).0
}
fn x(layout: &ShapedText, byte: usize) -> f32 {
    caret(layout, byte).unwrap().x
}
fn glyph_at(layout: &ShapedText, byte: usize) -> &incant_text::Glyph {
    layout
        .glyphs()
        .iter()
        .find(|g| g.source.start == byte)
        .unwrap()
}

#[test]
fn caret_follows_cluster_edges_bidi_levels_and_downstream_affinity() {
    let latin = shaped("Hello", 600., Wrap::None);
    assert_eq!(x(&latin, 0), latin.glyphs()[0].position[0]);
    assert_eq!(x(&latin, 1), glyph_at(&latin, 1).position[0]);
    let last = latin.glyphs().last().unwrap();
    assert_eq!(x(&latin, 5), last.position[0] + last.advance);
    assert!(caret(&latin, 6).is_none());

    // RTL: logical start is the right edge, logical end the left edge.
    let arabic = shaped("سلام", 600., Wrap::None);
    let right = arabic
        .glyphs()
        .iter()
        .map(|g| g.position[0] + g.advance)
        .fold(f32::MIN, f32::max);
    let left = arabic
        .glyphs()
        .iter()
        .map(|g| g.position[0])
        .fold(f32::MAX, f32::min);
    assert!(arabic.glyphs().iter().all(|g| g.bidi_level % 2 == 1));
    assert_eq!(x(&arabic, 0), right);
    assert_eq!(x(&arabic, "سلام".len()), left);
    assert!(x(&arabic, 2) < right && x(&arabic, 2) > left);

    // At an LTR→RTL boundary the caret belongs to the following character.
    let mixed = shaped("ab سلام", 600., Wrap::None);
    let first = glyph_at(&mixed, 3);
    assert_eq!(first.bidi_level % 2, 1);
    assert_eq!(x(&mixed, 3), first.position[0] + first.advance);

    // A ligature cluster splits its advance by scalar values.
    let ffi = shaped("ffi", 600., Wrap::None);
    if let Some(liga) = ffi.glyphs().iter().find(|g| g.source == (0..3)) {
        let third = liga.advance / 3.;
        assert!((x(&ffi, 1) - (liga.position[0] + third)).abs() < 1e-4);
        assert!((x(&ffi, 2) - (liga.position[0] + 2. * third)).abs() < 1e-4);
    } else {
        panic!("packaged NotoSans has an ffi ligature: {:?}", ffi.glyphs());
    }

    // A soft wrap point shows the caret at the start of the next line.
    let wrapped = shaped("alpha beta", 70., Wrap::Word);
    assert!(wrapped.lines().len() >= 2);
    let start = "alpha ".len();
    let c = caret(&wrapped, start).unwrap();
    assert_eq!(c.line, 1);
    assert_eq!(c.x, glyph_at(&wrapped, start).position[0]);
    assert_eq!(c.top, wrapped.lines()[1].top);
    assert_eq!(c.bottom, wrapped.lines()[1].top + wrapped.lines()[1].height);
}

#[test]
fn caret_on_empty_text_empty_paragraphs_and_every_line_ending() {
    let empty = shaped("", 600., Wrap::Word);
    assert!(!empty.lines().is_empty());
    assert_eq!(caret(&empty, 0).unwrap().line, 0);
    assert_eq!(caret(&empty, 0).unwrap().x, 0.);
    // A trailing break places the caret on the following empty line.
    let trailing = shaped("ab\n", 600., Wrap::Word);
    let c = caret(&trailing, 3).unwrap();
    assert_eq!((c.line, c.x), (1, 0.));
    assert_eq!(c.top, trailing.lines()[1].top);
    // The end of a paragraph is the trailing edge of its last glyph.
    let b = glyph_at(&trailing, 1);
    assert_eq!(caret(&trailing, 2).unwrap().x, b.position[0] + b.advance);
    for (text, byte, paragraph) in [
        ("a\r\n\r\nb", 3, 1),
        ("a\n\r\n\rb", 3, 1),
        ("a\r\rb", 2, 1),
        ("a\n\nb", 2, 1),
    ] {
        let layout = shaped(text, 600., Wrap::Word);
        let c = caret(&layout, byte).unwrap();
        assert_eq!(layout.lines()[c.line].paragraph, paragraph, "{text:?}");
        assert_eq!(c.x, 0., "{text:?}");
        let end = caret(&layout, text.len() - 1).unwrap();
        assert_eq!(
            layout.lines()[end.line].paragraph,
            layout.lines().last().unwrap().paragraph
        );
    }
}

#[test]
fn selection_spans_merge_per_line_and_split_mixed_direction_runs() {
    let text = "abc سلام def";
    let layout = shaped(text, 600., Wrap::None);
    let mut spans = Vec::new();
    // "c" + space + the first two logical Arabic letters (visually rightmost).
    let range = 2.."abc سل".len();
    spans_for(&layout, range.clone(), &mut spans);
    assert_eq!(spans.len(), 2, "{spans:?}");
    let c = glyph_at(&layout, 2);
    assert_eq!(spans[0].x0, c.position[0]);
    let arabic_right = layout
        .glyphs()
        .iter()
        .filter(|g| g.bidi_level % 2 == 1)
        .map(|g| g.position[0] + g.advance)
        .fold(f32::MIN, f32::max);
    assert!((spans[1].x1 - arabic_right).abs() < 1e-3);
    assert!(spans[0].x1 < spans[1].x0);
    // Selecting the whole wrapped paragraph yields one merged span per line.
    let wrapped = shaped("Wrapped ffi سلام 中文 and spaces", 120., Wrap::Word);
    spans_for(&wrapped, 0..wrapped.source().len(), &mut spans);
    let lines: std::collections::BTreeSet<_> = spans.iter().map(|s| s.line).collect();
    assert_eq!(lines.len(), wrapped.lines().len());
    assert_eq!(spans.len(), wrapped.lines().len(), "{spans:?}");
    for span in &spans {
        let line = &wrapped.lines()[span.line];
        let glyphs = &wrapped.glyphs()[line.glyphs.clone()];
        let left = glyphs
            .iter()
            .filter(|g| g.advance > 0.)
            .map(|g| g.position[0])
            .fold(f32::MAX, f32::min);
        assert!((span.x0 - left).abs() < 1e-3);
    }
    // A partial ligature selection covers its proportional share.
    let ffi = shaped("ffi", 600., Wrap::None);
    spans_for(&ffi, 1..2, &mut spans);
    let g = &ffi.glyphs()[0];
    assert_eq!(spans.len(), 1);
    assert!((spans[0].x0 - (g.position[0] + g.advance / 3.)).abs() < 1e-4);
    assert!((spans[0].x1 - (g.position[0] + 2. * g.advance / 3.)).abs() < 1e-4);
    spans_for(&ffi, 1..1, &mut spans);
    assert!(spans.is_empty());
}

fn kinds(quads: &[UiQuad]) -> Vec<u32> {
    quads.iter().map(|q| q.kind).collect()
}
#[test]
fn plan_layers_each_widget_in_canonical_order_with_state_colours() {
    let mut f = Fixture::new();
    let label = f.widget(1, UiRole::Label, [0., 0., 200., 30.], 0., "Hi", 16.);
    let mut pressed = f.widget(2, UiRole::Button, [0., 40., 100., 70.], 6., "Go", 16.);
    pressed.pressed = true;
    let mut focused = f.widget(3, UiRole::Button, [110., 40., 210., 70.], 6., "Go", 16.);
    focused.focused = true;
    let mut disabled = f.widget(4, UiRole::Button, [220., 40., 320., 70.], 6., "No", 16.);
    disabled.disabled = true;
    let mut field = f.widget(5, UiRole::TextInput, [0., 80., 200., 110.], 6., "abc", 16.);
    field.focused = true;
    field.edit = Some(UiEdit {
        selection: Some(Selection {
            anchor: 0,
            focus: 2,
        }),
        preedit: None,
    });
    let idle = f.widget(6, UiRole::TextInput, [0., 120., 200., 150.], 6., "x", 16.);
    let glyphs: Vec<_> = [&label, &pressed, &focused, &disabled, &field, &idle]
        .iter()
        .map(|w| w.layout.glyphs().len())
        .collect();
    let input = f.finish(
        [400., 200.],
        vec![
            label.clone(),
            pressed,
            focused,
            disabled,
            field.clone(),
            idle,
        ],
    );
    let plan = plan(&input).unwrap();
    let mut at = 0;
    let mut take = |n: usize| {
        let part = &plan.quads[at..at + n];
        at += n;
        part
    };
    // Label: its prepared halos, then the glyphs in the primary text colour
    // clipped to the content box; no surface. Both are ordinary glyph quads at
    // the same pen: each halo samples its own atlas entry, the glyph's coverage
    // dilated by the 1 px radius of 16 px text, one texel larger on every side.
    let part = take(2 * glyphs[0]);
    assert_eq!(kinds(part), vec![KIND_GLYPH; 2 * glyphs[0]]);
    let (halo, text) = part.split_at(glyphs[0]);
    assert!(
        text.iter()
            .all(|q| q.color == theme::TEXT && q.clip == label.clip)
    );
    assert_eq!(theme::LABEL_HALO_RULE.radius(16.).unwrap(), 1.);
    let handles = label.halos.as_ref().unwrap();
    for ((h, t), handle) in halo.iter().zip(text).zip(handles.iter()) {
        let entry = input.atlas.get(handle).unwrap();
        let [x, y, w, height] = entry.rect.unwrap().map(|v| v as f32);
        let [ox, oy] = entry.origin.map(|v| v as f32);
        assert_eq!(h.color, theme::LABEL_HALO);
        assert_eq!(h.pen, t.pen);
        assert_eq!(h.rect, [ox - 1., oy - 1., ox + w + 1., oy + height + 1.]);
        assert_eq!(h.extra, [x - 1., y - 1., x + w + 1., y + height + 1.]);
        assert_eq!(
            h.rect,
            [
                t.rect[0] - 1.,
                t.rect[1] - 1.,
                t.rect[2] + 1.,
                t.rect[3] + 1.
            ]
        );
        // No padding: the visible border box is the content box, so the halo
        // keeps the glyphs' own clip.
        assert_eq!(h.clip, label.clip);
    }
    // Pressed button: darker surface, then glyphs.
    let part = take(1 + glyphs[1]);
    assert_eq!(part[0].kind, KIND_FILL);
    assert_eq!(part[0].color, theme::BUTTON_PRESSED);
    assert_eq!(part[0].extra[0], theme::RADIUS);
    // Focused button: surface, glyphs, then the inset accent ring last.
    let part = take(2 + glyphs[2]);
    assert_eq!(part[0].color, theme::BUTTON);
    let ring = part.last().unwrap();
    assert_eq!(
        (ring.kind, ring.color, ring.extra[1]),
        (KIND_STROKE, theme::ACCENT, theme::FOCUS_WIDTH)
    );
    // Disabled button: an opaque, flatter surface and muted text, never a ring.
    let part = take(1 + glyphs[3]);
    assert_eq!(part[0].color, theme::BUTTON_DISABLED);
    assert_eq!(part[0].color[3], 255);
    assert!(
        part[1..]
            .iter()
            .all(|q| q.color[3] == theme::TEXT_DISABLED_ALPHA)
    );
    // Focused field: surface, selection, glyphs, ring; no outline or caret
    // while a range is selected.
    let mut spans = Vec::new();
    spans_for(&field.layout, 0..2, &mut spans);
    assert_eq!(spans.len(), 1);
    let part = take(1 + 1 + glyphs[4] + 1);
    assert_eq!(part[0].color, theme::FIELD);
    assert_eq!(part[1].color, theme::SELECTION);
    let line = &field.layout.lines()[0];
    assert_eq!(
        part[1].rect,
        [
            field.origin[0] + spans[0].x0,
            field.origin[1] + line.top,
            field.origin[0] + spans[0].x1,
            field.origin[1] + line.top + line.height
        ]
    );
    assert_eq!(part[1].clip, field.visible);
    assert_eq!(part.last().unwrap().color, theme::ACCENT);
    // Unfocused field: surface and 1 px outline; its stored edit state is not shown.
    let part = take(2 + glyphs[5]);
    assert_eq!(
        (part[1].kind, part[1].color, part[1].extra[1]),
        (KIND_STROKE, theme::FIELD_BORDER, theme::BORDER_WIDTH)
    );
    assert_eq!(at, plan.quads.len());
    assert_eq!(
        plan.glyphs,
        glyphs.iter().sum::<usize>() - plan_blank(&input)
    );
}
/// Shaped glyphs with empty outlines (spaces) produce no quad.
fn plan_blank(input: &UiInput) -> usize {
    input
        .widgets
        .iter()
        .flat_map(|w| w.glyphs.iter())
        .filter(|h| input.atlas.get(h).unwrap().rect.is_none())
        .count()
}

#[test]
fn caret_and_composition_follow_the_supplied_edit_ranges_only() {
    let mut f = Fixture::new();
    let text = "ab中文";
    let mut field = f.widget(1, UiRole::TextInput, [10., 10., 210., 40.], 6., text, 16.);
    field.focused = true;
    let atlas = Arc::new(f.atlas);
    let preedit = 2..text.len();
    let input = |selection| {
        let mut field = field.clone();
        field.edit = Some(UiEdit {
            selection,
            preedit: Some(preedit.clone()),
        });
        Arc::new(UiInput {
            viewport: [300., 100.],
            atlas: Arc::clone(&atlas),
            widgets: vec![field],
        })
    };
    let glyphs = field.layout.glyphs().len();
    // Composition with an IME caret: underline the whole preedit, then a caret.
    let shown = plan(&input(Some(Selection::caret(5)))).unwrap();
    let rest = &shown.quads[1 + glyphs..];
    assert_eq!(kinds(rest), [KIND_FILL, KIND_FILL, KIND_STROKE]);
    let expected = caret(&field.layout, 5).unwrap();
    assert_eq!(
        rest[1].rect,
        [
            field.origin[0] + expected.x - theme::CARET_WIDTH / 2.,
            field.origin[1] + expected.top,
            field.origin[0] + expected.x + theme::CARET_WIDTH / 2.,
            field.origin[1] + expected.bottom
        ]
    );
    let underline = rest[0];
    let mut spans = Vec::new();
    spans_for(&field.layout, preedit.clone(), &mut spans);
    assert_eq!(spans.len(), 1);
    assert_eq!(
        [underline.rect[0], underline.rect[2]],
        [field.origin[0] + spans[0].x0, field.origin[0] + spans[0].x1]
    );
    let size = spans[0].font_size;
    let baseline = field.layout.lines()[0].baseline;
    let thickness = (theme::UNDERLINE_THICKNESS * size).max(1.);
    let near = |a: f32, b: f32| assert!((a - b).abs() < 1e-4, "{a} != {b}");
    near(
        underline.rect[1],
        field.origin[1] + baseline + theme::UNDERLINE_OFFSET * size,
    );
    near(underline.rect[3] - underline.rect[1], thickness);
    assert_eq!(underline.color[3], theme::PREEDIT_ALPHA);
    // A converted clause is underlined at double weight and hides the caret.
    let clause = plan(&input(Some(Selection {
        anchor: 2,
        focus: 5,
    })))
    .unwrap();
    let rest = &clause.quads[1 + glyphs..];
    assert_eq!(kinds(rest), [KIND_FILL, KIND_FILL, KIND_STROKE]);
    near(
        rest[1].rect[3] - rest[1].rect[1],
        thickness * theme::CLAUSE_THICKNESS,
    );
    // The input method may hide its caret entirely.
    let hidden = plan(&input(None)).unwrap();
    assert_eq!(kinds(&hidden.quads[1 + glyphs..]), [KIND_FILL, KIND_STROKE]);
    // Without focus nothing of the edit state is drawn.
    let mut idle = field.clone();
    idle.focused = false;
    let idle = plan(&Arc::new(UiInput {
        viewport: [300., 100.],
        atlas: Arc::clone(&atlas),
        widgets: vec![idle],
    }))
    .unwrap();
    assert_eq!(kinds(&idle.quads[..2]), [KIND_FILL, KIND_STROKE]);
    assert_eq!(idle.quads.len(), 2 + glyphs - plan_blank(&input(None)));
}

#[test]
fn caret_accessor_returns_the_drawn_caret_and_only_for_a_focused_enabled_field() {
    let mut f = Fixture::new();
    let mut latin = f.widget(
        1,
        UiRole::TextInput,
        [10., 10., 210., 40.],
        6.,
        "Hello",
        16.,
    );
    let mut arabic = f.widget(2, UiRole::TextInput, [10., 50., 210., 80.], 6., "سلام", 16.);
    let mut cjk = f.widget(
        3,
        UiRole::TextInput,
        [10., 90., 210., 120.],
        6.,
        "ab中文",
        16.,
    );
    for field in [&mut latin, &mut arabic, &mut cjk] {
        field.focused = true;
    }
    // The input method's caret inside a composition is the drawn text cursor.
    cjk.edit = Some(UiEdit {
        selection: Some(Selection::caret(5)),
        preedit: Some(2.."ab中文".len()),
    });
    let input = f.finish(
        [300., 130.],
        vec![latin.clone(), arabic.clone(), cjk.clone()],
    );
    let drawn = plan(&input).unwrap();
    for widget in &input.widgets {
        let shown = ui_caret(widget).unwrap();
        let focus = widget.edit.as_ref().unwrap().selection.unwrap().focus;
        let expected = caret(&widget.layout, focus).unwrap();
        let half = theme::CARET_WIDTH / 2.;
        assert_eq!(
            shown.rect,
            [
                widget.origin[0] + expected.x - half,
                widget.origin[1] + expected.top,
                widget.origin[0] + expected.x + half,
                widget.origin[1] + expected.bottom
            ]
        );
        assert!(shown.visible);
        // Exactly the rectangle the layer draws, as a caret-coloured fill.
        assert_eq!(
            drawn
                .quads
                .iter()
                .filter(|q| q.kind == KIND_FILL && q.rect == shown.rect && q.color == theme::TEXT)
                .count(),
            1
        );
    }
    // The RTL logical end is the visual left edge of the run.
    let left = arabic
        .layout
        .glyphs()
        .iter()
        .map(|g| g.position[0])
        .fold(f32::MAX, f32::min);
    let shown = ui_caret(&arabic).unwrap();
    assert_eq!(
        (shown.rect[0] + shown.rect[2]) / 2.,
        arabic.origin[0] + left
    );

    // No caret is drawn, so none is reported: unfocused, disabled, a range
    // selection, a converted clause, an input method hiding its cursor, and
    // widgets other than text inputs.
    let mut cases = Vec::new();
    let mut unfocused = latin.clone();
    unfocused.focused = false;
    cases.push(unfocused);
    let mut disabled = latin.clone();
    disabled.disabled = true;
    cases.push(disabled);
    let mut range = latin.clone();
    range.edit = Some(UiEdit {
        selection: Some(Selection {
            anchor: 1,
            focus: 4,
        }),
        preedit: None,
    });
    cases.push(range);
    let mut clause = cjk.clone();
    clause.edit = Some(UiEdit {
        selection: Some(Selection {
            anchor: 2,
            focus: 5,
        }),
        preedit: Some(2.."ab中文".len()),
    });
    cases.push(clause);
    let mut hidden = cjk.clone();
    hidden.edit = Some(UiEdit {
        selection: None,
        preedit: Some(2.."ab中文".len()),
    });
    cases.push(hidden);
    for role in [UiRole::Label, UiRole::Button] {
        let mut other = latin.clone();
        other.role = role;
        other.edit = None;
        cases.push(other);
    }
    for case in &cases {
        assert_eq!(ui_caret(case), None, "{:?} {:?}", case.role, case.edit);
    }

    // A caret past the visible box is reported, but as not visible: text
    // overflowing a narrow field, and an ancestor clip short of the caret.
    let mut f = Fixture::new();
    let text = "Ada Lovelace, Countess";
    let mut long = f.widget(4, UiRole::TextInput, [10., 10., 70., 40.], 6., text, 16.);
    let (layout, glyphs) = f.shape(text, 16., 48., Wrap::None);
    long.layout = layout;
    long.glyphs = glyphs;
    long.focused = true;
    let past = ui_caret(&long).unwrap();
    assert!(past.rect[2] > long.visible[2] && !past.visible);
    long.edit = Some(UiEdit {
        selection: Some(Selection::caret(0)),
        preedit: None,
    });
    assert!(ui_caret(&long).unwrap().visible);
    let mut clipped = latin.clone();
    let shown = ui_caret(&clipped).unwrap();
    clipped.visible[2] = shown.rect[0];
    assert_eq!(ui_caret(&clipped).unwrap().rect, shown.rect);
    assert!(!ui_caret(&clipped).unwrap().visible);
}

#[test]
fn glyph_quads_use_exact_shaped_positions_and_keep_overflowing_rtl_glyphs() {
    let mut f = Fixture::new();
    // An RTL-base unbreakable token wider than its 40 px box starts at negative x.
    let token = "سلامعليكم";
    let mut label = f.widget(1, UiRole::Label, [20., 20., 60., 60.], 0., token, 20.);
    let (layout, glyphs) = f.shape(token, 20., 40., Wrap::Word);
    label.halos = Some(f.halos(&layout));
    label.layout = layout;
    label.glyphs = glyphs;
    assert!(
        label.layout.glyphs().iter().any(|g| g.position[0] < 0.),
        "{:?}",
        label.layout.glyphs()
    );
    let input = f.finish([100., 100.], vec![label.clone()]);
    let plan = plan(&input).unwrap();
    // The label's halo pass precedes its glyphs; check the glyph pass.
    assert_eq!(plan.quads.len(), 2 * plan.glyphs);
    assert!(
        plan.quads[..plan.glyphs]
            .iter()
            .all(|q| q.kind == KIND_GLYPH && q.color == theme::LABEL_HALO)
    );
    let mut quads = plan.quads[plan.glyphs..].iter();
    for (glyph, handle) in label.layout.glyphs().iter().zip(label.glyphs.iter()) {
        let entry = input.atlas.get(handle).unwrap();
        let Some([x, y, w, h]) = entry.rect else {
            continue;
        };
        let q = quads.next().unwrap();
        assert_eq!(q.kind, KIND_GLYPH);
        assert_eq!(
            q.pen,
            [
                label.origin[0] + glyph.position[0],
                label.origin[1] + glyph.position[1]
            ]
        );
        let [ox, oy] = entry.origin.map(|v| v as f32);
        let [x, y, w, h] = [x, y, w, h].map(|v| v as f32);
        assert_eq!(q.rect, [ox - 1., oy - 1., ox + w + 1., oy + h + 1.]);
        assert_eq!(q.extra, [x - 1., y - 1., x + w + 1., y + h + 1.]);
        assert_eq!(q.clip, label.clip);
    }
    assert!(quads.next().is_none());
    assert!(plan.extent[0] > 0 && plan.extent[1] > 0);
}

#[test]
fn label_halos_follow_font_size_padding_ancestor_clips_and_state() {
    // One sixteenth of the em, within 1 and 2.5 logical pixels.
    for (size, radius) in [
        (8., 1.),
        (12., 1.),
        (16., 1.),
        (17., 1.0625),
        (22., 1.375),
        (32., 2.),
        (48., 2.5),
    ] {
        assert_eq!(
            theme::LABEL_HALO_RULE.radius(size).unwrap(),
            radius,
            "{size}"
        );
    }
    let mut f = Fixture::new();
    // 32 px text: a 2 px radius, so a 2 texel margin and a halo clip 3 px past
    // the text clip. 4 px of padding keeps the whole halo.
    let roomy = f.widget(1, UiRole::Label, [10., 10., 210., 60.], 4., "Halo", 32.);
    // 1 px of padding: the halo stops at the visible border box.
    let tight = f.widget(2, UiRole::Label, [10., 70., 210., 120.], 1., "Halo", 32.);
    // An ancestor clip narrower than the border box limits the halo too.
    let mut nested = f.widget(3, UiRole::Label, [10., 130., 210., 180.], 4., "Halo", 32.);
    nested.visible = [40., 130., 120., 180.];
    nested.clip = [40., 134., 120., 176.];
    // Parked in a zero-size container: nothing is visible, so no halo either.
    let mut parked = f.widget(4, UiRole::Label, [10., 190., 210., 240.], 4., "Halo", 32.);
    parked.visible = [0., 0., 0., 0.];
    parked.clip = [0., 0., 0., 0.];
    let mut disabled = f.widget(5, UiRole::Label, [10., 250., 210., 300.], 4., "Halo", 32.);
    disabled.disabled = true;
    let button = f.widget(6, UiRole::Button, [220., 10., 320., 60.], 6., "Go", 16.);
    let field = f.widget(7, UiRole::TextInput, [220., 70., 320., 120.], 6., "ab", 16.);
    assert!(button.halos.is_none() && field.halos.is_none());
    let labels = [&roomy, &tight, &nested, &parked, &disabled].map(|w| (*w).clone());
    let input = f.finish(
        [400., 320.],
        labels.iter().cloned().chain([button, field]).collect(),
    );
    let plan = plan(&input).unwrap();
    let inked = roomy
        .glyphs
        .iter()
        .filter(|h| input.atlas.get(h).unwrap().rect.is_some())
        .count();
    assert!(inked >= 4);
    // Only labels draw halos: one per inked glyph, before that label's glyphs.
    let halos: Vec<_> = plan
        .quads
        .iter()
        .filter(|q| q.color[..3] == theme::LABEL_HALO[..3])
        .collect();
    assert_eq!(halos.len(), labels.len() * inked);
    assert!(halos.iter().all(|q| q.kind == KIND_GLYPH));
    let grown = |c: UiRect| [c[0] - 3., c[1] - 3., c[2] + 3., c[3] + 3.];
    let expected = [
        grown(roomy.clip),
        tight.visible,
        [40., 131., 120., 179.],
        [0., 0., 0., 0.],
        grown(disabled.clip),
    ];
    for ((label, chunk), clip) in labels.iter().zip(halos.chunks(inked)).zip(expected) {
        assert_eq!(halo_clip(label, 3.), clip);
        for q in chunk {
            assert_eq!(q.clip, clip);
            // Never past the visible border box.
            let v = label.visible;
            assert!(clip[0] >= v[0] && clip[1] >= v[1] && clip[2] <= v[2] && clip[3] <= v[3]);
        }
    }
    assert!(roomy.visible[0] < expected[0][0] && expected[0][2] < roomy.visible[2]);
    let dim = f32::from(theme::TEXT_DISABLED_ALPHA) / 255.;
    assert!(
        halos[..4 * inked]
            .iter()
            .all(|q| q.color == theme::LABEL_HALO)
    );
    assert!(
        halos[4 * inked..]
            .iter()
            .all(|q| q.color == theme::alpha(theme::LABEL_HALO, dim))
    );
}

/// The fixture's prepared halos, as the UI owner prepares them, hold what the
/// contract documents, recomputed here independently in f64 over every glyph
/// texel: the largest `clamp(r + c - d, 0, 1)` for coverage `c` at centre
/// distance `d`, with `r` from the theme rule, inside a rectangle `ceil(r)`
/// texels larger on every side and nothing beyond it.
#[test]
fn prepared_label_halos_are_the_documented_dilation_of_their_glyphs() {
    let mut f = Fixture::new();
    let [stride, _] = f.atlas.size();
    let texel = |atlas: &GlyphAtlas, x: u32, y: u32| atlas.pixels()[(y * stride + x) as usize];
    for size in [12., 17., 22., 40.] {
        let label = f.widget(1, UiRole::Label, [0., 0., 400., 80.], 0., "Ag中س", size);
        let r = f64::from((size / 16.).clamp(1., 2.5));
        let margin = r.ceil() as u32;
        let halos = label.halos.as_ref().unwrap();
        assert_eq!(halos.len(), label.glyphs.len());
        let mut checked = 0;
        for (glyph, halo) in label.glyphs.iter().zip(halos.iter()) {
            let (g, h) = (f.atlas.get(glyph).unwrap(), f.atlas.get(halo).unwrap());
            let (Some([gx, gy, gw, gh]), Some([hx, hy, hw, hh])) = (g.rect, h.rect) else {
                panic!("{size}: every glyph here has ink");
            };
            assert_eq!([hw, hh], [gw + 2 * margin, gh + 2 * margin], "{size}");
            let m = margin as i32;
            assert_eq!(h.origin, [g.origin[0] - m, g.origin[1] - m], "{size}");
            let covered: Vec<(f64, f64, f64)> = (0..gh)
                .flat_map(|y| (0..gw).map(move |x| (x, y)))
                .filter_map(|(x, y)| {
                    let c = texel(&f.atlas, gx + x, gy + y);
                    (c > 0).then_some((f64::from(x), f64::from(y), f64::from(c) / 255.))
                })
                .collect();
            let expected = |x: i64, y: i64| {
                // Halo texel (x, y), in glyph texel coordinates.
                let (cx, cy) = (
                    (x - i64::from(margin)) as f64,
                    (y - i64::from(margin)) as f64,
                );
                covered
                    .iter()
                    .map(|(u, v, c)| (r + c - (u - cx).hypot(v - cy)).clamp(0., 1.))
                    .fold(0., f64::max)
            };
            for y in -1..=i64::from(hh) {
                for x in -1..=i64::from(hw) {
                    let want = expected(x, y) * 255.;
                    let inside = (0..i64::from(hw)).contains(&x) && (0..i64::from(hh)).contains(&y);
                    if !inside {
                        assert!(want < 0.5, "{size}: halo ink outside its margin");
                        continue;
                    }
                    let got = texel(&f.atlas, hx + x as u32, hy + y as u32);
                    assert!(
                        (f64::from(got) - want).abs() <= 1.,
                        "{size}: {got} vs {want}"
                    );
                    checked += 1;
                }
            }
        }
        assert!(checked > 100, "{size}");
    }
}

#[test]
fn plan_refuses_foreign_reset_and_inconsistent_input_before_building() {
    let base = || {
        let mut f = Fixture::new();
        let w = f.widget(1, UiRole::TextInput, [0., 0., 100., 30.], 4., "ab", 16.);
        (f, w)
    };
    let refuse = |input: Arc<UiInput>, want: &str| {
        let error = plan(&input).err().expect(want);
        assert!(
            format!("{error:?}").contains(want),
            "{error:?} does not mention {want}"
        );
    };
    // Handle from another atlas of identical content.
    let (f, w) = base();
    let (other, foreign) = {
        let mut g = Fixture::new();
        g.shape("ab", 16., 92., Wrap::Word)
    };
    let mut bad = w.clone();
    bad.layout = other;
    bad.glyphs = foreign;
    refuse(f.finish([200., 100.], vec![bad]), "UiGlyph");
    // Handle from before an atlas reset.
    let (mut f, w) = base();
    f.atlas.reset().unwrap();
    refuse(f.finish([200., 100.], vec![w]), "UiGlyph");
    let (f, mut w) = base();
    w.glyphs = w.glyphs[..1].to_vec().into();
    refuse(f.finish([200., 100.], vec![w]), "disagree");
    for viewport in [[0., 10.], [f32::NAN, 10.], [70000., 10.]] {
        let (f, w) = base();
        refuse(f.finish(viewport, vec![w]), "viewport");
    }
    let (f, mut w) = base();
    w.clip[2] = f32::INFINITY;
    refuse(f.finish([200., 100.], vec![w]), "geometry");
    let (f, mut w) = base();
    w.edit = Some(UiEdit {
        selection: Some(Selection::caret(3)),
        preedit: None,
    });
    refuse(f.finish([200., 100.], vec![w]), "edit range");
    let (f, mut w) = base();
    w.edit = Some(UiEdit {
        selection: None,
        preedit: Some(std::ops::Range { start: 2, end: 1 }),
    });
    refuse(f.finish([200., 100.], vec![w]), "edit range");
    let (f, mut w) = base();
    w.role = UiRole::Label;
    refuse(f.finish([200., 100.], vec![w]), "non-input");
    let (f, w) = base();
    refuse(
        f.finish([200., 100.], vec![w; MAX_UI_WIDGETS + 1]),
        "widget limit",
    );
    // Labels carry exactly one prepared halo per glyph at the theme's radius;
    // no other role carries any.
    let label = || {
        let mut f = Fixture::new();
        let w = f.widget(1, UiRole::Label, [0., 0., 100., 30.], 4., "ab", 16.);
        (f, w)
    };
    let (f, w) = label();
    assert!(plan(&f.finish([200., 100.], vec![w])).is_ok());
    let (f, mut w) = label();
    w.halos = None;
    refuse(f.finish([200., 100.], vec![w]), "halo handles disagree");
    let (f, mut w) = label();
    w.halos = w.halos.map(|h| h[..1].to_vec().into());
    refuse(f.finish([200., 100.], vec![w]), "halo handles disagree");
    let (mut f, mut w) = base();
    w.halos = Some(f.halos(&w.layout));
    refuse(f.finish([200., 100.], vec![w]), "halo handles disagree");
    // A handle that is not its glyph's halo at the theme radius: the glyph's
    // own coverage, the other glyph's halo, or a halo of another radius.
    let (f, mut w) = label();
    w.halos = Some(Arc::clone(&w.glyphs));
    refuse(f.finish([200., 100.], vec![w]), "halo entry disagrees");
    let (f, mut w) = label();
    w.halos = w.halos.map(|h| h.iter().rev().cloned().collect());
    refuse(f.finish([200., 100.], vec![w]), "halo entry disagrees");
    let (mut f, mut w) = label();
    let wide: Vec<_> = (0..w.layout.glyphs().len())
        .map(|i| f.engine.cache_halo(&w.layout, i, &mut f.atlas, 2.).unwrap())
        .collect();
    w.halos = Some(wide.into());
    refuse(f.finish([200., 100.], vec![w]), "halo entry disagrees");
    // A halo entry is never drawn as a glyph.
    let (f, mut w) = label();
    w.glyphs = w.halos.clone().unwrap();
    refuse(f.finish([200., 100.], vec![w]), "halo entry disagrees");
    let (mut f, mut w) = base();
    w.glyphs = f.halos(&w.layout);
    refuse(f.finish([200., 100.], vec![w]), "glyph handle is a halo");
    // Aggregate glyph placements are counted before any quad is built.
    let mut f = Fixture::new();
    let long = "a".repeat(16 * 1024);
    let (layout, glyphs) = f.shape(&long, 4., 65536., Wrap::None);
    assert_eq!(layout.glyphs().len(), 16 * 1024);
    let mut w = f.widget(1, UiRole::Label, [0., 0., 10., 10.], 0., "", 16.);
    w.layout = layout;
    w.glyphs = glyphs;
    refuse(f.finish([200., 100.], vec![w; 5]), "glyph limit");
}

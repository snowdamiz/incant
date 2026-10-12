//! CPU half of the UI layer: validation, theme application, caret/selection
//! geometry and quads in canonical order. No GPU, font, shaping or layout work;
//! every position comes from the supplied layout and shaped glyphs.
use super::theme;
use crate::SceneError;
use incant_render_data::{UiInput, UiRect, UiRole, UiWidget};
use incant_text::{GlyphAtlas, ShapedText};
use std::ops::Range;

/// The native UI owner's own limits; the renderer refuses larger input.
pub const MAX_UI_WIDGETS: usize = 1024;
pub const MAX_UI_GLYPHS: usize = 65_536;
const MAX_FIELD_BYTES: usize = incant_text::MAX_TEXT_BYTES;
/// Every glyph plus a label shadow, at most one selection and one composition
/// span per byte of one focused field, and six surface/caret/ring quads per widget.
pub const MAX_UI_QUADS: usize = 2 * MAX_UI_GLYPHS + 2 * MAX_FIELD_BYTES + 6 * MAX_UI_WIDGETS;
const MAX_EXTENT: f32 = 65_536.;
/// Logical coordinates may extend past the viewport (overflow, nested offsets)
/// but must stay far inside f32 integer precision after physical scaling.
const MAX_COORDINATE: f32 = 4_194_304.;

pub(crate) const KIND_FILL: u32 = 0;
pub(crate) const KIND_STROKE: u32 = 1;
pub(crate) const KIND_GLYPH: u32 = 2;

/// One instance. Fill/stroke `rect` is logical `[l, t, r, b]`; glyph `rect` is
/// the padded coverage rectangle in atlas texels relative to the pen.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct UiQuad {
    pub rect: [f32; 4],
    /// Logical `[l, t, r, b]`; the quad is intersected with it.
    pub clip: [f32; 4],
    /// Glyph: padded atlas texels `[u0, v0, u1, v1]`. Fill/stroke: logical
    /// corner radius and stroke width.
    pub extra: [f32; 4],
    /// Glyph pen position: content origin plus the shaped glyph position.
    pub pen: [f32; 2],
    /// sRGB-encoded, straight alpha.
    pub color: [u8; 4],
    pub kind: u32,
}
pub(crate) const QUAD_BYTES: u64 = size_of::<UiQuad>() as u64;

pub(crate) struct UiPlan {
    pub quads: Vec<UiQuad>,
    /// Smallest atlas prefix `[width, height]` holding every padded rectangle.
    pub extent: [u32; 2],
    pub glyphs: usize,
    pub missing: usize,
}

fn finite_rect(rect: UiRect) -> bool {
    rect.iter()
        .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE)
}

/// Validate and count every widget/handle before building any quad. Handles
/// must resolve in this exact atlas; a foreign or reset handle is refused.
pub(crate) fn plan(input: &UiInput) -> Result<UiPlan, SceneError> {
    if input
        .viewport
        .iter()
        .any(|v| !v.is_finite() || !(1.0..=MAX_EXTENT).contains(v))
    {
        return Err(SceneError::Input("UI viewport bounds"));
    }
    if input.widgets.len() > MAX_UI_WIDGETS {
        return Err(SceneError::Input("UI widget limit"));
    }
    let atlas = &*input.atlas;
    let [width, height] = atlas.size();
    if !(1..=2048).contains(&width)
        || !(1..=2048).contains(&height)
        || atlas.pixels().len() != width as usize * height as usize
    {
        return Err(SceneError::Input("UI atlas dimensions"));
    }
    let mut placements = 0usize;
    for widget in &input.widgets {
        if widget.layout.glyphs().len() != widget.glyphs.len() {
            return Err(SceneError::Input("UI layout and glyph handles disagree"));
        }
        placements = placements
            .checked_add(widget.glyphs.len())
            .filter(|n| *n <= MAX_UI_GLYPHS)
            .ok_or(SceneError::Input("UI glyph limit"))?;
        if ![widget.bounds, widget.visible, widget.clip]
            .into_iter()
            .all(finite_rect)
            || !widget
                .origin
                .iter()
                .all(|v| v.is_finite() && v.abs() <= MAX_COORDINATE)
        {
            return Err(SceneError::Input("UI widget geometry"));
        }
        let source = widget.layout.source();
        match (&widget.edit, widget.role) {
            (None, _) => {}
            (Some(edit), UiRole::TextInput) => {
                let valid = |b: usize| b <= source.len() && source.is_char_boundary(b);
                if edit
                    .selection
                    .is_some_and(|s| !valid(s.anchor) || !valid(s.focus))
                    || edit
                        .preedit
                        .as_ref()
                        .is_some_and(|p| p.start > p.end || !valid(p.start) || !valid(p.end))
                {
                    return Err(SceneError::Input("UI edit range outside its display text"));
                }
            }
            (Some(_), _) => return Err(SceneError::Input("UI edit state on a non-input widget")),
        }
    }
    let mut out = UiPlan {
        quads: Vec::with_capacity(placements + 4 * input.widgets.len()),
        extent: [0; 2],
        glyphs: 0,
        missing: 0,
    };
    let mut spans = Vec::new();
    for widget in &input.widgets {
        build(atlas, widget, &mut out, &mut spans)?;
        if out.quads.len() > MAX_UI_QUADS {
            return Err(SceneError::Input("UI quad limit"));
        }
    }
    Ok(out)
}

fn quad(kind: u32, rect: UiRect, clip: UiRect, extra: [f32; 2], color: [u8; 4]) -> UiQuad {
    UiQuad {
        rect,
        clip,
        extra: [extra[0], extra[1], 0., 0.],
        pen: [0.; 2],
        color,
        kind,
    }
}
fn translate(rect: [f32; 4], origin: [f32; 2]) -> UiRect {
    [
        rect[0] + origin[0],
        rect[1] + origin[1],
        rect[2] + origin[0],
        rect[3] + origin[1],
    ]
}

/// Canonical per-widget layering: surface, selection, glyphs (a label's
/// shadow first), composition underline, caret, focus ring. Decorations clip
/// to the visible border box; glyphs clip to the effective content clip,
/// using their actual bounds.
fn build(
    atlas: &GlyphAtlas,
    widget: &UiWidget,
    out: &mut UiPlan,
    spans: &mut Vec<Span>,
) -> Result<(), SceneError> {
    let disabled = widget.disabled;
    let dim = f32::from(theme::TEXT_DISABLED_ALPHA) / 255.;
    let text_color = if disabled {
        theme::alpha(theme::TEXT, dim)
    } else {
        theme::TEXT
    };
    let radius = [theme::RADIUS, 0.];
    match widget.role {
        UiRole::Label => {}
        UiRole::Button => {
            let fill = if disabled {
                theme::BUTTON_DISABLED
            } else if widget.pressed {
                theme::BUTTON_PRESSED
            } else {
                theme::BUTTON
            };
            out.quads
                .push(quad(KIND_FILL, widget.bounds, widget.visible, radius, fill));
        }
        UiRole::TextInput => {
            out.quads.push(quad(
                KIND_FILL,
                widget.bounds,
                widget.visible,
                radius,
                theme::FIELD,
            ));
            if !widget.focused {
                out.quads.push(quad(
                    KIND_STROKE,
                    widget.bounds,
                    widget.visible,
                    [theme::RADIUS, theme::BORDER_WIDTH],
                    if disabled {
                        theme::FIELD_BORDER_DISABLED
                    } else {
                        theme::FIELD_BORDER
                    },
                ));
            }
        }
    }
    let layout = &*widget.layout;
    let edit = widget.edit.as_ref().filter(|_| widget.focused && !disabled);
    // Ordinary selection highlight; inside a composition the input method's
    // converted clause is underlined instead (below).
    if let Some(edit) = edit
        && edit.preedit.is_none()
        && let Some(selection) = edit.selection
        && selection.anchor != selection.focus
    {
        spans_for(layout, selection.range(), spans);
        for span in spans.iter() {
            let line = &layout.lines()[span.line];
            out.quads.push(quad(
                KIND_FILL,
                translate(
                    [span.x0, line.top, span.x1, line.top + line.height],
                    widget.origin,
                ),
                widget.visible,
                [0.; 2],
                theme::SELECTION,
            ));
        }
    }
    if widget.role == UiRole::Label {
        let shadow = if disabled {
            theme::alpha(theme::LABEL_SHADOW, dim)
        } else {
            theme::LABEL_SHADOW
        };
        glyphs(atlas, widget, shadow, theme::LABEL_SHADOW_OFFSET, out)?;
    }
    let (drawn, missing) = glyphs(atlas, widget, text_color, [0.; 2], out)?;
    out.glyphs += drawn;
    out.missing += missing;
    if let Some(edit) = edit {
        if let Some(preedit) = &edit.preedit {
            underline(widget, preedit.clone(), 1., out, spans);
        }
        match edit.selection {
            Some(s) if s.anchor == s.focus => {
                if let Some(caret) = ui_caret(widget) {
                    out.quads.push(quad(
                        KIND_FILL,
                        caret.rect,
                        widget.visible,
                        [0.; 2],
                        theme::TEXT,
                    ));
                }
            }
            Some(s) if edit.preedit.is_some() => {
                underline(widget, s.range(), theme::CLAUSE_THICKNESS, out, spans);
            }
            _ => {}
        }
    }
    if widget.focused && !disabled && widget.role != UiRole::Label {
        out.quads.push(quad(
            KIND_STROKE,
            widget.bounds,
            widget.visible,
            [theme::RADIUS, theme::FOCUS_WIDTH],
            theme::ACCENT,
        ));
    }
    Ok(())
}

/// The text cursor a TextInput shows: the caret of a collapsed selection, or
/// the input method's cursor inside a composition.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UiCaret {
    /// Logical viewport `[left, top, right, bottom]`, the exact rectangle this
    /// layer draws before physical snapping: the caret's line box,
    /// `theme::CARET_WIDTH` wide and centred on the insertion edge.
    pub rect: UiRect,
    /// Whether the widget's visible border box (after ancestor clips) contains
    /// the whole rectangle. False when overflowing text or a clip cuts or hides
    /// the drawn caret.
    pub visible: bool,
}
/// Read-only: the caret this layer draws for `widget`, or None when it draws
/// none (not a focused, enabled TextInput; a non-collapsed selection or
/// converted clause; an input method hiding its cursor). It reads only the
/// supplied focus and edit state: the caller chooses the widget from the UI
/// owner's text focus, and nothing here decides or changes ownership.
pub fn ui_caret(widget: &UiWidget) -> Option<UiCaret> {
    if widget.role != UiRole::TextInput || !widget.focused || widget.disabled {
        return None;
    }
    let selection = widget.edit.as_ref()?.selection?;
    if selection.anchor != selection.focus {
        return None;
    }
    let caret = caret(&widget.layout, selection.focus)?;
    let half = theme::CARET_WIDTH * 0.5;
    let rect = translate(
        [caret.x - half, caret.top, caret.x + half, caret.bottom],
        widget.origin,
    );
    let [left, top, right, bottom] = widget.visible;
    Some(UiCaret {
        rect,
        visible: rect[0] >= left && rect[1] >= top && rect[2] <= right && rect[3] <= bottom,
    })
}

/// One quad per glyph with coverage, at `offset` logical pixels from its pen.
/// Returns drawn glyphs and `.notdef` placements; refuses foreign handles.
fn glyphs(
    atlas: &GlyphAtlas,
    widget: &UiWidget,
    color: [u8; 4],
    offset: [f32; 2],
    out: &mut UiPlan,
) -> Result<(usize, usize), SceneError> {
    let (mut drawn, mut missing) = (0, 0);
    let [width, height] = atlas.size();
    for (glyph, handle) in widget.layout.glyphs().iter().zip(widget.glyphs.iter()) {
        let entry = atlas.get(handle).ok_or(SceneError::UiGlyph(widget.id))?;
        missing += usize::from(entry.missing);
        let Some(rect) = entry.rect else {
            continue;
        };
        if !glyph.position.iter().all(|v| v.is_finite()) {
            return Err(SceneError::Input("UI glyph position"));
        }
        // The atlas surrounds each inner rectangle with one zero texel.
        if rect[0] == 0 || rect[1] == 0 || rect[0] + rect[2] >= width || rect[1] + rect[3] >= height
        {
            return Err(SceneError::Input("UI atlas entry bounds"));
        }
        let [x, y, w, h] = rect.map(|v| v as f32);
        let [ox, oy] = entry.origin.map(|v| v as f32);
        out.quads.push(UiQuad {
            rect: [ox - 1., oy - 1., ox + w + 1., oy + h + 1.],
            clip: widget.clip,
            extra: [x - 1., y - 1., x + w + 1., y + h + 1.],
            pen: [
                widget.origin[0] + glyph.position[0] + offset[0],
                widget.origin[1] + glyph.position[1] + offset[1],
            ],
            color,
            kind: KIND_GLYPH,
        });
        out.extent[0] = out.extent[0].max(rect[0] + rect[2] + 1);
        out.extent[1] = out.extent[1].max(rect[1] + rect[3] + 1);
        drawn += 1;
    }
    Ok((drawn, missing))
}

fn underline(
    widget: &UiWidget,
    range: Range<usize>,
    weight: f32,
    out: &mut UiPlan,
    spans: &mut Vec<Span>,
) {
    let layout = &*widget.layout;
    spans_for(layout, range, spans);
    for span in spans.iter() {
        let size = span.font_size;
        let thickness = (theme::UNDERLINE_THICKNESS * size).max(1.) * weight;
        let top = layout.lines()[span.line].baseline + theme::UNDERLINE_OFFSET * size;
        out.quads.push(quad(
            KIND_FILL,
            translate([span.x0, top, span.x1, top + thickness], widget.origin),
            widget.visible,
            [0.; 2],
            [
                theme::TEXT[0],
                theme::TEXT[1],
                theme::TEXT[2],
                theme::PREEDIT_ALPHA,
            ],
        ));
    }
}

/// Adjacent glyphs of one line sharing one source cluster (base plus marks,
/// or one ligature), with the visual extent of their advance boxes.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Cluster {
    pub line: usize,
    pub source: Range<usize>,
    pub left: f32,
    pub right: f32,
    pub rtl: bool,
    pub font_size: f32,
}
pub(crate) fn clusters(layout: &ShapedText) -> impl Iterator<Item = Cluster> + '_ {
    layout
        .lines()
        .iter()
        .enumerate()
        .flat_map(move |(index, line)| {
            let glyphs = &layout.glyphs()[line.glyphs.clone()];
            let mut at = 0;
            std::iter::from_fn(move || {
                let first = glyphs.get(at)?;
                let end = glyphs[at..]
                    .iter()
                    .position(|g| g.source != first.source)
                    .map_or(glyphs.len(), |n| at + n);
                let members = &glyphs[at..end];
                at = end;
                // Zero-advance marks are positioned inside their base's box.
                let advancing: Vec<_> = members.iter().filter(|g| g.advance > 0.).collect();
                let (left, right) = if advancing.is_empty() {
                    (first.position[0], first.position[0])
                } else {
                    advancing
                        .iter()
                        .fold((f32::INFINITY, f32::NEG_INFINITY), |(l, r), g| {
                            (l.min(g.position[0]), r.max(g.position[0] + g.advance))
                        })
                };
                Some(Cluster {
                    line: index,
                    source: first.source.clone(),
                    left,
                    right,
                    rtl: first.bidi_level % 2 == 1,
                    font_size: first.font_size,
                })
            })
        })
}

/// Paragraph index of `byte`, using cosmic-text's LineIter endings: CRLF and
/// LFCR are one break, a lone CR or LF is one break.
fn paragraph(source: &str, byte: usize) -> usize {
    let bytes = source.as_bytes();
    let (mut i, mut count) = (0, 0);
    while i < byte.min(bytes.len()) {
        match bytes[i] {
            b'\r' | b'\n' => {
                let pair = bytes.get(i + 1).is_some_and(|&next| {
                    matches!((bytes[i], next), (b'\r', b'\n') | (b'\n', b'\r'))
                });
                i += if pair { 2 } else { 1 };
                count += 1;
            }
            _ => i += 1,
        }
    }
    count
}

fn fraction(source: &str, cluster: &Range<usize>, byte: usize) -> f32 {
    let total = source[cluster.clone()].chars().count().max(1) as f32;
    source[cluster.start..byte].chars().count() as f32 / total
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Caret {
    pub line: usize,
    pub x: f32,
    pub top: f32,
    pub bottom: f32,
}
/// Caret before `byte` with downstream affinity: the leading edge of the
/// cluster starting there, a proportional point inside a multi-character
/// cluster, else the trailing edge of the cluster ending there. Edges follow
/// each glyph's own bidi level. Empty paragraphs place it at x = 0.
pub(crate) fn caret(layout: &ShapedText, byte: usize) -> Option<Caret> {
    let source = layout.source();
    if byte > source.len() || !source.is_char_boundary(byte) {
        return None;
    }
    let at = |line: usize, x: f32| {
        let line_box = &layout.lines()[line];
        Caret {
            line,
            x,
            top: line_box.top,
            bottom: line_box.top + line_box.height,
        }
    };
    let leading = |c: &Cluster| if c.rtl { c.right } else { c.left };
    let trailing = |c: &Cluster| if c.rtl { c.left } else { c.right };
    if let Some(c) = clusters(layout).find(|c| c.source.start == byte) {
        return Some(at(c.line, leading(&c)));
    }
    if let Some(c) = clusters(layout).find(|c| c.source.start < byte && byte < c.source.end) {
        let t = fraction(source, &c.source, byte);
        return Some(at(c.line, leading(&c) + (trailing(&c) - leading(&c)) * t));
    }
    if let Some(c) = clusters(layout).find(|c| c.source.end == byte) {
        return Some(at(c.line, trailing(&c)));
    }
    // No glyph touches `byte`: an empty paragraph, or blanks trimmed at a wrap.
    let paragraph = paragraph(source, byte);
    let lines = || {
        layout
            .lines()
            .iter()
            .enumerate()
            .filter(|(_, line)| line.paragraph == paragraph)
    };
    let previous = clusters(layout)
        .filter(|c| layout.lines()[c.line].paragraph == paragraph && c.source.end <= byte)
        .max_by_key(|c| c.source.end);
    match previous {
        Some(c) => Some(at(c.line, trailing(&c))),
        None => lines().next().map(|(line, _)| at(line, 0.)),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Span {
    pub line: usize,
    pub x0: f32,
    pub x1: f32,
    pub font_size: f32,
}
/// Visual extents of `range` per line, merged where adjacent. A logical range
/// can map to several disjoint segments on one line in mixed-direction text.
pub(crate) fn spans_for(layout: &ShapedText, range: Range<usize>, out: &mut Vec<Span>) {
    out.clear();
    if range.is_empty() {
        return;
    }
    let source = layout.source();
    for c in clusters(layout) {
        let start = range.start.max(c.source.start);
        let end = range.end.min(c.source.end);
        if start >= end {
            continue;
        }
        let (a, b) = (
            fraction(source, &c.source, start),
            fraction(source, &c.source, end),
        );
        let width = c.right - c.left;
        let (x0, x1) = if c.rtl {
            (c.right - width * b, c.right - width * a)
        } else {
            (c.left + width * a, c.left + width * b)
        };
        out.push(Span {
            line: c.line,
            x0,
            x1,
            font_size: c.font_size,
        });
    }
    out.sort_by(|a, b| a.line.cmp(&b.line).then(a.x0.total_cmp(&b.x0)));
    out.dedup_by(|next, kept| {
        if next.line == kept.line && next.x0 <= kept.x1 + 0.01 {
            kept.x1 = kept.x1.max(next.x1);
            kept.font_size = kept.font_size.max(next.font_size);
            true
        } else {
            false
        }
    });
    out.retain(|s| s.x1 > s.x0);
}

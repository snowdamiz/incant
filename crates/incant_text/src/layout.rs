use crate::{
    FontSource, TextError,
    fonts::{Fonts, primary_family},
};
use cosmic_text::{Attrs, AttrsList, Buffer, BufferLine, LineEnding, LineIter, Metrics, Shaping};
use std::{ops::Range, sync::Arc};
use ulid::Ulid;

pub const MAX_TEXT_BYTES: usize = 16 * 1024;
const MAX_GLYPHS: usize = 64 * 1024;
const MAX_LINES: usize = 16 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Wrap {
    None,
    Word,
    WordOrGlyph,
}

/// Logical pixel metrics supplied by the widget layout. Direction is Unicode bidi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub font_size: f32,
    pub line_height: f32,
    pub width: f32,
    pub wrap: Wrap,
}

impl TextStyle {
    fn validate(self) -> Result<(), TextError> {
        if !self.font_size.is_finite()
            || !(1.0..=512.0).contains(&self.font_size)
            || !self.line_height.is_finite()
            || !(1.0..=4096.0).contains(&self.line_height)
            || !self.width.is_finite()
            || !(1.0..=65536.0).contains(&self.width)
        {
            return Err(TextError::Invalid(
                "metrics must be finite and within supported bounds",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Glyph {
    pub font: Ulid,
    pub id: u16,
    /// Absolute UTF-8 byte cluster in ShapedText::source, not a scalar index.
    pub source: Range<usize>,
    /// Logical glyph origin, including shaping offsets and the line baseline.
    pub position: [f32; 2],
    pub advance: f32,
    pub font_size: f32,
    pub bidi_level: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub paragraph: usize,
    pub glyphs: Range<usize>,
    pub top: f32,
    pub baseline: f32,
    pub height: f32,
    pub width: f32,
    pub rtl: bool,
}

/// Immutable shaping result; borrowed observations do not clone text or glyphs.
#[derive(Debug)]
pub struct ShapedText {
    source: Arc<str>,
    glyphs: Vec<Glyph>,
    lines: Vec<Line>,
    missing: Vec<Range<usize>>,
    size: [f32; 2],
}

impl ShapedText {
    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn glyphs(&self) -> &[Glyph] {
        &self.glyphs
    }
    pub fn lines(&self) -> &[Line] {
        &self.lines
    }
    /// Clusters containing .notdef glyphs. Missing coverage is never hidden.
    pub fn missing_clusters(&self) -> &[Range<usize>] {
        &self.missing
    }
    pub fn size(&self) -> [f32; 2] {
        self.size
    }
}

/// One explicit font chain and one retained text buffer. Create at UI load time.
/// Unchanged text/metrics returns the same Arc; shaping only happens on updates.
/// No runtime world, project document, renderer, filesystem or provider dependency.
pub struct TextEngine {
    fonts: Fonts,
    buffer: Buffer,
    cached: Option<(TextStyle, Arc<ShapedText>)>,
}

impl TextEngine {
    pub fn new(fonts: Vec<FontSource>) -> Result<Self, TextError> {
        let fonts = Fonts::new(fonts)?;
        Ok(Self {
            fonts,
            buffer: Buffer::new_empty(Metrics::new(16.0, 20.0)),
            cached: None,
        })
    }

    pub fn shape(&mut self, text: &str, style: TextStyle) -> Result<Arc<ShapedText>, TextError> {
        style.validate()?;
        if text.len() > MAX_TEXT_BYTES {
            return Err(TextError::Limit("text bytes"));
        }
        if let Some((old_style, old)) = &self.cached
            && *old_style == style
            && old.source() == text
        {
            return Ok(Arc::clone(old));
        }
        // Input is bounded before the shaper can allocate. No visible-height
        // clipping: the returned layout describes every line, including missing
        // coverage outside the widget viewport. The renderer will clip later.
        self.buffer.set_metrics_and_size(
            &mut self.fonts.system,
            Metrics::new(style.font_size, style.line_height),
            Some(style.width),
            None,
        );
        self.buffer.set_wrap(
            &mut self.fonts.system,
            match style.wrap {
                Wrap::None => cosmic_text::Wrap::None,
                Wrap::Word => cosmic_text::Wrap::Word,
                Wrap::WordOrGlyph => cosmic_text::Wrap::WordOrGlyph,
            },
        );
        let attrs = Attrs::new().family(primary_family());
        self.buffer
            .set_text(&mut self.fonts.system, text, &attrs, Shaping::Advanced);
        let mut paragraphs: Vec<_> = LineIter::new(text).map(|(range, _)| range).collect();
        if paragraphs.is_empty() {
            paragraphs.push(0..0);
        }
        // cosmic-text's LineIter omits the final empty paragraph. A game text
        // field must still measure the caret's next line after a trailing break.
        if text.ends_with(['\r', '\n']) {
            paragraphs.push(text.len()..text.len());
            self.buffer.lines.push(BufferLine::new(
                "",
                LineEnding::None,
                AttrsList::new(&attrs),
                Shaping::Advanced,
            ));
        }
        self.buffer
            .shape_until_scroll(&mut self.fonts.system, false);
        let mut out = ShapedText {
            source: Arc::from(text),
            glyphs: Vec::new(),
            lines: Vec::new(),
            missing: Vec::new(),
            size: [0.0, 0.0],
        };
        for run in self.buffer.layout_runs() {
            if out.lines.len() >= MAX_LINES || run.glyphs.len() > MAX_GLYPHS - out.glyphs.len() {
                return Err(TextError::Limit("shaped output"));
            }
            let paragraph = paragraphs.get(run.line_i).ok_or(TextError::Geometry)?;
            if ![run.line_top, run.line_y, run.line_height, run.line_w]
                .iter()
                .all(|n| n.is_finite())
            {
                return Err(TextError::Geometry);
            }
            let first = out.glyphs.len();
            for glyph in run.glyphs {
                let source = paragraph.start + glyph.start..paragraph.start + glyph.end;
                if source.start > source.end
                    || source.end > paragraph.end
                    || !text.is_char_boundary(source.start)
                    || !text.is_char_boundary(source.end)
                {
                    return Err(TextError::Geometry);
                }
                let position = [
                    glyph.x + glyph.font_size * glyph.x_offset,
                    run.line_y + glyph.y - glyph.font_size * glyph.y_offset,
                ];
                if !position
                    .iter()
                    .chain([&glyph.w, &glyph.font_size])
                    .all(|n| n.is_finite())
                {
                    return Err(TextError::Geometry);
                }
                out.glyphs.push(Glyph {
                    font: *self
                        .fonts
                        .ids
                        .get(&glyph.font_id)
                        .ok_or(TextError::Geometry)?,
                    id: glyph.glyph_id,
                    source: source.clone(),
                    position,
                    advance: glyph.w,
                    font_size: glyph.font_size,
                    bidi_level: glyph.level.number(),
                });
                if glyph.glyph_id == 0 {
                    out.missing.push(source);
                }
            }
            out.size[0] = out.size[0].max(run.line_w);
            out.size[1] = out.size[1].max(run.line_top + run.line_height);
            out.lines.push(Line {
                paragraph: run.line_i,
                glyphs: first..out.glyphs.len(),
                top: run.line_top,
                baseline: run.line_y,
                height: run.line_height,
                width: run.line_w,
                rtl: run.rtl,
            });
        }
        out.missing.sort_by_key(|range| (range.start, range.end));
        out.missing.dedup();
        let out = Arc::new(out);
        self.cached = Some((style, Arc::clone(&out)));
        Ok(out)
    }
}

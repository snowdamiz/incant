//! v13 UI suffix; fixed style records and bounded UTF-8/font chains. No serde
//! or document parsing in the runtime loader.
use crate::{CookedUi, SceneError, StableId, binary::Reader};
use incant_types::{
    TextWrap, UiAlign, UiDirection, UiJustify, UiKind, UiLength, UiNode, UiStyle, UiText,
};
use std::sync::Arc;
const FIXED: usize = 91;
pub(crate) fn size(nodes: &[CookedUi]) -> usize {
    4 + nodes
        .iter()
        .map(|n| {
            FIXED
                + n.node
                    .kind
                    .text()
                    .map_or(0, |t| 14 + t.content.len() + 16 * t.fonts.len())
        })
        .sum::<usize>()
}
pub(crate) fn write(out: &mut Vec<u8>, nodes: &[CookedUi]) {
    out.extend((nodes.len() as u32).to_le_bytes());
    for n in nodes {
        out.extend(n.entity.0);
        out.push(match n.node.kind {
            UiKind::Root => 0,
            UiKind::Container => 1,
            UiKind::Label { .. } => 2,
            UiKind::Button { .. } => 3,
            UiKind::TextInput { .. } => 4,
        });
        out.push(u8::from(n.node.disabled));
        out.extend(n.node.order.to_le_bytes());
        let s = &n.node.style;
        for v in s.size {
            length(out, v);
        }
        for v in s.min_size {
            out.extend(v.to_le_bytes());
        }
        for v in s.max_size {
            out.push(u8::from(v.is_some()));
            out.extend(v.unwrap_or(0.0).to_le_bytes());
        }
        for v in s.padding.into_iter().chain(s.gap) {
            out.extend(v.to_le_bytes());
        }
        out.push(match s.direction {
            UiDirection::Row => 0,
            UiDirection::RowReverse => 1,
            UiDirection::Column => 2,
            UiDirection::ColumnReverse => 3,
        });
        out.push(u8::from(s.wrap));
        out.push(match s.align {
            UiAlign::Start => 0,
            UiAlign::End => 1,
            UiAlign::Center => 2,
            UiAlign::Stretch => 3,
        });
        out.push(match s.justify {
            UiJustify::Start => 0,
            UiJustify::End => 1,
            UiJustify::Center => 2,
            UiJustify::SpaceBetween => 3,
            UiJustify::SpaceAround => 4,
            UiJustify::SpaceEvenly => 5,
        });
        out.extend(s.grow.to_le_bytes());
        out.extend(s.shrink.to_le_bytes());
        length(out, s.basis);
        if let Some(t) = n.node.kind.text() {
            out.extend((t.content.len() as u32).to_le_bytes());
            out.extend(t.content.as_bytes());
            out.push(t.fonts.len() as u8);
            for f in &t.fonts {
                out.extend(f.0);
            }
            out.extend(t.font_size.to_le_bytes());
            out.extend(t.line_height.to_le_bytes());
            out.push(match t.wrap {
                TextWrap::None => 0,
                TextWrap::Word => 1,
                TextWrap::WordOrGlyph => 2,
            });
        }
    }
}
fn length(out: &mut Vec<u8>, n: UiLength) {
    let (tag, value) = match n {
        UiLength::Auto => (0, 0.0),
        UiLength::Pixels(v) => (1, v),
        UiLength::Fraction(v) => (2, v),
    };
    out.push(tag);
    out.extend(value.to_le_bytes());
}
fn float(r: &mut Reader<'_>) -> Result<f32, SceneError> {
    Ok(f32::from_le_bytes(r.array()?))
}
fn boolean(r: &mut Reader<'_>) -> Result<bool, SceneError> {
    match r.byte()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(SceneError::Invalid("UI boolean")),
    }
}
fn read_length(r: &mut Reader<'_>) -> Result<UiLength, SceneError> {
    let tag = r.byte()?;
    let v = float(r)?;
    match tag {
        0 if v.to_bits() == 0 => Ok(UiLength::Auto),
        1 => Ok(UiLength::Pixels(v)),
        2 => Ok(UiLength::Fraction(v)),
        _ => Err(SceneError::Invalid("UI length tag/value")),
    }
}
pub(crate) fn read(r: &mut Reader<'_>, entities: usize) -> Result<Vec<CookedUi>, SceneError> {
    let n = r.u32()? as usize;
    if n == 0 || n > incant_types::MAX_UI_NODES || n > entities || n > r.remaining_len() / FIXED {
        return Err(SceneError::Invalid("UI node count"));
    }
    let mut result: Vec<CookedUi> = Vec::with_capacity(n);
    let mut total = 0usize;
    for _ in 0..n {
        let entity = StableId(r.array()?);
        if result.last().is_some_and(|n| n.entity >= entity) {
            return Err(SceneError::Invalid("UI node order"));
        }
        let role = r.byte()?;
        if role > 4 {
            return Err(SceneError::Invalid("UI role"));
        }
        let disabled = boolean(r)?;
        let order = i32::from_le_bytes(r.array()?);
        let size = [read_length(r)?, read_length(r)?];
        let min_size = [float(r)?, float(r)?];
        let mut max_size = [None; 2];
        for v in &mut max_size {
            let present = boolean(r)?;
            let n = float(r)?;
            if present {
                *v = Some(n);
            } else if n.to_bits() != 0 {
                return Err(SceneError::Invalid("UI absent max size"));
            }
        }
        let padding = [float(r)?, float(r)?, float(r)?, float(r)?];
        let gap = [float(r)?, float(r)?];
        let direction = match r.byte()? {
            0 => UiDirection::Row,
            1 => UiDirection::RowReverse,
            2 => UiDirection::Column,
            3 => UiDirection::ColumnReverse,
            _ => return Err(SceneError::Invalid("UI direction")),
        };
        let wrap = boolean(r)?;
        let align = match r.byte()? {
            0 => UiAlign::Start,
            1 => UiAlign::End,
            2 => UiAlign::Center,
            3 => UiAlign::Stretch,
            _ => return Err(SceneError::Invalid("UI align")),
        };
        let justify = match r.byte()? {
            0 => UiJustify::Start,
            1 => UiJustify::End,
            2 => UiJustify::Center,
            3 => UiJustify::SpaceBetween,
            4 => UiJustify::SpaceAround,
            5 => UiJustify::SpaceEvenly,
            _ => return Err(SceneError::Invalid("UI justify")),
        };
        let grow = float(r)?;
        let shrink = float(r)?;
        let basis = read_length(r)?;
        let style = UiStyle {
            size,
            min_size,
            max_size,
            padding,
            gap,
            direction,
            wrap,
            align,
            justify,
            grow,
            shrink,
            basis,
        };
        style.validate().map_err(SceneError::Invalid)?;
        let kind = match role {
            0 => UiKind::Root,
            1 => UiKind::Container,
            _ => {
                let len = r.u32()? as usize;
                total = total
                    .checked_add(len)
                    .filter(|n| *n <= incant_types::MAX_SCENE_TEXT_BYTES)
                    .ok_or(SceneError::Invalid("UI text aggregate"))?;
                if len > incant_types::MAX_TEXT_BYTES {
                    return Err(SceneError::Invalid("UI text length"));
                }
                let content = std::str::from_utf8(r.bytes(len)?)
                    .map_err(|_| SceneError::Invalid("UI UTF-8"))?;
                let count = r.byte()? as usize;
                if count == 0
                    || count > incant_types::MAX_TEXT_FONTS
                    || count > r.remaining_len() / 16
                {
                    return Err(SceneError::Invalid("UI font count"));
                }
                let mut fonts = Vec::with_capacity(count);
                for _ in 0..count {
                    fonts.push(StableId(r.array()?));
                }
                let font_size = float(r)?;
                let line_height = float(r)?;
                let wrap = match r.byte()? {
                    0 => TextWrap::None,
                    1 => TextWrap::Word,
                    2 => TextWrap::WordOrGlyph,
                    _ => return Err(SceneError::Invalid("UI text wrap")),
                };
                let text = UiText {
                    content: content.into(),
                    fonts,
                    font_size,
                    line_height,
                    wrap,
                };
                match role {
                    2 => UiKind::Label { text },
                    3 => UiKind::Button { text },
                    4 => UiKind::TextInput { text },
                    _ => unreachable!(),
                }
            }
        };
        let node = UiNode {
            kind,
            style,
            order,
            disabled,
        };
        node.validate().map_err(SceneError::Invalid)?;
        result.push(CookedUi {
            entity,
            node: Arc::new(node),
        });
    }
    Ok(result)
}

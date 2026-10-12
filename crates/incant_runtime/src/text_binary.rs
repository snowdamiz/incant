//! Scene-v12 suffix. Counts and aggregate UTF-8 bytes are checked before allocation.
use crate::{
    CookedText, FontAssetRef, NativeText, SceneError, StableId, TextLayout, TextWrap,
    binary::Reader,
};
use std::sync::Arc;
pub(crate) fn size(fonts: &[FontAssetRef], nodes: &[CookedText]) -> usize {
    8 + fonts.len() * 48
        + nodes
            .iter()
            .map(|n| 16 + 4 + n.text.content.len() + 1 + n.text.fonts.len() * 16 + 13)
            .sum::<usize>()
}
pub(crate) fn write(out: &mut Vec<u8>, fonts: &[FontAssetRef], nodes: &[CookedText]) {
    out.extend((fonts.len() as u32).to_le_bytes());
    for a in fonts {
        out.extend(a.id.0);
        out.extend(a.fingerprint);
    }
    out.extend((nodes.len() as u32).to_le_bytes());
    for n in nodes {
        out.extend(n.entity.0);
        out.extend((n.text.content.len() as u32).to_le_bytes());
        out.extend(n.text.content.as_bytes());
        out.push(n.text.fonts.len() as u8);
        for id in n.text.fonts.iter() {
            out.extend(id.0);
        }
        for f in [
            n.text.layout.font_size,
            n.text.layout.line_height,
            n.text.layout.width,
        ] {
            out.extend(f.to_le_bytes());
        }
        out.push(match n.text.layout.wrap {
            TextWrap::None => 0,
            TextWrap::Word => 1,
            TextWrap::WordOrGlyph => 2,
        });
    }
}
pub(crate) fn read(
    r: &mut Reader<'_>,
    entities: usize,
) -> Result<(Vec<FontAssetRef>, Vec<CookedText>), SceneError> {
    let n = r.u32()? as usize;
    if n > crate::MAX_TEXT_FONTS || n > r.remaining_len() / 48 {
        return Err(SceneError::Invalid("font catalog count"));
    }
    let mut fonts: Vec<FontAssetRef> = Vec::with_capacity(n);
    for _ in 0..n {
        let a = FontAssetRef {
            id: StableId(r.array()?),
            fingerprint: r.array()?,
        };
        if fonts.last().is_some_and(|p| p.id >= a.id) {
            return Err(SceneError::Invalid("font catalog order"));
        }
        fonts.push(a);
    }
    let n = r.u32()? as usize;
    if n > crate::MAX_TEXT_NODES
        || n > entities
        || n > r.remaining_len() / 50
        || (n == 0 && fonts.is_empty())
    {
        return Err(SceneError::Invalid("text node count"));
    }
    let mut nodes: Vec<CookedText> = Vec::with_capacity(n);
    let mut total = 0usize;
    for _ in 0..n {
        let entity = StableId(r.array()?);
        if nodes.last().is_some_and(|p| p.entity >= entity) {
            return Err(SceneError::Invalid("text node order"));
        }
        let len = r.u32()? as usize;
        if len > crate::MAX_TEXT_BYTES {
            return Err(SceneError::Invalid("text length"));
        }
        total = total
            .checked_add(len)
            .filter(|&n| n <= crate::MAX_SCENE_TEXT_BYTES)
            .ok_or(SceneError::Invalid("scene text bytes"))?;
        let content =
            std::str::from_utf8(r.bytes(len)?).map_err(|_| SceneError::Invalid("text UTF-8"))?;
        let count = usize::from(r.byte()?);
        if count == 0 || count > crate::MAX_TEXT_FONTS || count > r.remaining_len() / 16 {
            return Err(SceneError::Invalid("text font chain count"));
        }
        let mut chain = Vec::with_capacity(count);
        for _ in 0..count {
            let id = StableId(r.array()?);
            if chain.contains(&id) || fonts.binary_search_by_key(&id, |a| a.id).is_err() {
                return Err(SceneError::Invalid("text font chain reference"));
            }
            chain.push(id);
        }
        let font_size = f32::from_le_bytes(r.array()?);
        let line_height = f32::from_le_bytes(r.array()?);
        let width = f32::from_le_bytes(r.array()?);
        let wrap = match r.byte()? {
            0 => TextWrap::None,
            1 => TextWrap::Word,
            2 => TextWrap::WordOrGlyph,
            _ => return Err(SceneError::Invalid("text wrap tag")),
        };
        let layout = TextLayout {
            font_size,
            line_height,
            width,
            wrap,
        };
        layout.validate().map_err(SceneError::Invalid)?;
        nodes.push(CookedText {
            entity,
            text: Arc::new(NativeText {
                content: Arc::from(content),
                fonts: chain.into(),
                layout,
            }),
        });
    }
    Ok((fonts, nodes))
}

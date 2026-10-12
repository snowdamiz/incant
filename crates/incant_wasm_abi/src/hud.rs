//! Canonical copied HUD declarations and routed outcomes. Shared by the native
//! encoder's admission check and the allocation-free guest decoder.
use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Invalid;
#[derive(Clone, Copy)]
pub struct Frame<'a> {
    pub header: HudHeader,
    widgets: &'a [u8],
    texts: &'a [u8],
    events: &'a [u8],
    strings: &'a [u8],
}
impl<'a> Frame<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self, Invalid> {
        let h = HudHeader::decode(bytes).ok_or(Invalid)?;
        if &h.magic != HUD_MAGIC
            || h.reserved != 0
            || h.flags & !3 != 0
            || h.flags & 2 != 0 && h.flags & 1 == 0
            || h.flags & 1 == 0 && h.focused != [0; 16]
            || h.widgets as usize > HUD_MAX_WIDGETS
            || h.texts as usize > HUD_MAX_TEXTS
            || h.events as usize > HUD_MAX_EVENTS
            || h.discarded as usize > HUD_MAX_EVENTS
            || h.bytes as usize > HUD_MAX_DECLARATION_BYTES + HUD_MAX_EVENT_BYTES
            || h.widget_stride as usize != HudWidget::SIZE
            || h.text_stride as usize != HudText::SIZE
            || h.event_stride as usize != HudEvent::SIZE
        {
            return Err(Invalid);
        }
        // Counts are bounded above before arithmetic (including on wasm32).
        let a = HudHeader::SIZE + h.widgets as usize * HudWidget::SIZE;
        let b = a + h.texts as usize * HudText::SIZE;
        let c = b + h.events as usize * HudEvent::SIZE;
        if bytes.len() != c + h.bytes as usize {
            return Err(Invalid);
        }
        let frame = Self {
            header: h,
            widgets: &bytes[HudHeader::SIZE..a],
            texts: &bytes[a..b],
            events: &bytes[b..c],
            strings: &bytes[c..],
        };
        let mut cursor = 0;
        let mut previous = None;
        for (row, _) in frame.widgets() {
            if previous.is_some_and(|id| id >= row.entity)
                || row.disabled > 1
                || row.role > 4
                || row.role < 2 && row.len != 0
            {
                return Err(Invalid);
            }
            frame.string(row.offset, row.len, &mut cursor)?;
            previous = Some(row.entity);
        }
        previous = None;
        for (row, _) in frame.texts() {
            if previous.is_some_and(|id| id >= row.entity) {
                return Err(Invalid);
            }
            frame.string(row.offset, row.len, &mut cursor)?;
            previous = Some(row.entity);
        }
        if cursor > HUD_MAX_DECLARATION_BYTES {
            return Err(Invalid);
        }
        let declaration_bytes = cursor;
        if h.flags & 1 != 0 {
            let focus = frame.widget(h.focused).ok_or(Invalid)?;
            if !(3..=4).contains(&focus.role)
                || focus.disabled != 0
                || (h.flags & 2 != 0) != (focus.role == 4)
            {
                return Err(Invalid);
            }
        }
        for (row, _) in frame.events() {
            if row.kind > 2 || row.reserved != 0 || row.kind == 0 && row.len != 0 {
                return Err(Invalid);
            }
            let owner = frame.widget(row.entity).ok_or(Invalid)?;
            if (row.kind == 0 && owner.role != 3) || (row.kind != 0 && owner.role != 4) {
                return Err(Invalid);
            }
            frame.string(row.offset, row.len, &mut cursor)?;
        }
        if cursor != frame.strings.len() || cursor - declaration_bytes > HUD_MAX_EVENT_BYTES {
            return Err(Invalid);
        }
        Ok(frame)
    }
    fn string(&self, offset: u32, len: u32, cursor: &mut usize) -> Result<(), Invalid> {
        if offset as usize != *cursor || len as usize > HUD_MAX_TEXT_BYTES {
            return Err(Invalid);
        }
        let end = cursor.checked_add(len as usize).ok_or(Invalid)?;
        core::str::from_utf8(self.strings.get(*cursor..end).ok_or(Invalid)?)
            .map_err(|_| Invalid)?;
        *cursor = end;
        Ok(())
    }
    // During parse the offsets are not yet validated, so iterators do not index
    // unchecked ranges or assume UTF-8. After parse these fallbacks are unreachable.
    fn content(&self, offset: u32, len: u32) -> &'a str {
        (offset as usize)
            .checked_add(len as usize)
            .and_then(|end| self.strings.get(offset as usize..end))
            .and_then(|s| core::str::from_utf8(s).ok())
            .unwrap_or("")
    }
    pub fn widgets(&self) -> impl ExactSizeIterator<Item = (HudWidget, &'a str)> + 'a {
        let frame = *self;
        self.widgets
            .as_chunks::<{ HudWidget::SIZE }>()
            .0
            .iter()
            .map(move |b| {
                let r = HudWidget::decode(b).expect("fixed record");
                (r, frame.content(r.offset, r.len))
            })
    }
    pub fn texts(&self) -> impl ExactSizeIterator<Item = (HudText, &'a str)> + 'a {
        let frame = *self;
        self.texts
            .as_chunks::<{ HudText::SIZE }>()
            .0
            .iter()
            .map(move |b| {
                let r = HudText::decode(b).expect("fixed record");
                (r, frame.content(r.offset, r.len))
            })
    }
    pub fn events(&self) -> impl ExactSizeIterator<Item = (HudEvent, &'a str)> + 'a {
        let frame = *self;
        self.events
            .as_chunks::<{ HudEvent::SIZE }>()
            .0
            .iter()
            .map(move |b| {
                let r = HudEvent::decode(b).expect("fixed record");
                (r, frame.content(r.offset, r.len))
            })
    }
    fn widget(&self, id: [u8; 16]) -> Option<HudWidget> {
        let rows = self.widgets.as_chunks::<{ HudWidget::SIZE }>().0;
        let index = rows
            .binary_search_by_key(&id, |b| HudWidget::decode(b).expect("fixed row").entity)
            .ok()?;
        HudWidget::decode(&rows[index])
    }
}

/// An ordered primitive command. The payload is borrowed only for this call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Control<'a> {
    UiText(&'a str),
    UiDisabled(bool),
    Text(&'a str),
}
pub fn command(bytes: &[u8]) -> Result<([u8; 16], Control<'_>), Invalid> {
    let h = HudCommand::decode(bytes).ok_or(Invalid)?;
    if h.reserved != 0
        || h.len as usize > HUD_MAX_TEXT_BYTES
        || bytes.len() != HudCommand::SIZE + h.len as usize
    {
        return Err(Invalid);
    }
    let text = core::str::from_utf8(&bytes[HudCommand::SIZE..]).map_err(|_| Invalid)?;
    let control = match (h.kind, h.value, h.len) {
        (0, 0, _) => Control::UiText(text),
        (1, value @ 0..=1, 0) => Control::UiDisabled(value == 1),
        (2, 0, _) => Control::Text(text),
        _ => return Err(Invalid),
    };
    Ok((h.entity, control))
}
pub fn encode_command(
    entity: [u8; 16],
    control: Control<'_>,
    out: &mut [u8],
) -> Result<usize, Invalid> {
    let (kind, value, text) = match control {
        Control::UiText(t) => (0, 0, t),
        Control::UiDisabled(v) => (1, u32::from(v), ""),
        Control::Text(t) => (2, 0, t),
    };
    let len = HudCommand::SIZE.checked_add(text.len()).ok_or(Invalid)?;
    if text.len() > HUD_MAX_TEXT_BYTES || out.len() < len {
        return Err(Invalid);
    }
    let h = HudCommand {
        entity,
        kind,
        value,
        len: text.len() as u32,
        reserved: 0,
    };
    out[..HudCommand::SIZE].copy_from_slice(&h.encode());
    out[HudCommand::SIZE..len].copy_from_slice(text.as_bytes());
    Ok(len)
}

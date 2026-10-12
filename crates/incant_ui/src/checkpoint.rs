//! Ordinary-load widget data, separate from native input contexts and rendering.
use crate::{StableId, TextMeasure, UiError, UiEvent, UiKind, UiRuntime, Widget};
use incant_text::editing::{Selection, TextBuffer};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const MAGIC: &[u8; 8] = b"INCUI001";
const HEADER: usize = 80;
const ROW: usize = 48;
pub const MAX_UI_CHECKPOINT_BYTES: usize =
    HEADER + ROW * crate::MAX_UI_NODES + incant_types::MAX_SCENE_TEXT_BYTES;

#[derive(Debug, thiserror::Error)]
pub enum UiCheckpointError {
    #[error("invalid UI checkpoint: {0}")]
    Invalid(&'static str),
    #[error("UI checkpoint integrity check failed")]
    Integrity,
    #[error("commit or cancel active text composition before saving UI state")]
    CompositionActive,
    #[error(transparent)]
    Ui(#[from] UiError),
}
type Result<T> = std::result::Result<T, UiCheckpointError>;

impl UiRuntime {
    /// Save accepted committed text, grapheme selection and logical focus.
    /// A prepared but uncommitted candidate has no effect on this snapshot.
    /// Composition must first be explicitly committed or cancelled; saving
    /// never does either implicitly. Input presses, pointer capture, OS focus,
    /// native context serials, notifications and display geometry are transient.
    ///
    /// This is a widget-state section, not a complete game/session checkpoint.
    /// Its caller must bind it to the same committed world and trusted program.
    /// The checksum detects corruption, not malicious edits or authenticity.
    pub fn checkpoint_bytes(&self) -> Result<Vec<u8>> {
        if self.layout().is_none() {
            return Err(UiCheckpointError::Invalid("no committed layout"));
        }
        if self
            .state
            .buffers
            .iter()
            .flatten()
            .any(|b| b.preedit().is_some())
        {
            return Err(UiCheckpointError::CompositionActive);
        }
        let mut indices: Vec<_> = (0..self.widgets().len()).collect();
        indices.sort_unstable_by_key(|&i| self.widgets()[i].id);
        let mut out = vec![0; HEADER];
        out[..8].copy_from_slice(MAGIC);
        write(&mut out[8..12], 1);
        // Policy 1: ordinary load, neutral physical controls, fresh contexts.
        write(&mut out[12..16], 1);
        write(&mut out[16..20], indices.len());
        if let Some(focus) = self.focus() {
            write(&mut out[20..24], 1);
            out[24..40].copy_from_slice(&focus.widget().0);
        }
        for i in indices {
            let widget = &self.widgets()[i];
            let mut row = [0; ROW];
            row[..16].copy_from_slice(&widget.id.0);
            if let Some(parent) = widget.parent {
                row[16..32].copy_from_slice(&parent.0);
                row[33] = 1;
            }
            row[32] = kind(&widget.node.kind);
            let text = if let Some(buffer) = &self.state.buffers[i] {
                write(&mut row[36..40], buffer.selection().anchor);
                write(&mut row[40..44], buffer.selection().focus);
                write(&mut row[44..48], buffer.text().len());
                buffer.text()
            } else {
                ""
            };
            out.extend(row);
            out.extend(text.as_bytes());
        }
        if out.len() > MAX_UI_CHECKPOINT_BYTES {
            return Err(UiCheckpointError::Invalid("checkpoint capacity"));
        }
        let body_len = out.len() - HEADER;
        write(&mut out[40..44], body_len);
        let hash = digest(&out);
        out[48..80].copy_from_slice(&hash);
        Ok(out)
    }

    /// Construct a fresh owner from widget data and a compatible committed tree.
    /// Stable IDs, parents and widget kinds must match exactly, independent of
    /// vector order or native entity incarnations. Authored styles, labels and
    /// fonts come from `widgets`, never from the checkpoint. Game hosts must
    /// additionally check the saved world/program identity before this call.
    ///
    /// Text and focus are admitted against newly measured layout before return.
    /// Ineligible saved focus is refused, not silently moved. No old focus or
    /// pointer token can address this owner. The caller must also create a fresh
    /// UiRouter and discard queued OS events/pressed controls, matching ordinary
    /// NativeSession load policy. This factory never changes an existing owner.
    pub fn from_checkpoint_bytes(
        widgets: Vec<Widget>,
        bytes: &[u8],
        viewport: [f32; 2],
        measure: &mut impl TextMeasure,
    ) -> Result<Self> {
        let decoded = decode(bytes)?;
        let mut candidate = Self::new(widgets)?;
        let mut indices: Vec<_> = (0..candidate.widgets().len()).collect();
        indices.sort_unstable_by_key(|&i| candidate.widgets()[i].id);
        if indices.len() != decoded.rows.len() {
            return Err(UiCheckpointError::Invalid("widget membership"));
        }
        let mut total_text = 0;
        for (i, row) in indices.into_iter().zip(decoded.rows) {
            let widget = &candidate.widgets()[i];
            if row.id != widget.id
                || row.parent != widget.parent
                || row.kind != kind(&widget.node.kind)
            {
                return Err(UiCheckpointError::Invalid("widget identity or kind"));
            }
            if row.kind == 4 {
                let mut buffer = TextBuffer::new(row.text).map_err(UiError::from)?;
                buffer.select(row.selection).map_err(UiError::from)?;
                candidate.state.buffers[i] = Some(Arc::new(buffer));
                total_text += row.text.len();
            } else {
                total_text += widget.node.kind.text().map_or(0, |text| text.content.len());
            }
        }
        if total_text > incant_types::MAX_SCENE_TEXT_BYTES {
            return Err(UiCheckpointError::Invalid("combined widget text capacity"));
        }
        let layout = candidate.prepare(&[], viewport, measure)?;
        candidate.commit(layout)?;
        if let Some(focus) = decoded.focus {
            let focused = candidate.prepare(&[UiEvent::Focus(focus)], viewport, measure)?;
            candidate.commit(focused)?;
        }
        Ok(candidate)
    }
}

struct Decoded<'a> {
    focus: Option<StableId>,
    rows: Vec<SavedWidget<'a>>,
}
struct SavedWidget<'a> {
    id: StableId,
    parent: Option<StableId>,
    kind: u8,
    text: &'a str,
    selection: Selection,
}
fn decode(bytes: &[u8]) -> Result<Decoded<'_>> {
    if !(HEADER..=MAX_UI_CHECKPOINT_BYTES).contains(&bytes.len()) || &bytes[..8] != MAGIC {
        return Err(UiCheckpointError::Invalid("framing"));
    }
    if number(&bytes[8..12]) != 1 || number(&bytes[12..16]) != 1 || bytes[44..48] != [0; 4] {
        return Err(UiCheckpointError::Invalid("version or input policy"));
    }
    if bytes[48..80] != digest(bytes) {
        return Err(UiCheckpointError::Integrity);
    }
    let count = number(&bytes[16..20]);
    let mut body = &bytes[HEADER..];
    if number(&bytes[40..44]) != body.len()
        || count > crate::MAX_UI_NODES
        || count > body.len() / ROW
    {
        return Err(UiCheckpointError::Invalid("section lengths"));
    }
    let focus = match number(&bytes[20..24]) {
        0 if bytes[24..40] == [0; 16] => None,
        1 => Some(StableId(bytes[24..40].try_into().unwrap())),
        _ => return Err(UiCheckpointError::Invalid("focus tag")),
    };
    let mut rows: Vec<SavedWidget<'_>> = Vec::with_capacity(count);
    for _ in 0..count {
        if body.len() < ROW {
            return Err(UiCheckpointError::Invalid("row truncation"));
        }
        let (row, tail) = body.split_at(ROW);
        let id = StableId(row[..16].try_into().unwrap());
        if rows.last().is_some_and(|old| old.id >= id) {
            return Err(UiCheckpointError::Invalid("widget order or duplicate"));
        }
        let parent = match row[33] {
            0 if row[16..32] == [0; 16] => None,
            1 => Some(StableId(row[16..32].try_into().unwrap())),
            _ => return Err(UiCheckpointError::Invalid("parent tag")),
        };
        let kind = row[32];
        let selection = Selection {
            anchor: number(&row[36..40]),
            focus: number(&row[40..44]),
        };
        let len = number(&row[44..48]);
        if kind > 4
            || row[34..36] != [0; 2]
            || len > incant_text::MAX_TEXT_BYTES
            || len > tail.len()
            || (kind != 4 && (len != 0 || selection != Selection::caret(0)))
        {
            return Err(UiCheckpointError::Invalid("widget record"));
        }
        let (text, rest) = tail.split_at(len);
        let text =
            std::str::from_utf8(text).map_err(|_| UiCheckpointError::Invalid("text UTF-8"))?;
        rows.push(SavedWidget {
            id,
            parent,
            kind,
            text,
            selection,
        });
        body = rest;
    }
    if !body.is_empty() {
        return Err(UiCheckpointError::Invalid("trailing bytes"));
    }
    Ok(Decoded { focus, rows })
}
fn kind(kind: &UiKind<StableId>) -> u8 {
    match kind {
        UiKind::Root => 0,
        UiKind::Container => 1,
        UiKind::Label { .. } => 2,
        UiKind::Button { .. } => 3,
        UiKind::TextInput { .. } => 4,
    }
}
fn number(bytes: &[u8]) -> usize {
    u32::from_le_bytes(bytes.try_into().unwrap()) as usize
}
fn write(bytes: &mut [u8], value: usize) {
    bytes.copy_from_slice(
        &u32::try_from(value)
            .expect("bounded UI checkpoint field")
            .to_le_bytes(),
    );
}
fn digest(bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(&bytes[..48]);
    hash.update(&bytes[HEADER..]);
    hash.finalize().into()
}

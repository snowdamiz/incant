//! Validated borrowed committed playback; queued controls do not change it.
use crate::{EntityId, Error, Result};
use incant_wasm_abi as abi;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Playback {
    pub entity: EntityId,
    pub frame: u16,
    pub playing: bool,
    pub completed: bool,
}
#[derive(Clone, Copy)]
pub struct FlipbookSnapshot<'a> {
    rows: &'a [u8],
}
impl<'a> FlipbookSnapshot<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let header = abi::FlipbookHeader::decode(bytes).ok_or(Error::Malformed)?;
        if &header.magic != abi::FLIPBOOK_MAGIC
            || header.stride as usize != abi::Flipbook::SIZE
            || header.count as usize > abi::FLIPBOOK_MAX_ROWS
        {
            return Err(Error::Malformed);
        }
        let length = (header.count as usize)
            .checked_mul(abi::Flipbook::SIZE)
            .and_then(|v| v.checked_add(abi::FlipbookHeader::SIZE))
            .ok_or(Error::Bounds)?;
        if bytes.len() != length {
            return Err(Error::Malformed);
        }
        let rows = &bytes[abi::FlipbookHeader::SIZE..];
        for bytes in rows.as_chunks::<{ abi::Flipbook::SIZE }>().0 {
            let row = abi::Flipbook::decode(bytes).ok_or(Error::Malformed)?;
            if row.frame as usize >= abi::FLIPBOOK_MAX_FRAMES || row.flags & !3 != 0 {
                return Err(Error::Malformed);
            }
        }
        Ok(Self { rows })
    }
    pub fn len(&self) -> usize {
        self.rows.len() / abi::Flipbook::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = Playback> + 'a {
        self.rows
            .as_chunks::<{ abi::Flipbook::SIZE }>()
            .0
            .iter()
            .map(|bytes| {
                let row = abi::Flipbook::decode(bytes).expect("validated playback record");
                Playback {
                    entity: EntityId(row.id),
                    frame: row.frame as u16,
                    playing: row.flags & 1 != 0,
                    completed: row.flags & 2 != 0,
                }
            })
    }
}

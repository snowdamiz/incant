//! Additive incant_flipbook_v1 snapshot. Initialized little-endian bytes only;
//! no Rust layouts, private clock phases, strings, assets or native pointers.
use crate::BoundaryError;
use incant_runtime::{FlipbookSnapshot, NativeWorld};
pub const MAGIC: &[u8; 8] = b"INCFB001";
pub const ROW_BYTES: usize = 24;
pub const MAX_BYTES: usize = 16 + incant_runtime::MAX_SPRITE_ENTITIES * ROW_BYTES;

pub(crate) fn encode(
    world: &mut NativeWorld,
    rows: &mut Vec<FlipbookSnapshot>,
    bytes: &mut Vec<u8>,
) -> Result<(), BoundaryError> {
    world.snapshot_flipbooks(rows)?;
    bytes.clear();
    bytes.extend_from_slice(MAGIC);
    bytes.extend_from_slice(&(rows.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&(ROW_BYTES as u32).to_le_bytes());
    for row in rows {
        bytes.extend_from_slice(&row.id.0);
        bytes.extend_from_slice(&u32::from(row.frame).to_le_bytes());
        let flags = u32::from(row.playing) | (u32::from(row.completed) << 1);
        bytes.extend_from_slice(&flags.to_le_bytes());
    }
    debug_assert!(bytes.len() <= MAX_BYTES);
    Ok(())
}

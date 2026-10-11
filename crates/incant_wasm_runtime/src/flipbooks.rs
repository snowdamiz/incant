//! Additive incant_flipbook_v1 snapshot. Initialized little-endian bytes only;
//! no Rust layouts, private clock phases, strings, assets or native pointers.
use crate::BoundaryError;
use incant_runtime::{FlipbookSnapshot, NativeWorld};
pub use incant_wasm_abi::{FLIPBOOK_MAGIC as MAGIC, FLIPBOOK_MAX_BYTES as MAX_BYTES};
pub const ROW_BYTES: usize = incant_wasm_abi::Flipbook::SIZE;
const _: () = assert!(incant_runtime::MAX_SPRITE_ENTITIES == incant_wasm_abi::FLIPBOOK_MAX_ROWS);

pub(crate) fn encode(
    world: &mut NativeWorld,
    rows: &mut Vec<FlipbookSnapshot>,
    bytes: &mut Vec<u8>,
) -> Result<(), BoundaryError> {
    world.snapshot_flipbooks(rows)?;
    bytes.clear();
    bytes.extend_from_slice(
        &incant_wasm_abi::FlipbookHeader {
            magic: *MAGIC,
            count: rows.len() as u32,
            stride: ROW_BYTES as u32,
        }
        .encode(),
    );
    for row in rows {
        bytes.extend_from_slice(
            &incant_wasm_abi::Flipbook {
                id: row.id.0,
                frame: u32::from(row.frame),
                flags: u32::from(row.playing) | (u32::from(row.completed) << 1),
            }
            .encode(),
        );
    }
    debug_assert!(bytes.len() <= MAX_BYTES);
    Ok(())
}

const _: () = assert!(incant_runtime::MAX_FLIPBOOK_FRAMES == incant_wasm_abi::FLIPBOOK_MAX_FRAMES);

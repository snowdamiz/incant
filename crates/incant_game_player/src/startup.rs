//! Bounded startup-only cooked clip table shared by offline and window hosts.
use crate::{bounded, session::Result};
use incant_audio_runtime::{ClipBinding, MAX_SCENE_CLIPS};
use incant_runtime::StableId;
use std::{fmt::Write, path::Path};
pub fn clips(cache: &str, path: &str) -> Result<Vec<ClipBinding>> {
    let table = bounded(path, 12 + MAX_SCENE_CLIPS * 48)?;
    if table.len() < 12 || &table[..8] != b"NCLIPS01" {
        return Err("clip table header".into());
    }
    let count = u32::from_le_bytes(table[8..12].try_into()?) as usize;
    if count > MAX_SCENE_CLIPS || table.len() != 12 + count * 48 {
        return Err("clip table length/count".into());
    }
    let mut clips = Vec::with_capacity(count);
    for row in table[12..].as_chunks::<48>().0 {
        let id = StableId(row[..16].try_into()?);
        let mut key = String::with_capacity(64);
        for byte in &row[16..] {
            write!(&mut key, "{byte:02x}")?;
        }
        clips.push(ClipBinding::load(id, Path::new(cache), &key)?);
    }
    Ok(clips)
}

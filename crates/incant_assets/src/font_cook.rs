//! Lossless single-face font cooking. All parsing is borrowed and names are ignored.
use crate::{AssetError, Result, SourceSet};
use incant_assets_runtime::font::{
    CookedFont, FONT_HEADER_BYTES, FONT_MAGIC, FontMetadata, MAX_FONT_BYTES, font_key, font_path,
    load_font, validate_source,
};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path};

pub fn cook_font(root: &Path, source: &Path, cache: &Path) -> Result<CookedFont> {
    if !source
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("ttf") || ext.eq_ignore_ascii_case("otf"))
    {
        return Err(AssetError::Unsupported("expected TTF or OTF font".into()));
    }
    let mut sources = SourceSet::new(root)?;
    let bytes = sources.read_limited(source, MAX_FONT_BYTES)?;
    let dependency = sources
        .dependencies()
        .into_iter()
        .next()
        .ok_or_else(|| crate::invalid("missing font source"))?;
    let fingerprint = font_key(&dependency)?;
    if let Ok(cached) = load_font(cache, &fingerprint) {
        return Ok(cached);
    }
    validate_source(&bytes)?;
    let metadata = FontMetadata {
        version: 1,
        fingerprint: fingerprint.clone(),
        bytes: bytes.len() as u32,
        content_sha256: dependency.sha256.clone(),
        dependency,
    };
    let mut header = [0_u8; FONT_HEADER_BYTES];
    header[..8].copy_from_slice(FONT_MAGIC);
    header[8..12].copy_from_slice(&1_u32.to_le_bytes());
    header[12..16].copy_from_slice(&metadata.bytes.to_le_bytes());
    header[16..].copy_from_slice(&Sha256::digest(&bytes));
    fs::create_dir_all(cache)?;
    let mut blob = tempfile::NamedTempFile::new_in(cache)?;
    blob.write_all(&header)?;
    blob.write_all(&bytes)?;
    blob.as_file().sync_all()?;
    let temporary = blob.into_temp_path();
    fs::rename(
        &temporary,
        font_path(cache, &metadata.content_sha256, "ifont")?,
    )?;
    let mut manifest = tempfile::NamedTempFile::new_in(cache)?;
    manifest.write_all(&serde_json::to_vec(&metadata)?)?;
    manifest.as_file().sync_all()?;
    let temporary = manifest.into_temp_path();
    fs::rename(&temporary, font_path(cache, &fingerprint, "json")?)?;
    let mut result = load_font(cache, &fingerprint)?;
    result.cache_hit = false;
    Ok(result)
}

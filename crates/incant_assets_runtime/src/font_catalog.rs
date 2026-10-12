//! Startup-only font bindings; no project or source path is consulted.
use crate::{AssetError, CookedFont, Result, font, invalid};
use incant_types::{FontAssetRef, StableId};
use std::{collections::BTreeMap, path::Path};

/// Checked immutable catalog. Aggregate declared bytes are admitted before any
/// font payload is read. The accepted metadata is consumed directly, so a second
/// manifest read cannot change the admitted size. Each asset ID counts its own
/// bytes even if recipes share a blob. Parsing/layout/raster work has separate caps.
pub struct FontCatalog {
    fonts: BTreeMap<StableId, CookedFont>,
    bytes: usize,
}
impl FontCatalog {
    pub fn load(cache: &Path, bindings: &[FontAssetRef]) -> Result<Self> {
        if bindings.len() > incant_text::MAX_FONTS
            || bindings.windows(2).any(|a| a[0].id >= a[1].id)
        {
            return Err(invalid("font catalog count/order"));
        }
        let mut metadata = Vec::with_capacity(bindings.len());
        let mut bytes = 0usize;
        for binding in bindings {
            let key: String = binding
                .fingerprint
                .iter()
                .map(|n| format!("{n:02x}"))
                .collect();
            let data = font::load_metadata(cache, &key)?;
            bytes = bytes
                .checked_add(data.bytes as usize)
                .filter(|&n| n <= incant_text::MAX_TOTAL_FONT_BYTES)
                .ok_or(AssetError::Limit("total font bytes"))?;
            metadata.push((binding.id, data));
        }
        let mut fonts = BTreeMap::new();
        for (id, metadata) in metadata {
            fonts.insert(id, font::load_data(cache, metadata)?);
        }
        Ok(Self { fonts, bytes })
    }
    pub fn get(&self, id: StableId) -> Option<&CookedFont> {
        self.fonts.get(&id)
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn len(&self) -> usize {
        self.fonts.len()
    }
    pub fn is_empty(&self) -> bool {
        self.fonts.is_empty()
    }
    pub fn into_fonts(self) -> impl ExactSizeIterator<Item = (StableId, CookedFont)> {
        self.fonts.into_iter()
    }
}

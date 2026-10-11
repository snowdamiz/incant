//! Checked CPU atlas binding for future tile consumers. This does not submit
//! geometry or grant tilemap support to NativeRenderInput.
use crate::{MAX_PAYLOAD_BYTES, NativeRenderError};
use incant_assets_runtime::{RuntimeAsset, RuntimeAssetData, TextureUsage};
use incant_runtime::{NativeTilemap, StableId, VisualAssetKind, VisualAssetRef};
use std::{collections::BTreeMap, path::Path};

pub struct TilemapAtlasBinding {
    source: RuntimeAsset,
    regions: BTreeMap<StableId, [u32; 4]>,
}
impl TilemapAtlasBinding {
    /// Bind descriptor identity/kind/recipe to a checked, sealed cache version.
    /// Source files are never consulted. The returned palette is immutable and
    /// remains valid if authoring later imports a replacement with the same ID.
    pub fn load(
        map: &NativeTilemap,
        reference: &VisualAssetRef,
        cache: &Path,
    ) -> Result<Self, NativeRenderError> {
        let invalid = || NativeRenderError::TilemapBinding;
        map.validate().map_err(|_| invalid())?;
        if reference.id != map.tileset || reference.kind != VisualAssetKind::Texture {
            return Err(invalid());
        }
        let source = RuntimeAsset::load_visual(reference, 0, cache).map_err(|source| {
            NativeRenderError::Asset {
                asset: reference.id,
                source,
            }
        })?;
        if source.info().payload_bytes > MAX_PAYLOAD_BYTES {
            return Err(NativeRenderError::PayloadLimit);
        }
        if source.usage() != Some(TextureUsage::Color) {
            return Err(invalid());
        }
        let RuntimeAssetData::Texture(texture) = source.data() else {
            return Err(invalid());
        };
        let atlas = texture.metadata.atlas.as_ref().ok_or_else(invalid)?;
        let mut regions = BTreeMap::new();
        for frame in map
            .layers
            .iter()
            .flat_map(|layer| layer.cells.iter().flatten())
        {
            if regions.contains_key(frame) {
                continue;
            }
            let label = ulid::Ulid::from_bytes(frame.0).to_string();
            let entry = atlas
                .frames
                .get(&label)
                .ok_or(NativeRenderError::TilemapFrame(*frame))?;
            regions.insert(*frame, entry.region);
        }
        Ok(Self { source, regions })
    }
    pub fn source(&self) -> &RuntimeAsset {
        &self.source
    }
    pub fn regions(&self) -> &BTreeMap<StableId, [u32; 4]> {
        &self.regions
    }
}

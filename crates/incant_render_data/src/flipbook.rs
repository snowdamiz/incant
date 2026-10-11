//! Sealed, source-version-bound frame regions. Construct at a binding boundary.
use crate::SceneError;
use incant_2d::{SpriteRegion, SpriteRenderer};
use incant_assets_runtime::{AssetVersionId, RuntimeAsset, RuntimeAssetData, TextureUsage};

pub struct SpriteFrames {
    source: AssetVersionId,
    regions: Box<[[u32; 4]]>,
}
impl SpriteFrames {
    pub fn resolve(
        source: &RuntimeAsset,
        sprite: &SpriteRenderer,
        frames: &[SpriteRegion],
    ) -> Result<Self, SceneError> {
        if frames.is_empty() || frames.len() > incant_2d::flipbook::MAX_FLIPBOOK_FRAMES {
            return Err(SceneError::Input("flipbook frame count"));
        }
        let RuntimeAssetData::Texture(texture) = source.data() else {
            return Err(SceneError::Input("flipbook texture"));
        };
        if source.usage() != Some(TextureUsage::Color) {
            return Err(SceneError::Input("flipbook texture usage"));
        }
        let mut regions = Vec::with_capacity(frames.len());
        let mut candidate = sprite.clone();
        for frame in frames {
            let region = match frame {
                SpriteRegion::Pixels(r) => *r,
                SpriteRegion::Atlas { frame } => {
                    texture
                        .metadata
                        .atlas
                        .as_ref()
                        .and_then(|a| a.frames.get(frame))
                        .ok_or_else(|| SceneError::MissingSpriteFrame {
                            texture: source.info().id.clone(),
                            frame: frame.clone(),
                        })?
                        .region
                }
            };
            candidate.region = region.into();
            candidate.validate()?;
            if u64::from(region[0]) + u64::from(region[2]) > u64::from(texture.texture.width)
                || u64::from(region[1]) + u64::from(region[3]) > u64::from(texture.texture.height)
            {
                return Err(SceneError::Input("flipbook texture bounds"));
            }
            regions.push(region);
        }
        Ok(Self {
            source: source.version_id(),
            regions: regions.into_boxed_slice(),
        })
    }
    pub fn regions(&self) -> &[[u32; 4]] {
        &self.regions
    }
    pub fn source_version(&self) -> AssetVersionId {
        self.source
    }
}

//! Stable frame identities and bounded deterministic cook-time packing.
mod packing;
pub use packing::pack;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use thiserror::Error;

pub const MAX_ATLAS_FRAMES: usize = 256;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AtlasSource {
    pub name: String,
    /// Relative to the atlas descriptor; resolved inside the granted project.
    pub path: String,
    /// Optional top-left source x/y and width/height; absent selects the whole image.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub region: Option<[u32; 4]>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpriteAtlas {
    pub version: u32,
    pub id: String,
    /// Upper bounds, not a request to allocate the entire rectangle.
    pub maximum_size: [u32; 2],
    /// Repeated edge texels around each frame. No rotation or transparent trimming.
    pub padding: u32,
    pub frames: BTreeMap<String, AtlasSource>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AtlasFrame {
    pub name: String,
    /// Packed content rectangle excluding extrusion; top-left pixel coordinates.
    pub region: [u32; 4],
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AtlasLayout {
    pub id: String,
    pub size: [u32; 2],
    pub padding: u32,
    pub frames: BTreeMap<String, AtlasFrame>,
}
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum AtlasError {
    #[error("atlas version must be 1 and atlas/frame IDs must be canonical ULIDs")]
    Identity,
    #[error("atlas must contain 1..256 frames with nonempty bounded names and paths")]
    Frames,
    #[error("atlas width and height must each be in 1..8192")]
    Dimensions,
    #[error("atlas page exceeds the 4194304-texel limit")]
    Texels,
    #[error("atlas padding must be in 1..16")]
    Padding,
    #[error("atlas frame rectangles must be nonempty and fit their source/packed extent")]
    Region,
    #[error(
        "could not pack every frame within the requested maximum using the bounded unrotated skyline heuristic"
    )]
    NoFit,
    #[error("atlas frame extrusion rectangles overlap")]
    Overlap,
}
pub fn valid_frame_id(id: &str) -> bool {
    ulid::Ulid::from_string(id).is_ok_and(|value| value.to_string() == id)
}
fn validate_size([w, h]: [u32; 2]) -> Result<(), AtlasError> {
    if w == 0 || h == 0 || w > 8192 || h > 8192 {
        return Err(AtlasError::Dimensions);
    }
    if u64::from(w) * u64::from(h) > 4 * 1024 * 1024 {
        return Err(AtlasError::Texels);
    }
    Ok(())
}
impl SpriteAtlas {
    pub fn validate(&self) -> Result<(), AtlasError> {
        if self.version != 1
            || !valid_frame_id(&self.id)
            || self.frames.keys().any(|id| !valid_frame_id(id))
        {
            return Err(AtlasError::Identity);
        }
        if self.frames.is_empty()
            || self.frames.len() > MAX_ATLAS_FRAMES
            || self.frames.values().any(|frame| {
                frame.name.trim().is_empty()
                    || frame.name.len() > 256
                    || frame.path.is_empty()
                    || frame.path.len() > 4096
            })
        {
            return Err(AtlasError::Frames);
        }
        validate_size(self.maximum_size)?;
        if !(1..=16).contains(&self.padding) {
            return Err(AtlasError::Padding);
        }
        for frame in self.frames.values() {
            if let Some([x, y, w, h]) = frame.region
                && (w == 0
                    || h == 0
                    || u64::from(x) + u64::from(w) > 8192
                    || u64::from(y) + u64::from(h) > 8192)
            {
                return Err(AtlasError::Region);
            }
        }
        Ok(())
    }
}
impl AtlasLayout {
    /// Validate cached metadata independently of source images or a packer call.
    pub fn validate(&self) -> Result<(), AtlasError> {
        if !valid_frame_id(&self.id) || self.frames.keys().any(|id| !valid_frame_id(id)) {
            return Err(AtlasError::Identity);
        }
        validate_size(self.size)?;
        if !(1..=16).contains(&self.padding) {
            return Err(AtlasError::Padding);
        }
        if self.frames.is_empty()
            || self.frames.len() > MAX_ATLAS_FRAMES
            || self
                .frames
                .values()
                .any(|f| f.name.trim().is_empty() || f.name.len() > 256)
        {
            return Err(AtlasError::Frames);
        }
        let mut occupied = Vec::new();
        for frame in self.frames.values() {
            let [x, y, w, h] = frame.region;
            let p = self.padding;
            if x < p
                || y < p
                || w == 0
                || h == 0
                || u64::from(x) + u64::from(w) + u64::from(p) > u64::from(self.size[0])
                || u64::from(y) + u64::from(h) + u64::from(p) > u64::from(self.size[1])
            {
                return Err(AtlasError::Region);
            }
            let rect = [x - p, y - p, x + w + p, y + h + p];
            if occupied.iter().any(|a: &[u32; 4]| {
                a[0] < rect[2] && rect[0] < a[2] && a[1] < rect[3] && rect[1] < a[3]
            }) {
                return Err(AtlasError::Overlap);
            }
            occupied.push(rect);
        }
        Ok(())
    }
}

//! Native sprite values. References are binary identities; no project or JSON.
use crate::StableId;
pub use incant_2d::{SpriteMode, SpriteSampling};

pub const MAX_SPRITE_ENTITIES: usize = 32768;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeSpriteRegion {
    Pixels([u32; 4]),
    Atlas { frame: StableId },
}
#[derive(Debug, Clone, PartialEq)]
pub struct NativeSpriteRenderer {
    pub texture: StableId,
    pub region: NativeSpriteRegion,
    pub size: [f64; 2],
    pub pivot: [f64; 2],
    pub tint: [f64; 4],
    pub flip_x: bool,
    pub flip_y: bool,
    pub layer: i16,
    pub order: i32,
    pub sampling: SpriteSampling,
    pub mode: SpriteMode,
}
impl NativeSpriteRenderer {
    /// Numeric geometry shared with the renderer. Called only when preparing or
    /// replacing bindings. The supplied region must come from the checked cache
    /// for atlas references. The informational texture label is filled by a host.
    pub fn resolved_values(&self, region: [u32; 4]) -> incant_2d::SpriteRenderer {
        incant_2d::SpriteRenderer {
            texture: String::new(),
            region: region.into(),
            size: self.size,
            pivot: self.pivot,
            tint: self.tint,
            flip_x: self.flip_x,
            flip_y: self.flip_y,
            layer: self.layer,
            order: self.order,
            sampling: self.sampling,
            mode: self.mode.clone(),
        }
    }
    pub(crate) fn validate(&self) -> Result<(), crate::SceneError> {
        // Every 128-bit identity is representable as a canonical ULID. Atlas
        // extents are checked against retained cooked metadata by the host.
        let region = match self.region {
            NativeSpriteRegion::Pixels(region) => region,
            NativeSpriteRegion::Atlas { .. } => [0, 0, 8192, 8192],
        };
        self.resolved_values(region)
            .validate()
            .map_err(|_| crate::SceneError::Invalid("sprite numeric range"))
    }
}

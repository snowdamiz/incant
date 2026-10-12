//! Bounded cooked-asset readers. No project, importer, command bus or agent.
//! Metadata is decoded once at load; consumers retain typed geometry, materials,
//! texture mips and audio. Source importing and cache writing remain authoring-side.
//! The optional `visual` feature enables model/texture readers; default audio-only
//! consumers do not compile the geometry codecs or 2D component dependency graph.
pub mod audio;
#[cfg(feature = "font")]
pub mod font;
#[cfg(feature = "font")]
mod font_catalog;
pub use audio::{AUDIO_CHUNK_FRAMES, AudioMetadata, CookedAudio, MAX_AUDIO_PCM_BYTES, load_audio};
#[cfg(feature = "font")]
pub use font::{CookedFont, FontMetadata, MAX_FONT_BYTES, load_font};
#[cfg(feature = "font")]
pub use font_catalog::FontCatalog;
#[cfg(feature = "visual")]
mod ktx;
#[cfg(feature = "visual")]
pub mod material;
#[cfg(feature = "visual")]
mod mesh;
#[cfg(feature = "visual")]
pub mod model;
#[cfg(feature = "visual")]
pub mod model_node;
#[cfg(feature = "visual")]
mod model_textures;
#[cfg(feature = "visual")]
pub mod texture;
#[cfg(feature = "visual")]
pub mod texture_cache;
#[cfg(feature = "visual")]
mod version;
#[cfg(feature = "visual")]
pub use ktx::decode_ktx2;
#[cfg(feature = "visual")]
pub use material::{AlphaMode, Material, MaterialTexture};
#[cfg(feature = "visual")]
pub use mesh::{Mesh, Vertex, decode_mesh};
#[cfg(feature = "visual")]
pub use model::{CookedModel, ModelMetadata, decode_model, load_model};
#[cfg(feature = "visual")]
pub use model_node::ModelNode;
#[cfg(feature = "visual")]
pub use model_textures::ModelTexture;
use serde::{Deserialize, Serialize};
#[cfg(feature = "visual")]
pub use texture::{Texture, TextureFormat, TextureUsage};
#[cfg(feature = "visual")]
pub use texture_cache::{CookedTexture, TextureMetadata, load_texture};
use thiserror::Error;
#[cfg(feature = "visual")]
pub use version::{AssetVersionId, RuntimeAsset, RuntimeAssetData, RuntimeAssetInfo};

pub const MAX_SOURCE_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_VERTICES: usize = 1_000_000;
pub const MAX_INDICES: usize = 3_000_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Dependency {
    pub path: String,
    pub sha256: String,
}
#[derive(Debug, Error)]
pub enum AssetError {
    #[cfg(feature = "visual")]
    #[error(transparent)]
    Atlas(#[from] incant_2d::atlas::AtlasError),
    #[error("asset IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("asset JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid asset: {0}")]
    Invalid(String),
    #[cfg(feature = "visual")]
    #[error("unsupported asset feature: {0}")]
    Unsupported(String),
    #[error("asset exceeds {0} limit")]
    Limit(&'static str),
}
pub type Result<T> = std::result::Result<T, AssetError>;
fn invalid(reason: impl Into<String>) -> AssetError {
    AssetError::Invalid(reason.into())
}
pub fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(feature = "visual")]
pub mod animation;

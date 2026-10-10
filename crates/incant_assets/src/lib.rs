//! CPU asset import and cooking. No account, renderer or editor is required.
mod audio;
mod audio_cook;
mod cache;
mod gltf_import;
mod ktx;
mod material;
mod mesh;
mod model_textures;
mod project_cache;
mod runtime;
mod source;
mod tangents;
mod texture;
mod texture_cache;

pub use audio::{AUDIO_CHUNK_FRAMES, AudioMetadata, CookedAudio, MAX_AUDIO_PCM_BYTES, load_audio};
pub use audio_cook::cook_audio;
pub use cache::{CookedModel, ModelMetadata, cook_gltf, load_model};
pub use gltf_import::{ImportedModel, ModelNode, import_gltf};
pub use ktx::{decode_ktx2, encode_ktx2};
pub use material::{AlphaMode, Material, MaterialTexture};
pub use mesh::{Mesh, Vertex, cook_mesh, decode_mesh};
pub use model_textures::ModelTexture;
pub use project_cache::{CacheKind, project_cache_directory};
pub use runtime::{
    AssetChanges, AssetStore, RuntimeAsset, RuntimeAssetData, RuntimeAssetError, RuntimeAssetInfo,
};
pub use source::{Dependency, SourceSet};
pub use texture::{Texture, TextureFormat, TextureUsage, import_image};
pub use texture_cache::{CookedTexture, TextureMetadata, cook_texture, load_texture};
use thiserror::Error;

pub const MAX_SOURCE_BYTES: usize = 128 * 1024 * 1024;
pub const MAX_VERTICES: usize = 1_000_000;
pub const MAX_INDICES: usize = 3_000_000;

#[derive(Debug, Error)]
pub enum AssetError {
    #[error("asset IO: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid glTF: {0}")]
    Gltf(#[from] gltf::Error),
    #[error("asset JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid asset: {0}")]
    Invalid(String),
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

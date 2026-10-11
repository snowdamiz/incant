pub use incant_assets_runtime::{AlphaMode, Material, MaterialTexture};

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn resolve(
    values: &[serde_json::Value],
    textures: &[crate::ModelTexture],
    images: &[crate::Texture],
) -> crate::Result<Vec<Material>> {
    incant_assets_runtime::material::resolve(values, textures, images).map_err(Into::into)
}

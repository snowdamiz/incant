use crate::{AssetError, ImportedModel, MAX_SOURCE_BYTES, Result, cook_mesh, import_gltf};
use incant_assets_runtime::model::key_path;
pub use incant_assets_runtime::{CookedModel, ModelMetadata};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path};
impl From<&ImportedModel> for ModelMetadata {
    fn from(m: &ImportedModel) -> Self {
        Self {
            fingerprint: m.fingerprint.clone(),
            dependencies: m.dependencies.clone(),
            nodes: m.nodes.clone(),
            scenes: m.scenes.clone(),
            default_scene: m.default_scene,
            materials: m.materials.clone(),
            mesh_materials: m.mesh_materials.clone(),
            textures: m.textures.clone(),
        }
    }
}
/// Re-snapshot every dependency before considering a hit, so changed external
/// buffers invalidate the entry even when the glTF JSON has not changed.
pub fn cook_gltf(root: &Path, source: &Path, cache: &Path) -> Result<CookedModel> {
    cook_imported(import_gltf(root, source)?, cache)
}
/// Static FBX shares the checked model/meshopt/KTX2 format with glTF.
pub fn cook_fbx(root: &Path, source: &Path, cache: &Path) -> Result<CookedModel> {
    cook_imported(crate::import_fbx(root, source)?, cache)
}
/// Authoring import dispatch. Runtime readers never link either source parser.
pub fn cook_model(root: &Path, source: &Path, cache: &Path) -> Result<CookedModel> {
    match source
        .extension()
        .and_then(|s| s.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("gltf" | "glb") => cook_gltf(root, source, cache),
        Some("fbx") => cook_fbx(root, source, cache),
        _ => Err(AssetError::Unsupported(
            "model format (expected glTF, GLB or FBX)".into(),
        )),
    }
}
fn cook_imported(imported: ImportedModel, cache: &Path) -> Result<CookedModel> {
    let metadata = ModelMetadata::from(&imported);
    crate::material::resolve(&metadata.materials, &metadata.textures, &imported.images)?;
    if let Ok(mut model) = load_model(cache, &metadata.fingerprint)
        && model.metadata == metadata
    {
        model.cache_hit = true;
        return Ok(model);
    }
    let json = serde_json::to_vec(&metadata)?;
    if json.len() > 8 * 1024 * 1024 {
        return Err(AssetError::Limit("model metadata"));
    }
    let mut bytes = b"INCMOD02".to_vec();
    bytes.extend_from_slice(&(json.len() as u32).to_le_bytes());
    bytes.extend(json);
    bytes.extend_from_slice(&(imported.meshes.len() as u32).to_le_bytes());
    for mesh in &imported.meshes {
        let cooked = cook_mesh(mesh)?;
        bytes.extend_from_slice(&(cooked.len() as u32).to_le_bytes());
        bytes.extend(cooked);
        if bytes.len() > MAX_SOURCE_BYTES - 32 {
            return Err(AssetError::Limit("cooked model bytes"));
        }
    }
    bytes.extend_from_slice(&(imported.images.len() as u32).to_le_bytes());
    for image in &imported.images {
        let ktx = crate::encode_ktx2(image)?;
        bytes.extend_from_slice(&(ktx.len() as u32).to_le_bytes());
        bytes.extend(ktx);
        if bytes.len() > MAX_SOURCE_BYTES - 32 {
            return Err(AssetError::Limit("cooked model bytes"));
        }
    }
    let digest = Sha256::digest(&bytes);
    bytes.extend_from_slice(&digest);
    fs::create_dir_all(cache)?;
    let path = key_path(cache, &metadata.fingerprint)?;
    let mut file = tempfile::NamedTempFile::new_in(cache)?;
    file.write_all(&bytes)?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| AssetError::Io(e.error))?;
    // Return the exact optimized representation consumers will see on a hit.
    let mut model = incant_assets_runtime::decode_model(&bytes, &metadata.fingerprint)?;
    model.cache_hit = false;
    Ok(model)
}
pub fn load_model(cache: &Path, fingerprint: &str) -> Result<CookedModel> {
    incant_assets_runtime::load_model(cache, fingerprint).map_err(Into::into)
}

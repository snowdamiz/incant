use crate::{
    AssetError, Dependency, ImportedModel, MAX_SOURCE_BYTES, Mesh, ModelNode, Result, cook_mesh,
    decode_mesh, import_gltf, invalid,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelMetadata {
    pub fingerprint: String,
    pub dependencies: Vec<Dependency>,
    pub nodes: Vec<ModelNode>,
    pub scenes: Vec<Vec<usize>>,
    pub default_scene: Option<usize>,
    pub materials: Vec<serde_json::Value>,
    pub mesh_materials: Vec<Option<usize>>,
    #[serde(default)]
    pub textures: Vec<crate::ModelTexture>,
}
#[derive(Debug)]
pub struct CookedModel {
    pub metadata: ModelMetadata,
    pub materials: Vec<crate::Material>,
    pub meshes: Vec<Mesh>,
    pub images: Vec<crate::Texture>,
    pub cache_hit: bool,
}
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
fn key_path(cache: &Path, key: &str) -> Result<std::path::PathBuf> {
    if key.len() != 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("invalid cache key"));
    }
    Ok(cache.join(format!("{key}.incmodel")))
}
/// Re-snapshot every dependency before considering a hit, so changed external
/// buffers invalidate the entry even when the glTF JSON has not changed.
pub fn cook_gltf(root: &Path, source: &Path, cache: &Path) -> Result<CookedModel> {
    let imported = import_gltf(root, source)?;
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
    let mut model = decode_model(&bytes, &metadata.fingerprint)?;
    model.cache_hit = false;
    Ok(model)
}
/// Runtime entry point: load cooked geometry without the original glTF files.
pub fn load_model(cache: &Path, fingerprint: &str) -> Result<CookedModel> {
    let path = key_path(cache, fingerprint)?;
    if fs::symlink_metadata(&path)?.file_type().is_symlink() {
        return Err(invalid("cache entry is a symlink"));
    }
    let file = fs::File::open(path)?;
    if !file.metadata()?.is_file() {
        return Err(invalid("cache entry is not a file"));
    }
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(AssetError::Limit("cooked model bytes"));
    }
    decode_model(&bytes, fingerprint)
}
fn take<'a>(bytes: &mut &'a [u8], n: usize) -> Result<&'a [u8]> {
    if n > bytes.len() {
        return Err(invalid("truncated model cache"));
    }
    let (head, tail) = bytes.split_at(n);
    *bytes = tail;
    Ok(head)
}
fn count(bytes: &mut &[u8]) -> Result<usize> {
    Ok(u32::from_le_bytes(take(bytes, 4)?.try_into().unwrap()) as usize)
}
fn decode_model(bytes: &[u8], key: &str) -> Result<CookedModel> {
    if bytes.len() < 48 || (&bytes[..8] != b"INCMOD01" && &bytes[..8] != b"INCMOD02") {
        return Err(invalid("invalid model cache header"));
    }
    let end = bytes.len() - 32;
    if Sha256::digest(&bytes[..end]).as_slice() != &bytes[end..] {
        return Err(invalid("model cache checksum mismatch"));
    }
    let mut payload = &bytes[8..end];
    let json_len = count(&mut payload)?;
    if json_len > 8 * 1024 * 1024 {
        return Err(AssetError::Limit("model metadata"));
    }
    let metadata: ModelMetadata = serde_json::from_slice(take(&mut payload, json_len)?)?;
    if metadata.fingerprint != key {
        return Err(invalid("model cache key mismatch"));
    }
    let n = count(&mut payload)?;
    if n == 0 || n > 10_000 || n != metadata.mesh_materials.len() {
        return Err(invalid("invalid model mesh count"));
    }
    let mut meshes = Vec::new();
    let (mut vertices, mut indices) = (0, 0);
    for _ in 0..n {
        let size = count(&mut payload)?;
        let mesh = decode_mesh(take(&mut payload, size)?)?;
        vertices += mesh.vertices.len();
        indices += mesh.indices.len();
        if vertices > crate::MAX_VERTICES || indices > crate::MAX_INDICES {
            return Err(AssetError::Limit("total cached geometry"));
        }
        meshes.push(mesh);
    }
    let mut images = Vec::new();
    if &bytes[..8] == b"INCMOD02" {
        let n = count(&mut payload)?;
        if n > 768 {
            return Err(AssetError::Limit("cached textures"));
        }
        for _ in 0..n {
            let size = count(&mut payload)?;
            images.push(crate::decode_ktx2(take(&mut payload, size)?)?);
        }
    }
    crate::model_textures::validate_textures(&metadata.textures, images.len())?;
    if !payload.is_empty() {
        return Err(invalid("trailing bytes in model cache"));
    }
    if metadata.nodes.iter().any(|node| {
        node.children.iter().any(|&i| i >= metadata.nodes.len())
            || node.meshes.iter().any(|&i| i >= n)
            || node.transform.iter().flatten().any(|v| !v.is_finite())
    }) || metadata
        .scenes
        .iter()
        .flatten()
        .any(|&i| i >= metadata.nodes.len())
        || metadata
            .default_scene
            .is_some_and(|i| i >= metadata.scenes.len())
        || metadata
            .mesh_materials
            .iter()
            .flatten()
            .any(|&i| i >= metadata.materials.len())
    {
        return Err(invalid("invalid model metadata reference"));
    }
    crate::gltf_import::validate_nodes(&metadata.nodes)?;
    let materials = crate::material::resolve(&metadata.materials, &metadata.textures, &images)?;
    Ok(CookedModel {
        metadata,
        materials,
        meshes,
        images,
        cache_hit: true,
    })
}

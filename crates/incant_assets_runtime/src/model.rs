use crate::{
    AssetError, Dependency, MAX_SOURCE_BYTES, Mesh, ModelNode, Result, decode_mesh, invalid,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::Path};
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
/// Derive the existing cache filename after validating its content-key syntax.
/// Shared with the authoring writer; this function performs no filesystem access.
pub fn key_path(cache: &Path, key: &str) -> Result<std::path::PathBuf> {
    if key.len() != 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid("invalid cache key"));
    }
    Ok(cache.join(format!("{key}.incmodel")))
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
/// Decode a bounded version-1 or version-2 cache image without source access.
/// The digest checks corruption, not executable trust; models contain data only.
pub fn decode_model(bytes: &[u8], key: &str) -> Result<CookedModel> {
    if bytes.len() > MAX_SOURCE_BYTES {
        return Err(AssetError::Limit("cooked model bytes"));
    }
    key_path(Path::new(""), key)?;
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
    crate::model_node::validate_nodes(&metadata.nodes)?;
    let materials = crate::material::resolve(&metadata.materials, &metadata.textures, &images)?;
    Ok(CookedModel {
        metadata,
        materials,
        meshes,
        images,
        cache_hit: true,
    })
}

use crate::{AssetError, MAX_INDICES, MAX_SOURCE_BYTES, MAX_VERTICES, Result, invalid};
use sha2::{Digest, Sha256};

/// Twelve initialized floats, no padding: position, normal, UV and tangent.
/// Explicit little-endian words keep the on-disk stream independent of host ABI.
pub type Vertex = [f32; 12];
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}
impl Mesh {
    pub fn validate(&self) -> Result<()> {
        if self.vertices.is_empty() || self.vertices.len() > MAX_VERTICES {
            return Err(AssetError::Limit("vertex count"));
        }
        if self.indices.is_empty()
            || self.indices.len() > MAX_INDICES
            || !self.indices.len().is_multiple_of(3)
        {
            return Err(invalid("mesh requires bounded triangle indices"));
        }
        if self.vertices.iter().flatten().any(|v| !v.is_finite()) {
            return Err(invalid("nonfinite vertex data"));
        }
        if self
            .indices
            .iter()
            .any(|i| *i as usize >= self.vertices.len())
        {
            return Err(invalid("mesh index outside vertex buffer"));
        }
        Ok(())
    }
}
const HEADER: usize = 8 + 4 * 4;
/// Versioned, checksummed meshopt stream. Triangle order may improve for the GPU;
/// winding, vertex attributes and topology are retained.
pub fn cook_mesh(mesh: &Mesh) -> Result<Vec<u8>> {
    mesh.validate()?;
    let mut indices = meshopt::optimize_vertex_cache(&mesh.indices, mesh.vertices.len());
    let vertices = meshopt::optimize_vertex_fetch(&mut indices, &mesh.vertices);
    let words: Vec<[u8; 48]> = vertices
        .iter()
        .map(|v| {
            let mut out = [0; 48];
            for (i, f) in v.iter().enumerate() {
                out[i * 4..i * 4 + 4].copy_from_slice(&f.to_le_bytes());
            }
            out
        })
        .collect();
    let vertex_data = meshopt::encode_vertex_buffer(&words).map_err(|e| invalid(e.to_string()))?;
    let index_data = meshopt::encode_index_buffer(&indices, vertices.len())
        .map_err(|e| invalid(e.to_string()))?;
    let mut out = b"INCMSH01".to_vec();
    for n in [
        vertices.len(),
        indices.len(),
        vertex_data.len(),
        index_data.len(),
    ] {
        out.extend_from_slice(&(n as u32).to_le_bytes());
    }
    out.extend(vertex_data);
    out.extend(index_data);
    let digest = Sha256::digest(&out);
    out.extend_from_slice(&digest);
    Ok(out)
}
pub fn decode_mesh(bytes: &[u8]) -> Result<Mesh> {
    if bytes.len() < HEADER + 32 || bytes.len() > MAX_SOURCE_BYTES || &bytes[..8] != b"INCMSH01" {
        return Err(invalid("invalid cooked mesh header"));
    }
    let payload = bytes.len() - 32;
    if Sha256::digest(&bytes[..payload]).as_slice() != &bytes[payload..] {
        return Err(invalid("cooked mesh checksum mismatch"));
    }
    let fields: Vec<usize> = bytes[8..HEADER]
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| u32::from_le_bytes(*b) as usize)
        .collect();
    let [nv, ni, vb, ib] = fields[..] else {
        unreachable!()
    };
    if nv == 0 || nv > MAX_VERTICES || ni == 0 || ni > MAX_INDICES || !ni.is_multiple_of(3) {
        return Err(AssetError::Limit("cooked geometry counts"));
    }
    if vb.checked_add(ib).and_then(|n| n.checked_add(HEADER)) != Some(payload) {
        return Err(invalid("invalid cooked buffer lengths"));
    }
    // Arrays > 32 have no Default implementation; twelve u32 words are also 48
    // initialized bytes, then converted from their little-endian representation.
    let words = meshopt::decode_vertex_buffer::<[u32; 12]>(&bytes[HEADER..HEADER + vb], nv)
        .map_err(|e| invalid(e.to_string()))?;
    let vertices = words
        .into_iter()
        .map(|v| v.map(|f| f32::from_bits(u32::from_le(f))))
        .collect();
    let indices = meshopt::decode_index_buffer::<u32>(&bytes[HEADER + vb..payload], ni)
        .map_err(|e| invalid(e.to_string()))?;
    let mesh = Mesh { vertices, indices };
    mesh.validate()?;
    Ok(mesh)
}

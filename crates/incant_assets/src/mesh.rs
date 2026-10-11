use crate::{Result, invalid};
pub use incant_assets_runtime::{Mesh, Vertex};
use sha2::{Digest, Sha256};

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
    incant_assets_runtime::decode_mesh(bytes).map_err(Into::into)
}

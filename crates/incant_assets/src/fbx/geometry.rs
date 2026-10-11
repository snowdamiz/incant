use super::unsupported;
use crate::{AssetError, MAX_INDICES, MAX_VERTICES, Mesh, Result, invalid};
use std::collections::BTreeMap;

pub(super) fn validate(mesh: &ufbx::Mesh) -> Result<()> {
    if mesh.faces.iter().any(|f| f.num_indices > 4096) {
        return Err(AssetError::Limit("FBX polygon corners"));
    }
    if mesh.max_face_triangles > mesh.num_triangles
        || mesh.num_triangles > MAX_INDICES / 3
        || mesh.num_indices > MAX_INDICES
        || mesh.num_vertices > MAX_VERTICES
    {
        return Err(AssetError::Limit("FBX geometry"));
    }
    if mesh.num_empty_faces != 0
        || mesh.num_point_faces != 0
        || mesh.num_line_faces != 0
        || mesh.face_hole.iter().any(|h| *h)
    {
        return Err(unsupported("empty/point/line faces or polygon holes"));
    }
    if mesh.uv_sets.len() > 1
        || !mesh.color_sets.is_empty()
        || mesh.vertex_color.exists
        || mesh.vertex_crease.exists
    {
        return Err(unsupported("vertex color/crease or multiple UV sets"));
    }
    if mesh.subdivision_preview_levels != 0 || mesh.subdivision_render_levels != 0 {
        return Err(unsupported("subdivision surfaces"));
    }
    Ok(())
}

pub(super) fn import(
    mesh: &ufbx::Mesh,
    node: &ufbx::Node,
    total_vertices: &mut usize,
    total_indices: &mut usize,
) -> Result<Vec<(Mesh, Option<usize>)>> {
    let count = mesh
        .num_triangles
        .checked_mul(3)
        .ok_or(AssetError::Limit("FBX triangle count"))?;
    // Expanded corner storage is bounded before any allocation. It preserves
    // hard edges/UV seams; the existing meshopt writer performs final optimization.
    if count == 0
        || count > MAX_VERTICES.saturating_sub(*total_vertices)
        || count > MAX_INDICES.saturating_sub(*total_indices)
    {
        return Err(AssetError::Limit("total FBX geometry"));
    }
    let mut parts: BTreeMap<Option<usize>, Mesh> = BTreeMap::new();
    let mut scratch = vec![0; mesh.max_face_triangles * 3];
    for (face_index, face) in mesh.faces.iter().enumerate() {
        let material = if node.materials.is_empty() {
            None
        } else {
            let slot = mesh.face_material.get(face_index).copied().unwrap_or(0) as usize;
            Some(
                node.materials
                    .get(slot)
                    .ok_or_else(|| invalid("FBX face material outside node bindings"))?,
            )
        };
        if let Some(material) = material {
            for texture in &material.textures {
                if !mesh.vertex_uv.exists {
                    return Err(invalid("textured FBX mesh requires UV0"));
                }
                if !texture.texture.uv_set.is_empty()
                    && mesh
                        .uv_sets
                        .first()
                        .is_none_or(|s| s.name.as_ref() != texture.texture.uv_set.as_ref())
                {
                    return Err(unsupported("texture references a UV set other than UV0"));
                }
            }
        }
        let key = material.map(|m| m.element.typed_id as usize);
        let part = parts.entry(key).or_insert_with(|| Mesh {
            vertices: Vec::new(),
            indices: Vec::new(),
        });
        let triangles = mesh.triangulate_face(&mut scratch, *face) as usize;
        if triangles != face.num_indices as usize - 2 {
            return Err(invalid("FBX polygon could not be fully triangulated"));
        }
        for &index in &scratch[..triangles * 3] {
            let i = index as usize;
            let p = mesh.vertex_position[i];
            let n = mesh.vertex_normal[i];
            // FBX UVs have a bottom-left origin; cooked glTF-style image UVs are
            // top-left. Tangents for normal maps are generated after this flip.
            let uv = if mesh.vertex_uv.exists {
                let v = mesh.vertex_uv[i];
                [v.x, 1. - v.y]
            } else {
                [0.; 2]
            };
            let vertex =
                [p.x, p.y, p.z, n.x, n.y, n.z, uv[0], uv[1], 1., 0., 0., 1.].map(|v| v as f32);
            if vertex.iter().any(|v| !v.is_finite()) {
                return Err(invalid("FBX vertex outside finite f32 range"));
            }
            part.indices.push(part.vertices.len() as u32);
            part.vertices.push(vertex);
        }
    }
    let mut out = Vec::with_capacity(parts.len());
    for (material, mut part) in parts {
        let normal_map = material.is_some_and(|id| {
            node.materials.iter().any(|m| {
                m.element.typed_id as usize == id
                    && (m.fbx.normal_map.texture.is_some() || m.pbr.normal_map.texture.is_some())
            })
        });
        if normal_map {
            crate::tangents::generate(&mut part)?;
        }
        *total_vertices += part.vertices.len();
        *total_indices += part.indices.len();
        if *total_vertices > MAX_VERTICES || *total_indices > MAX_INDICES {
            return Err(AssetError::Limit("generated FBX geometry"));
        }
        part.validate()?;
        out.push((part, material));
    }
    Ok(out)
}

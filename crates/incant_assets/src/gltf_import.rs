use crate::{AssetError, Dependency, MAX_INDICES, MAX_VERTICES, Mesh, Result, SourceSet, invalid};
use gltf::{
    buffer::Source,
    mesh::{Mode, Semantic},
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelNode {
    pub name: Option<String>,
    pub children: Vec<usize>,
    /// Primitive indices in ImportedModel.meshes.
    pub meshes: Vec<usize>,
    /// Column-major local transform, as authored by glTF.
    pub transform: [[f32; 4]; 4],
}
#[derive(Debug, Clone)]
pub struct ImportedModel {
    pub meshes: Vec<Mesh>,
    pub nodes: Vec<ModelNode>,
    pub scenes: Vec<Vec<usize>>,
    pub default_scene: Option<usize>,
    pub materials: Vec<serde_json::Value>,
    pub mesh_materials: Vec<Option<usize>>,
    pub dependencies: Vec<Dependency>,
    pub fingerprint: String,
}

/// First static-model importer. Unsupported content fails explicitly instead of
/// silently losing animation, texture bindings, morph targets or extension data.
pub fn import_gltf(root: &Path, source: &Path) -> Result<ImportedModel> {
    let mut sources = SourceSet::new(root)?;
    let bytes = sources.read(source)?;
    let gltf = gltf::Gltf::from_slice(&bytes)?;
    let doc = &gltf.document;
    if let Some(extension) = doc.extensions_used().next() {
        return Err(AssetError::Unsupported(format!(
            "glTF extension {extension}"
        )));
    }
    if doc.animations().next().is_some() || doc.skins().next().is_some() {
        return Err(AssetError::Unsupported("skinned or animated glTF".into()));
    }
    if doc.cameras().next().is_some() {
        return Err(AssetError::Unsupported("glTF cameras".into()));
    }
    if doc.textures().next().is_some() || doc.images().next().is_some() {
        return Err(AssetError::Unsupported(
            "textured glTF (texture cooking is pending)".into(),
        ));
    }
    if doc.nodes().len() > 100_000 || doc.meshes().len() > 10_000 {
        return Err(AssetError::Limit("model objects"));
    }
    let mut buffers = Vec::new();
    let mut loaded_bytes = 0usize;
    for buffer in doc.buffers() {
        let data = match buffer.source() {
            Source::Bin => gltf
                .blob
                .clone()
                .ok_or_else(|| invalid("missing GLB binary chunk"))?,
            Source::Uri(uri) => sources.uri(source, uri)?,
        };
        loaded_bytes = loaded_bytes
            .checked_add(data.len())
            .filter(|n| *n <= crate::MAX_SOURCE_BYTES)
            .ok_or(AssetError::Limit("loaded buffer bytes"))?;
        if data.len() < buffer.length() {
            return Err(invalid("truncated glTF buffer"));
        }
        buffers.push(data);
    }
    for view in doc.views() {
        if view
            .offset()
            .checked_add(view.length())
            .filter(|end| *end <= buffers[view.buffer().index()].len())
            .is_none()
        {
            return Err(invalid("buffer view outside resource"));
        }
    }
    for accessor in doc.accessors() {
        if accessor.sparse().is_some() {
            return Err(AssetError::Unsupported("sparse glTF accessors".into()));
        }
        if accessor.count() == 0 || accessor.count() > MAX_INDICES {
            return Err(AssetError::Limit("accessor count"));
        }
        let view = accessor
            .view()
            .ok_or_else(|| invalid("accessor has no buffer view"))?;
        let stride = view.stride().unwrap_or(accessor.size());
        if stride < accessor.size() {
            return Err(invalid("accessor stride smaller than element"));
        }
        let end = (accessor.count() - 1)
            .checked_mul(stride)
            .and_then(|n| n.checked_add(accessor.size()))
            .and_then(|n| n.checked_add(accessor.offset()));
        if end.is_none_or(|end| end > view.length()) {
            return Err(invalid("accessor outside buffer view"));
        }
    }
    let mut meshes = Vec::new();
    let mut mesh_materials = Vec::new();
    let mut primitive_indices = Vec::new();
    let (mut total_vertices, mut total_indices) = (0, 0);
    for mesh in doc.meshes() {
        let mut ids = Vec::new();
        for primitive in mesh.primitives() {
            if meshes.len() >= 10_000 {
                return Err(AssetError::Limit("mesh primitives"));
            }
            if primitive.mode() != Mode::Triangles {
                return Err(AssetError::Unsupported(
                    "non-triangle glTF primitive".into(),
                ));
            }
            if primitive.morph_targets().next().is_some() {
                return Err(AssetError::Unsupported("glTF morph targets".into()));
            }
            let n = primitive
                .get(&Semantic::Positions)
                .ok_or_else(|| invalid("mesh has no POSITION"))?
                .count();
            if n > MAX_VERTICES {
                return Err(AssetError::Limit("vertex count"));
            }
            for (semantic, accessor) in primitive.attributes() {
                if !matches!(
                    semantic,
                    Semantic::Positions
                        | Semantic::Normals
                        | Semantic::Tangents
                        | Semantic::TexCoords(0)
                ) {
                    return Err(AssetError::Unsupported(format!(
                        "vertex attribute {semantic:?}"
                    )));
                }
                if accessor.count() != n {
                    return Err(invalid("vertex attribute counts differ"));
                }
            }
            let reader = primitive.reader(|b| Some(buffers[b.index()].as_slice()));
            let positions: Vec<_> = reader
                .read_positions()
                .ok_or_else(|| invalid("invalid POSITION buffer"))?
                .collect();
            let indices: Vec<_> = match reader.read_indices() {
                Some(it) => it.into_u32().collect(),
                None => (0..n as u32).collect(),
            };
            total_vertices += n;
            total_indices += indices.len();
            if total_vertices > MAX_VERTICES || total_indices > MAX_INDICES {
                return Err(AssetError::Limit("total model geometry"));
            }
            let normals = reader.read_normals().map(|it| it.collect::<Vec<_>>());
            let uv = reader
                .read_tex_coords(0)
                .map(|it| it.into_f32().collect::<Vec<_>>());
            let tangents = reader.read_tangents().map(|it| it.collect::<Vec<_>>());
            if positions.len() != n
                || normals.as_ref().is_some_and(|v| v.len() != n)
                || uv.as_ref().is_some_and(|v| v.len() != n)
                || tangents.as_ref().is_some_and(|v| v.len() != n)
            {
                return Err(invalid("truncated vertex attributes"));
            }
            let vertices = positions
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let norm = normals.as_ref().map(|v| v[i]).unwrap_or([0.; 3]);
                    let uv = uv.as_ref().map(|v| v[i]).unwrap_or([0.; 2]);
                    let t = tangents.as_ref().map(|v| v[i]).unwrap_or([1., 0., 0., 1.]);
                    [
                        p[0], p[1], p[2], norm[0], norm[1], norm[2], uv[0], uv[1], t[0], t[1],
                        t[2], t[3],
                    ]
                })
                .collect();
            let mut imported = Mesh { vertices, indices };
            imported.validate()?;
            if normals.is_none() {
                generate_flat_normals(&mut imported)?;
                total_vertices = total_vertices - n + imported.vertices.len();
                if total_vertices > MAX_VERTICES {
                    return Err(AssetError::Limit("generated normal vertices"));
                }
            }
            ids.push(meshes.len());
            mesh_materials.push(primitive.material().index());
            meshes.push(imported);
        }
        primitive_indices.push(ids);
    }
    if meshes.is_empty() {
        return Err(invalid("glTF has no triangle meshes"));
    }
    let mut nodes = Vec::new();
    for node in doc.nodes() {
        let transform = node.transform().matrix();
        if transform.iter().flatten().any(|f| !f.is_finite()) {
            return Err(invalid("nonfinite node transform"));
        }
        nodes.push(ModelNode {
            name: node.name().map(str::to_owned),
            children: node.children().map(|c| c.index()).collect(),
            meshes: node
                .mesh()
                .map(|m| primitive_indices[m.index()].clone())
                .unwrap_or_default(),
            transform,
        });
    }
    validate_nodes(&nodes)?;
    let document_json = serde_json::to_value(doc.clone().into_json())?;
    let materials = document_json
        .get("materials")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(ImportedModel {
        meshes,
        nodes,
        materials,
        mesh_materials,
        scenes: doc
            .scenes()
            .map(|s| s.nodes().map(|n| n.index()).collect())
            .collect(),
        default_scene: doc.default_scene().map(|s| s.index()),
        dependencies: sources.dependencies(),
        fingerprint: sources.fingerprint(),
    })
}
pub(crate) fn validate_nodes(nodes: &[ModelNode]) -> Result<()> {
    let mut parents = vec![0; nodes.len()];
    for n in nodes {
        for &c in &n.children {
            parents[c] += 1;
            if parents[c] > 1 {
                return Err(invalid("node has multiple parents"));
            }
        }
    }
    let mut queue: Vec<_> = parents
        .iter()
        .enumerate()
        .filter_map(|(i, p)| (*p == 0).then_some(i))
        .collect();
    let mut visited = 0;
    while let Some(i) = queue.pop() {
        visited += 1;
        for &c in &nodes[i].children {
            parents[c] -= 1;
            if parents[c] == 0 {
                queue.push(c);
            }
        }
    }
    if visited != nodes.len() {
        return Err(invalid("glTF node cycle"));
    }
    Ok(())
}
// glTF specifies flat normals when NORMAL is absent. Split shared vertices at
// triangle boundaries rather than smoothing an authored hard edge accidentally.
fn generate_flat_normals(mesh: &mut Mesh) -> Result<()> {
    if mesh.indices.len() > MAX_VERTICES {
        return Err(AssetError::Limit("flat normal vertices"));
    }
    let mut vertices = Vec::with_capacity(mesh.indices.len());
    for &[a, b, c] in mesh.indices.as_chunks::<3>().0 {
        let [a, b, c] = [a as usize, b as usize, c as usize];
        let u: [f64; 3] =
            std::array::from_fn(|i| mesh.vertices[b][i] as f64 - mesh.vertices[a][i] as f64);
        let v: [f64; 3] =
            std::array::from_fn(|i| mesh.vertices[c][i] as f64 - mesh.vertices[a][i] as f64);
        let cross = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        let length = cross.iter().map(|f| f * f).sum::<f64>().sqrt();
        for id in [a, b, c] {
            let mut vertex = mesh.vertices[id];
            for i in 0..3 {
                vertex[3 + i] = if length > 0. {
                    (cross[i] / length) as f32
                } else {
                    (i == 1) as u8 as f32
                };
            }
            vertices.push(vertex);
        }
    }
    mesh.indices = (0..vertices.len() as u32).collect();
    mesh.vertices = vertices;
    Ok(())
}

use crate::{AssetStore, CookedModel, RuntimeAssetData};

use incant_nav::{NavigationAsset, NavigationError, NavigationGeometry, NavigationResources};

fn invalid(message: impl Into<String>) -> NavigationError {
    NavigationError::Invalid(message.into())
}

impl AssetStore {
    /// Capture immutable cooked model versions. Conversion is lazy so an unused
    /// high-detail art model does not consume the navigation geometry budget.
    /// Runtime-created rooms can select any model in this resource snapshot.
    pub fn navigation_resources(&self) -> NavigationResources {
        self.snapshot()
            .into_iter()
            .filter(|(_, info)| info.kind == "model")
            .map(|(id, info)| {
                let asset = self.get(&id).expect("snapshot resource");
                (
                    id,
                    NavigationAsset::lazy(info.fingerprint, move || {
                        let RuntimeAssetData::Model(model) = asset.data() else {
                            return Err(invalid("navigation resource is not a model"));
                        };
                        model_navigation_geometry(model)
                    }),
                )
            })
            .collect()
    }
}
/// Same default glTF scene and parent-first transforms as the render consumer.
pub fn model_navigation_geometry(
    model: &CookedModel,
) -> Result<NavigationGeometry, NavigationError> {
    let roots = model
        .metadata
        .scenes
        .get(model.metadata.default_scene.unwrap_or(0))
        .ok_or_else(|| invalid("navigation model has no default scene"))?;
    let mut pending: Vec<_> = roots
        .iter()
        .rev()
        .map(|&i| (i, glam::DMat4::IDENTITY))
        .collect();
    let mut visited = vec![false; model.metadata.nodes.len()];
    let mut out = NavigationGeometry {
        vertices: vec![],
        triangles: vec![],
    };
    while let Some((index, parent)) = pending.pop() {
        let seen = visited
            .get_mut(index)
            .ok_or_else(|| invalid("invalid model node"))?;
        if std::mem::replace(seen, true) {
            return Err(invalid("repeated navigation model node"));
        }
        let node = &model.metadata.nodes[index];
        let world = parent
            * glam::DMat4::from_cols_array_2d(&node.transform.map(|column| column.map(f64::from)));
        for &index in &node.meshes {
            let mesh = model
                .meshes
                .get(index)
                .ok_or_else(|| invalid("invalid model primitive"))?;
            if mesh.indices.len() / 3 + out.triangles.len() > incant_nav::MAX_TRIANGLES
                || mesh.vertices.len() + out.vertices.len() > incant_nav::MAX_TRIANGLES * 3
            {
                return Err(NavigationError::Limit("model navigation geometry".into()));
            }
            mesh.validate().map_err(|e| invalid(e.to_string()))?;
            let geometry = NavigationGeometry {
                vertices: mesh.vertices.iter().map(|v| [v[0], v[1], v[2]]).collect(),
                triangles: mesh.indices.as_chunks::<3>().0.to_vec(),
            };
            out.append(geometry.transformed(world.to_cols_array_2d())?)?;
        }
        pending.extend(node.children.iter().rev().map(|&i| (i, world)));
    }
    out.validate()?;
    Ok(out)
}

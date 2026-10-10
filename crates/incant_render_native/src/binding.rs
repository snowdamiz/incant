use super::*;
use incant_assets_runtime::{RuntimeAssetData, TextureUsage};
use incant_render_data::{EnvironmentInput, MeshInput, SceneLight, SceneNode};
use incant_runtime::{VisualAssetKind, VisualAssetRef};
use std::fmt::Write;

pub(crate) fn load(
    refs: &[VisualAssetRef],
    caches: VisualCaches<'_>,
) -> Result<(BTreeMap<StableId, Arc<RuntimeAsset>>, usize), NativeRenderError> {
    let mut assets = BTreeMap::new();
    let mut payload = 0usize;
    for reference in refs {
        let cache = match reference.kind {
            VisualAssetKind::Model => caches.models,
            VisualAssetKind::Texture => caches.textures,
        };
        let asset = RuntimeAsset::load_visual(reference, 0, cache).map_err(|source| {
            NativeRenderError::Asset {
                asset: reference.id,
                source,
            }
        })?;
        payload = add_payload(payload, asset.info().payload_bytes)?;
        assets.insert(reference.id, Arc::new(asset));
    }
    Ok((assets, payload))
}
pub(crate) struct Prepared {
    pub input: SceneInput,
    pub slots: Vec<Option<usize>>,
    pub undo: Vec<DMat4>,
    pub camera_slot: usize,
}
pub(crate) fn prepare(
    view: &SpatialView<'_>,
    assets: &BTreeMap<StableId, Arc<RuntimeAsset>>,
    camera: StableId,
) -> Result<Prepared, NativeRenderError> {
    let mut input = SceneInput::default();
    let mut slots = vec![None; view.nodes().len()];
    let mut camera_slot = None;
    let asset = |id: &StableId| assets.get(id).ok_or(NativeRenderError::MissingAsset(*id));
    for node in view.nodes() {
        let Some(decl) = node.declaration else {
            continue;
        };
        if let Some(env) = &decl.environment {
            let source = asset(&env.texture)?;
            if !matches!(source.data(), RuntimeAssetData::Texture(_)) {
                return Err(NativeRenderError::MissingAsset(env.texture));
            }
            if source.usage() == Some(TextureUsage::Normal) {
                return Err(NativeRenderError::EnvironmentUsage);
            }
            input.environment = Some(EnvironmentInput {
                source: Arc::clone(source),
                intensity: env.intensity,
                rotation_degrees: env.rotation_degrees,
            });
        }
        if decl.camera.is_none()
            && decl.mesh.is_none()
            && decl.directional.is_none()
            && decl.point.is_none()
            && decl.spot.is_none()
        {
            continue;
        }
        let mesh = decl
            .mesh
            .as_ref()
            .map(|mesh| {
                let source = asset(&mesh.mesh)?;
                if !matches!(source.data(), RuntimeAssetData::Model(_)) {
                    return Err(NativeRenderError::MissingAsset(mesh.mesh));
                }
                Ok::<_, NativeRenderError>(MeshInput {
                    source: Arc::clone(source),
                    cast_shadows: mesh.cast_shadows,
                })
            })
            .transpose()?;
        let light = if let Some(light) = &decl.directional {
            Some(SceneLight::Directional(light.clone()))
        } else if let Some(light) = &decl.point {
            Some(SceneLight::Point(light.clone()))
        } else {
            decl.spot
                .as_ref()
                .map(|light| SceneLight::Spot(light.clone()))
        };
        let slot = input.nodes.len();
        slots[node.index] = Some(slot);
        if node.id == camera && decl.camera.is_some() {
            camera_slot = Some(slot);
        }
        let mut id = String::with_capacity(32);
        for byte in node.id.0 {
            write!(&mut id, "{byte:02x}").unwrap();
        }
        input.nodes.push(SceneNode {
            id,
            world: *node.global,
            camera: decl.camera.clone(),
            light,
            mesh,
            sprite: None,
            diagnostic: false,
        });
    }
    let camera_slot = camera_slot.ok_or(NativeRenderError::MissingCamera(camera))?;
    let undo = vec![DMat4::IDENTITY; input.nodes.len()];
    Ok(Prepared {
        input,
        slots,
        undo,
        camera_slot,
    })
}

fn add_payload(current: usize, next: usize) -> Result<usize, NativeRenderError> {
    current
        .checked_add(next)
        .filter(|n| *n <= MAX_PAYLOAD_BYTES)
        .ok_or(NativeRenderError::PayloadLimit)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_payload_limit_includes_every_version_and_checks_overflow() {
        assert_eq!(
            add_payload(MAX_PAYLOAD_BYTES - 1, 1).unwrap(),
            MAX_PAYLOAD_BYTES
        );
        assert!(matches!(
            add_payload(MAX_PAYLOAD_BYTES, 1),
            Err(NativeRenderError::PayloadLimit)
        ));
        assert!(matches!(
            add_payload(1, usize::MAX),
            Err(NativeRenderError::PayloadLimit)
        ));
    }
}

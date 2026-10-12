use super::*;
use incant_assets_runtime::{RuntimeAssetData, TextureUsage};
use incant_render_data::{
    EnvironmentInput, MeshInput, SceneLight, SceneNode, SpriteInput, TilemapInput,
};
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
    pub undo: Vec<super::NumericUndo>,
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
            && decl.sprite.is_none()
            && decl.tilemap.is_none()
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
                super::animation::validate(node.id, source, decl.animation.as_deref())?;
                Ok::<_, NativeRenderError>(MeshInput {
                    source: Arc::clone(source),
                    cast_shadows: mesh.cast_shadows,
                    animation: node.animation,
                    animation_blend: node.animation_blend,
                    animation_pose: node.animation_pose,
                })
            })
            .transpose()?;
        let sprite = decl
            .sprite
            .as_ref()
            .map(|s| {
                let invalid = || NativeRenderError::Sprite { entity: node.id };
                let source = asset(&s.texture)?;
                let RuntimeAssetData::Texture(texture) = source.data() else {
                    return Err(invalid());
                };
                if source.usage() != Some(TextureUsage::Color) {
                    return Err(invalid());
                }
                // Animation replaces the static region. Validate and resolve
                // every prepared frame, without requiring an unused base frame
                // to still exist in the atlas.
                let selected = decl
                    .flipbook
                    .as_ref()
                    .and_then(|f| f.frames.first())
                    .copied()
                    .unwrap_or(s.region);
                let region = match selected {
                    incant_runtime::NativeSpriteRegion::Pixels(r) => r,
                    incant_runtime::NativeSpriteRegion::Atlas { frame } => {
                        let frame = ulid::Ulid::from_bytes(frame.0).to_string();
                        texture
                            .metadata
                            .atlas
                            .as_ref()
                            .and_then(|a| a.frames.get(&frame))
                            .ok_or_else(invalid)?
                            .region
                    }
                };
                let mut sprite = s.resolved_values(region);
                sprite.texture = source.info().id.clone();
                sprite.validate().map_err(|_| invalid())?;
                if u64::from(region[0]) + u64::from(region[2]) > u64::from(texture.texture.width)
                    || u64::from(region[1]) + u64::from(region[3])
                        > u64::from(texture.texture.height)
                {
                    return Err(invalid());
                }
                let frames = decl
                    .flipbook
                    .as_ref()
                    .map(|f| {
                        let regions: Vec<_> = f
                            .frames
                            .iter()
                            .map(|r| match r {
                                incant_runtime::NativeSpriteRegion::Pixels(r) => {
                                    incant_2d::SpriteRegion::Pixels(*r)
                                }
                                incant_runtime::NativeSpriteRegion::Atlas { frame } => {
                                    incant_2d::SpriteRegion::Atlas {
                                        frame: ulid::Ulid::from_bytes(frame.0).to_string(),
                                    }
                                }
                            })
                            .collect();
                        incant_render_data::SpriteFrames::resolve(source, &sprite, &regions)
                            .map(Arc::new)
                            .map_err(|_| invalid())
                    })
                    .transpose()?;
                Ok::<_, NativeRenderError>(SpriteInput {
                    source: Arc::clone(source),
                    sprite,
                    frames,
                    frame: node.flipbook.map_or(0, |p| p.frame()),
                })
            })
            .transpose()?;
        let tilemap = decl
            .tilemap
            .as_ref()
            .map(|map| {
                TilemapInput::new(Arc::clone(asset(&map.tileset)?), map)
                    .map(Arc::new)
                    .map_err(NativeRenderError::TilemapRender)
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
            sprite,
            tilemap,
            diagnostic: false,
        });
    }
    let camera_slot = camera_slot.ok_or(NativeRenderError::MissingCamera(camera))?;
    let undo = vec![
        super::NumericUndo {
            world: DMat4::IDENTITY,
            frame: 0,
            animation: None,
            animation_blend: None,
            animation_pose: None,
        };
        input.nodes.len()
    ];
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

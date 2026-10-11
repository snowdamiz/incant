//! Authoring conversion only; no asset I/O and no runtime text decoding.
use crate::{CookError, stable_id};
use incant_doc::{Entity, Project};
use incant_runtime::{
    CookedSpatial, NativeEnvironmentLight, NativeMeshRenderer, StableId, VisualAssetKind,
    VisualAssetRef,
};
use std::collections::BTreeMap;

pub(crate) fn cook(
    project: &Project,
    entity: &Entity,
    assets: &mut BTreeMap<StableId, VisualAssetRef>,
) -> Result<Option<CookedSpatial>, CookError> {
    macro_rules! decode {
        ($name:literal, $ty:ty) => {
            entity
                .components
                .get($name)
                .map(|v| serde_json::from_value::<$ty>(v.clone()))
                .transpose()?
        };
    }
    let mut node = CookedSpatial::empty(stable_id(&entity.id)?);
    node.camera = decode!("Camera", incant_doc::Camera);
    node.directional = decode!("DirectionalLight", incant_doc::DirectionalLight);
    node.point = decode!("PointLight", incant_doc::PointLight);
    node.spot = decode!("SpotLight", incant_doc::SpotLight);
    if let Some(s) = decode!("SpriteRenderer", incant_doc::SpriteRenderer) {
        let texture = bind(project, &s.texture, VisualAssetKind::Texture, assets)?;
        let region = match s.region {
            incant_doc::SpriteRegion::Pixels(r) => incant_runtime::NativeSpriteRegion::Pixels(r),
            incant_doc::SpriteRegion::Atlas { frame } => {
                incant_runtime::NativeSpriteRegion::Atlas {
                    frame: stable_id(&frame)?,
                }
            }
        };
        node.sprite = Some(Box::new(incant_runtime::NativeSpriteRenderer {
            texture,
            region,
            size: s.size,
            pivot: s.pivot,
            tint: s.tint,
            flip_x: s.flip_x,
            flip_y: s.flip_y,
            layer: s.layer,
            order: s.order,
            sampling: s.sampling,
            mode: s.mode,
        }));
    }
    if let Some(mesh) = decode!("MeshRenderer", incant_doc::MeshRenderer) {
        if !mesh.materials.is_empty() {
            return Err(CookError::MaterialOverrides(entity.id.clone()));
        }
        let mesh_id = bind(project, &mesh.mesh, VisualAssetKind::Model, assets)?;
        node.mesh = Some(NativeMeshRenderer {
            mesh: mesh_id,
            cast_shadows: mesh.cast_shadows,
        });
    }
    if let Some(env) = decode!("EnvironmentLight", incant_doc::EnvironmentLight) {
        let texture = bind(project, &env.texture, VisualAssetKind::Texture, assets)?;
        node.environment = Some(NativeEnvironmentLight {
            texture,
            intensity: env.intensity,
            rotation_degrees: env.rotation_degrees,
        });
    }
    Ok((!node.is_empty()).then_some(node))
}

fn bind(
    project: &Project,
    id: &str,
    kind: VisualAssetKind,
    bindings: &mut BTreeMap<StableId, VisualAssetRef>,
) -> Result<StableId, CookError> {
    let invalid = || CookError::VisualAsset(id.into());
    let asset = project.assets.get(id).ok_or_else(invalid)?;
    let expected = match kind {
        VisualAssetKind::Model => "model",
        VisualAssetKind::Texture => "texture",
    };
    if asset.kind != expected || asset.sha256.len() != 64 {
        return Err(invalid());
    }
    let mut fingerprint = [0; 32];
    for (byte, hex) in fingerprint
        .iter_mut()
        .zip(asset.sha256.as_bytes().as_chunks::<2>().0)
    {
        let digit = |v: u8| {
            char::from(v)
                .to_digit(16)
                .map(|v| v as u8)
                .ok_or_else(invalid)
        };
        *byte = (digit(hex[0])? << 4) | digit(hex[1])?;
    }
    let id = stable_id(id)?;
    // Same project ID always has one kind/fingerprint. The checked immutable
    // runtime asset loader must still verify the actual cache version at startup.
    bindings.insert(
        id,
        VisualAssetRef {
            id,
            kind,
            fingerprint,
        },
    );
    Ok(id)
}

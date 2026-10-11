//! Authoring-only projection. Mesh geometry is already decoded from a checked
//! cooked asset by the supplied resource provider, then embedded in the scene.
use crate::scene::{CookError, stable_id};
use incant_doc::{Entity, NavigationSourceKind, Project};
use incant_nav::NavigationResources;
use incant_runtime::{
    CookedNavigation, NativeNavigationGeometry, NativeNavigationLink, NativeNavigationMesh,
    NativeNavigationSource, VisualAssetKind, VisualAssetRef,
};

pub(crate) fn cook(
    project: &Project,
    scene: &incant_doc::Scene,
    entity: &Entity,
    resources: &NavigationResources,
) -> Result<Option<CookedNavigation>, CookError> {
    let grid = entity
        .components
        .get("NavigationGrid")
        .map(|v| serde_json::from_value(v.clone()))
        .transpose()?;
    let mesh = entity
        .components
        .get("NavigationMesh")
        .map(|v| serde_json::from_value::<incant_doc::NavigationMesh>(v.clone()))
        .transpose()?;
    if grid.is_none() && mesh.is_none() {
        return Ok(None);
    }
    let mesh = mesh
        .map(|m| {
            let sources = m
                .sources
                .into_iter()
                .map(|source| {
                    let geometry = match source.geometry {
                        NavigationSourceKind::Collider => NativeNavigationGeometry::Collider,
                        NavigationSourceKind::Mesh => {
                            let e = &scene.entities[&source.entity];
                            let binding: incant_doc::MeshRenderer =
                                serde_json::from_value(e.components["MeshRenderer"].clone())?;
                            let asset = &project.assets[&binding.mesh];
                            let resource = resources
                                .get(&binding.mesh)
                                .filter(|r| r.fingerprint == asset.sha256)
                                .ok_or_else(|| {
                                    CookError::NavigationResource(binding.mesh.clone())
                                })?;
                            let fingerprint = fingerprint(&asset.sha256)
                                .ok_or_else(|| CookError::VisualAsset(binding.mesh.clone()))?;
                            let geometry = resource.geometry().map_err(|e| {
                                CookError::Scene(incant_runtime::SceneError::Navigation(
                                    e.to_string(),
                                ))
                            })?;
                            NativeNavigationGeometry::Mesh {
                                asset: VisualAssetRef {
                                    id: stable_id(&binding.mesh)?,
                                    kind: VisualAssetKind::Model,
                                    fingerprint,
                                },
                                geometry,
                            }
                        }
                    };
                    Ok(NativeNavigationSource {
                        entity: stable_id(&source.entity)?,
                        geometry,
                    })
                })
                .collect::<Result<_, CookError>>()?;
            let links = m
                .links
                .into_iter()
                .map(|l| {
                    Ok(NativeNavigationLink {
                        id: stable_id(&l.id)?,
                        start: l.start,
                        end: l.end,
                        snap_distance: l.snap_distance,
                        bidirectional: l.bidirectional,
                        enabled: l.enabled,
                        extra_cost: l.extra_cost,
                    })
                })
                .collect::<Result<_, CookError>>()?;
            Ok::<_, CookError>(NativeNavigationMesh {
                settings: m.settings,
                sources,
                links,
            })
        })
        .transpose()?;
    Ok(Some(CookedNavigation {
        entity: stable_id(&entity.id)?,
        mesh,
        grid,
    }))
}

fn fingerprint(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut bytes = [0; 32];
    for (byte, pair) in bytes.iter_mut().zip(value.as_bytes().as_chunks::<2>().0) {
        *byte =
            ((char::from(pair[0]).to_digit(16)? << 4) | char::from(pair[1]).to_digit(16)?) as u8;
    }
    Some(bytes)
}

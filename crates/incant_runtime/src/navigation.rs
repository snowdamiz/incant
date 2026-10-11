//! Bounded cooked navigation declarations. Geometry is local to each source;
//! settings and link endpoints are world-space, as in the authoring contract.
use crate::{SceneError, StableId, VisualAssetRef};
use incant_nav::{NavigationGeometry, NavigationGrid, NavigationSettings, OffMeshLink};
use std::sync::Arc;

pub const MAX_NAVIGATION_MESHES: usize = 8;
pub const MAX_NAVIGATION_GRIDS: usize = 8;
pub const MAX_NAVIGATION_SOURCES: usize = 1024;
pub const MAX_NAVIGATION_DEPENDENCIES: usize = 4096;
pub const MAX_NAVIGATION_LINKS: usize = 128;

#[derive(Debug, Clone, PartialEq)]
pub struct CookedNavigation {
    pub entity: StableId,
    pub mesh: Option<NativeNavigationMesh>,
    pub grid: Option<NavigationGrid>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct NativeNavigationMesh {
    pub settings: NavigationSettings,
    pub sources: Vec<NativeNavigationSource>,
    pub links: Vec<NativeNavigationLink>,
}
#[derive(Debug, Clone, PartialEq)]
pub struct NativeNavigationSource {
    pub entity: StableId,
    pub geometry: NativeNavigationGeometry,
}
#[derive(Debug, Clone, PartialEq)]
pub enum NativeNavigationGeometry {
    Collider,
    /// Checked cooked-model geometry embedded by the cooker. The recipe binding
    /// detects accidental mismatches, not authentication or bytecode trust.
    Mesh {
        asset: VisualAssetRef,
        geometry: Arc<NavigationGeometry>,
    },
}
#[derive(Debug, Clone, PartialEq)]
pub struct NativeNavigationLink {
    pub id: StableId,
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub snap_distance: f32,
    pub bidirectional: bool,
    pub enabled: bool,
    pub extra_cost: f32,
}
impl NativeNavigationLink {
    pub(crate) fn algorithm(&self) -> OffMeshLink {
        OffMeshLink {
            id: id_text(self.id),
            start: self.start,
            end: self.end,
            snap_distance: self.snap_distance,
            bidirectional: self.bidirectional,
            enabled: self.enabled,
            extra_cost: self.extra_cost,
        }
    }
}
pub(crate) fn id_text(id: StableId) -> String {
    let mut value = u128::from_be_bytes(id.0);
    let mut bytes = [b'0'; 26];
    for byte in bytes.iter_mut().rev() {
        *byte = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ"[(value & 31) as usize];
        value >>= 5;
    }
    String::from_utf8(bytes.to_vec()).expect("ASCII ULID")
}
impl NativeNavigationSource {
    pub(crate) fn key(&self) -> (StableId, u8) {
        (
            self.entity,
            u8::from(matches!(
                self.geometry,
                NativeNavigationGeometry::Mesh { .. }
            )),
        )
    }
}
pub(crate) fn failure(error: impl std::fmt::Display) -> SceneError {
    SceneError::Navigation(error.to_string())
}

pub(crate) fn canonicalize(nodes: &mut [CookedNavigation]) -> Result<(), SceneError> {
    nodes.sort_by_key(|n| n.entity);
    if nodes.len() > MAX_NAVIGATION_MESHES + MAX_NAVIGATION_GRIDS
        || nodes.windows(2).any(|n| n[0].entity == n[1].entity)
    {
        return Err(SceneError::Invalid(
            "duplicate or excessive navigation declarations",
        ));
    }
    let (
        mut meshes,
        mut grids,
        mut cells,
        mut sources,
        mut vertices,
        mut triangles,
        mut tiles,
        mut links,
    ) = (0, 0, 0, 0, 0, 0, 0, 0);
    for node in nodes {
        if node.mesh.is_none() && node.grid.is_none() {
            return Err(SceneError::Invalid("empty navigation declaration"));
        }
        if let Some(grid) = &node.grid {
            grids += 1;
            cells += grid.costs.len();
            grid.validate().map_err(failure)?;
        }
        if let Some(mesh) = &mut node.mesh {
            meshes += 1;
            sources += mesh.sources.len();
            if sources > MAX_NAVIGATION_SOURCES {
                return Err(SceneError::Invalid("aggregate navigation source capacity"));
            }
            mesh.settings.validate().map_err(failure)?;
            let tile_size = mesh.settings.cell_size * f32::from(mesh.settings.tile_cells);
            tiles += ((mesh.settings.max[0] - mesh.settings.min[0]) / tile_size).ceil() as usize
                * ((mesh.settings.max[2] - mesh.settings.min[2]) / tile_size).ceil() as usize;
            links += mesh.links.len();
            if tiles > incant_nav::MAX_TILES || links > MAX_NAVIGATION_LINKS {
                return Err(SceneError::Invalid(
                    "aggregate navigation tile/link capacity",
                ));
            }
            mesh.sources.sort_by_key(NativeNavigationSource::key);
            if mesh.sources.windows(2).any(|s| s[0].key() == s[1].key()) {
                return Err(SceneError::Invalid("duplicate navigation source"));
            }
            mesh.links.sort_by_key(|l| l.id);
            incant_nav::validate_links(
                &mesh
                    .links
                    .iter()
                    .map(NativeNavigationLink::algorithm)
                    .collect::<Vec<_>>(),
            )
            .map_err(failure)?;
            for source in &mesh.sources {
                if let NativeNavigationGeometry::Mesh { asset, geometry } = &source.geometry {
                    if asset.kind != crate::VisualAssetKind::Model {
                        return Err(SceneError::Invalid("navigation mesh asset kind"));
                    }
                    vertices += geometry.vertices.len();
                    triangles += geometry.triangles.len();
                    if vertices > incant_nav::MAX_TRIANGLES * 3
                        || triangles > incant_nav::MAX_TRIANGLES
                    {
                        return Err(SceneError::Invalid(
                            "aggregate navigation geometry capacity",
                        ));
                    }
                    geometry.validate().map_err(failure)?;
                }
            }
        }
    }
    if meshes > MAX_NAVIGATION_MESHES
        || grids > MAX_NAVIGATION_GRIDS
        || cells > incant_nav::MAX_GRID_CELLS
        || sources > MAX_NAVIGATION_SOURCES
        || vertices > incant_nav::MAX_TRIANGLES * 3
        || triangles > incant_nav::MAX_TRIANGLES
    {
        return Err(SceneError::Invalid("aggregate navigation capacity"));
    }
    Ok(())
}

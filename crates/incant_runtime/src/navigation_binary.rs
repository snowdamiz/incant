//! Version-9 navigation suffix. All counts are checked before allocation.
use crate::binary::Reader;
use crate::{
    CookedNavigation, NativeNavigationGeometry, NativeNavigationLink, NativeNavigationMesh,
    NativeNavigationSource, SceneError, StableId,
};
use incant_nav::{NavigationGeometry, NavigationGrid, NavigationSettings};
use std::sync::Arc;
#[derive(Default)]
struct Budget {
    sources: usize,
    vertices: usize,
    triangles: usize,
    cells: usize,
    links: usize,
}
fn float(bytes: &mut Vec<u8>, value: f32) {
    bytes.extend((if value == 0. { 0. } else { value }).to_le_bytes());
}
fn floats(bytes: &mut Vec<u8>, values: impl IntoIterator<Item = f32>) {
    for value in values {
        float(bytes, value);
    }
}
pub(crate) fn size(nodes: &[CookedNavigation]) -> usize {
    if nodes.is_empty() {
        return 0;
    }
    4 + nodes
        .iter()
        .map(|n| {
            17 + n.grid.as_ref().map_or(0, |g| 8 + 2 * g.costs.len())
                + n.mesh.as_ref().map_or(0, |m| {
                    58 + 50 * m.links.len()
                        + m.sources
                            .iter()
                            .map(|s| {
                                17 + match &s.geometry {
                                    NativeNavigationGeometry::Collider => 0,
                                    NativeNavigationGeometry::Mesh { geometry, .. } => {
                                        57 + 12 * geometry.vertices.len()
                                            + 12 * geometry.triangles.len()
                                    }
                                }
                            })
                            .sum::<usize>()
                })
        })
        .sum::<usize>()
}
pub(crate) fn write(bytes: &mut Vec<u8>, nodes: &[CookedNavigation]) {
    bytes.extend((nodes.len() as u32).to_le_bytes());
    for node in nodes {
        bytes.extend(node.entity.0);
        bytes.push(u8::from(node.mesh.is_some()) | (u8::from(node.grid.is_some()) << 1));
        if let Some(mesh) = &node.mesh {
            let s = &mesh.settings;
            floats(bytes, s.min.into_iter().chain(s.max));
            floats(
                bytes,
                [
                    s.cell_size,
                    s.cell_height,
                    s.agent_radius,
                    s.agent_height,
                    s.max_climb,
                    s.max_slope_degrees,
                ],
            );
            bytes.extend(s.tile_cells.to_le_bytes());
            bytes.extend((mesh.sources.len() as u32).to_le_bytes());
            for source in &mesh.sources {
                bytes.extend(source.entity.0);
                match &source.geometry {
                    NativeNavigationGeometry::Collider => bytes.push(0),
                    NativeNavigationGeometry::Mesh { asset, geometry } => {
                        bytes.push(1);
                        crate::spatial_binary::write_asset(bytes, asset);
                        bytes.extend((geometry.vertices.len() as u32).to_le_bytes());
                        bytes.extend((geometry.triangles.len() as u32).to_le_bytes());
                        for v in &geometry.vertices {
                            floats(bytes, *v);
                        }
                        for t in &geometry.triangles {
                            for i in t {
                                bytes.extend(i.to_le_bytes());
                            }
                        }
                    }
                }
            }
            bytes.extend((mesh.links.len() as u32).to_le_bytes());
            for link in &mesh.links {
                bytes.extend(link.id.0);
                floats(bytes, link.start.into_iter().chain(link.end));
                float(bytes, link.snap_distance);
                bytes.push(u8::from(link.bidirectional));
                bytes.push(u8::from(link.enabled));
                float(bytes, link.extra_cost);
            }
        }
        if let Some(grid) = &node.grid {
            for n in grid.dimensions {
                bytes.extend(n.to_le_bytes());
            }
            bytes.extend((grid.costs.len() as u32).to_le_bytes());
            for n in &grid.costs {
                bytes.extend(n.to_le_bytes());
            }
        }
    }
}
fn count(r: &mut Reader<'_>, cap: usize, min: usize) -> Result<usize, SceneError> {
    let n = r.u32()? as usize;
    if n > cap || n > r.remaining_len() / min {
        return Err(SceneError::Invalid("navigation count"));
    }
    Ok(n)
}
fn f32s<const N: usize>(r: &mut Reader<'_>) -> Result<[f32; N], SceneError> {
    let mut out = [0.; N];
    for v in &mut out {
        *v = f32::from_le_bytes(r.array()?);
        if !v.is_finite() || (*v == 0. && v.is_sign_negative()) {
            return Err(SceneError::Invalid("noncanonical navigation float"));
        }
    }
    Ok(out)
}
fn boolean(r: &mut Reader<'_>) -> Result<bool, SceneError> {
    match r.byte()? {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(SceneError::Invalid("navigation boolean")),
    }
}
pub(crate) fn read(r: &mut Reader<'_>) -> Result<Vec<CookedNavigation>, SceneError> {
    let n = count(r, 16, 17)?;
    let mut nodes = Vec::with_capacity(n);
    let mut budget = Budget::default();
    for _ in 0..n {
        let entity = StableId(r.array()?);
        if nodes
            .last()
            .is_some_and(|n: &CookedNavigation| n.entity >= entity)
        {
            return Err(SceneError::Invalid("noncanonical navigation order"));
        }
        let mask = r.byte()?;
        if !(1..=3).contains(&mask) {
            return Err(SceneError::Invalid("navigation mask"));
        }
        let mesh = if mask & 1 != 0 {
            let min = f32s(r)?;
            let max = f32s(r)?;
            let [
                cell_size,
                cell_height,
                agent_radius,
                agent_height,
                max_climb,
                max_slope_degrees,
            ] = f32s(r)?;
            let tile_cells = u16::from_le_bytes(r.array()?);
            let settings = NavigationSettings {
                min,
                max,
                cell_size,
                cell_height,
                tile_cells,
                agent_radius,
                agent_height,
                max_climb,
                max_slope_degrees,
            };
            settings.validate().map_err(crate::navigation::failure)?;
            let n = count(r, crate::MAX_NAVIGATION_SOURCES - budget.sources, 17)?;
            budget.sources += n;
            let mut sources = Vec::with_capacity(n);
            for _ in 0..n {
                let entity = StableId(r.array()?);
                let geometry = match r.byte()? {
                    0 => NativeNavigationGeometry::Collider,
                    1 => {
                        let asset = crate::spatial_binary::read_asset(r)?;
                        let nv = count(r, incant_nav::MAX_TRIANGLES * 3 - budget.vertices, 1)?;
                        let nt = count(r, incant_nav::MAX_TRIANGLES - budget.triangles, 1)?;
                        if nv + nt > r.remaining_len() / 12 {
                            return Err(SceneError::Invalid("navigation geometry length"));
                        }
                        budget.vertices += nv;
                        budget.triangles += nt;
                        let mut vertices = Vec::with_capacity(nv);
                        for _ in 0..nv {
                            vertices.push(f32s(r)?);
                        }
                        let mut triangles = Vec::with_capacity(nt);
                        for _ in 0..nt {
                            triangles.push([r.u32()?, r.u32()?, r.u32()?]);
                        }
                        NativeNavigationGeometry::Mesh {
                            asset,
                            geometry: Arc::new(NavigationGeometry {
                                vertices,
                                triangles,
                            }),
                        }
                    }
                    _ => return Err(SceneError::Invalid("navigation source tag")),
                };
                let source = NativeNavigationSource { entity, geometry };
                if sources
                    .last()
                    .is_some_and(|s: &NativeNavigationSource| s.key() >= source.key())
                {
                    return Err(SceneError::Invalid("noncanonical navigation source order"));
                }
                sources.push(source);
            }
            let n = count(r, crate::MAX_NAVIGATION_LINKS - budget.links, 50)?;
            budget.links += n;
            let mut links = Vec::with_capacity(n);
            for _ in 0..n {
                let id = StableId(r.array()?);
                if links
                    .last()
                    .is_some_and(|l: &NativeNavigationLink| l.id >= id)
                {
                    return Err(SceneError::Invalid("noncanonical navigation link order"));
                }
                links.push(NativeNavigationLink {
                    id,
                    start: f32s(r)?,
                    end: f32s(r)?,
                    snap_distance: f32s::<1>(r)?[0],
                    bidirectional: boolean(r)?,
                    enabled: boolean(r)?,
                    extra_cost: f32s::<1>(r)?[0],
                });
            }
            Some(NativeNavigationMesh {
                settings,
                sources,
                links,
            })
        } else {
            None
        };
        let grid = if mask & 2 != 0 {
            let dimensions = [
                u16::from_le_bytes(r.array()?),
                u16::from_le_bytes(r.array()?),
            ];
            let n = count(r, incant_nav::MAX_GRID_CELLS - budget.cells, 2)?;
            budget.cells += n;
            if usize::from(dimensions[0]) * usize::from(dimensions[1]) != n {
                return Err(SceneError::Invalid("navigation grid length"));
            }
            let mut costs = Vec::with_capacity(n);
            for _ in 0..n {
                costs.push(u16::from_le_bytes(r.array()?));
            }
            Some(NavigationGrid { dimensions, costs })
        } else {
            None
        };
        nodes.push(CookedNavigation { entity, mesh, grid });
    }
    Ok(nodes)
}

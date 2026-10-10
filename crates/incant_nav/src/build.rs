use crate::*;
use glam::{UVec3, Vec3, Vec3A};
use rerecast::{Aabb3d, AreaType, BuildContoursFlags, DetailNavmesh, HeightfieldBuilder, TriMesh};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone)]
pub(crate) struct Portal {
    pub to: usize,
    pub a: [f32; 3],
    pub b: [f32; 3],
}
pub(crate) struct Tile {
    fingerprint: [u8; 32],
    polygons: Vec<NavigationPolygon>,
    neighbors: Vec<Vec<Option<usize>>>,
}
#[derive(Default, Clone)]
pub struct NavigationMesh {
    pub(crate) settings: Option<NavigationSettings>,
    tiles: BTreeMap<TileId, Arc<Tile>>,
    pub(crate) polygons: Vec<NavigationPolygon>,
    pub(crate) portals: Vec<Vec<Portal>>,
    generation: u64,
    authored_links: Vec<OffMeshLink>,
    pub(crate) links: Vec<crate::links::ResolvedLink>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RebuildReport {
    pub rebuilt: Vec<TileId>,
    pub reused: Vec<TileId>,
    pub removed: Vec<TileId>,
    pub polygons: usize,
    pub generation: u64,
    pub active_links: usize,
}
impl NavigationMesh {
    pub fn polygons(&self) -> &[NavigationPolygon] {
        &self.polygons
    }
    pub fn generation(&self) -> u64 {
        self.generation
    }
    /// Construct all changed tiles before swapping any state. Geometry is world-space.
    pub fn rebuild(
        &mut self,
        settings: &NavigationSettings,
        sources: &BTreeMap<String, NavigationGeometry>,
    ) -> Result<RebuildReport, NavigationError> {
        self.rebuild_with_links(settings, sources, &[])
    }
    /// Links are staged and snapped with the geometry; failure preserves the old mesh.
    pub fn rebuild_with_links(
        &mut self,
        settings: &NavigationSettings,
        sources: &BTreeMap<String, NavigationGeometry>,
        links: &[OffMeshLink],
    ) -> Result<RebuildReport, NavigationError> {
        settings.validate()?;
        validate_links(links)?;
        let mut authored_links = links.to_vec();
        authored_links.sort_by(|a, b| a.id.cmp(&b.id));
        let mut triangles = Vec::new();
        if sources.len() > 4096 {
            return Err(limit("more than 4096 geometry sources"));
        }
        for (id, source) in sources {
            if id.is_empty() || id.len() > 128 {
                return Err(invalid("invalid geometry source ID"));
            }
            source.validate()?;
            if triangles.len() + source.triangles.len() > MAX_TRIANGLES {
                return Err(limit("combined geometry exceeds 32768 triangles"));
            }
            triangles.extend(
                source
                    .triangles
                    .iter()
                    .map(|tri| tri.map(|i| source.vertices[i as usize])),
            );
        }
        let config_hash: [u8; 32] =
            Sha256::digest(serde_json::to_vec(settings).map_err(|e| invalid(e.to_string()))?)
                .into();
        let mut tiles = BTreeMap::new();
        let mut report = RebuildReport {
            rebuilt: vec![],
            reused: vec![],
            removed: vec![],
            polygons: 0,
            generation: self.generation,
            active_links: self.links.len(),
        };
        let [nx, nz] = settings.tile_counts();
        let mut raster_work = 0usize;
        let mut detail_count = 0usize;
        for z in 0..nz {
            for x in 0..nx {
                let id = TileId { x, z };
                let aabb = tile_bounds(settings, id);
                let mut mesh = TriMesh::default();
                let mut hash = Sha256::new();
                hash.update(config_hash);
                for tri in &triangles {
                    let points = tri.map(Vec3A::from_array);
                    let min = points[0].min(points[1]).min(points[2]);
                    let max = points[0].max(points[1]).max(points[2]);
                    if min.cmpgt(aabb.max.into()).any() || max.cmplt(aabb.min.into()).any() {
                        continue;
                    }
                    let extent =
                        (max.min(aabb.max.into()) - min.max(aabb.min.into())) / settings.cell_size;
                    raster_work += (extent.x.ceil() as usize + 1) * (extent.z.ceil() as usize + 1);
                    if raster_work > 8_000_000 {
                        return Err(limit(
                            "rasterization exceeds 8 million projected cell visits",
                        ));
                    }
                    for p in tri {
                        for n in p {
                            hash.update(n.to_le_bytes());
                        }
                    }
                    let index = mesh.vertices.len() as u32;
                    mesh.vertices.extend(points);
                    mesh.indices.push(UVec3::new(index, index + 1, index + 2));
                    mesh.area_types.push(AreaType::NOT_WALKABLE);
                }
                let fingerprint: [u8; 32] = hash.finalize().into();
                let tile = if let Some(previous) =
                    self.tiles.get(&id).filter(|t| t.fingerprint == fingerprint)
                {
                    report.reused.push(id);
                    previous.clone()
                } else {
                    let built = build_tile(settings, aabb, mesh, fingerprint)?;
                    report.rebuilt.push(id);
                    Arc::new(built)
                };
                report.polygons += tile.polygons.len();
                detail_count += tile
                    .polygons
                    .iter()
                    .map(|p| p.triangles.len())
                    .sum::<usize>();
                if detail_count > MAX_DETAIL_TRIANGLES {
                    return Err(limit("more than 262144 detail triangles"));
                }
                if report.polygons > MAX_POLYGONS {
                    return Err(limit("more than 32768 navigation polygons"));
                }
                tiles.insert(id, tile);
            }
        }
        report.removed = self
            .tiles
            .keys()
            .filter(|id| !tiles.contains_key(id))
            .copied()
            .collect();
        if report.rebuilt.is_empty()
            && report.removed.is_empty()
            && self.authored_links == authored_links
        {
            return Ok(report);
        }
        let (polygons, portals) = connect_tiles(settings, &tiles)?;
        report.generation = self
            .generation
            .checked_add(1)
            .ok_or_else(|| limit("generation exhausted"))?;
        let mut next = Self {
            settings: Some(settings.clone()),
            tiles,
            polygons,
            portals,
            generation: report.generation,
            authored_links,
            links: vec![],
        };
        next.links = next.resolve_links(&next.authored_links)?;
        report.active_links = next.links.len();
        *self = next;
        Ok(report)
    }
}
const CONTOUR_ERROR_CELLS: f32 = 0.25;
fn erosion_cells(s: &NavigationSettings) -> u16 {
    // Recast's 2/3 chamfer distance overestimates Euclidean distance by at most
    // sqrt(1^2 + 0.5^2). Reserve that factor and the contour simplification error
    // before rounding outwards; otherwise convex corners can cut inside radius.
    (s.agent_radius / s.cell_size * 1.118_034 + CONTOUR_ERROR_CELLS).ceil() as u16
}
fn tile_bounds(s: &NavigationSettings, id: TileId) -> Aabb3d {
    let width = s.cell_size * f32::from(s.tile_cells);
    let border = (f32::from(erosion_cells(s)) + 3.) * s.cell_size;
    let x = s.min[0] + id.x as f32 * width;
    let z = s.min[2] + id.z as f32 * width;
    // The final partial tile rounds outward by less than one voxel.
    let max_x = s.min[0] + ((s.max[0] - s.min[0]) / s.cell_size).ceil() * s.cell_size;
    let max_z = s.min[2] + ((s.max[2] - s.min[2]) / s.cell_size).ceil() * s.cell_size;
    Aabb3d {
        min: Vec3::new(x - border, s.min[1], z - border),
        max: Vec3::new(
            (x + width).min(max_x) + border,
            s.max[1],
            (z + width).min(max_z) + border,
        ),
    }
}
fn build_tile(
    s: &NavigationSettings,
    aabb: Aabb3d,
    mut input: TriMesh,
    fingerprint: [u8; 32],
) -> Result<Tile, NavigationError> {
    let build_error = |e: &dyn std::fmt::Display| NavigationError::Build(e.to_string());
    input.mark_walkable_triangles(s.max_slope_degrees.to_radians());
    let height = (s.agent_height / s.cell_height).ceil() as u16;
    let climb = (s.max_climb / s.cell_height).floor() as u16;
    let radius = erosion_cells(s);
    let mut heightfield = HeightfieldBuilder {
        aabb,
        cell_size: s.cell_size,
        cell_height: s.cell_height,
    }
    .build()
    .map_err(|e| build_error(&e))?;
    for (triangle, area) in input.indices.iter().zip(&input.area_types) {
        let points = triangle.to_array().map(|i| input.vertices[i as usize]);
        heightfield
            .rasterize_triangle(points, *area, climb)
            .map_err(|e| build_error(&e))?;
        if heightfield.allocated_spans.len() > 262_144 {
            return Err(limit("tile exceeds 262144 raster spans"));
        }
    }
    heightfield.filter_low_hanging_walkable_obstacles(climb);
    heightfield.filter_ledge_spans(height, climb);
    heightfield.filter_walkable_low_height_spans(height);
    let mut compact = heightfield
        .into_compact(height, climb)
        .map_err(|e| build_error(&e))?;
    compact.erode_walkable_area(radius);
    compact.build_distance_field();
    compact
        .build_regions(radius + 3, 0, 20)
        .map_err(|e| build_error(&e))?;
    let contours = compact.build_contours(
        CONTOUR_ERROR_CELLS,
        radius * 8,
        BuildContoursFlags::default(),
    );
    let poly = contours.into_polygon_mesh(6).map_err(|e| build_error(&e))?;
    if poly.polygon_count() > 4096 {
        return Err(limit("tile exceeds 4096 polygons"));
    }
    let detail = DetailNavmesh::new(&poly, &compact, s.cell_size, s.cell_height * 0.25)
        .map_err(|e| build_error(&e))?;
    let mut polygons = vec![];
    let mut neighbors = vec![];
    for (i, indices) in poly.polygons().enumerate() {
        let vertices: Vec<_> = indices
            .map(|index| {
                let p = poly.vertices[index as usize].as_vec3();
                (poly.aabb.min + p * Vec3::new(s.cell_size, s.cell_height, s.cell_size)).to_array()
            })
            .collect();
        if vertices.len() < 3 {
            return Err(NavigationError::Build("degenerate output polygon".into()));
        }
        neighbors.push(
            (0..vertices.len())
                .map(|j| {
                    let n = poly.polygon_neighbors[i * 6 + j];
                    if n & 0x8000 != 0 {
                        None
                    } else {
                        Some(n as usize)
                    }
                })
                .collect(),
        );
        let sub = &detail.meshes[i];
        let triangles = detail.triangles[sub.base_triangle_index as usize..]
            [..sub.triangle_count as usize]
            .iter()
            .map(|indices| {
                indices.map(|j| {
                    let mut p = detail.vertices[sub.base_vertex_index as usize + j as usize];
                    // Recast detail output adds one vertical cell to every vertex. Keep the
                    // detail surface on the same height convention as our polygon portals.
                    p.y -= s.cell_height;
                    p.to_array()
                })
            })
            .collect();
        polygons.push(NavigationPolygon {
            vertices,
            triangles,
        });
    }
    Ok(Tile {
        fingerprint,
        polygons,
        neighbors,
    })
}
type Connected = (Vec<NavigationPolygon>, Vec<Vec<Portal>>);
type BoundaryEdge = (usize, [f32; 3], [f32; 3]);
fn connect_tiles(
    s: &NavigationSettings,
    tiles: &BTreeMap<TileId, Arc<Tile>>,
) -> Result<Connected, NavigationError> {
    let mut polygons = vec![];
    let mut portals = vec![];
    let mut boundaries: BTreeMap<TileId, Vec<BoundaryEdge>> = BTreeMap::new();
    for (id, tile) in tiles {
        let base = polygons.len();
        for (i, p) in tile.polygons.iter().enumerate() {
            let mut links = vec![];
            for j in 0..p.vertices.len() {
                let a = p.vertices[j];
                let b = p.vertices[(j + 1) % p.vertices.len()];
                if let Some(n) = tile.neighbors[i][j] {
                    if n >= tile.polygons.len() {
                        return Err(NavigationError::Build("invalid polygon neighbor".into()));
                    }
                    links.push(Portal { to: base + n, a, b });
                } else {
                    boundaries.entry(*id).or_default().push((base + i, a, b));
                }
            }
            polygons.push(p.clone());
            portals.push(links);
        }
    }
    let tolerance = s.cell_size * 0.01;
    let mut comparisons = 0usize;
    for (id, edges) in &boundaries {
        for next in [
            TileId {
                x: id.x + 1,
                z: id.z,
            },
            TileId {
                x: id.x,
                z: id.z + 1,
            },
        ] {
            let Some(others) = boundaries.get(&next) else {
                continue;
            };
            for &(i, a, b) in edges {
                for &(j, c, d) in others {
                    comparisons += 1;
                    if comparisons > 4_000_000 {
                        return Err(limit("tile border connectivity work"));
                    }
                    if let Some((u, v)) = overlap(a, b, c, d, tolerance, s.max_climb) {
                        portals[i].push(Portal { to: j, a: u, b: v });
                        portals[j].push(Portal { to: i, a: v, b: u });
                    }
                }
            }
        }
    }
    Ok((polygons, portals))
}
fn overlap(
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
    d: [f32; 3],
    eps: f32,
    climb: f32,
) -> Option<([f32; 3], [f32; 3])> {
    // Only axis-aligned tile border edges can connect two tiles.
    let axis = if (a[0] - b[0]).abs() < eps
        && (c[0] - d[0]).abs() < eps
        && (a[0] - c[0]).abs() < eps
    {
        2
    } else if (a[2] - b[2]).abs() < eps && (c[2] - d[2]).abs() < eps && (a[2] - c[2]).abs() < eps {
        0
    } else {
        return None;
    };
    if (a[axis] - b[axis]).abs() < eps || (c[axis] - d[axis]).abs() < eps {
        return None;
    }
    let lo = a[axis].min(b[axis]).max(c[axis].min(d[axis]));
    let hi = a[axis].max(b[axis]).min(c[axis].max(d[axis]));
    if hi - lo < eps {
        return None;
    }
    let point = |v: f32| {
        let t = (v - a[axis]) / (b[axis] - a[axis]);
        let q = (v - c[axis]) / (d[axis] - c[axis]);
        let y = a[1] + t * (b[1] - a[1]);
        let other = c[1] + q * (d[1] - c[1]);
        if (y - other).abs() > climb + eps {
            return None;
        }
        let mut p = a;
        p[axis] = v;
        p[1] = y.max(other);
        Some(p)
    };
    let (u, v) = (point(lo)?, point(hi)?);
    Some(if a[axis] < b[axis] { (u, v) } else { (v, u) })
}

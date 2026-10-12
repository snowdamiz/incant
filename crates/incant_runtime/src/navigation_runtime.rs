//! Retained derived state. Numeric sync compares only bounded dependency values;
//! topology/configuration walks and geometry expansion happen at explicit changes.
use crate::navigation::*;
use crate::{CookedNavigation, NativeWorld, RuntimeError, SceneError, StableId, VisualAssetRef};
use glam::DMat4;
use incant_nav::{
    GridPath, GridPathRequest, NavigationGeometry, NavigationMesh, NavigationPath, PathRequest,
    PreparedGrid, RebuildReport,
};
use incant_types::Collider;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub(crate) struct Input<'a> {
    pub parent: Option<StableId>,
    pub world: DMat4,
    pub stationary: bool,
    pub collider: Option<&'a Collider>,
    pub mesh: Option<VisualAssetRef>,
}
#[derive(Clone)]
struct MeshEntry {
    poses: Vec<DMat4>,
    local: Vec<Arc<NavigationGeometry>>,
    mesh: NavigationMesh,
    report: RebuildReport,
}
#[derive(Clone, Default)]
pub(crate) struct NavigationRuntime {
    pub nodes: Vec<Arc<CookedNavigation>>,
    meshes: BTreeMap<StableId, Arc<MeshEntry>>,
    grids: BTreeMap<StableId, (Arc<PreparedGrid>, u64)>,
    dependencies: BTreeSet<StableId>,
    generation: u64,
}
/// Immutable committed path-query data. At most eight mesh and eight grid
/// handles are copied; geometry, grids and solvers remain shared native values.
/// Retained snapshots intentionally observe their captured generation.
pub struct NavigationSnapshot {
    meshes: BTreeMap<StableId, Arc<MeshEntry>>,
    grids: BTreeMap<StableId, (Arc<PreparedGrid>, u64)>,
}
impl NavigationSnapshot {
    pub fn navigation_path(
        &self,
        entity: StableId,
        request: &PathRequest,
    ) -> Result<Option<NavigationPath>, SceneError> {
        mesh_path(&self.meshes, entity, request)
    }
    pub fn grid_path(
        &self,
        entity: StableId,
        request: &GridPathRequest,
    ) -> Result<Option<GridPath>, SceneError> {
        grid_path(&self.grids, entity, request)
    }
}
impl std::fmt::Debug for NavigationRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavigationRuntime")
            .field("nodes", &self.nodes)
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}
impl NavigationRuntime {
    /// Restored caches are freshly derived. Invalidate all paths returned by the
    /// old incarnation, even when the declarations and geometry are identical.
    pub(crate) fn invalidate_after_restore(&mut self, previous: &Self) -> Result<(), SceneError> {
        let generation = bump(previous.generation)?;
        self.generation = generation;
        for entry in self.meshes.values_mut() {
            Arc::make_mut(entry).report.generation = generation;
        }
        for (_, value) in self.grids.values_mut() {
            *value = generation;
        }
        Ok(())
    }

    pub fn prepare<'a>(
        &self,
        nodes: Vec<Arc<CookedNavigation>>,
        input: impl Fn(StableId) -> Option<Input<'a>>,
    ) -> Result<Self, SceneError> {
        let mut next = Self {
            nodes,
            generation: self.generation,
            ..Self::default()
        };
        let mut total_triangles = 0;
        let mut total_vertices = 0;
        for node in &next.nodes {
            input(node.entity).ok_or(SceneError::Invalid("navigation owner is absent"))?;
            if let Some(grid) = &node.grid {
                let old = self
                    .grids
                    .get(&node.entity)
                    .filter(|(g, _)| g.grid() == grid);
                let entry = match old {
                    Some(old) => old.clone(),
                    None => {
                        next.generation = bump(next.generation)?;
                        (
                            Arc::new(PreparedGrid::new(grid.clone()).map_err(failure)?),
                            next.generation,
                        )
                    }
                };
                next.grids.insert(node.entity, entry);
            }
            if let Some(config) = &node.mesh {
                let mut poses = Vec::with_capacity(config.sources.len());
                let mut local = Vec::with_capacity(config.sources.len());
                for source in &config.sources {
                    let state = input(source.entity)
                        .ok_or(SceneError::Invalid("navigation source is absent"))?;
                    let mut cursor = Some(source.entity);
                    let mut visited = BTreeSet::new();
                    while let Some(id) = cursor {
                        if !visited.insert(id) {
                            return Err(SceneError::Invalid("navigation ancestry cycle"));
                        }
                        next.dependencies.insert(id);
                        if next.dependencies.len() > MAX_NAVIGATION_DEPENDENCIES {
                            return Err(SceneError::Invalid("navigation dependency capacity"));
                        }
                        let state = input(id)
                            .ok_or(SceneError::Invalid("navigation ancestor is absent"))?;
                        if !state.stationary {
                            return Err(SceneError::Invalid(
                                "navigation sources and ancestors must be stationary",
                            ));
                        }
                        cursor = state.parent;
                    }
                    let geometry =
                        match &source.geometry {
                            NativeNavigationGeometry::Collider => {
                                let collider = state.collider.filter(|c| !c.sensor).ok_or(
                                    SceneError::Invalid("navigation requires a nonsensor Collider"),
                                )?;
                                Arc::new(
                                    incant_physics_runtime::navigation_geometry(&collider.shape)
                                        .map_err(failure)?,
                                )
                            }
                            NativeNavigationGeometry::Mesh { asset, geometry } => {
                                if state.mesh != Some(*asset) {
                                    return Err(SceneError::Invalid(
                                        "navigation model binding mismatch",
                                    ));
                                }
                                geometry.clone()
                            }
                        };
                    total_triangles += geometry.triangles.len();
                    total_vertices += geometry.vertices.len();
                    if total_triangles > incant_nav::MAX_TRIANGLES
                        || total_vertices > incant_nav::MAX_TRIANGLES * 3
                    {
                        return Err(SceneError::Invalid(
                            "aggregate navigation geometry capacity",
                        ));
                    }
                    poses.push(state.world);
                    local.push(geometry);
                }
                let old = self.meshes.get(&node.entity);
                let same_config = self
                    .nodes
                    .iter()
                    .find(|n| n.entity == node.entity)
                    .is_some_and(|n| n.mesh.as_ref() == Some(config));
                if let Some(old) =
                    old.filter(|old| same_config && old.poses == poses && old.local == local)
                {
                    next.meshes.insert(node.entity, old.clone());
                } else {
                    let entry = rebuild(
                        config,
                        poses,
                        local,
                        old.map(Arc::as_ref),
                        &mut next.generation,
                    )?;
                    next.meshes.insert(node.entity, Arc::new(entry));
                }
            }
        }
        if self
            .nodes
            .iter()
            .any(|n| !next.nodes.iter().any(|new| new.entity == n.entity))
            && next.generation == self.generation
        {
            next.generation = bump(next.generation)?;
        }
        next.check_derived_capacity()?;
        Ok(next)
    }
    /// No allocation or geometry copying when every bounded source pose agrees.
    /// Dependency ancestry was expanded once at the previous structural boundary.
    pub fn current(
        &self,
        pose: impl Fn(StableId) -> DMat4,
        stationary: impl Fn(StableId) -> bool,
    ) -> bool {
        self.dependencies.iter().all(|&id| stationary(id))
            && self.nodes.iter().all(|n| {
                n.mesh.as_ref().is_none_or(|m| {
                    m.sources
                        .iter()
                        .zip(&self.meshes[&n.entity].poses)
                        .all(|(s, p)| pose(s.entity) == *p)
                })
            })
    }
    pub fn motion(
        &self,
        pose: impl Fn(StableId) -> DMat4,
        stationary: impl Fn(StableId) -> bool,
    ) -> Result<Option<Self>, SceneError> {
        for &id in &self.dependencies {
            if !stationary(id) {
                return Err(SceneError::Invalid(
                    "navigation sources and ancestors must be stationary",
                ));
            }
        }
        let changed = self
            .nodes
            .iter()
            .filter_map(|n| n.mesh.as_ref().map(|m| (n.entity, m)))
            .any(|(id, m)| {
                m.sources
                    .iter()
                    .zip(&self.meshes[&id].poses)
                    .any(|(s, p)| pose(s.entity) != *p)
            });
        if !changed {
            return Ok(None);
        }
        let mut next = self.clone();
        for node in &self.nodes {
            let Some(config) = &node.mesh else {
                continue;
            };
            let old = &self.meshes[&node.entity];
            if config
                .sources
                .iter()
                .zip(&old.poses)
                .all(|(s, p)| pose(s.entity) == *p)
            {
                continue;
            }
            let poses = config.sources.iter().map(|s| pose(s.entity)).collect();
            let entry = rebuild(
                config,
                poses,
                old.local.clone(),
                Some(old),
                &mut next.generation,
            )?;
            next.meshes.insert(node.entity, Arc::new(entry));
        }
        next.check_derived_capacity()?;
        Ok(Some(next))
    }
    fn check_derived_capacity(&self) -> Result<(), SceneError> {
        // Only on rebuild/structural paths; never scan polygon/detail storage on
        // an unchanged tick or query. Old published entries remain intact on error.
        let polygons: usize = self
            .meshes
            .values()
            .map(|entry| entry.mesh.polygons().len())
            .sum();
        let details: usize = self
            .meshes
            .values()
            .flat_map(|entry| entry.mesh.polygons())
            .map(|polygon| polygon.triangles.len())
            .sum();
        if polygons > incant_nav::MAX_POLYGONS || details > incant_nav::MAX_DETAIL_TRIANGLES {
            return Err(SceneError::Invalid(
                "aggregate navigation polygon/detail capacity",
            ));
        }
        Ok(())
    }
}
fn bump(generation: u64) -> Result<u64, SceneError> {
    generation
        .checked_add(1)
        .ok_or(SceneError::Invalid("navigation generation exhausted"))
}
fn rebuild(
    config: &crate::NativeNavigationMesh,
    poses: Vec<DMat4>,
    local: Vec<Arc<NavigationGeometry>>,
    old: Option<&MeshEntry>,
    generation: &mut u64,
) -> Result<MeshEntry, SceneError> {
    let mut sources = BTreeMap::new();
    for (source, (geometry, pose)) in config.sources.iter().zip(local.iter().zip(&poses)) {
        // Stable source keys retain triangle ordering when another source is
        // inserted or removed, so unrelated tiles keep their fingerprints.
        sources.insert(
            format!("{}:{}", id_text(source.entity), source.key().1),
            geometry
                .transformed(pose.to_cols_array_2d())
                .map_err(failure)?,
        );
    }
    let mut mesh = old.map_or_else(NavigationMesh::default, |old| old.mesh.clone());
    let links: Vec<_> = config
        .links
        .iter()
        .map(NativeNavigationLink::algorithm)
        .collect();
    let mut report = mesh
        .rebuild_with_links(&config.settings, &sources, &links)
        .map_err(failure)?;
    if let Some(old) = old.filter(|old| old.mesh.generation() == mesh.generation()) {
        report.generation = old.report.generation;
    } else {
        *generation = bump(*generation)?;
        report.generation = *generation;
    }
    Ok(MeshEntry {
        poses,
        local,
        mesh,
        report,
    })
}
impl NativeWorld {
    pub(crate) fn ensure_navigation_current(&self) -> Result<(), RuntimeError> {
        self.ensure_usable()?;
        if !self.navigation.current(
            |id| {
                self.world
                    .get::<crate::world::Global>(self.entities[&id])
                    .unwrap()
                    .0
            },
            |id| {
                self.world
                    .get::<crate::world::Motion>(self.entities[&id])
                    .is_none_or(|m| m.0.linear == [0.; 3])
            },
        ) {
            return Err(SceneError::Invalid(
                "navigation requires a committed apply or step after numeric writes",
            )
            .into());
        }
        Ok(())
    }

    pub fn navigation_path(
        &self,
        entity: StableId,
        request: &PathRequest,
    ) -> Result<Option<NavigationPath>, RuntimeError> {
        self.ensure_navigation_current()?;
        self.navigation.path(entity, request).map_err(Into::into)
    }
    /// Capture only committed read-only cache handles for copied foreign queries.
    pub fn navigation_snapshot(&self) -> Result<NavigationSnapshot, RuntimeError> {
        self.ensure_navigation_current()?;
        Ok(NavigationSnapshot {
            meshes: self.navigation.meshes.clone(),
            grids: self.navigation.grids.clone(),
        })
    }
    pub fn grid_path(
        &self,
        entity: StableId,
        request: &GridPathRequest,
    ) -> Result<Option<GridPath>, RuntimeError> {
        self.ensure_navigation_current()?;
        self.navigation
            .grid_path(entity, request)
            .map_err(Into::into)
    }
    /// Report for the last actual rebuild; unchanged ticks retain it and its generation.
    pub fn navigation_report(&self, entity: StableId) -> Result<&RebuildReport, RuntimeError> {
        self.ensure_navigation_current()?;
        self.navigation
            .meshes
            .get(&entity)
            .map(|e| &e.report)
            .ok_or_else(|| SceneError::Invalid("navigation mesh is absent").into())
    }
    pub(crate) fn navigation_motion(&self) -> Result<Option<NavigationRuntime>, SceneError> {
        self.navigation.motion(
            |id| {
                self.world
                    .get::<crate::world::Global>(self.entities[&id])
                    .unwrap()
                    .0
            },
            |id| {
                self.world
                    .get::<crate::world::Motion>(self.entities[&id])
                    .is_none_or(|m| m.0.linear == [0.; 3])
            },
        )
    }
}

// Pure retained-cache reads shared by ordinary world queries and the exclusive
// numeric loan. Neither path can rebuild from pending ECS writes.
impl NavigationRuntime {
    pub(crate) fn path(
        &self,
        entity: StableId,
        request: &PathRequest,
    ) -> Result<Option<NavigationPath>, SceneError> {
        mesh_path(&self.meshes, entity, request)
    }
    pub(crate) fn grid_path(
        &self,
        entity: StableId,
        request: &GridPathRequest,
    ) -> Result<Option<GridPath>, SceneError> {
        grid_path(&self.grids, entity, request)
    }
}
fn mesh_path(
    meshes: &BTreeMap<StableId, Arc<MeshEntry>>,
    entity: StableId,
    request: &PathRequest,
) -> Result<Option<NavigationPath>, SceneError> {
    let entry = meshes
        .get(&entity)
        .ok_or(SceneError::Invalid("navigation mesh is absent"))?;
    Ok(entry
        .mesh
        .find_path(request)
        .map_err(failure)?
        .map(|mut path| {
            path.generation = entry.report.generation;
            path
        }))
}
fn grid_path(
    grids: &BTreeMap<StableId, (Arc<PreparedGrid>, u64)>,
    entity: StableId,
    request: &GridPathRequest,
) -> Result<Option<GridPath>, SceneError> {
    let (entry, generation) = grids
        .get(&entity)
        .ok_or(SceneError::Invalid("navigation grid is absent"))?;
    Ok(entry.find_path(request).map_err(failure)?.map(|mut path| {
        path.generation = *generation;
        path
    }))
}

//! Explicit connections between walkable surfaces. Gameplay owns traversal.
use crate::{NavigationError, NavigationMesh, invalid, limit};
use glam::Vec3;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct OffMeshLink {
    /// Stable project ULID used by gameplay to choose a traversal behavior.
    pub id: String,
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub snap_distance: f32,
    pub bidirectional: bool,
    pub enabled: bool,
    /// Nonnegative distance-equivalent penalty, added to straight-line length.
    pub extra_cost: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OffMeshTraversal {
    pub link_id: String,
    /// The consecutive point pair crosses a link, not walkable mesh.
    pub from_index: u32,
    pub to_index: u32,
    pub reversed: bool,
}
#[derive(Clone)]
pub(crate) struct ResolvedLink {
    pub id: String,
    pub first: usize,
    pub last: usize,
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub bidirectional: bool,
    pub extra_cost: f32,
}
pub fn validate_links(links: &[OffMeshLink]) -> Result<(), NavigationError> {
    if links.len() > 128 {
        return Err(limit("navigation exceeds 128 off-mesh links"));
    }
    let mut ids = BTreeSet::new();
    for link in links {
        if link.id.len() != 26
            || !link
                .id
                .bytes()
                .all(|b| b"0123456789ABCDEFGHJKMNPQRSTVWXYZ".contains(&b))
            || link.id.as_bytes()[0] > b'7'
            || !ids.insert(&link.id)
        {
            return Err(invalid("off-mesh links require distinct canonical ULIDs"));
        }
        if !link
            .start
            .iter()
            .chain(&link.end)
            .all(|n| n.is_finite() && n.abs() <= 100_000.)
            || !link.snap_distance.is_finite()
            || !(0.001..=10.).contains(&link.snap_distance)
            || !link.extra_cost.is_finite()
            || !(0. ..=100_000.).contains(&link.extra_cost)
            || Vec3::from_array(link.start).distance_squared(Vec3::from_array(link.end)) < 1e-6
        {
            return Err(invalid("invalid off-mesh endpoints, snap distance or cost"));
        }
    }
    Ok(())
}
impl NavigationMesh {
    pub(crate) fn resolve_links(
        &self,
        links: &[OffMeshLink],
    ) -> Result<Vec<ResolvedLink>, NavigationError> {
        let triangles: usize = self.polygons.iter().map(|p| p.triangles.len()).sum();
        if triangles
            .saturating_mul(links.iter().filter(|l| l.enabled).count())
            .saturating_mul(2)
            > 2_000_000
        {
            return Err(limit(
                "off-mesh endpoint snapping exceeds 2 million triangle checks",
            ));
        }
        let mut output = vec![];
        for link in links.iter().filter(|l| l.enabled) {
            let (first, start) = self
                .nearest(link.start, link.snap_distance)
                .ok_or_else(|| {
                    invalid(format!(
                        "off-mesh link {} start has no nearby walkable surface",
                        link.id
                    ))
                })?;
            let (last, end) = self.nearest(link.end, link.snap_distance).ok_or_else(|| {
                invalid(format!(
                    "off-mesh link {} end has no nearby walkable surface",
                    link.id
                ))
            })?;
            if Vec3::from_array(start).distance_squared(Vec3::from_array(end)) < 1e-6 {
                return Err(invalid("off-mesh endpoints snap to the same point"));
            }
            output.push(ResolvedLink {
                id: link.id.clone(),
                first,
                last,
                start,
                end,
                bidirectional: link.bidirectional,
                extra_cost: link.extra_cost,
            });
        }
        output.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(output)
    }
}

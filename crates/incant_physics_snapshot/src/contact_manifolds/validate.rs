use super::*;
use crate::{
    Budget, Dimension, Error, GeometryKind, MAX_BYTES, MAX_ITEMS, Result,
    contact_points::validate_contact_slices, vector,
};

pub(super) const HEADER: usize = 68;
pub(super) fn frame_length(counts: [usize; 5]) -> Result<usize> {
    counts
        .into_iter()
        .zip([8, 32, 188, 100, 48])
        .try_fold(HEADER, |v, (n, stride)| {
            if n > MAX_ITEMS {
                return Err(Error::Limit);
            }
            n.checked_mul(stride)
                .and_then(|b| v.checked_add(b))
                .filter(|v| *v <= MAX_BYTES)
                .ok_or(Error::Limit)
        })
}
pub(super) fn context_lengths(
    identity: Identity,
    counts: [usize; 5],
    c: ManifoldContext<'_>,
) -> Result<()> {
    if c.journals != PendingJournals::default() {
        return Err(Error::PendingMutation);
    }
    if identity != c.graph.record().identity || identity != c.attachments.record().identity {
        return Err(Error::Identity);
    }
    if counts[0] != c.graph.record().nodes.len()
        || counts[1] != c.graph.record().edges.len()
        || c.geometries.len() != c.attachments.record().colliders.len()
    {
        return Err(Error::Length);
    }
    frame_length(counts)?;
    for g in c.geometries {
        if g.record().identity != identity {
            return Err(Error::Identity);
        }
    }
    Ok(())
}
fn span(s: RecordSpan, end: &mut usize, total: usize) -> Result<std::ops::Range<usize>> {
    if s.start as usize != *end {
        return Err(Error::Ownership);
    }
    let start = *end;
    *end = end.checked_add(s.count as usize).ok_or(Error::Limit)?;
    if *end > total {
        return Err(Error::Ownership);
    }
    Ok(start..*end)
}
fn normal(v: [u32; 3], d: Dimension, zero_allowed: bool) -> Result<()> {
    if d == Dimension::Planar && v[2] != 0 {
        return Err(Error::Manifold);
    }
    let norm = v
        .into_iter()
        .map(|b| f64::from(f32::from_bits(b)).powi(2))
        .sum::<f64>();
    // Explicit supported numeric domain, preserving every admitted bit. Same
    // strict f64 tolerance as inert local rotations; no rounded normalization.
    if norm.is_finite() && ((zero_allowed && norm == 0.) || (norm - 1.).abs() < 2e-4) {
        Ok(())
    } else {
        Err(Error::Manifold)
    }
}
fn metadata(m: &ContactManifoldRecord, d: Dimension) -> Result<()> {
    for n in m.local_normals {
        normal(n, d, m.points.count == 0)?;
    }
    normal(m.normal, d, m.solver.count == 0)?;
    for bits in [m.friction, m.restitution] {
        let v = f32::from_bits(bits);
        if !v.is_finite() || v < 0. {
            return Err(Error::Manifold);
        }
    }
    // Native effective dominance is i8 for dynamic/kinematic or 128 for fixed.
    // Relative values are in -256..=256. Actual body-type equality is pending.
    if !(-256..=256).contains(&m.relative_dominance)
        || m.solver_flags & !1 != 0
        || !(m.solver_color <= 128 || m.solver_color == 255)
        || m.solver_body_ids
            .into_iter()
            .any(|i| i != u32::MAX && i as usize >= MAX_ITEMS)
        || (m.graph_position != u32::MAX
            && (m.graph_position >> 22 > 129
                || (m.graph_position & 0x3f_ffff) as usize >= MAX_ITEMS))
    {
        return Err(Error::Manifold);
    }
    Ok(())
}
fn shape_side(
    m: &ContactManifoldRecord,
    side: usize,
    cluster: bool,
    g: &ValidatedGeometry,
) -> Result<()> {
    let g = g.record();
    let child = g
        .parts
        .get(m.subshapes[side] as usize)
        .ok_or(Error::Geometry)?;
    let actual = m.subshape_poses.and_then(|p| p[side]);
    let expected = if cluster || g.kind == GeometryKind::Primitive {
        None
    } else {
        Some(LocalPose {
            translation: child.translation,
            rotation: if g.identity.dimension == Dimension::Planar {
                [1f32.to_bits(), 0, 0, 0]
            } else {
                child.rotation
            },
        })
    };
    if actual != expected {
        return Err(Error::Geometry);
    }
    // Clusters use collider-space points and retain just a representative child
    // id. Their per-point mixed-child provenance is NOT established by this.
    Ok(())
}
pub fn validate_manifolds<'a>(
    record: ManifoldTable,
    c: ManifoldContext<'a>,
    budget: &mut Budget,
) -> Result<ValidatedManifolds<'a>> {
    context_lengths(
        record.identity,
        [
            record.nodes.len(),
            record.pairs.len(),
            record.manifolds.len(),
            record.points.len(),
            record.solver.len(),
        ],
        c,
    )?;
    let g = c.graph.record();
    let collider_count = c.geometries.len();
    budget.charge(
        collider_count
            .checked_add(
                record
                    .pairs
                    .len()
                    .checked_mul(size_of::<[u32; 2]>())
                    .ok_or(Error::Limit)?,
            )
            .ok_or(Error::Limit)?,
    )?;
    let mut nodes = vector::<u8>(collider_count)?;
    nodes.resize(collider_count, 0);
    let mut pairs = vector::<[u32; 2]>(record.pairs.len())?;
    for h in &record.nodes {
        let row = c.attachments.collider_row(*h).ok_or(Error::Generation)? as usize;
        if nodes[row] != 0 {
            return Err(Error::Ownership);
        }
        nodes[row] = 1;
    }
    for e in &g.edges {
        let p = &record.pairs[e.row as usize];
        let handles = e
            .node
            .map(|n| record.nodes[g.nodes[n as usize].row as usize]);
        if handles != p.colliders || e.node[0] == e.node[1] {
            return Err(Error::Ownership);
        }
        pairs.push([e.node[0].min(e.node[1]), e.node[0].max(e.node[1])]);
    }
    pairs.sort_unstable();
    if pairs.windows(2).any(|p| p[0] == p[1]) {
        return Err(Error::Ownership);
    }
    let mut mend = 0;
    let mut pend = 0;
    let mut send = 0;
    for pair in &record.pairs {
        if record.identity.dimension == Dimension::Planar && pair.clusters.count != 0 {
            return Err(Error::Manifold);
        }
        for (range, cluster) in [(pair.plain, false), (pair.clusters, true)] {
            for i in span(range, &mut mend, record.manifolds.len())? {
                let m = &record.manifolds[i];
                if cluster && m.points.count > 255 {
                    return Err(Error::Manifold);
                }
                let points = span(m.points, &mut pend, record.points.len())?;
                let solver = span(m.solver, &mut send, record.solver.len())?;
                validate_contact_slices(
                    record.identity.dimension,
                    &record.points[points],
                    &record.solver[solver],
                )?;
                metadata(m, record.identity.dimension)?;
                for side in 0..2 {
                    let h = pair.colliders[side];
                    let body = c
                        .attachments
                        .collider_parent(h)
                        .ok_or(Error::Generation)?
                        .map(|p| p.body);
                    if m.bodies[side] != body {
                        return Err(Error::Ownership);
                    }
                    let row = c.attachments.collider_row(h).ok_or(Error::Generation)? as usize;
                    shape_side(m, side, cluster, &c.geometries[row])?;
                }
            }
        }
    }
    if mend != record.manifolds.len() || pend != record.points.len() || send != record.solver.len()
    {
        return Err(Error::Ownership);
    }
    Ok(ValidatedManifolds { record, context: c })
}

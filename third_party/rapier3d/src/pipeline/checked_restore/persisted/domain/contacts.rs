//! Consumed contact identity and cache admission, with exact saved bits retained.
use super::{E, Result, dominance};
use crate::pipeline::checked_restore::{NumericFrame, convert, derived_shape::DerivedShape};
use incant_physics_snapshot::{CombineRule, GeometryKind, Primitive, ValidatedGeometry};
use parry::math::VectorExt;
use parry::shape::{Cuboid, PackedFeatureId, Shape};
fn combine(a: u32, b: u32, ar: CombineRule, br: CombineRule) -> u32 {
    use crate::dynamics::CoefficientCombineRule as N;
    let rule = |r| match r {
        CombineRule::Average => N::Average,
        CombineRule::Min => N::Min,
        CombineRule::Multiply => N::Multiply,
        CombineRule::Max => N::Max,
    };
    N::combine(f32::from_bits(a), f32::from_bits(b), rule(ar), rule(br)).to_bits()
}
fn radius(g: &ValidatedGeometry) -> Result<f32> {
    let bounds = match DerivedShape::new(g)? {
        DerivedShape::Box(s) => s.compute_local_aabb(),
        DerivedShape::Ball(s) => s.compute_local_aabb(),
        DerivedShape::Capsule(s) => s.compute_local_aabb(),
        DerivedShape::Compound(c) => {
            let b = c.root_aabb();
            parry::bounding_volume::Aabb::new(convert::v(b.mins), convert::v(b.maxs))
        }
    };
    let r = bounds.mins.length().max(bounds.maxs.length());
    if !r.is_finite() {
        return Err(E::Arithmetic);
    }
    Ok(r)
}
pub(super) fn admit(n: &NumericFrame<'_>) -> Result<()> {
    let f = n.boundary();
    let rigid = f.broad().context().narrow.context().rigid;
    let cs = rigid.collider_state();
    let bs = cs.context().bodies;
    let table = f.manifolds().record();
    let pairs = rigid.pairs();
    let config = f.pipeline().context().integration;
    let recycle = f32::from_bits(config.record().normalized_contact_recycle_distance)
        * f32::from_bits(config.record().length_unit);
    let prediction = f32::from_bits(config.prediction_distance());
    if !recycle.is_finite() {
        return Err(E::Arithmetic);
    }
    for (row, pair) in table.pairs.iter().enumerate() {
        let rows = pair
            .colliders
            .map(|h| f.attachments().collider_row(h).ok_or(E::Contact));
        let rows = [rows[0]?, rows[1]?];
        let geometries = rows.map(|r| &cs.context().geometries[r as usize]);
        let colliders = rows.map(|r| &cs.record().colliders[r as usize]);
        let span = pair.plain;
        let ordinary = &table.manifolds[span.start as usize..(span.start + span.count) as usize];
        if let Some(cache) = pairs.record().pairs[row].recycle {
            let extent = radius(geometries[0])?.max(radius(geometries[1])?);
            let cap = if ordinary.iter().any(|m| m.solver.count != 0) {
                recycle
            } else {
                recycle.min(prediction)
            };
            // Smaller historical drift limits only force earlier regeneration.
            // Larger/current-shape-inconsistent caches could skip required work.
            if cache.max_extent != extent.to_bits() || f32::from_bits(cache.max_drift) > cap {
                return Err(E::Recycle);
            }
        }
        if !cfg!(feature = "enhanced-determinism") {
            let d = pairs.detectors(row as u32).ok_or(E::Contact)?;
            if d.windows(2).any(|x| x[0].key >= x[1].key) {
                return Err(E::WorkspaceOrder);
            }
        }
        for m in ordinary {
            if m.points.count > u8::MAX as u32 || m.solver.count > 4 {
                return Err(E::Contact);
            }
            if m.solver.count != 0 {
                let expected = dominance(m.bodies[0].and_then(|h| bs.body(h)))
                    - dominance(m.bodies[1].and_then(|h| bs.body(h)));
                if m.relative_dominance != expected {
                    return Err(E::Dominance);
                }
                let [a, b] = colliders.map(|c| c.material);
                if m.friction != combine(a.friction, b.friction, a.friction_rule, b.friction_rule)
                    || m.restitution
                        != combine(
                            a.restitution,
                            b.restitution,
                            a.restitution_rule,
                            b.restitution_rule,
                        )
                {
                    return Err(E::Material);
                }
            }
            let shapes = [0, 1].map(|i| {
                let g = geometries[i].record();
                g.parts[if g.kind == GeometryKind::Primitive {
                    0
                } else {
                    m.subshapes[i] as usize
                }]
                .shape
            });
            for point in
                &table.points[m.points.start as usize..(m.points.start + m.points.count) as usize]
            {
                for i in 0..2 {
                    if !feature(shapes[i], point.features[i]) {
                        return Err(E::Feature);
                    }
                }
            }
        }
    }
    Ok(())
}
/// Native point-projection IDs and polygonal clipping IDs are different for a
/// cuboid. Enumerate the latter through the pinned support-feature routine;
/// never mistake them for dense face/edge indices. UNKNOWN is an actual GJK
/// result and stays admissible. No feature-normal call consumes a guessed index.
fn feature(shape: Primitive, word: u32) -> bool {
    if word == PackedFeatureId::UNKNOWN.0 {
        return true;
    }
    let tag = word >> 30;
    let id = word & 0x3fff_ffff;
    match shape {
        Primitive::Ball { .. } => tag == 3 && id == 0,
        Primitive::CapsuleY { .. } => {
            if cfg!(feature = "dim2") {
                (tag == 3 && id <= 2) || (tag == 1 && (id == 0 || id == 2))
            } else {
                (tag == 3 && id == 0) || (tag == 1 && id <= 1) || (tag == 2 && id == 0)
            }
        }
        Primitive::Box { half_extents } => {
            let d = crate::math::DIM as u32;
            if (tag == 1 && id < (1 << d)) || (tag == 3 && id < 2 * d) {
                return true;
            }
            #[cfg(feature = "dim3")]
            if tag == 2 && (id & 3) < 3 && id >> 2 < 8 && (id >> 2) & (1 << (id & 3)) == 0 {
                return true;
            }
            let shape = Cuboid::new(convert::v(half_extents));
            for axis in 0..crate::math::DIM {
                for sign in [-1., 1.] {
                    let dir = crate::math::Vector::ith(axis, sign);
                    let f = shape.support_feature(dir);
                    if word == f.fid.0 || f.vids[..f.num_vertices].iter().any(|x| x.0 == word) {
                        return true;
                    }
                    #[cfg(feature = "dim3")]
                    if f.eids[..f.num_vertices].iter().any(|x| x.0 == word) {
                        return true;
                    }
                }
            }
            false
        }
    }
}

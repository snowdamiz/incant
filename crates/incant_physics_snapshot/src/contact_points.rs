use crate::{Budget, Dimension, Error, Identity, MAX_BYTES, MAX_ITEMS, Result};

/// Rapier f32 contact index bit, not a generation or feature flag.
pub const NEW_CONTACT_BIT: u32 = 1 << 31;

/// Exact pinned ContactData fields. Every scalar is an f32 bit pattern.
/// Planar tangent impulses use element 0; all inactive entries must be +0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ContactWarmstart {
    pub impulse: u32,
    pub tangent_impulse: [u32; 2],
    pub warmstart_impulse: u32,
    pub warmstart_tangent_impulse: [u32; 2],
    pub warmstart_twist_impulse: u32,
    pub warmstart_tangent_world: [u32; 3],
    pub solver_dp1: [u32; 3],
    pub solver_dp2: [u32; 3],
}

/// One tracked point, including the warm-start source consumed by the solver.
/// Feature words retain their top-two-bit kind and low-30-bit index. Zero is
/// UNKNOWN; no shape-specific feature correspondence is established here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackedContactPoint {
    pub local_p1: [u32; 3],
    pub local_p2: [u32; 3],
    pub distance: u32,
    pub features: [u32; 2],
    pub data: ContactWarmstart,
}

/// One solver row in native order. Anchors are CoM-local or world-space according
/// to the enclosing manifold's body/dominance state, which this section cannot
/// authenticate. `contact_id` retains the high new-contact bit unchanged.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SolverContactPoint {
    pub anchor1: [u32; 3],
    pub anchor2: [u32; 3],
    pub distance: u32,
    pub tangent_velocity: [u32; 3],
    pub contact_id: u32,
    /// Explicit Rapier 3D SIMD padding; +0 also in this format's 2D row.
    pub padding: u32,
}
impl SolverContactPoint {
    pub fn point_index(&self) -> u32 {
        self.contact_id & !NEW_CONTACT_BIT
    }
    pub fn is_new(&self) -> bool {
        self.contact_id & NEW_CONTACT_BIT != 0
    }
}

/// The complete tracked-point and solver-row arrays of one rigid manifold.
/// This is deliberately not a complete manifold/contact pair or a save section
/// that may be installed into a solver. Normals, materials, subshapes, owners,
/// contact graphs, islands, recycling and dispatcher workspaces remain separate.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContactPointsRecord {
    pub identity: Identity,
    pub points: Vec<TrackedContactPoint>,
    pub solver: Vec<SolverContactPoint>,
}
pub struct ValidatedContactPoints {
    record: ContactPointsRecord,
}
impl ValidatedContactPoints {
    pub fn record(&self) -> &ContactPointsRecord {
        &self.record
    }
    /// Checked row lookup, retaining duplicates and native order (hooks may
    /// select/reorder solver rows). No pointer to a live solver is returned.
    pub fn solver_point(&self, row: usize) -> Option<&TrackedContactPoint> {
        let row = self.record.solver.get(row)?;
        self.record.points.get(row.point_index() as usize)
    }
}

pub(crate) fn contact_lengths(d: Dimension, points: usize, solver: usize) -> Result<()> {
    if points > MAX_ITEMS || solver > MAX_ITEMS {
        return Err(Error::Limit);
    }
    // Both pinned planar containers are ArrayVec<_, 2>. Spatial containers are
    // Vec, including clustered manifolds; do not invent a four-point limit.
    if d == Dimension::Planar && (points > 2 || solver > 2) {
        return Err(Error::Length);
    }
    if points == 0 && solver != 0 {
        return Err(Error::Contact);
    }
    contact_frame_length(points, solver)?;
    Ok(())
}

pub(crate) fn contact_frame_length(points: usize, solver: usize) -> Result<usize> {
    points
        .checked_mul(100)
        .and_then(|v| solver.checked_mul(48).and_then(|s| v.checked_add(s)))
        .and_then(|v| v.checked_add(56))
        .filter(|v| *v <= MAX_BYTES)
        .ok_or(Error::Limit)
}

fn finite(v: u32) -> Result<()> {
    if !f32::from_bits(v).is_finite() {
        return Err(Error::Contact);
    }
    Ok(())
}
fn vector(v: [u32; 3], d: Dimension) -> Result<()> {
    for value in v {
        finite(value)?;
    }
    if d == Dimension::Planar && v[2] != 0 {
        return Err(Error::Contact);
    }
    Ok(())
}
fn feature(v: u32, d: Dimension) -> Result<()> {
    // PackedFeatureId::unpack also maps malformed 00 words (and planar edge
    // words) to Unknown. This admitted encoding is deliberately stricter: only
    // the exact UNKNOWN sentinel or constructor-produced kind encodings pass.
    match v >> 30 {
        0 if v == 0 => Ok(()),
        1 | 3 => Ok(()),
        2 if d == Dimension::Spatial => Ok(()),
        _ => Err(Error::Contact),
    }
}

pub fn validate_contact_points(
    record: ContactPointsRecord,
    _budget: &mut Budget,
) -> Result<ValidatedContactPoints> {
    let d = record.identity.dimension;
    contact_lengths(d, record.points.len(), record.solver.len())?;
    for p in &record.points {
        vector(p.local_p1, d)?;
        vector(p.local_p2, d)?;
        finite(p.distance)?;
        for v in p.features {
            feature(v, d)?;
        }
        let w = p.data;
        for v in [w.impulse, w.warmstart_impulse, w.warmstart_twist_impulse]
            .into_iter()
            .chain(w.tangent_impulse)
            .chain(w.warmstart_tangent_impulse)
        {
            finite(v)?;
        }
        vector(w.solver_dp1, d)?;
        vector(w.solver_dp2, d)?;
        vector(w.warmstart_tangent_world, d)?;
        if d == Dimension::Planar
            && (w.tangent_impulse[1] != 0
                || w.warmstart_tangent_impulse[1] != 0
                || w.warmstart_twist_impulse != 0
                || w.warmstart_tangent_world != [0; 3])
        {
            return Err(Error::Contact);
        }
    }
    for s in &record.solver {
        vector(s.anchor1, d)?;
        vector(s.anchor2, d)?;
        vector(s.tangent_velocity, d)?;
        finite(s.distance)?;
        if s.padding != 0 || s.point_index() as usize >= record.points.len() {
            return Err(Error::Contact);
        }
    }
    // O(points + solver), no scratch allocation. Already-owned input allocation
    // is the caller's responsibility; decode charges all requested Vec payloads.
    Ok(ValidatedContactPoints { record })
}

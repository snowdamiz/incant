use super::{AdmissionError as E, Result};
use incant_physics_snapshot::WorldBoundary;

/// Caller-authorized logical visit budget, monotonically consumed. Units count
/// conservative row/iteration visits, not CPU instructions, time or allocation.
pub struct WorkBudget {
    remaining: u64,
}
impl WorkBudget {
    /// Start an explicit caller-authorized logical budget.
    pub fn new(units: u64) -> Self {
        Self { remaining: units }
    }
    /// Remaining units after successful and partially failed preflights.
    pub fn remaining(&self) -> u64 {
        self.remaining
    }
    /// Share this cumulative visit budget with capture and outer-owner checks.
    pub fn charge(&mut self, n: u64) -> Result<()> {
        self.remaining = self.remaining.checked_sub(n).ok_or(E::Work)?;
        Ok(())
    }
}
/// Checked integer arithmetic for the documented limited solver visit model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorkReport {
    /// Precharged visits for the derived-data walks.
    pub validation_units: u64,
    /// Conservative row/iteration visits, excluding collision search and other algorithms.
    pub conservative_solver_units: u64,
    /// Largest native full-step substep count, including clamped body extras.
    pub max_substeps: usize,
    /// Largest native PGS count, including clamped body extras.
    pub max_pgs: usize,
    /// Conservative CCD repetition count (at least one ordinary solve).
    pub max_ccd_passes: usize,
}
fn add(a: u64, b: u64) -> Result<u64> {
    a.checked_add(b).ok_or(E::Work)
}
fn mul(a: u64, b: u64) -> Result<u64> {
    a.checked_mul(b).ok_or(E::Work)
}
fn native(v: u64) -> Result<usize> {
    usize::try_from(v).map_err(|_| E::Work)
}
pub(super) fn admit(frame: WorldBoundary<'_>, budget: &mut WorkBudget) -> Result<WorkReport> {
    let c = frame.pipeline().context();
    let p = c.integration.record();
    let colliders = c.ccd.context().colliders;
    let bodies = &colliders.context().bodies.record().bodies;
    let manifolds = frame.manifolds().record();
    // Precharge all validation walks before inspecting individual rows. Three
    // attachment passes cover mass, unit-density fallback and extent; fixed
    // target lookup is by checked arena, never a quadratic scan.
    let rows = add(
        add(
            bodies.len() as u64,
            colliders.record().colliders.len() as u64,
        )?,
        add(
            manifolds.manifolds.len() as u64,
            manifolds.solver.len() as u64,
        )?,
    )?;
    let base_validation = add(1, mul(rows, 8)?)?;
    budget.charge(base_validation)?;
    // The metadata walk below is covered by the collider-row charge above.
    // Compound native-plan checks and two-pass mass arithmetic revisit each
    // flat child. Charge their full declared count before any child walk; this
    // remains logical validation work, not a collision-search/CPU-time bound.
    let compound_parts = colliders.context().geometries.iter().try_fold(0, |n, g| {
        if g.record().kind == incant_physics_snapshot::GeometryKind::Primitive {
            Ok(n)
        } else {
            add(n, g.record().parts.len() as u64)
        }
    })?;
    let child_validation = mul(compound_parts, 16)?;
    budget.charge(child_validation)?;
    let validation = add(base_validation, child_validation)?;
    let mut extra = [0u64; 2];
    for b in bodies {
        for (i, value) in b.additional_iterations.into_iter().enumerate() {
            let value = native(value)?;
            // Exact pinned substep_groups::key: native usize -> u32 first,
            // then clamp each partition key to sixteen bits. Saved values are
            // not changed, even when unused high bits exist on 64-bit hosts.
            extra[i] = extra[i].max(u64::from((value as u32).min(0xffff)));
        }
    }
    let steps = add(p.num_solver_iterations, extra[0])?;
    let pgs = add(p.num_internal_pgs_iterations, extra[1])?;
    let ccd = p.max_ccd_substeps.max(1);
    let report = WorkReport {
        validation_units: validation,
        // Bias/PGS, stabilization, warmstart and restitution/body stages. This
        // deliberately overcounts inactive rows, but excludes collision search
        // and other algorithms; it is not advertised as a whole-frame time bound.
        conservative_solver_units: mul(
            mul(mul(rows.max(1), steps)?, ccd)?,
            add(add(pgs, p.num_internal_stabilization_iterations)?, 8)?,
        )?,
        max_substeps: native(steps)?,
        max_pgs: native(pgs)?,
        max_ccd_passes: native(ccd)?,
    };
    native(p.num_internal_stabilization_iterations)?;
    budget.charge(report.conservative_solver_units)?;
    let dt = f32::from_bits(p.dt) / (report.max_substeps as f32);
    if !dt.is_finite() || dt <= 0. || !(1.0 / dt).is_finite() {
        return Err(E::Arithmetic);
    }
    // Check actual pinned spring formulas at the smallest full-step solve dt.
    // CCD TOI-generated shorter intervals and contact Jacobian arithmetic are
    // separate remaining admission; do not infer them from this finite sample.
    for s in [p.contact_softness, p.static_contact_softness] {
        let s = crate::dynamics::SpringCoefficients {
            natural_frequency: f32::from_bits(s.natural_frequency),
            damping_ratio: f32::from_bits(s.damping_ratio),
        };
        for x in [
            s.angular_frequency(),
            s.erp_inv_dt(dt),
            s.erp(dt),
            s.cfm_factor(dt),
        ] {
            if !x.is_finite() {
                return Err(E::Arithmetic);
            }
        }
    }
    Ok(report)
}

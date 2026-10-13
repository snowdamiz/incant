//! Default-off checked inert admission and exclusive in-memory checkpoints.
//! The numeric token alone is not a restore capability. Only the capture owner
//! authenticates a completed step and issues an opaque NativeFrame; its private
//! reconstruction preserves the supported primitive world under shared budgets.
//! There is no public arbitrary bytes-to-live installer. Persisted byte admission,
//! outer stable IDs/ECS/events and public physics file saves remain separate.
#[cfg(not(feature = "f32"))]
compile_error!("checked Incant physics frames require the pinned f32 solver");
// Native staging remains private behind the exclusive opaque capture owner.
#[allow(dead_code)]
pub(crate) mod allocation;
#[allow(dead_code)]
mod body;
#[allow(dead_code)]
mod broad;
#[allow(dead_code)]
mod collider;
pub(crate) mod convert;
mod derived;
#[allow(dead_code)]
mod islands;
#[allow(dead_code)]
mod narrow;
mod work;
use incant_physics_snapshot::{Identity, WorldBoundary};
pub use work::{WorkBudget, WorkReport};

/// Failed numerical preflight; no native owner has been constructed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    /// The frame does not match the independently supplied native identity.
    Identity,
    /// This first numeric stage accepts single primitives only.
    UnsupportedGeometry,
    /// External physics hooks need an authenticated runtime policy.
    UnsupportedHooks,
    /// Checked native integer arithmetic or the caller work budget was exceeded.
    Work,
    /// A supported derived native floating-point operation is nonfinite.
    Arithmetic,
    /// Ordered native local-mass recomputation differs from the saved bits.
    LocalMass,
    /// World center, effective inverse mass or inertia differs.
    WorldMass,
    /// The native maximum collider extent differs.
    Extent,
    /// Attached collider pose differs from the saved body/local-pose product.
    ColliderPose,
    /// A retained fixed-target bound differs from its historical native formula.
    CcdBounds,
}
type Result<T> = core::result::Result<T, AdmissionError>;

/// Borrowed, immutable numeric stage. There is intentionally no conversion to
/// a live native owner, and no phase or provenance authentication claim.
pub struct NumericFrame<'a> {
    frame: WorldBoundary<'a>,
    work: WorkReport,
}
impl<'a> NumericFrame<'a> {
    pub(crate) fn boundary(&self) -> WorldBoundary<'a> {
        self.frame
    }
    /// Charged logical visits and admitted native integer maxima.
    pub fn report(&self) -> WorkReport {
        self.work
    }
    /// Independently matched frame identity, without owner authentication.
    pub fn identity(&self) -> Identity {
        self.frame.pipeline().record().identity
    }
}

/// Expected identity is supplied independently by the trusted caller. The
/// concrete build fingerprint is not discovered from the untrusted frame.
/// Charges are cumulative, including on later failure; this is not a CPU-time
/// or complete future solver bound. Every saved bit stays unchanged.
pub fn admit_numeric_frame<'a>(
    frame: WorldBoundary<'a>,
    expected: Identity,
    budget: &mut WorkBudget,
) -> Result<NumericFrame<'a>> {
    if frame.pipeline().record().identity != expected
        || expected.dimension as usize != crate::math::DIM
    {
        return Err(AdmissionError::Identity);
    }
    let work = work::admit(frame, budget)?;
    derived::admit(frame)?;
    Ok(NumericFrame { frame, work })
}

#[cfg(test)]
pub(crate) mod allocation_test_meter;

#[allow(dead_code)]
mod integration;
#[allow(dead_code)]
pub(crate) mod world;

/// Default-off exclusive in-memory native capture ownership.
pub mod capture;

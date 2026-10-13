//! Default-off numeric admission for a checked inert frame. Public functions
//! never construct a live owner. Private staged storage constructors are tested
//! in-crate only and are not a complete body/collider/PhysicsWorld installer.
//! This token is deliberately not a restore capability: compound/subshape,
//! historical broad bounds, full native allocation and runtime-owner admission
//! remain necessary before the future private exact-field constructor.
#[cfg(not(feature = "f32"))]
compile_error!("checked Incant physics frames require the pinned f32 solver");
// Private staging is exercised in native unit tests until the whole owner
// constructor is complete. No public constructor or installer is exposed.
#[allow(dead_code)]
pub(crate) mod allocation;
#[allow(dead_code)]
mod body;
#[allow(dead_code)]
mod collider;
pub(crate) mod convert;
mod derived;
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
impl NumericFrame<'_> {
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

//! Actual selected native implementation, independent of any caller frame tag.
use super::CaptureError;
use incant_physics_snapshot::{Dimension, Identity};
use sha2::{Digest, Sha256};
mod own {
    include!(concat!(env!("OUT_DIR"), "/incant_identity.rs"));
}
/// Actual linked solver mode. Target SIMD code generation is separately bound
/// in the build digest; lane width alone is not a performance claim.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CompiledMode {
    /// Selected enhanced deterministic scalar math.
    pub deterministic: bool,
    /// Native vector lane width.
    pub lanes: usize,
    /// Native parallel implementation is compiled.
    pub parallel: bool,
    /// Pinned planar block solver (not admitted for spatial capture yet).
    pub block_solver: bool,
}
/// Refuse independently unified dependency modes which disagree with Rapier.
pub fn compiled_mode() -> Result<CompiledMode, CaptureError> {
    let deterministic = cfg!(feature = "enhanced-determinism");
    let parallel = cfg!(feature = "parallel");
    let lanes = if cfg!(feature = "simd8") { 8 } else { 4 };
    if !cfg!(feature = "f32")
        || cfg!(feature = "fem")
        || cfg!(feature = "unsync-callbacks")
        || cfg!(all(feature = "block-solver", not(feature = "dim2")))
        || cfg!(feature = "debug-disable-legitimate-fe-exceptions")
        || parry::checkpoint_identity::DIMENSION != crate::math::DIM
        || parry::checkpoint_identity::DETERMINISTIC != deterministic
        || parry::checkpoint_identity::PARALLEL != parallel
        || parry::checkpoint_identity::LANES != lanes
    {
        return Err(CaptureError::Build);
    }
    Ok(CompiledMode {
        deterministic,
        lanes,
        parallel,
        block_solver: cfg!(feature = "block-solver"),
    })
}
/// Source/config/toolchain identity of Rapier and the actual linked Parry.
/// This is conservative compatibility metadata, not an authenticity credential.
pub fn compiled_identity() -> Result<Identity, CaptureError> {
    let _ = compiled_mode()?;
    let mut hash = Sha256::new();
    hash.update(b"Incant linked primitive checkpoint v1");
    hash.update(own::SOURCE_ID);
    hash.update(parry::checkpoint_identity::SOURCE_ID);
    Ok(Identity {
        dimension: if crate::math::DIM == 2 {
            Dimension::Planar
        } else {
            Dimension::Spatial
        },
        build: hash.finalize().into(),
    })
}

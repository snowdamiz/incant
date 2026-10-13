//! Compiled Parry identity, consumed by the exact native checkpoint boundary.
include!(concat!(env!("OUT_DIR"), "/incant_identity.rs"));
/// Actual dimension of this linked library.
pub const DIMENSION: usize = crate::math::DIM;
/// Actual selected deterministic math feature.
pub const DETERMINISTIC: bool = cfg!(feature = "enhanced-determinism");
/// Actual selected parallel implementation feature.
pub const PARALLEL: bool = cfg!(feature = "parallel");
/// Actual native lane width, including independently unified dependency features.
pub const LANES: usize = crate::simd::SIMD_WIDTH;

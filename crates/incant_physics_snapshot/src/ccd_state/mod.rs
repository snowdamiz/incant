//! Persistent CCD fixed-target cache. No live constructor or cache installation.
//! Membership joins completed rigid witnesses; shape-derived AABB coherence and
//! enclosing owner authentication remain requirements of future composition.
mod codec;
mod validate;
use crate::{Handle, Identity, ValidatedColliderState, ValidatedIntegrationConfig};
pub use codec::{decode_ccd_state, encode_ccd_state};
pub use validate::validate_ccd_state;

/// Exact pinned collect_fixed_targets threshold, not a configurable limit.
pub const MAX_CCD_FIXED_TARGETS: usize = 512;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CcdTarget {
    pub collider: Handle,
    /// Exact native f32 bits; unused planar Z lanes are positive zero.
    pub mins: [u32; 3],
    pub maxs: [u32; 3],
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CcdCache {
    Uninitialized,
    Targets {
        /// Historical prediction distance. No normalization or substitution
        /// from the current integration configuration, even for signed zero.
        prediction: u32,
        /// Pinned collider arena slot order; sensors are included if enabled.
        targets: Vec<CcdTarget>,
    },
    FullBvh {
        prediction: u32,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CcdState {
    pub identity: Identity,
    pub cache: CcdCache,
}
#[derive(Clone, Copy)]
pub struct CcdContext<'a> {
    pub colliders: &'a ValidatedColliderState<'a>,
    pub integration: &'a ValidatedIntegrationConfig,
}
pub struct ValidatedCcdState<'a> {
    record: CcdState,
    context: CcdContext<'a>,
}
impl<'a> ValidatedCcdState<'a> {
    pub fn record(&self) -> &CcdState {
        &self.record
    }
    pub fn context(&self) -> CcdContext<'a> {
        self.context
    }
}

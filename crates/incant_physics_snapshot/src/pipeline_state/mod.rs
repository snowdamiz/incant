//! Retained pipeline verdict and exact gravity. No live installation or phase
//! authentication: an enclosing capture must establish the claimed boundary.
mod codec;
mod validate;
use crate::{Identity, ValidatedCcdState, ValidatedIntegrationConfig};
pub use codec::{decode_pipeline_state, encode_pipeline_state};
pub use validate::validate_pipeline_state;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum PipelinePhase {
    /// Bootstrap, interrupted or otherwise unauthenticated capture is refused.
    Unverified = 0,
    /// Claimed completed positive step; not authenticated by this scalar record.
    CompletedPositive = 1,
    /// Native nonpositive-dt collision maintenance is outside this capture scope.
    CollisionOnly = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PipelineState {
    pub identity: Identity,
    /// Exact finite f32 bits. Planar Z is canonical positive zero. Admission
    /// does not yet establish safe gravity/body/substep arithmetic.
    pub gravity: [u32; 3],
    /// Historical staged-solver result, not recomputed from current bodies.
    pub post_solve_ccd_active: Option<bool>,
    pub phase: PipelinePhase,
    pub deferred_bvh_receiver: bool,
    pub deferred_bvh_inline: bool,
    /// Ordered rigid/collider/soft report counts followed by rigid/collider
    /// unconsumed workspace counts. Any nonzero entry is explicitly refused.
    pub quarantine: [u64; 5],
}
#[derive(Clone, Copy)]
pub struct PipelineContext<'a> {
    pub integration: &'a ValidatedIntegrationConfig,
    pub ccd: &'a ValidatedCcdState<'a>,
}
pub struct ValidatedPipelineState<'a> {
    record: PipelineState,
    context: PipelineContext<'a>,
}
impl<'a> ValidatedPipelineState<'a> {
    pub fn record(&self) -> &PipelineState {
        &self.record
    }
    pub fn context(&self) -> PipelineContext<'a> {
        self.context
    }
}

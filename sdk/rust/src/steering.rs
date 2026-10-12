//! Local avoidance proposals over world-space samples. Obstacles are explicit;
//! this API neither gathers collision geometry nor moves entities automatically.
//! Input/output IDs are canonical `EntityId::0` bytes and need not name entities.
pub use incant_wasm_abi::steering::{Obstacle, Request, Velocities};
pub use incant_wasm_abi::{
    STEERING_MAX_REQUEST_BYTES, STEERING_MAX_RESULT_BYTES, SteeringAgent, SteeringVelocity,
};
pub(crate) fn error(error: incant_wasm_abi::steering::Error) -> crate::Error {
    match error {
        incant_wasm_abi::steering::Error::Bounds => crate::Error::Bounds,
        incant_wasm_abi::steering::Error::Malformed => crate::Error::Malformed,
    }
}

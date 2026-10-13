//! Exact pinned rigid integration settings, not permission to execute a solver.
//! Floats are IEEE-754 f32 bits. Negative zero and MAX sentinels are retained.
mod codec;
mod validate;
use crate::Identity;
pub use codec::{decode_integration_config, encode_integration_config};
pub use validate::validate_integration_config;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContactSoftness {
    pub natural_frequency: u32,
    pub damping_ratio: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum IntegrationFriction {
    /// No such field exists in planar Rapier. This is not Simplified friction.
    Planar = 0,
    Simplified = 1,
    Coulomb = 2,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum IntegrationSoftBodies {
    /// Trusted capture must check every pinned native soft setting against its
    /// default. Nondefault settings and any actual soft owner are unsupported.
    PinnedDefaults = 0,
    /// An explicit refusal, never an instruction to discard changed settings.
    Unsupported = 1,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntegrationConfig {
    pub identity: Identity,
    /// Finite nonpositive dt is native collision-only maintenance, not an error.
    pub dt: u32,
    pub min_ccd_dt: u32,
    pub contact_softness: ContactSoftness,
    pub static_contact_softness: ContactSoftness,
    pub warmstart_coefficient: u32,
    pub length_unit: u32,
    pub normalized_allowed_linear_error: u32,
    pub normalized_max_corrective_velocity: u32,
    pub normalized_prediction_distance: u32,
    pub normalized_max_linear_velocity: u32,
    pub normalized_contact_recycle_distance: u32,
    /// Exact native usize represented as u64. These do not allocate during
    /// decoding. Future owner admission must bound total work including body
    /// additional iterations and CCD substeps, before any native execution.
    pub num_solver_iterations: u64,
    pub num_internal_pgs_iterations: u64,
    pub num_internal_stabilization_iterations: u64,
    /// Zero disables all CCD; it is not coerced to one.
    pub max_ccd_substeps: u64,
    pub contact_clustering: bool,
    pub contact_recycling: bool,
    pub friction_in_bias_pass: bool,
    pub warmstart_joints: bool,
    pub friction_model: IntegrationFriction,
    pub soft_bodies: IntegrationSoftBodies,
}
pub struct ValidatedIntegrationConfig {
    record: IntegrationConfig,
}
impl ValidatedIntegrationConfig {
    pub fn record(&self) -> &IntegrationConfig {
        &self.record
    }
    /// Pinned native multiplication, including signed zero. A CCD cache can
    /// retain an earlier prediction value; it need not equal this value.
    pub fn prediction_distance(&self) -> u32 {
        (f32::from_bits(self.record.normalized_prediction_distance)
            * f32::from_bits(self.record.length_unit))
        .to_bits()
    }
}

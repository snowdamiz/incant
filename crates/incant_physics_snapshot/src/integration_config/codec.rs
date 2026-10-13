use super::*;
use crate::{Budget, Error, Identity, Result, arena_codec::*, codec::Reader, vector};
const MAGIC: &[u8; 8] = b"ICINT001";
const SIZE: usize = 144;

/// Fixed-size decoding uses only stack storage; no allocation or native call.
pub fn decode_integration_config(
    bytes: &[u8],
    expected: Identity,
) -> Result<ValidatedIntegrationConfig> {
    if bytes.len() != SIZE {
        return Err(Error::Length);
    }
    let mut r = Reader { bytes, position: 0 };
    read_identity(&mut r, MAGIC, expected)?;
    let mut f = [0; 13];
    for value in &mut f {
        *value = r.word()?;
    }
    let mut n = [0; 4];
    for value in &mut n {
        *value = u64::from_le_bytes(r.take()?);
    }
    let flags = r.word()?;
    let friction_model = match r.word()? {
        0 => IntegrationFriction::Planar,
        1 => IntegrationFriction::Simplified,
        2 => IntegrationFriction::Coulomb,
        _ => return Err(Error::Integration),
    };
    let soft_bodies = match r.word()? {
        0 => IntegrationSoftBodies::PinnedDefaults,
        1 => IntegrationSoftBodies::Unsupported,
        _ => return Err(Error::Integration),
    };
    if flags & !15 != 0 || r.word()? != 0 {
        return Err(Error::Header);
    }
    validate_integration_config(IntegrationConfig {
        identity: expected,
        dt: f[0],
        min_ccd_dt: f[1],
        contact_softness: ContactSoftness {
            natural_frequency: f[2],
            damping_ratio: f[3],
        },
        static_contact_softness: ContactSoftness {
            natural_frequency: f[4],
            damping_ratio: f[5],
        },
        warmstart_coefficient: f[6],
        length_unit: f[7],
        normalized_allowed_linear_error: f[8],
        normalized_max_corrective_velocity: f[9],
        normalized_prediction_distance: f[10],
        normalized_max_linear_velocity: f[11],
        normalized_contact_recycle_distance: f[12],
        num_solver_iterations: n[0],
        num_internal_pgs_iterations: n[1],
        num_internal_stabilization_iterations: n[2],
        max_ccd_substeps: n[3],
        contact_clustering: flags & 1 != 0,
        contact_recycling: flags & 2 != 0,
        friction_in_bias_pass: flags & 4 != 0,
        warmstart_joints: flags & 8 != 0,
        friction_model,
        soft_bodies,
    })
}
pub fn encode_integration_config(
    value: &ValidatedIntegrationConfig,
    budget: &mut Budget,
) -> Result<Vec<u8>> {
    budget.charge(SIZE)?;
    let mut bytes = vector(SIZE)?;
    let r = value.record();
    write_identity(&mut bytes, MAGIC, r.identity);
    for word in [
        r.dt,
        r.min_ccd_dt,
        r.contact_softness.natural_frequency,
        r.contact_softness.damping_ratio,
        r.static_contact_softness.natural_frequency,
        r.static_contact_softness.damping_ratio,
        r.warmstart_coefficient,
        r.length_unit,
        r.normalized_allowed_linear_error,
        r.normalized_max_corrective_velocity,
        r.normalized_prediction_distance,
        r.normalized_max_linear_velocity,
        r.normalized_contact_recycle_distance,
    ] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    for count in [
        r.num_solver_iterations,
        r.num_internal_pgs_iterations,
        r.num_internal_stabilization_iterations,
        r.max_ccd_substeps,
    ] {
        bytes.extend_from_slice(&count.to_le_bytes());
    }
    for word in [
        u32::from(r.contact_clustering)
            | (u32::from(r.contact_recycling) << 1)
            | (u32::from(r.friction_in_bias_pass) << 2)
            | (u32::from(r.warmstart_joints) << 3),
        r.friction_model as u32,
        r.soft_bodies as u32,
        0,
    ] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    Ok(bytes)
}

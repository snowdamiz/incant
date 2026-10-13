use super::*;
use crate::{Dimension, Error, Result};

fn nonnegative(bits: u32) -> Result<f32> {
    let value = f32::from_bits(bits);
    if !value.is_finite() || value < 0. {
        Err(Error::Integration)
    } else {
        Ok(value)
    }
}
pub fn validate_integration_config(
    record: IntegrationConfig,
) -> Result<ValidatedIntegrationConfig> {
    if record.soft_bodies != IntegrationSoftBodies::PinnedDefaults {
        return Err(Error::UnsupportedIntegration);
    }
    if (record.identity.dimension == Dimension::Planar)
        != (record.friction_model == IntegrationFriction::Planar)
    {
        return Err(Error::Integration);
    }
    let dt = f32::from_bits(record.dt);
    if !dt.is_finite() {
        return Err(Error::Integration);
    }
    nonnegative(record.min_ccd_dt)?;
    let unit = nonnegative(record.length_unit)?;
    if unit == 0. || nonnegative(record.warmstart_coefficient)? > 1. {
        return Err(Error::Integration);
    }
    for (bits, max_sentinel) in [
        (record.normalized_allowed_linear_error, false),
        (record.normalized_max_corrective_velocity, true),
        (record.normalized_prediction_distance, false),
        (record.normalized_max_linear_velocity, true),
        (record.normalized_contact_recycle_distance, false),
    ] {
        let value = nonnegative(bits)?;
        if !(max_sentinel && value == f32::MAX) && !(value * unit).is_finite() {
            return Err(Error::Integration);
        }
    }
    for softness in [record.contact_softness, record.static_contact_softness] {
        let frequency = nonnegative(softness.natural_frequency)?;
        let damping = nonnegative(softness.damping_ratio)?;
        // Stricter admitted rigid domain: reject a provably undefined active
        // spring, but do not claim all future substep arithmetic is checked.
        if dt > 0. && frequency == 0. && damping == 0. {
            return Err(Error::Integration);
        }
    }
    // The native fields are usize, not NonZeroUsize. Positive-step base zero
    // can work with positive body extras, but this scalar section cannot prove
    // every group has a nonzero total. Refuse it as a stricter domain policy.
    // Nonpositive maintenance never solves.
    // Zero PGS, stabilization and CCD counts are valid retained switches.
    if dt > 0. && (record.num_solver_iterations == 0 || !(1. / dt).is_finite()) {
        return Err(Error::Integration);
    }
    Ok(ValidatedIntegrationConfig { record })
}

use incant_types::{Camera, CameraProjection};

pub(crate) fn validate(camera: &Camera) -> Result<(), String> {
    if !(0.1..179.).contains(&camera.fov_degrees)
        || !camera.near.is_finite()
        || camera.near <= 0.
        || camera.far <= camera.near
        || !camera.far.is_finite()
    {
        return Err("invalid camera projection".into());
    }
    if let CameraProjection::Orthographic { vertical_size } = camera.projection
        && (!vertical_size.is_finite() || vertical_size <= 0.)
    {
        return Err("orthographic vertical size must be finite and positive".into());
    }
    if let CameraProjection::PixelPerfect {
        pixels_per_unit,
        reference_resolution,
    } = camera.projection
        && (!pixels_per_unit.is_finite()
            || !(1.0..=4096.0).contains(&pixels_per_unit)
            || reference_resolution
                .iter()
                .any(|d| !(16..=4096).contains(d))
            || u64::from(reference_resolution[0]) * u64::from(reference_resolution[1])
                > 4 * 1024 * 1024)
    {
        return Err("pixel-perfect cameras require 1..4096 pixels per metre and a 16..4096 logical resolution of at most 4194304 pixels".into());
    }
    Ok(())
}

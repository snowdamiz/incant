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
    Ok(())
}

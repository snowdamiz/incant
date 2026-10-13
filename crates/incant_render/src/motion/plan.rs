use super::MotionError;

pub const MAX_MOTION_PIXELS: u64 = 4 * 1024 * 1024;
/// One RGBA32F motion target plus two Depth32F surfaces. Logical payload only.
pub const MAX_MOTION_TARGET_BYTES: u64 = 24 * MAX_MOTION_PIXELS;
/// All retained motion CPU rows/maps/headers and GPU previous-instance buffers.
pub const MAX_MOTION_INSTANCE_BYTES: u64 = 64 * 1024 * 1024;

pub(super) fn target_bytes(size: [u32; 2], device_limit: u32) -> Result<u64, MotionError> {
    if size.contains(&0) || size.iter().any(|&d| d > device_limit.min(8192)) {
        return Err(MotionError::Capacity);
    }
    let pixels = u64::from(size[0]) * u64::from(size[1]);
    if pixels > MAX_MOTION_PIXELS {
        return Err(MotionError::Capacity);
    }
    pixels.checked_mul(24).ok_or(MotionError::Capacity)
}
/// Admission includes simultaneous old and candidate ownership. A resize may
/// retain two admitted sets until acceptance, but never a third cached set.
pub(super) fn overlap(old: u64, candidate: u64, cap: u64) -> Result<(), MotionError> {
    let ceiling = cap.checked_mul(2).ok_or(MotionError::Capacity)?;
    if old > cap || candidate > cap || old.checked_add(candidate).is_none_or(|n| n > ceiling) {
        return Err(MotionError::Capacity);
    }
    Ok(())
}
pub(super) fn charge(total: &mut u64, count: usize, bytes: usize) -> Result<(), MotionError> {
    *total = u64::try_from(count)
        .ok()
        .and_then(|n| n.checked_mul(bytes as u64))
        .and_then(|n| total.checked_add(n))
        .filter(|&n| n <= MAX_MOTION_INSTANCE_BYTES)
        .ok_or(MotionError::Capacity)?;
    Ok(())
}

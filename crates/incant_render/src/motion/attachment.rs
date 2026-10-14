//! Logical accepted + candidate allocations, not physical/queued GPU residency.
use super::{MotionError, plan};
pub const MAX_OWNER_TARGET_BYTES: u64 = 4 * plan::MAX_MOTION_PIXELS;
fn pixels(s: [u32; 2], limit: u32) -> Result<u64, MotionError> {
    plan::target_bytes(s, limit).map(|n| n / 24)
}
/// Equal extents must reuse the exact retained allocation, not just its shape.
/// Callers use this only for the existing single-cache Targets/Output/Overlay
/// prepare functions. Queued command buffers can retain older physical resources.
fn component(
    old: Option<[u32; 2]>,
    next: [u32; 2],
    stride: u64,
    cap: u64,
    limit: u32,
) -> Result<u64, MotionError> {
    let candidate = pixels(next, limit)?
        .checked_mul(stride)
        .ok_or(MotionError::Capacity)?;
    let old_bytes = old
        .map(|s| pixels(s, limit).and_then(|p| p.checked_mul(stride).ok_or(MotionError::Capacity)))
        .transpose()?
        .unwrap_or(0);
    plan::overlap(old_bytes, candidate, cap)?;
    old_bytes
        .checked_add(if old == Some(next) { 0 } else { candidate })
        .ok_or(MotionError::Capacity)
}
/// Preserve the producer aggregate ceiling without importing private upscaler
/// caches. This main path owns only motion/depth and the integer attachment;
/// fixed temporal history explicitly refuses while the attachment is enabled.
const MAX_OWNER_TRANSITION_BYTES: u64 = 671105024;
pub(crate) fn admit(
    source: [u32; 2],
    old_motion: Option<[u32; 2]>,
    limit: u32,
) -> Result<u64, MotionError> {
    let mut total = 2 * plan::MAX_MOTION_INSTANCE_BYTES + 16384;
    for bytes in [
        component(old_motion, source, 24, plan::MAX_MOTION_TARGET_BYTES, limit)?,
        component(old_motion, source, 4, MAX_OWNER_TARGET_BYTES, limit)?,
    ] {
        total = total.checked_add(bytes).ok_or(MotionError::Capacity)?;
    }
    if total > MAX_OWNER_TRANSITION_BYTES {
        return Err(MotionError::Capacity);
    }
    Ok(total)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owner_attachment_admission_uses_exact_reuse_and_preserves_ceiling() {
        assert_eq!(MAX_OWNER_TRANSITION_BYTES, 671105024);
        let full = [2048, 2048];
        let base = 2 * plan::MAX_MOTION_INSTANCE_BYTES + 16384;
        assert_eq!(admit(full, Some(full), 8192).unwrap(), 28 * 4194304 + base);
        assert_eq!(admit(full, None, 8192).unwrap(), 28 * 4194304 + base);
        assert_eq!(
            admit([2048, 1920], Some(full), 8192).unwrap(),
            28 * (4194304 + 2048 * 1920) + base
        );
        for size in [[u32::MAX; 2], [0, 1], [8193, 1], [2049, 2048]] {
            assert!(admit(size, None, u32::MAX).is_err());
            assert!(admit(full, Some(size), u32::MAX).is_err());
        }
        assert!(admit(full, None, 1024).is_err());
        assert_eq!(
            component(Some(full), full, u64::MAX, u64::MAX, 8192),
            Err(MotionError::Capacity)
        );
        // Distinct odd extents allocate both, regardless of equal pixel counts.
        assert_eq!(
            component(Some([3, 5]), [5, 3], 4, MAX_OWNER_TARGET_BYTES, 8192).unwrap(),
            120
        );
        assert_eq!(
            component(Some([3, 5]), [3, 5], 4, MAX_OWNER_TARGET_BYTES, 8192).unwrap(),
            60
        );
    }
}

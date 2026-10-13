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
/// Owner-only and portable-upscale paths share this admission. Fixed temporal
/// and MetalFX combinations refuse before allocation until their complete cached
/// resource accounting is reviewed. The existing aggregate ceiling is unchanged.
pub(crate) fn admit(
    source: [u32; 2],
    old_motion: Option<[u32; 2]>,
    output: Option<[u32; 2]>,
    old_output: Option<[u32; 2]>,
    old_overlay: Option<[u32; 2]>,
    limit: u32,
) -> Result<u64, MotionError> {
    let mut total = 2 * plan::MAX_MOTION_INSTANCE_BYTES + 16384;
    for bytes in [
        component(old_motion, source, 24, plan::MAX_MOTION_TARGET_BYTES, limit)?,
        component(old_motion, source, 4, MAX_OWNER_TARGET_BYTES, limit)?,
    ] {
        total = total.checked_add(bytes).ok_or(MotionError::Capacity)?;
    }
    if let Some(output) = output {
        for bytes in [
            component(old_output, output, 32, 32 * plan::MAX_MOTION_PIXELS, limit)?,
            component(old_overlay, source, 8, 8 * plan::MAX_MOTION_PIXELS, limit)?,
        ] {
            total = total.checked_add(bytes).ok_or(MotionError::Capacity)?;
        }
    } else if old_output.is_some() || old_overlay.is_some() {
        return Err(MotionError::Capacity);
    }
    if total > crate::upscaling::MAX_UPSCALE_TRANSITION_BYTES {
        return Err(MotionError::Capacity);
    }
    Ok(total)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn owner_attachment_admission_uses_exact_reuse_and_preserves_ceiling() {
        assert_eq!(crate::upscaling::MAX_UPSCALE_TRANSITION_BYTES, 671105024);
        let full = [2048, 2048];
        let reduced = [1920, 1920];
        assert_eq!(
            admit(full, Some(full), Some(full), Some(full), Some(full), 8192).unwrap(),
            68 * 4194304 + 2 * plan::MAX_MOTION_INSTANCE_BYTES + 16384
        );
        assert!(
            admit(
                [2048, 1920],
                Some(full),
                Some([2048, 1920]),
                Some(full),
                Some(full),
                8192
            )
            .is_err()
        );
        assert!(
            admit(
                reduced,
                Some(full),
                Some(full),
                Some(full),
                Some(full),
                8192
            )
            .is_ok()
        );
        assert!(admit(full, None, None, None, None, 8192).is_ok());
        for size in [[u32::MAX; 2], [0, 1], [8193, 1]] {
            assert!(admit(size, None, None, None, None, u32::MAX).is_err());
        }
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

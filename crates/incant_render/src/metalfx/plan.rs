#[cfg(any(feature = "native-metalfx", test))]
use super::{Fallback, MetalFxError};

/// Additional engine-owned native packing/output textures: accepted, candidate
/// and callback-retired allocations. Vendor hidden history and base renderer
/// resources are excluded. This is a logical limit, not a device memory budget.
pub const MAX_METALFX_TRANSITION_BYTES: u64 = 200 * 1024 * 1024 + 4096;
#[cfg(any(feature = "native-metalfx", test))]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Plan {
    pub source: [u32; 4],
    pub output: [u32; 4],
}
#[cfg(any(feature = "native-metalfx", test))]
fn integral(rect: [f32; 4]) -> Option<[u32; 4]> {
    if rect
        .iter()
        .any(|x| !x.is_finite() || *x < 0. || x.fract() != 0. || *x > 8192.)
        || rect[2] == 0.
        || rect[3] == 0.
    {
        return None;
    }
    Some(rect.map(|x| x as u32))
}
#[cfg(any(feature = "native-metalfx", test))]
impl Plan {
    pub fn new(source: [f32; 4], output: [f32; 4], range: [f32; 2]) -> Result<Self, Fallback> {
        let source = integral(source).ok_or(Fallback::FractionalViewport)?;
        let output = integral(output).ok_or(Fallback::FractionalViewport)?;
        if u64::from(source[2]) * u64::from(output[3])
            != u64::from(source[3]) * u64::from(output[2])
        {
            return Err(Fallback::Aspect);
        }
        let scale = source[2] as f32 / output[2] as f32;
        if !range.iter().all(|x| x.is_finite() && *x > 0.)
            || range[0] > range[1]
            || scale < range[0]
            || scale > range[1]
            || scale > 1.
        {
            return Err(Fallback::Scale);
        }
        Ok(Self { source, output })
    }
    pub fn dimensions(self) -> [u32; 4] {
        [
            self.source[2],
            self.source[3],
            self.output[2],
            self.output[3],
        ]
    }
    pub fn size(self) -> [u32; 2] {
        [self.output[2], self.output[3]]
    }
    pub fn bytes(self) -> Result<u64, MetalFxError> {
        let pixels = u64::from(self.output[2]) * u64::from(self.output[3]);
        // Fixed maximum input allocation permits admitted content-size changes
        // without reallocating or recompiling the native scaler: HDR8+Z4+XY4+R1,
        // output HDR8. Current/candidate are each charged even if reused.
        if pixels > 4_194_304 {
            return Err(MetalFxError::Capacity);
        }
        Ok(pixels * 25 + 2048)
    }
    pub fn preflight(self, old: u64, max_dimension: u32) -> Result<(), MetalFxError> {
        if self.size().iter().any(|x| *x > max_dimension)
            || self
                .bytes()?
                .checked_add(old)
                .is_none_or(|n| n > MAX_METALFX_TRANSITION_BYTES)
        {
            return Err(MetalFxError::Capacity);
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn integral_offsets_and_fractional_drs_never_change_native_aspect() {
        let p = Plan::new([3., 2., 48., 32.], [6., 4., 96., 64.], [0.25, 1.]).unwrap();
        assert_eq!(p.size(), [96, 64]);
        assert_eq!(p.dimensions(), [48, 32, 96, 64]);
        assert_eq!(
            Plan::new([0., 0., 48.5, 32.5], [0., 0., 97., 65.], [0.25, 1.]),
            Err(Fallback::FractionalViewport)
        );
        assert_eq!(
            Plan::new([0., 0., 49., 32.], [0., 0., 97., 65.], [0.25, 1.]),
            Err(Fallback::Aspect)
        );
        assert_eq!(
            Plan::new([0., 0., 32., 32.], [0., 0., 128., 128.], [0.5, 1.]),
            Err(Fallback::Scale)
        );
        assert_eq!(
            Plan::new([0., 0., 32., 32.], [0., 0., 32., 32.], [f32::NAN, 1.]),
            Err(Fallback::Scale)
        );
    }
    #[test]
    fn allocation_accounts_old_and_candidate_before_allocating() {
        let p = Plan::new([0., 0., 2048., 2048.], [0., 0., 2048., 2048.], [0.25, 1.]).unwrap();
        p.preflight(p.bytes().unwrap(), 8192).unwrap();
        assert_eq!(
            p.preflight(p.bytes().unwrap() + 1, 8192),
            Err(MetalFxError::Capacity)
        );
        assert_eq!(p.preflight(0, 1024), Err(MetalFxError::Capacity));
    }
}

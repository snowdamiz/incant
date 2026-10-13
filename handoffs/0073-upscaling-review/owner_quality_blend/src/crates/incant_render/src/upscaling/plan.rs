use super::UpscaleError;
/// Includes old+candidate output color/depth pairs+composite+output overlay depth,
/// internal overlay, motion targets, maximum motion instance payloads and fixed
/// packet metadata. Excludes ordinary renderer targets/assets, driver overhead
/// and older queued work.
pub const MAX_UPSCALE_TRANSITION_BYTES: u64 =
    128 * crate::motion::MAX_MOTION_PIXELS + 2 * crate::motion::MAX_MOTION_INSTANCE_BYTES + 16384;
/// Per output pixel: two RGBA16F colors and two R32F depths of history, the RGBA16F
/// composite and the Depth32F output overlay depth (36 bytes).
pub(super) const OUTPUT_PIXEL_BYTES: u64 = 36;
pub(super) const MAX_OUTPUT_BYTES: u64 = OUTPUT_PIXEL_BYTES * crate::motion::MAX_MOTION_PIXELS;
pub(super) const MAX_OVERLAY_BYTES: u64 = 8 * crate::motion::MAX_MOTION_PIXELS;
#[derive(Clone, Copy, Debug)]
pub(crate) struct Plan {
    pub output: [u32; 2],
    pub rect: [f32; 4],
    pub source: [u32; 2],
    pub source_rect: [f32; 4],
}
fn rect_valid(size: [u32; 2], r: [f32; 4]) -> bool {
    r.iter().all(|x| x.is_finite()) && r[0] >= 0. && r[1] >= 0.
        && r[2] > 0. && r[3] > 0. && r[0]+r[2] <= size[0] as f32
        && r[1]+r[3] <= size[1] as f32
        // Every dimension has an actual raster sample; empty fractional views refuse.
        && (r[0]+r[2]-0.5).ceil() > (r[0]-0.5).ceil()
        && (r[1]+r[3]-0.5).ceil() > (r[1]-0.5).ceil()
}
impl Plan {
    pub fn new(
        output: [u32; 2],
        rect: [f32; 4],
        source: &crate::resolution::FramePlan,
    ) -> Result<Self, UpscaleError> {
        if !rect_valid(output, rect)
            || !rect_valid(source.size, source.rect)
            || source.size[0] > output[0]
            || source.size[1] > output[1]
        {
            return Err(UpscaleError::Footprint);
        }
        // Source must be the exact binary DRS scale, with ceil only on attachments.
        let exact = (4..=16).any(|n| {
            let scale = n as f32 / 16.;
            rect.map(|x| x * scale) == source.rect
                && output.map(|x| (x as f32 * scale).ceil() as u32) == source.size
        });
        if !exact {
            return Err(UpscaleError::Footprint);
        }
        Ok(Self {
            output,
            rect,
            source: source.size,
            source_rect: source.rect,
        })
    }
    pub fn preflight(
        self,
        limit: u32,
        old_output: u64,
        old_overlay: u64,
        old_motion: u64,
    ) -> Result<u64, UpscaleError> {
        let pixels = |s: [u32; 2]| -> Result<u64, UpscaleError> {
            let n = u64::from(s[0]) * u64::from(s[1]);
            if s.contains(&0)
                || s.iter().any(|&x| x > limit.min(8192))
                || n > crate::motion::MAX_MOTION_PIXELS
            {
                return Err(UpscaleError::Capacity);
            }
            Ok(n)
        };
        let output = pixels(self.output)?;
        let source = pixels(self.source)?;
        if old_output > MAX_OUTPUT_BYTES
            || old_overlay > MAX_OVERLAY_BYTES
            || old_motion > crate::motion::MAX_MOTION_TARGET_BYTES
        {
            return Err(UpscaleError::Capacity);
        }
        let total = old_output
            .checked_add(old_overlay)
            .and_then(|x| x.checked_add(old_motion))
            .and_then(|x| x.checked_add(OUTPUT_PIXEL_BYTES * output + 32 * source))
            .and_then(|x| x.checked_add(2 * crate::motion::MAX_MOTION_INSTANCE_BYTES + 16384))
            .ok_or(UpscaleError::Capacity)?;
        if total > MAX_UPSCALE_TRANSITION_BYTES {
            return Err(UpscaleError::Capacity);
        }
        Ok(total)
    }
    /// Adds the output-viewport light grid of output-resolution BLEND models: the
    /// retained grid plus a worst-case candidate (maximum mask words), charged even
    /// when this frame turns out to reuse the retained one.
    pub fn with_output_grid(self, total: u64, old_grid: u64) -> Option<u64> {
        total
            .checked_add(old_grid)?
            .checked_add(crate::lighting::output_grid_allowance(self.rect)?)
            .filter(|t| *t <= MAX_UPSCALE_TRANSITION_BYTES)
    }
}

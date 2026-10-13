use super::ResolutionScale;
use crate::{Result, SceneError, pixel_grid::PixelGrid};

/// Shared actual attachment/viewport selection for ordinary and retained draws.
pub(crate) struct FramePlan {
    pub size: [u32; 2],
    pub rect: [f32; 4],
    pub output_mapping: Option<[f32; 4]>,
}
impl FramePlan {
    pub fn new(
        output: [u32; 2],
        rect: [f32; 4],
        pixel: Option<[u32; 2]>,
        scale: ResolutionScale,
    ) -> Result<Self> {
        if output.contains(&0)
            || rect.iter().any(|v| !v.is_finite())
            || rect[0] < 0.
            || rect[1] < 0.
            || rect[2] <= 0.
            || rect[3] <= 0.
            || rect[0] + rect[2] > output[0] as f32
            || rect[1] + rect[3] > output[1] as f32
        {
            return Err(SceneError::Input("viewport outside render target").into());
        }
        if let Some(pixel) = pixel {
            let grid = PixelGrid::fit(pixel, rect)?;
            return Ok(Self {
                size: grid.resolution,
                rect: [0., 0., pixel[0] as f32, pixel[1] as f32],
                output_mapping: Some(grid.output),
            });
        }
        let factor = scale.fraction();
        Ok(Self {
            size: output.map(|v| (v as f32 * factor).ceil() as u32),
            // One exact binary factor preserves projection aspect and viewport placement.
            // UI and chrome continue using the unscaled physical output rectangle.
            rect: rect.map(|v| v * factor),
            output_mapping: (scale != ResolutionScale::FULL).then_some([0., 0., 1. / factor, 1.]),
        })
    }
}

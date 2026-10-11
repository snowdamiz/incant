//! Fixed logical camera targets and integer presentation inside a physical viewport.
use crate::SceneError;

pub(crate) struct PixelGrid {
    pub resolution: [u32; 2],
    /// Physical origin, integer scale and enabled flag for the output pass.
    pub output: [f32; 4],
}

impl PixelGrid {
    /// `rect` has already passed the renderer's finite, positive output bounds check.
    pub fn fit(resolution: [u32; 2], rect: [f32; 4]) -> Result<Self, SceneError> {
        if resolution.contains(&0) {
            return Err(SceneError::CameraProjection);
        }
        let [x, y, w, h] = rect;
        let origin = [x.ceil() as u32, y.ceil() as u32];
        let available = [
            ((x + w).floor() as u32).saturating_sub(origin[0]),
            ((y + h).floor() as u32).saturating_sub(origin[1]),
        ];
        let scale = (available[0] / resolution[0]).min(available[1] / resolution[1]);
        if scale == 0 {
            return Err(SceneError::PixelPerfectViewport(
                resolution[0],
                resolution[1],
            ));
        }
        Ok(Self {
            resolution,
            output: [
                (origin[0] + (available[0] - resolution[0] * scale) / 2) as f32,
                (origin[1] + (available[1] - resolution[1] * scale) / 2) as f32,
                scale as f32,
                1.,
            ],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_pixels_fit_wholly_inside_integer_and_fractional_viewports() {
        for (rect, expected) in [
            ([0., 0., 320., 180.], [0., 0., 1., 1.]),
            ([0., 0., 1920., 1080.], [0., 0., 6., 1.]),
            ([13., 19., 1001., 701.], [33., 99., 3., 1.]),
            ([13.2, 19.2, 1000.6, 700.6], [33., 99., 3., 1.]),
        ] {
            let grid = PixelGrid::fit([320, 180], rect).unwrap();
            assert_eq!(grid.resolution, [320, 180]);
            assert_eq!(grid.output, expected);
            for axis in 0..2 {
                assert!(grid.output[axis] >= rect[axis]);
                assert!(
                    grid.output[axis] + grid.output[2] * grid.resolution[axis] as f32
                        <= rect[axis] + rect[axis + 2]
                );
            }
        }
        for rect in [
            [0., 0., 319., 180.],
            [0., 0., 320., 179.],
            [0.5, 0.5, 320., 180.],
        ] {
            assert!(matches!(
                PixelGrid::fit([320, 180], rect),
                Err(SceneError::PixelPerfectViewport(320, 180))
            ));
        }
    }
}

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum CameraProjection {
    Perspective {},
    Orthographic {
        /// Visible world-space height. Width follows the output aspect ratio.
        #[schemars(extend("x-incant-unit" = "m"))]
        vertical_size: f64,
    },
    PixelPerfect {
        /// Logical pixels per world metre. Camera motion snaps to this grid.
        #[schemars(range(min = 1, max = 4096), extend("x-incant-unit" = "px/m"))]
        pixels_per_unit: f64,
        /// Fixed logical width and height, scaled by whole pixels with letterboxing.
        #[schemars(extend("x-incant-unit" = "px"))]
        reference_resolution: [u32; 2],
    },
}
impl Default for CameraProjection {
    fn default() -> Self {
        Self::Perspective {}
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Camera {
    /// Vertical perspective field of view, retained when switching projection.
    #[schemars(extend("x-incant-unit" = "°"))]
    pub fov_degrees: f64,
    #[schemars(extend("x-incant-unit" = "m"))]
    pub near: f64,
    #[schemars(extend("x-incant-unit" = "m"))]
    pub far: f64,
    /// Perspective, orthographic, or pixel-perfect with a fixed logical resolution.
    #[serde(default)]
    pub projection: CameraProjection,
}

impl Camera {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if !(0.1..179.).contains(&self.fov_degrees)
            || !self.near.is_finite()
            || self.near <= 0.
            || self.far <= self.near
            || !self.far.is_finite()
        {
            return Err("invalid camera projection".into());
        }
        if let CameraProjection::Orthographic { vertical_size } = self.projection
            && (!vertical_size.is_finite() || vertical_size <= 0.)
        {
            return Err("orthographic vertical size must be finite and positive".into());
        }
        if let CameraProjection::PixelPerfect {
            pixels_per_unit,
            reference_resolution,
        } = self.projection
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
}

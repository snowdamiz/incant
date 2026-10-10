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

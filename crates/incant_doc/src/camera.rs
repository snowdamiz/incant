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
    /// Perspective, or orthographic with parallel view rays and a fixed world-space height.
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
        Ok(())
    }
}

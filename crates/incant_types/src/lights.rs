use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DirectionalLight {
    /// Linear RGB multiplier. Light travels along transformed local -Z.
    pub color: [f64; 3],
    /// Illuminance in lux.
    #[schemars(range(min = 0, max = 1000000))]
    pub intensity: f64,
    /// Optional cascaded directional shadows. Omit to preserve unshadowed lighting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shadows: Option<DirectionalShadows>,
}
/// Four 1024×1024 depth cascades. Distance is in world meters, independent of scale.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DirectionalShadows {
    /// Maximum view-space receiver distance; capped by the selected camera's far plane.
    #[schemars(range(min = 0.01, max = 10000))]
    pub distance: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PointLight {
    pub color: [f64; 3],
    /// Luminous intensity in candela.
    #[schemars(range(min = 0, max = 1000000))]
    pub intensity: f64,
    /// World-space cutoff in meters, independent of entity scale.
    #[schemars(range(min = 0.001, max = 10000))]
    pub range: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct SpotLight {
    pub color: [f64; 3],
    /// Luminous intensity in candela. Light travels along transformed local -Z.
    #[schemars(range(min = 0, max = 1000000))]
    pub intensity: f64,
    #[schemars(range(min = 0.001, max = 10000))]
    pub range: f64,
    #[schemars(range(min = 0, max = 89.9))]
    pub inner_degrees: f64,
    #[schemars(range(min = 0.1, max = 89.9))]
    pub outer_degrees: f64,
}

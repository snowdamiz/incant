//! Authored punctual lights. Linear RGB and physical intensity units follow
//! glTF KHR_lights_punctual; import of that extension is a separate capability.
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
pub(crate) fn validate(kind: &str, value: &serde_json::Value) -> Result<(), String> {
    let (color, intensity, range) = match kind {
        "DirectionalLight" => {
            let light: DirectionalLight =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            (light.color, light.intensity, None)
        }
        "PointLight" => {
            let light: PointLight =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            (light.color, light.intensity, Some(light.range))
        }
        "SpotLight" => {
            let light: SpotLight =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if !(0.0..=89.9).contains(&light.inner_degrees)
                || !(0.1..=89.9).contains(&light.outer_degrees)
                || light.inner_degrees >= light.outer_degrees
            {
                return Err("spot angles require 0 <= inner < outer <= 89.9 degrees".into());
            }
            (light.color, light.intensity, Some(light.range))
        }
        _ => return Err("unregistered light type".into()),
    };
    if !color.iter().all(|v| (0.0..=1.0).contains(v)) || !(0.0..=1_000_000.0).contains(&intensity) {
        return Err("light requires linear RGB in 0..1 and intensity in 0..1000000".into());
    }
    if range.is_some_and(|range| !(0.001..=10000.0).contains(&range)) {
        return Err("light range must be 0.001..10000 meters".into());
    }
    Ok(())
}

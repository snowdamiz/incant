use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Id;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum TextureUsage {
    Color,
    Linear,
    Normal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MeshRenderer {
    /// Stable ID of a cooked model asset. Its selected glTF scene supplies the
    /// model's primitives and local node transforms.
    pub mesh: Id,
    pub materials: Vec<Id>,
    pub cast_shadows: bool,
}
/// A distant, equirectangular image light. Rotation is about world +Y and does
/// not inherit the entity transform. One environment is allowed per project
/// until active-scene selection is implemented.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct EnvironmentLight {
    pub texture: Id,
    #[schemars(range(min = 0, max = 100))]
    pub intensity: f64,
    #[schemars(range(min = -360, max = 360))]
    pub rotation_degrees: f64,
}

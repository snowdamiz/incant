use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::Id;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AudioBus {
    /// Parent AudioBus entity ID, or the main output when absent.
    pub parent: Option<Id>,
    #[schemars(range(min=-80,max=24))]
    pub gain_db: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AudioSpatial {
    #[schemars(range(min = 0.01, max = 1000000))]
    pub min_distance: f32,
    #[schemars(range(min = 0.01, max = 1000000))]
    pub max_distance: f32,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AudioSource {
    pub clip: Id,
    pub bus: Option<Id>,
    #[schemars(range(min=-80,max=24))]
    pub gain_db: f32,
    #[schemars(range(min=-1,max=1))]
    pub pan: f32,
    #[schemars(range(min = 0.25, max = 4))]
    pub rate: f64,
    #[schemars(range(min = 0, max = 3600))]
    pub start_seconds: f64,
    pub playing: bool,
    pub looping: bool,
    pub streaming: bool,
    /// Distances in world meters; requires Transform and a unique listener.
    pub spatial: Option<AudioSpatial>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AudioListener {}

//! Persistent audio declarations. Runtime mixer and device handles never enter
//! the project. Shared command validation applies to human and agent edits alike.
use crate::{Id, Project};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

pub(crate) fn validate(
    kind: &str,
    value: &serde_json::Value,
    project: &Project,
) -> Result<(), String> {
    match kind {
        "AudioSource" => {
            let source: AudioSource =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if project
                .assets
                .get(&source.clip)
                .is_none_or(|a| a.kind != "audio")
            {
                return Err("clip must identify an audio asset".into());
            }
            if !valid_gain(source.gain_db)
                || !(-1. ..=1.).contains(&source.pan)
                || !(0.25..=4.).contains(&source.rate)
                || !(0. ..=3600.).contains(&source.start_seconds)
            {
                return Err("audio gain, pan, rate or start time outside supported range".into());
            }
            if let Some(spatial) = source.spatial {
                if !(0.01..=1_000_000.).contains(&spatial.min_distance)
                    || !(0.01..=1_000_000.).contains(&spatial.max_distance)
                    || spatial.max_distance <= spatial.min_distance
                {
                    return Err("audio spatial distances must be positive and ordered".into());
                }
                if source.pan != 0. {
                    return Err(
                        "spatial sources use listener-relative panning; pan must be zero".into(),
                    );
                }
            }
        }
        "AudioBus" => {
            let bus: AudioBus = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
            if !valid_gain(bus.gain_db) {
                return Err("audio gain must be in [-80,24] dB".into());
            }
        }
        "AudioListener" => {
            let _: AudioListener =
                serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        }
        _ => return Err("unknown audio component".into()),
    }
    Ok(())
}
fn valid_gain(gain: f32) -> bool {
    (-80. ..=24.).contains(&gain)
}
pub(crate) fn validate_graph(project: &Project) -> Result<(), String> {
    let mut buses = BTreeMap::new();
    let mut sources = Vec::new();
    let mut listeners = 0;
    for entity in project.scenes.values().flat_map(|s| s.entities.values()) {
        if let Some(value) = entity.components.get("AudioBus") {
            if entity.components.contains_key("AudioSource")
                || entity.components.contains_key("AudioListener")
            {
                return Err("an audio bus cannot also be a source or listener".into());
            }
            if let Ok(bus) = serde_json::from_value::<AudioBus>(value.clone()) {
                buses.insert(entity.id.as_str(), bus);
            }
        }
        if entity.components.contains_key("AudioListener") {
            listeners += 1;
            if !entity.components.contains_key("Transform") {
                return Err("audio listener requires Transform".into());
            }
        }
        if let Some(value) = entity.components.get("AudioSource")
            && let Ok(source) = serde_json::from_value::<AudioSource>(value.clone())
        {
            if source.spatial.is_some() && !entity.components.contains_key("Transform") {
                return Err("spatial audio source requires Transform".into());
            }
            sources.push(source);
        }
    }
    if buses.len() > 32 || sources.len() > 128 || listeners > 1 {
        return Err("audio supports at most 32 buses, 128 sources and one listener".into());
    }
    for (id, bus) in &buses {
        let mut seen = BTreeSet::from([*id]);
        let mut next = bus.parent.as_deref();
        while let Some(parent) = next {
            if !seen.insert(parent) {
                return Err("audio bus parent cycle".into());
            }
            next = buses
                .get(parent)
                .ok_or("audio bus parent does not exist")?
                .parent
                .as_deref();
        }
    }
    for source in sources {
        if source
            .bus
            .as_deref()
            .is_some_and(|id| !buses.contains_key(id))
        {
            return Err("audio source bus does not exist".into());
        }
        if source.spatial.is_some() && listeners != 1 {
            return Err("spatial audio requires one authored listener".into());
        }
    }
    Ok(())
}

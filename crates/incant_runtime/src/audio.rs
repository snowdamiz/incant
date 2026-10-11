//! Bounded native audio declarations. IDs are binary, not document paths. The
//! audio host owns decoded clips, mixer handles and playback cursors separately.
use crate::{SceneError, StableId};
use bevy_ecs::prelude::Component;
use incant_types::AudioSpatial;
use std::collections::{BTreeMap, BTreeSet};

pub const MAX_AUDIO_BUSES: usize = 32;
pub const MAX_AUDIO_SOURCES: usize = 128;
pub const MAX_AUDIO_NODES: usize = MAX_AUDIO_BUSES + MAX_AUDIO_SOURCES + 1;

#[derive(Debug, Clone, PartialEq)]
pub struct NativeAudioBus {
    pub parent: Option<StableId>,
    pub gain_db: f32,
}
#[derive(Debug, Clone, PartialEq)]
pub struct NativeAudioSource {
    pub clip: StableId,
    pub bus: Option<StableId>,
    pub gain_db: f32,
    pub pan: f32,
    pub rate: f64,
    pub start_seconds: f64,
    pub playing: bool,
    pub looping: bool,
    pub streaming: bool,
    pub spatial: Option<AudioSpatial>,
}
#[derive(Debug, Clone, PartialEq, Component)]
pub struct CookedAudio {
    pub entity: StableId,
    pub bus: Option<NativeAudioBus>,
    pub source: Option<NativeAudioSource>,
    pub listener: bool,
}

pub(crate) fn validate(
    audio: &[CookedAudio],
    exists: impl Fn(StableId) -> bool,
) -> Result<(), SceneError> {
    let invalid = || SceneError::Invalid("audio declarations, limits or references");
    if audio.len() > MAX_AUDIO_NODES {
        return Err(invalid());
    }
    let mut ids = BTreeSet::new();
    let mut buses = BTreeMap::new();
    let mut sources = 0;
    let mut listeners = 0;
    for node in audio {
        if !exists(node.entity)
            || !ids.insert(node.entity)
            || (node.bus.is_none() && node.source.is_none() && !node.listener)
            || (node.bus.is_some() && (node.source.is_some() || node.listener))
        {
            return Err(invalid());
        }
        listeners += usize::from(node.listener);
        if let Some(bus) = &node.bus {
            if !(-80. ..=24.).contains(&bus.gain_db) {
                return Err(invalid());
            }
            buses.insert(node.entity, bus);
        }
        if let Some(source) = &node.source {
            sources += 1;
            if !(-80. ..=24.).contains(&source.gain_db)
                || !(-1. ..=1.).contains(&source.pan)
                || !(0.25..=4.).contains(&source.rate)
                || !(0. ..=3600.).contains(&source.start_seconds)
            {
                return Err(invalid());
            }
            if let Some(spatial) = &source.spatial
                && (!(0.01..=1_000_000.).contains(&spatial.min_distance)
                    || !(0.01..=1_000_000.).contains(&spatial.max_distance)
                    || spatial.max_distance <= spatial.min_distance
                    || source.pan != 0.)
            {
                return Err(invalid());
            }
        }
    }
    if buses.len() > MAX_AUDIO_BUSES || sources > MAX_AUDIO_SOURCES || listeners > 1 {
        return Err(invalid());
    }
    for (&id, bus) in &buses {
        let mut visited = BTreeSet::from([id]);
        let mut parent = bus.parent;
        while let Some(id) = parent {
            if !visited.insert(id) {
                return Err(invalid());
            }
            parent = buses.get(&id).ok_or_else(invalid)?.parent;
        }
    }
    for source in audio.iter().filter_map(|node| node.source.as_ref()) {
        if source.bus.is_some_and(|id| !buses.contains_key(&id))
            || (source.spatial.is_some() && listeners != 1)
        {
            return Err(invalid());
        }
    }
    Ok(())
}

use super::{Listener, PreparedSource, SceneAudio};
use crate::{AudioError, PlayOptions, Result, Spatial};
use incant_runtime::{NativeAudioSource, NativeWorld};
use kira::backend::Backend;
use std::fmt::Debug;

pub(super) fn options(
    source: &NativeAudioSource,
    position: [f32; 3],
    bus: Option<String>,
) -> PlayOptions {
    PlayOptions {
        bus,
        gain_db: source.gain_db,
        pan: source.pan,
        rate: source.rate,
        start_seconds: source.start_seconds,
        looping: source.looping,
        streaming: source.streaming,
        spatial: source.spatial.as_ref().map(|s| Spatial {
            position,
            min_distance: s.min_distance,
            max_distance: s.max_distance,
        }),
    }
}
fn position(matrix: [[f64; 4]; 4]) -> Result<[f32; 3]> {
    let value = [matrix[3][0], matrix[3][1], matrix[3][2]];
    if value.iter().any(|v| !v.is_finite() || v.abs() > 1_000_000.) {
        return Err(AudioError::Invalid(
            "audio world position outside mixer bounds",
        ));
    }
    Ok(value.map(|v| v as f32))
}
impl<B: Backend> SceneAudio<B>
where
    B::Error: Debug,
{
    pub(super) fn prepare(&mut self, world: &NativeWorld) -> Result<Option<Listener>> {
        if world.scene_id() != self.scene || world.tick() < self.tick {
            return Err(AudioError::Invalid("audio scene/clock changed"));
        }
        if world.audio_nodes().len() > incant_runtime::MAX_AUDIO_NODES {
            return Err(AudioError::Capacity);
        }
        self.prepared.clear();
        let mut buses = 0;
        let mut listener = None;
        let mut static_clips = [None; crate::MAX_VOICES];
        let mut static_count = 0;
        let mut static_frames = 0;
        for (node, matrix) in world.audio_nodes() {
            if let Some(bus) = &node.bus {
                let previous = self
                    .buses
                    .get(&node.entity)
                    .ok_or(AudioError::Unsupported("audio bus topology edits"))?;
                if previous.parent != bus.parent {
                    return Err(AudioError::Unsupported("audio bus topology edits"));
                }
                crate::config::gain(bus.gain_db)?;
                buses += 1;
            }
            if node.listener {
                if listener.is_some() {
                    return Err(AudioError::Invalid("multiple audio listeners"));
                }
                let rotation = world
                    .world_rotation(node.entity)
                    .ok_or(AudioError::Invalid("listener pose"))?;
                let rotation = rotation.map(|v| v as f32);
                if rotation.iter().any(|v| !v.is_finite())
                    || (rotation.iter().map(|v| v * v).sum::<f32>() - 1.).abs() > 1e-4
                {
                    return Err(AudioError::Invalid("listener rotation"));
                }
                listener = Some(Listener {
                    position: position(matrix)?,
                    rotation,
                });
            }
            if let Some(source) = &node.source {
                if source.streaming && !self.streaming_supported {
                    return Err(AudioError::Unsupported(
                        "browser streaming adapter is not connected",
                    ));
                }
                if self.prepared.len() == crate::MAX_VOICES {
                    return Err(AudioError::Capacity);
                }
                let clip = &self
                    .clips
                    .get(&source.clip)
                    .ok_or(AudioError::Invalid("missing cooked clip binding"))?
                    .clip;
                if source.bus.is_some_and(|id| !self.buses.contains_key(&id)) {
                    return Err(AudioError::UnknownBus);
                }
                let position = if source.spatial.is_some() {
                    position(matrix)?
                } else {
                    [0.; 3]
                };
                options(source, position, None)
                    .validate(clip.metadata.frames as f64 / f64::from(clip.metadata.sample_rate))?;
                if source.spatial.is_some() && source.pan != 0. {
                    return Err(AudioError::Invalid("spatial source pan"));
                }
                if !source.streaming && !static_clips[..static_count].contains(&Some(source.clip)) {
                    static_clips[static_count] = Some(source.clip);
                    static_count += 1;
                    static_frames += clip.metadata.frames as usize;
                    if static_frames > crate::mixer::MAX_STATIC_FRAMES {
                        return Err(AudioError::Capacity);
                    }
                }
                self.prepared.push(PreparedSource {
                    id: node.entity,
                    desired: source.clone(),
                    position,
                });
            }
        }
        if buses != self.buses.len() {
            return Err(AudioError::Unsupported("audio bus topology edits"));
        }
        if listener.is_none() && self.prepared.iter().any(|p| p.desired.spatial.is_some()) {
            return Err(AudioError::Invalid("missing audio listener"));
        }
        Ok(listener)
    }
}

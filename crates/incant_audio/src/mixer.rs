use crate::{AudioError, Bus, PlayOptions, Result, config};
use incant_assets::CookedAudio;
#[cfg(not(target_arch = "wasm32"))]
use kira::sound::streaming::{StreamingSoundData, StreamingSoundHandle};
use kira::{
    AudioManager, AudioManagerSettings, Frame, Tween,
    backend::Backend,
    listener::ListenerHandle,
    sound::{
        PlaybackState, SoundData,
        static_sound::{StaticSoundData, StaticSoundHandle, StaticSoundSettings},
    },
    track::{SpatialTrackBuilder, SpatialTrackHandle, TrackBuilder, TrackHandle},
};
use std::{collections::BTreeMap, fmt::Debug, sync::Arc, time::Duration};

pub const MAX_VOICES: usize = 128;
pub const MAX_BUSES: usize = 32;
const MAX_STATIC_FRAMES: usize = 8 * 1024 * 1024;
pub type VoiceId = u64;
pub struct Mixer<B: Backend> {
    manager: AudioManager<B>,
    buses: BTreeMap<String, TrackHandle>,
    listener: ListenerHandle,
    voices: BTreeMap<VoiceId, Voice>,
    next: VoiceId,
    samples: BTreeMap<String, Arc<[Frame]>>,
    errors: std::collections::VecDeque<(VoiceId, String)>,
    synchronous_streams: bool,
}
struct Voice {
    sound: VoiceSound,
    spatial: Option<SpatialTrackHandle>,
}
enum VoiceSound {
    Static(StaticSoundHandle),
    Offline(crate::offline_stream::Handle),
    #[cfg(not(target_arch = "wasm32"))]
    Streaming(StreamingSoundHandle<incant_assets::AssetError>),
}
impl VoiceSound {
    fn state(&self) -> PlaybackState {
        match self {
            Self::Static(h) => h.state(),
            Self::Offline(h) => h.state(),
            #[cfg(not(target_arch = "wasm32"))]
            Self::Streaming(h) => h.state(),
        }
    }
}
fn immediate() -> Tween {
    Tween {
        duration: Duration::ZERO,
        ..Default::default()
    }
}
fn resource(error: impl Debug) -> AudioError {
    AudioError::Backend(format!("{error:?}"))
}
impl<B: Backend> Mixer<B>
where
    B::Error: Debug,
{
    pub fn new(mut settings: AudioManagerSettings<B>, definitions: &[Bus]) -> Result<Self> {
        if definitions.len() > MAX_BUSES {
            return Err(AudioError::Capacity);
        }
        let mut pending = BTreeMap::new();
        for bus in definitions {
            config::gain(bus.gain_db)?;
            if bus.id.is_empty()
                || bus.id.len() > 128
                || pending.insert(bus.id.clone(), bus).is_some()
            {
                return Err(AudioError::Invalid("bus IDs must be unique and bounded"));
            }
        }
        // Resolve and validate the complete graph before creating an audio backend.
        let mut order = Vec::new();
        while !pending.is_empty() {
            let id = pending
                .iter()
                .find(|(_, bus)| {
                    bus.parent
                        .as_ref()
                        .is_none_or(|p| order.iter().any(|b: &&Bus| b.id == *p))
                })
                .map(|(id, _)| id.clone())
                .ok_or(AudioError::Invalid("bus parent missing or cycle detected"))?;
            order.push(pending.remove(&id).unwrap());
        }
        settings.capacities.sub_track_capacity = MAX_BUSES + MAX_VOICES;
        settings.capacities.listener_capacity = 1;
        let mut manager = AudioManager::<B>::new(settings).map_err(resource)?;
        let listener = manager
            .add_listener([0., 0., 0.], [0., 0., 0., 1.])
            .map_err(resource)?;
        let mut buses: BTreeMap<String, TrackHandle> = BTreeMap::new();
        for bus in order {
            let builder = TrackBuilder::new()
                .volume(bus.gain_db)
                .sound_capacity(MAX_VOICES)
                .sub_track_capacity(MAX_BUSES + MAX_VOICES);
            let track = if let Some(parent) = &bus.parent {
                buses.get_mut(parent).unwrap().add_sub_track(builder)
            } else {
                manager.add_sub_track(builder)
            }
            .map_err(resource)?;
            buses.insert(bus.id.clone(), track);
        }
        Ok(Self {
            manager,
            buses,
            listener,
            voices: BTreeMap::new(),
            next: 1,
            samples: BTreeMap::new(),
            errors: std::collections::VecDeque::new(),
            synchronous_streams: false,
        })
    }
    pub fn backend_mut(&mut self) -> &mut B {
        self.manager.backend_mut()
    }
    pub fn set_bus_gain(&mut self, id: Option<&str>, db: f32) -> Result<()> {
        config::gain(db)?;
        if let Some(id) = id {
            self.buses
                .get_mut(id)
                .ok_or(AudioError::UnknownBus)?
                .set_volume(db, immediate());
        } else {
            self.manager.main_track().set_volume(db, immediate());
        }
        Ok(())
    }
    pub fn set_listener(&mut self, position: [f32; 3], rotation: [f32; 4]) -> Result<()> {
        config::position(position)?;
        if rotation.iter().any(|v| !v.is_finite())
            || (rotation.iter().map(|v| v * v).sum::<f32>() - 1.).abs() > 1e-4
        {
            return Err(AudioError::Invalid("listener requires a unit quaternion"));
        }
        self.listener.set_position(position, immediate());
        self.listener.set_orientation(rotation, immediate());
        Ok(())
    }
    pub fn play(&mut self, clip: Arc<CookedAudio>, options: PlayOptions) -> Result<VoiceId> {
        options.validate(clip.metadata.frames as f64 / f64::from(clip.metadata.sample_rate))?;
        if options
            .bus
            .as_ref()
            .is_some_and(|id| !self.buses.contains_key(id))
        {
            return Err(AudioError::UnknownBus);
        }
        self.collect_stream_errors()?;
        self.voices
            .retain(|_, v| v.sound.state() != PlaybackState::Stopped);
        if self.voices.len() >= MAX_VOICES || self.next == u64::MAX {
            return Err(AudioError::Capacity);
        }
        #[cfg(target_arch = "wasm32")]
        if options.streaming && !self.synchronous_streams {
            return Err(AudioError::Unsupported(
                "browser streaming adapter is not connected",
            ));
        }
        // Decode/check static data before allocating a spatial track or voice ID.
        let static_data = if options.streaming {
            None
        } else {
            self.samples
                .retain(|_, frames| Arc::strong_count(frames) > 1);
            let frames = if let Some(frames) = self.samples.get(&clip.metadata.content_sha256) {
                Arc::clone(frames)
            } else {
                let resident = self.samples.values().map(|f| f.len()).sum::<usize>();
                if clip.metadata.frames as usize > MAX_STATIC_FRAMES.saturating_sub(resident) {
                    return Err(AudioError::Capacity);
                }
                let frames: Arc<[Frame]> = clip
                    .read_all(MAX_STATIC_FRAMES)?
                    .into_iter()
                    .map(|f| Frame::new(f[0], f[1]))
                    .collect::<Vec<_>>()
                    .into();
                self.samples
                    .insert(clip.metadata.content_sha256.clone(), Arc::clone(&frames));
                frames
            };
            let mut data = StaticSoundData {
                sample_rate: clip.metadata.sample_rate,
                frames,
                settings: StaticSoundSettings::default(),
                slice: None,
            }
            .volume(options.gain_db)
            .panning(options.pan)
            .playback_rate(options.rate)
            .start_position(options.start_seconds);
            if options.looping {
                data = data.loop_region(..);
            }
            Some(data)
        };
        let mut spatial = if let Some(s) = options.spatial {
            let builder = SpatialTrackBuilder::new()
                .distances(s.min_distance..=s.max_distance)
                .spatialization_strength(1.)
                .sound_capacity(1);
            Some(
                if let Some(bus) = options.bus.as_ref() {
                    self.buses.get_mut(bus).unwrap().add_spatial_sub_track(
                        self.listener.id(),
                        s.position,
                        builder,
                    )
                } else {
                    self.manager
                        .add_spatial_sub_track(self.listener.id(), s.position, builder)
                }
                .map_err(resource)?,
            )
        } else {
            None
        };
        let sound = if let Some(data) = static_data {
            VoiceSound::Static(self.route(data, options.bus.as_deref(), spatial.as_mut())?)
        } else if self.synchronous_streams {
            VoiceSound::Offline(self.route(
                crate::offline_stream::Data {
                    clip,
                    options: options.clone(),
                },
                options.bus.as_deref(),
                spatial.as_mut(),
            )?)
        } else {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let mut data = StreamingSoundData::from_decoder(crate::ClipDecoder::new(clip))
                    .volume(options.gain_db)
                    .panning(options.pan)
                    .playback_rate(options.rate)
                    .start_position(options.start_seconds);
                if options.looping {
                    data = data.loop_region(..);
                }
                VoiceSound::Streaming(self.route(data, options.bus.as_deref(), spatial.as_mut())?)
            }
            #[cfg(target_arch = "wasm32")]
            {
                return Err(AudioError::Unsupported(
                    "browser streaming adapter is not connected",
                ));
            }
        };
        let id = self.next;
        self.next += 1;
        self.voices.insert(id, Voice { sound, spatial });
        Ok(id)
    }
    fn route<D: SoundData>(
        &mut self,
        data: D,
        bus: Option<&str>,
        spatial: Option<&mut SpatialTrackHandle>,
    ) -> Result<D::Handle>
    where
        D::Error: Debug,
    {
        if let Some(track) = spatial {
            track.play(data)
        } else if let Some(bus) = bus {
            self.buses
                .get_mut(bus)
                .ok_or(AudioError::UnknownBus)?
                .play(data)
        } else {
            self.manager.play(data)
        }
        .map_err(resource)
    }
    pub fn set_position(&mut self, id: VoiceId, position: [f32; 3]) -> Result<()> {
        config::position(position)?;
        self.voices
            .get_mut(&id)
            .ok_or(AudioError::UnknownVoice)?
            .spatial
            .as_mut()
            .ok_or(AudioError::Invalid("voice is not spatial"))?
            .set_position(position, immediate());
        Ok(())
    }
    pub fn stop(&mut self, id: VoiceId) -> Result<()> {
        match &mut self
            .voices
            .get_mut(&id)
            .ok_or(AudioError::UnknownVoice)?
            .sound
        {
            VoiceSound::Static(h) => h.stop(immediate()),
            VoiceSound::Offline(h) => h.stop(),
            #[cfg(not(target_arch = "wasm32"))]
            VoiceSound::Streaming(h) => h.stop(immediate()),
        }
        Ok(())
    }
    pub fn set_paused(&mut self, id: VoiceId, paused: bool) -> Result<()> {
        match &mut self
            .voices
            .get_mut(&id)
            .ok_or(AudioError::UnknownVoice)?
            .sound
        {
            VoiceSound::Offline(h) => h.pause(paused),
            VoiceSound::Static(h) => {
                if paused {
                    h.pause(immediate())
                } else {
                    h.resume(immediate())
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            VoiceSound::Streaming(h) => {
                if paused {
                    h.pause(immediate())
                } else {
                    h.resume(immediate())
                }
            }
        }
        Ok(())
    }
    pub fn state(&self, id: VoiceId) -> Result<PlaybackState> {
        Ok(self
            .voices
            .get(&id)
            .ok_or(AudioError::UnknownVoice)?
            .sound
            .state())
    }
    pub fn configure_voice(
        &mut self,
        id: VoiceId,
        gain_db: f32,
        pan: f32,
        rate: f64,
    ) -> Result<()> {
        PlayOptions {
            gain_db,
            pan,
            rate,
            ..Default::default()
        }
        .validate(0.)?;
        match &mut self
            .voices
            .get_mut(&id)
            .ok_or(AudioError::UnknownVoice)?
            .sound
        {
            VoiceSound::Static(h) => {
                h.set_volume(gain_db, immediate());
                h.set_panning(pan, immediate());
                h.set_playback_rate(rate, immediate());
            }
            VoiceSound::Offline(h) => h.configure(gain_db, pan, rate),
            #[cfg(not(target_arch = "wasm32"))]
            VoiceSound::Streaming(h) => {
                h.set_volume(gain_db, immediate());
                h.set_panning(pan, immediate());
                h.set_playback_rate(rate, immediate());
            }
        }
        Ok(())
    }
    fn collect_stream_errors(&mut self) -> Result<()> {
        for (&id, voice) in &mut self.voices {
            if let VoiceSound::Offline(h) = &voice.sound {
                if self.errors.len() >= 1024 {
                    return Err(AudioError::Capacity);
                }
                if let Some(error) = h.pop_error() {
                    self.errors.push_back((id, error));
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            if let VoiceSound::Streaming(h) = &mut voice.sound {
                loop {
                    if self.errors.len() >= 1024 {
                        return Err(AudioError::Capacity);
                    }
                    let Some(error) = h.pop_error() else {
                        break;
                    };
                    self.errors.push_back((id, error.to_string()));
                }
            }
        }
        Ok(())
    }
    /// Drain bounded diagnostics, including errors from voices already reaped.
    /// Hosts should poll this regularly; a full error queue blocks new playback.
    pub fn stream_errors(&mut self) -> Vec<(VoiceId, String)> {
        let _ = self.collect_stream_errors();
        self.errors.drain(..).collect()
    }
}

impl Mixer<crate::OfflineBackend> {
    pub fn offline(sample_rate: u32, buses: &[Bus]) -> Result<Self> {
        let mut mixer = Self::new(
            AudioManagerSettings {
                backend_settings: sample_rate,
                ..Default::default()
            },
            buses,
        )?;
        mixer.synchronous_streams = true;
        Ok(mixer)
    }
    pub fn render(&mut self, output: &mut [f32]) -> Result<()> {
        self.manager
            .backend_mut()
            .render(output)
            .map_err(|e| AudioError::Backend(e.into()))
    }
}

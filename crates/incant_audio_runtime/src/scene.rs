//! Typed native-world audio host. Synchronize only after successful simulation
//! and script publication. This host never rolls back the world or a device cursor.
mod binding;
mod checkpoint;
pub use checkpoint::{MAX_AUDIO_CHECKPOINT_BYTES, PendingAudioCheckpoint};
mod prepare;
use crate::{
    AudioError, AudioManagerSettings, Bus, Mixer, OfflineBackend, PlaybackState, Result, VoiceId,
};
pub use binding::{ClipBinding, MAX_CLIP_METADATA_BYTES, MAX_SCENE_CLIPS};
use incant_runtime::{NativeAudioSource, NativeWorld, StableId};
use kira::backend::Backend;
use std::{collections::BTreeMap, fmt::Debug};

/// Pending replacements wait for the stopped voice to be acknowledged by the
/// renderer. A subsequent sync starts them; sync alone never advances a cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceState {
    Pending,
    Playing,
    Paused,
    Stopped,
}

struct Source {
    desired: NativeAudioSource,
    position: [f32; 3],
    voice: Option<VoiceId>,
    pending: bool,
}
struct BusState {
    parent: Option<StableId>,
    name: String,
    gain: f32,
}
#[derive(Clone, PartialEq)]
struct Listener {
    position: [f32; 3],
    rotation: [f32; 4],
}
struct PreparedSource {
    id: StableId,
    desired: NativeAudioSource,
    position: [f32; 3],
}

/// Bounded retained clips, a fixed bus topology and at most 128 mixer voices,
/// including stopped voices awaiting renderer acknowledgement. Preflight uses
/// reusable scratch; steady-state sync allocates no per-source strings or arrays.
pub struct SceneAudio<B: Backend> {
    mixer: Option<Mixer<B>>,
    clips: BTreeMap<StableId, ClipBinding>,
    buses: BTreeMap<StableId, BusState>,
    sources: BTreeMap<StableId, Source>,
    retired: Vec<VoiceId>,
    prepared: Vec<PreparedSource>,
    listener: Option<Listener>,
    scene: StableId,
    tick: u64,
    streaming_supported: bool,
    identity: incant_runtime::WorldIdentity,
    incarnation: incant_runtime::WorldIncarnation,
}

impl<B: Backend> SceneAudio<B>
where
    B::Error: Debug,
{
    /// All declarations, clip bindings and mixer-domain poses are checked before
    /// the backend is created. Bindings cannot be added/reloaded during a session.
    pub fn new(
        world: &NativeWorld,
        clips: Vec<ClipBinding>,
        settings: AudioManagerSettings<B>,
    ) -> Result<Self> {
        Self::create(world, clips, !cfg!(target_arch = "wasm32"), |buses| {
            Mixer::new(settings, buses)
        })
    }
    fn create(
        world: &NativeWorld,
        clips: Vec<ClipBinding>,
        streaming_supported: bool,
        make: impl FnOnce(&[Bus]) -> Result<Mixer<B>>,
    ) -> Result<Self> {
        let clips = binding::collect(clips)?;
        let mut buses = BTreeMap::new();
        let mut definitions = Vec::new();
        for (node, _) in world.audio_nodes() {
            if let Some(bus) = &node.bus {
                let name = binding::name(node.entity);
                definitions.push(Bus {
                    id: name.clone(),
                    parent: bus.parent.map(binding::name),
                    gain_db: bus.gain_db,
                });
                buses.insert(
                    node.entity,
                    BusState {
                        parent: bus.parent,
                        name,
                        gain: bus.gain_db,
                    },
                );
            }
        }
        let mut host = Self {
            mixer: None,
            clips,
            buses,
            sources: BTreeMap::new(),
            retired: Vec::with_capacity(crate::MAX_VOICES),
            prepared: Vec::with_capacity(crate::MAX_VOICES),
            listener: None,
            scene: world.scene_id(),
            tick: world.tick(),
            streaming_supported,
            identity: world
                .spatial_view()
                .map_err(|_| AudioError::Invalid("audio world unavailable"))?
                .identity()
                .clone(),
            incarnation: world.incarnation(),
        };
        let listener = host.prepare(world)?;
        host.mixer = Some(make(&definitions)?);
        host.apply(world, listener)?;
        Ok(host)
    }

    /// Preflight rejection issues zero mixer commands and is retryable. A device
    /// may continue playing meanwhile. Decode/backend/apply errors are terminal:
    /// the mixer is dropped, and all later calls fail. A game session must treat
    /// a failed post-simulation sync as an unsuccessful tick, not world rollback.
    pub fn sync(&mut self, world: &NativeWorld) -> Result<()> {
        if self.mixer.is_none() {
            return Err(AudioError::FailedSession);
        }
        let listener = self.prepare(world)?;
        let result = self.apply(world, listener);
        self.terminal(result)
    }

    /// Host suspension freezes every voice cursor at the next mixer boundary;
    /// individual desired play/pause states and scene declarations are unchanged.
    /// Asynchronous decoding errors remain terminal, including while suspended.
    pub fn set_suspended(&mut self, suspended: bool) -> Result<()> {
        self.check_errors()?;
        self.mixer
            .as_mut()
            .ok_or(AudioError::FailedSession)?
            .set_suspended(suspended);
        Ok(())
    }

    pub fn is_failed(&self) -> bool {
        self.mixer.is_none()
    }

    pub fn source_state(&self, id: StableId) -> Result<SourceState> {
        let mixer = self.mixer.as_ref().ok_or(AudioError::FailedSession)?;
        let source = self.sources.get(&id).ok_or(AudioError::UnknownVoice)?;
        if source.pending {
            return Ok(SourceState::Pending);
        }
        Ok(
            match source.voice.map(|voice| mixer.state(voice)).transpose()? {
                Some(PlaybackState::Playing | PlaybackState::Resuming) => SourceState::Playing,
                Some(PlaybackState::Paused | PlaybackState::Pausing) => SourceState::Paused,
                _ => SourceState::Stopped,
            },
        )
    }

    /// Poll asynchronous stream failures even when no world tick is due.
    pub fn check_errors(&mut self) -> Result<()> {
        let result = self
            .mixer
            .as_mut()
            .ok_or(AudioError::FailedSession)?
            .check_stream_errors();
        self.terminal(result)
    }

    fn terminal<T>(&mut self, result: Result<T>) -> Result<T> {
        if result.is_err() {
            self.mixer.take();
        }
        result
    }

    fn apply(&mut self, world: &NativeWorld, listener: Option<Listener>) -> Result<()> {
        let mixer = self.mixer.as_mut().ok_or(AudioError::FailedSession)?;
        mixer.check_stream_errors()?;
        // Only this host forgets handles. play_owned never reaps another source
        // that the device thread may finish after this acknowledgement pass.
        for source in self.sources.values_mut() {
            if let Some(id) = source.voice
                && mixer.state(id)? == PlaybackState::Stopped
            {
                mixer.forget_stopped(source.voice.take().unwrap())?;
            }
        }
        let mut i = 0;
        while i < self.retired.len() {
            if mixer.state(self.retired[i])? == PlaybackState::Stopped {
                mixer.forget_stopped(self.retired.swap_remove(i))?;
            } else {
                i += 1;
            }
        }
        if listener != self.listener {
            if let Some(pose) = &listener {
                mixer.set_listener(pose.position, pose.rotation)?;
            }
            self.listener = listener;
        }
        for (node, _) in world.audio_nodes() {
            if let Some(bus) = &node.bus {
                let cached = self.buses.get_mut(&node.entity).unwrap();
                if cached.gain != bus.gain_db {
                    mixer.set_bus_gain(Some(&cached.name), bus.gain_db)?;
                    cached.gain = bus.gain_db;
                }
            }
        }
        let mut error = None;
        self.sources.retain(|id, source| {
            if self.prepared.binary_search_by_key(id, |s| s.id).is_ok() {
                return true;
            }
            if let Some(voice) = source.voice {
                if let Err(e) = mixer.stop(voice) {
                    error = Some(e);
                }
                self.retired.push(voice);
            }
            false
        });
        if let Some(error) = error {
            return Err(error);
        }
        for prepared in &self.prepared {
            if let Some(source) = self.sources.get_mut(&prepared.id) {
                if restart(&source.desired, &prepared.desired) {
                    if let Some(voice) = source.voice.take() {
                        mixer.stop(voice)?;
                        self.retired.push(voice);
                    }
                    source.pending = prepared.desired.playing;
                } else {
                    if source.desired.playing != prepared.desired.playing {
                        if let Some(voice) = source.voice {
                            mixer.set_paused(voice, !prepared.desired.playing)?;
                        } else {
                            source.pending = prepared.desired.playing;
                        }
                    }
                    if let Some(voice) = source.voice {
                        if source.desired.gain_db != prepared.desired.gain_db
                            || source.desired.pan != prepared.desired.pan
                            || source.desired.rate != prepared.desired.rate
                        {
                            mixer.configure_voice(
                                voice,
                                prepared.desired.gain_db,
                                prepared.desired.pan,
                                prepared.desired.rate,
                            )?;
                        }
                        if prepared.desired.spatial.is_some()
                            && source.position != prepared.position
                        {
                            mixer.set_position(voice, prepared.position)?;
                        }
                    }
                }
                source.desired = prepared.desired.clone();
                source.position = prepared.position;
            } else {
                self.sources.insert(
                    prepared.id,
                    Source {
                        desired: prepared.desired.clone(),
                        position: prepared.position,
                        voice: None,
                        pending: prepared.desired.playing,
                    },
                );
            }
        }
        let mut voices =
            self.retired.len() + self.sources.values().filter(|s| s.voice.is_some()).count();
        for source in self.sources.values_mut() {
            if source.pending && source.desired.playing && voices < crate::MAX_VOICES {
                // Drain the bounded previous generation before starting new
                // voices; no overlapping playback after a stop command.
                if !self.retired.is_empty() {
                    continue;
                }
                let options = prepare::options(
                    &source.desired,
                    source.position,
                    source.desired.bus.map(|id| self.buses[&id].name.clone()),
                );
                if !mixer.can_start(options.bus.as_deref(), options.spatial.is_some())? {
                    continue;
                }
                source.voice =
                    Some(mixer.play_owned(self.clips[&source.desired.clip].clip.clone(), options)?);
                source.pending = false;
                voices += 1;
            }
        }
        self.tick = world.tick();
        Ok(())
    }
}

fn restart(a: &NativeAudioSource, b: &NativeAudioSource) -> bool {
    a.clip != b.clip
        || a.bus != b.bus
        || a.start_seconds != b.start_seconds
        || a.looping != b.looping
        || a.streaming != b.streaming
        || a.spatial != b.spatial
}

impl SceneAudio<OfflineBackend> {
    pub fn offline(world: &NativeWorld, clips: Vec<ClipBinding>, sample_rate: u32) -> Result<Self> {
        Self::create(world, clips, true, |buses| {
            Mixer::offline(sample_rate, buses)
        })
    }
    /// Actual Kira rendering, including synchronous bounded cooked streaming.
    /// On failure the entire caller output is silenced and the host is terminal.
    pub fn render(&mut self, output: &mut [f32]) -> Result<()> {
        let result = (|| {
            let mixer = self.mixer.as_mut().ok_or(AudioError::FailedSession)?;
            mixer.check_stream_errors()?;
            mixer.render(output)?;
            mixer.check_stream_errors()?;
            if output.iter().any(|v| !v.is_finite()) {
                return Err(AudioError::Invalid("nonfinite mixed audio"));
            }
            Ok(())
        })();
        if result.is_err() {
            output.fill(0.);
        }
        self.terminal(result)
    }
}

#[cfg(all(feature = "device", not(target_arch = "wasm32")))]
impl SceneAudio<crate::DeviceBackend> {
    /// Observe the real CPAL backend error queue. Any reported or discarded
    /// error is terminal; absence of an observed error is not a liveness proof.
    pub fn check_device(&mut self) -> Result<()> {
        let backend = self
            .mixer
            .as_mut()
            .ok_or(AudioError::FailedSession)?
            .backend_mut();
        let result = if backend
            .num_stream_errors_discarded()
            .is_none_or(|count| count != 0)
        {
            Err(AudioError::Backend(
                "device error queue unavailable or errors discarded".into(),
            ))
        } else if let Some(error) = backend.pop_error() {
            Err(AudioError::Backend(error.to_string()))
        } else {
            Ok(())
        };
        self.terminal(result)
    }
}

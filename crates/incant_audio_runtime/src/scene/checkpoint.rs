//! Data-only ordinary offline continuation. The sealed pending record is bound
//! to one exact native world before any mixer or guest is constructed.
mod codec;
use super::*;
use crate::{
    PlayOptions,
    mixer::{MixerCheckpoint, TransportCheckpoint},
};
use incant_runtime::{WorldIdentity, WorldIncarnation};
use std::{collections::BTreeSet, sync::Arc};

pub use codec::MAX_AUDIO_CHECKPOINT_BYTES;
#[derive(Clone)]
struct Catalog {
    id: StableId,
    fingerprint: [u8; 32],
    content: [u8; 32],
    rate: u32,
    frames: u64,
}
#[derive(Clone, Copy)]
enum Transport {
    Static([u8; 144]),
    Stream(crate::offline_stream::Checkpoint),
}
#[derive(Clone)]
struct SavedVoice {
    id: VoiceId,
    clip: StableId,
    bus: Option<StableId>,
    options: PlayOptions,
    retained: bool,
    state: Transport,
}
#[derive(Clone)]
struct SavedSource {
    id: StableId,
    transport_epoch: u64,
    voice: Option<VoiceId>,
    pending: bool,
}
#[derive(Clone)]
struct State {
    origin: [u8; 32],
    scene: StableId,
    tick: u64,
    sample_rate: u32,
    frames: u64,
    next: u64,
    listener: ([f32; 3], [f32; 4]),
    main_gain: f32,
    startup: bool,
    suspended: bool,
    master_paused: bool,
    pause_pending: bool,
    resume_pending: bool,
    catalog: Vec<Catalog>,
    sources: Vec<SavedSource>,
    retired: Vec<VoiceId>,
    voices: Vec<SavedVoice>,
}
/// Validated world-bound audio section. Private fields prevent unchecked owner
/// pairing. File hashes detect corruption and compatibility, not authenticity.
pub struct PendingAudioCheckpoint {
    state: State,
    bytes: Vec<u8>,
    identity: WorldIdentity,
    incarnation: WorldIncarnation,
}
impl PendingAudioCheckpoint {
    /// Parse bounded data and require exact committed world/source membership.
    /// Asset fingerprints and static sample history are checked by `restore`
    /// against the actual trusted package before any guest/consumer starts.
    pub fn new(world: &NativeWorld, bytes: &[u8]) -> Result<Self> {
        let state = codec::decode(bytes)?;
        validate_world(&state, world)?;
        let identity = world
            .spatial_view()
            .map_err(|_| AudioError::Invalid("audio world unavailable"))?
            .identity()
            .clone();
        Ok(Self {
            state,
            bytes: bytes.to_vec(),
            identity,
            incarnation: world.incarnation(),
        })
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn suspended(&self) -> bool {
        self.state.suspended
    }
    pub fn sample_rate(&self) -> u32 {
        self.state.sample_rate
    }
    pub fn rendered_frames(&self) -> u64 {
        self.state.frames
    }
    pub fn validate_world(&self, world: &NativeWorld) -> Result<()> {
        if world
            .spatial_view()
            .map_err(|_| AudioError::Invalid("audio world unavailable"))?
            .identity()
            != &self.identity
            || world.incarnation() != self.incarnation
        {
            return Err(AudioError::Invalid("audio owner world lifetime"));
        }
        validate_world(&self.state, world)
    }
    /// Require the composed session's exact rational output clock. Standalone
    /// mixer users may render arbitrary blocks; a game save cannot substitute
    /// that clock for its committed native tick.
    pub fn validate_session(&self, world: &NativeWorld) -> Result<()> {
        self.validate_world(world)?;
        if u128::from(self.state.frames)
            != u128::from(world.tick()) * u128::from(self.state.sample_rate)
                / u128::from(world.tick_rate())
        {
            return Err(AudioError::Invalid("audio/world sample clock"));
        }
        Ok(())
    }
    pub fn validate_restored(
        &self,
        world: &NativeWorld,
        audio: &SceneAudio<OfflineBackend>,
    ) -> Result<()> {
        self.validate_session(world)?;
        let restored = audio.checkpoint_for_world(world)?;
        if restored.bytes != self.bytes {
            return Err(AudioError::Invalid("restored audio owner state"));
        }
        Ok(())
    }
    /// Fresh offline owner. No old host, renderer queue or device is reused.
    pub fn restore(
        &self,
        world: &NativeWorld,
        clips: Vec<ClipBinding>,
        sample_rate: u32,
    ) -> Result<SceneAudio<OfflineBackend>> {
        self.validate_world(world)?;
        if sample_rate != self.state.sample_rate {
            return Err(AudioError::Invalid("audio output sample rate"));
        }
        let clips = binding::collect(clips)?;
        if catalog(&clips)? != self.state.catalog {
            return Err(AudioError::Invalid("audio package catalog"));
        }
        let buses = world
            .audio_nodes()
            .filter_map(|(n, _)| {
                n.bus.as_ref().map(|b| {
                    (
                        n.entity,
                        BusState {
                            parent: b.parent,
                            name: binding::name(n.entity),
                            gain: b.gain_db,
                        },
                    )
                })
            })
            .collect::<BTreeMap<_, _>>();
        let mut candidate = SceneAudio {
            mixer: None,
            clips,
            buses,
            sources: BTreeMap::new(),
            retired: self.state.retired.clone(),
            prepared: Vec::with_capacity(crate::MAX_VOICES),
            listener: None,
            scene: world.scene_id(),
            tick: world.tick(),
            streaming_supported: true,
            identity: self.identity.clone(),
            incarnation: self.incarnation,
        };
        let listener = candidate.prepare(world)?;
        let definitions = candidate
            .buses
            .values()
            .map(|b| Bus {
                id: b.name.clone(),
                parent: b.parent.map(binding::name),
                gain_db: b.gain,
            })
            .collect();
        // Charge the complete unique static PCM set before decoding any of it.
        // Retired voices count as well as current declarations. Each immutable
        // sample array is decoded once and transferred into the candidate mixer.
        let mut static_sizes = BTreeMap::new();
        for row in &self.state.voices {
            if matches!(row.state, Transport::Static(_)) {
                let clip = &candidate.clips[&row.clip].clip;
                static_sizes.insert(clip.metadata.content_sha256.clone(), clip.metadata.frames);
            }
        }
        static_sizes.values().try_fold(0u64, |n, &frames| {
            n.checked_add(frames)
                .filter(|&n| n <= crate::mixer::MAX_STATIC_FRAMES as u64)
                .ok_or(AudioError::Capacity)
        })?;
        let mut samples = BTreeMap::<String, Arc<[kira::Frame]>>::new();
        let mut voices = Vec::with_capacity(self.state.voices.len());
        for row in &self.state.voices {
            let clip = Arc::clone(&candidate.clips[&row.clip].clip);
            let transport = match row.state {
                Transport::Stream(state) => TransportCheckpoint::Stream(state),
                Transport::Static(bytes) => {
                    let key = &clip.metadata.content_sha256;
                    let frames = if let Some(frames) = samples.get(key) {
                        Arc::clone(frames)
                    } else {
                        let frames: Arc<[kira::Frame]> = clip
                            .read_all(crate::mixer::MAX_STATIC_FRAMES)?
                            .into_iter()
                            .map(|v| kira::Frame::new(v[0], v[1]))
                            .collect::<Vec<_>>()
                            .into();
                        samples.insert(key.clone(), Arc::clone(&frames));
                        frames
                    };
                    let mut data = kira::sound::static_sound::StaticSoundData {
                        sample_rate: clip.metadata.sample_rate,
                        frames,
                        settings: Default::default(),
                        slice: None,
                    };
                    if row.options.looping {
                        data = data.loop_region(..);
                    }
                    TransportCheckpoint::Static(
                        kira::sound::static_sound::Checkpoint::from_bytes(&bytes, &data)
                            .map_err(|_| AudioError::Invalid("static audio state/PCM"))?,
                    )
                }
            };
            let mut options = row.options.clone();
            options.bus = row.bus.map(binding::name);
            voices.push(crate::mixer::checkpoint::VoiceCheckpoint {
                id: row.id,
                clip,
                options,
                transport,
                retained: row.retained,
            });
        }
        let mixer = Mixer::restore_with_samples(
            &MixerCheckpoint {
                startup: self.state.startup,
                suspended: self.state.suspended,
                master_paused: self.state.master_paused,
                pause_pending: self.state.pause_pending,
                resume_pending: self.state.resume_pending,
                sample_rate,
                rendered_frames: self.state.frames,
                next: self.state.next,
                buses: definitions,
                listener: self.state.listener,
                main_gain: self.state.main_gain,
                voices,
            },
            samples,
        )?;
        for (saved, prepared) in self.state.sources.iter().zip(&candidate.prepared) {
            candidate.sources.insert(
                saved.id,
                Source {
                    desired: prepared.desired.clone(),
                    position: prepared.position,
                    voice: saved.voice,
                    pending: saved.pending,
                },
            );
        }
        candidate.listener = listener;
        candidate.mixer = Some(mixer);
        Ok(candidate)
    }
}
impl<B: Backend> SceneAudio<B>
where
    B::Error: Debug,
{
    /// Capture the retained owner at a settled offline boundary. Device-backed
    /// owners explicitly refuse; physical output cannot be rewound by this API.
    pub fn checkpoint_for_world(&self, world: &NativeWorld) -> Result<PendingAudioCheckpoint> {
        if self.identity
            != *world
                .spatial_view()
                .map_err(|_| AudioError::Invalid("audio world unavailable"))?
                .identity()
            || self.incarnation != world.incarnation()
            || self.tick != world.tick()
        {
            return Err(AudioError::Invalid("audio host/world lifetime or tick"));
        }
        let mixer = self
            .mixer
            .as_ref()
            .ok_or(AudioError::FailedSession)?
            .checkpoint()?;
        let catalog = catalog(&self.clips)?;
        let mut voices = Vec::with_capacity(mixer.voices.len());
        for voice in mixer.voices {
            let clip = self
                .clips
                .iter()
                .find(|(_, binding)| Arc::ptr_eq(&voice.clip, &binding.clip))
                .map(|(&id, _)| id)
                .ok_or(AudioError::Invalid("voice clip ownership"))?;
            let bus = voice
                .options
                .bus
                .as_ref()
                .map(|name| {
                    self.buses
                        .iter()
                        .find(|(_, b)| &b.name == name)
                        .map(|(&id, _)| id)
                        .ok_or(AudioError::UnknownBus)
                })
                .transpose()?;
            let state = match voice.transport {
                TransportCheckpoint::Static(s) => Transport::Static(*s.as_bytes()),
                TransportCheckpoint::Stream(s) => Transport::Stream(s),
            };
            voices.push(SavedVoice {
                id: voice.id,
                clip,
                bus,
                options: voice.options,
                retained: voice.retained,
                state,
            });
        }
        let retired = self.retired.clone();
        let state = State {
            origin: world.source_scene_fingerprint(),
            scene: self.scene,
            tick: self.tick,
            sample_rate: mixer.sample_rate,
            frames: mixer.rendered_frames,
            next: mixer.next,
            listener: mixer.listener,
            main_gain: mixer.main_gain,
            startup: mixer.startup,
            suspended: mixer.suspended,
            master_paused: mixer.master_paused,
            pause_pending: mixer.pause_pending,
            resume_pending: mixer.resume_pending,
            catalog,
            sources: self
                .sources
                .iter()
                .map(|(&id, s)| SavedSource {
                    id,
                    transport_epoch: s.desired.transport_epoch,
                    voice: s.voice,
                    pending: s.pending,
                })
                .collect(),
            retired,
            voices,
        };
        validate_world(&state, world)?;
        // Also compare declarations with the accepted host cache. Same-tick
        // structural edits must synchronize before a checkpoint can publish.
        let mut listener = None;
        for (node, matrix) in world.audio_nodes() {
            if node.listener {
                listener = Some(Listener {
                    position: prepare::position(matrix)?,
                    rotation: world
                        .world_rotation(node.entity)
                        .ok_or(AudioError::Invalid("listener rotation"))?
                        .map(|v| v as f32),
                });
            }
            if let Some(source) = &node.source {
                let cached = &self.sources[&node.entity];
                let position = if source.spatial.is_some() {
                    prepare::position(matrix)?
                } else {
                    [0.; 3]
                };
                if source != &cached.desired || position != cached.position {
                    return Err(AudioError::Invalid("uncommitted audio declarations"));
                }
            }
            if let Some(bus) = &node.bus {
                let cached = self.buses.get(&node.entity).ok_or(AudioError::UnknownBus)?;
                if cached.parent != bus.parent || cached.gain != bus.gain_db {
                    return Err(AudioError::Invalid("uncommitted audio bus"));
                }
            }
        }
        if listener != self.listener {
            return Err(AudioError::Invalid("uncommitted listener pose"));
        }
        let bytes = codec::encode(&state)?;
        PendingAudioCheckpoint::new(world, &bytes)
    }
}
impl PartialEq for Catalog {
    fn eq(&self, b: &Self) -> bool {
        self.id == b.id
            && self.fingerprint == b.fingerprint
            && self.content == b.content
            && self.rate == b.rate
            && self.frames == b.frames
    }
}
fn digest(s: &str) -> Result<[u8; 32]> {
    if s.len() != 64 {
        return Err(AudioError::Invalid("audio fingerprint"));
    }
    let mut out = [0; 32];
    for (i, p) in s.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        let digit = |b: u8| match b {
            b'0'..=b'9' => Some(b - b'0'),
            b'a'..=b'f' => Some(b - b'a' + 10),
            _ => None,
        };
        out[i] = digit(p[0])
            .zip(digit(p[1]))
            .map(|(a, b)| a * 16 + b)
            .ok_or(AudioError::Invalid("audio fingerprint"))?;
    }
    Ok(out)
}
fn catalog(clips: &BTreeMap<StableId, ClipBinding>) -> Result<Vec<Catalog>> {
    clips
        .iter()
        .map(|(&id, b)| {
            Ok(Catalog {
                id,
                fingerprint: digest(&b.clip.metadata.fingerprint)?,
                content: digest(&b.clip.metadata.content_sha256)?,
                rate: b.clip.metadata.sample_rate,
                frames: b.clip.metadata.frames,
            })
        })
        .collect()
}
fn validate_world(state: &State, world: &NativeWorld) -> Result<()> {
    if state.scene != world.scene_id()
        || state.tick != world.tick()
        || state.origin != world.source_scene_fingerprint()
    {
        return Err(AudioError::Invalid("audio world binding"));
    }
    let nodes = world.audio_nodes().collect::<Vec<_>>();
    for (node, matrix) in &nodes {
        if node.listener {
            let pose = (
                prepare::position(*matrix)?,
                world
                    .world_rotation(node.entity)
                    .ok_or(AudioError::Invalid("listener rotation"))?
                    .map(|v| v as f32),
            );
            if pose != state.listener {
                return Err(AudioError::Invalid("saved listener pose"));
            }
        }
        if node.source.is_some()
            && let Some(voice) = state
                .sources
                .iter()
                .find(|s| s.id == node.entity)
                .and_then(|s| s.voice)
            && let Some(spatial) = state
                .voices
                .iter()
                .find(|v| v.id == voice)
                .and_then(|v| v.options.spatial)
            && spatial.position != prepare::position(*matrix)?
        {
            return Err(AudioError::Invalid("saved source position"));
        }
    }
    let sources = nodes
        .iter()
        .filter_map(|(n, _)| n.source.as_ref().map(|s| (n.entity, s)))
        .collect::<Vec<_>>();
    if state.sources.len() != sources.len() {
        return Err(AudioError::Invalid("audio source membership"));
    }
    let mut referenced = BTreeSet::new();
    for (saved, (id, source)) in state.sources.iter().zip(sources) {
        if saved.id != id
            || saved.transport_epoch != source.transport_epoch
            || (saved.pending && (saved.voice.is_some() || !source.playing))
        {
            return Err(AudioError::Invalid("audio source state"));
        }
        if !state.catalog.iter().any(|c| c.id == source.clip) {
            return Err(AudioError::Invalid("source clip catalog"));
        }
        if let Some(voice) = saved.voice {
            if !referenced.insert(voice) {
                return Err(AudioError::Invalid("duplicate audio owner"));
            }
            let row = state
                .voices
                .iter()
                .find(|v| v.id == voice)
                .ok_or(AudioError::Invalid("missing source voice"))?;
            let transport_state = match row.state {
                Transport::Static(b) => b[12],
                Transport::Stream(s) => s.state,
            };
            if (transport_state == 0 && !source.playing) || (transport_state == 1 && source.playing)
            {
                return Err(AudioError::Invalid("source play/transport state"));
            }
            if row.clip != source.clip
                || row.bus != source.bus
                || row.options.looping != source.looping
                || row.options.streaming != source.streaming
                || row.options.start_seconds != source.start_seconds
                || row.options.gain_db != source.gain_db
                || row.options.pan != source.pan
                || row.options.rate != source.rate
                || row
                    .options
                    .spatial
                    .map(|s| (s.min_distance, s.max_distance))
                    != source
                        .spatial
                        .as_ref()
                        .map(|s| (s.min_distance, s.max_distance))
            {
                return Err(AudioError::Invalid("source voice configuration"));
            }
        }
    }
    for &id in &state.retired {
        if state
            .voices
            .iter()
            .find(|v| v.id == id)
            .is_none_or(|v| match v.state {
                Transport::Static(b) => b[12] != 2,
                Transport::Stream(s) => s.state != 2,
            })
        {
            return Err(AudioError::Invalid("retired voice is not stopped"));
        }
        if !referenced.insert(id) {
            return Err(AudioError::Invalid("duplicate retired voice"));
        }
    }
    if referenced.len() != state.voices.len()
        || state.voices.iter().any(|v| !referenced.contains(&v.id))
    {
        return Err(AudioError::Invalid("voice owner membership"));
    }
    for row in &state.voices {
        if !state.catalog.iter().any(|c| c.id == row.clip)
            || row
                .bus
                .is_some_and(|id| !nodes.iter().any(|(n, _)| n.entity == id && n.bus.is_some()))
        {
            return Err(AudioError::Invalid("voice asset/bus membership"));
        }
    }
    Ok(())
}

//! Project audio declarations projected onto a mixer. Hosts call sync after a
//! committed simulation tick. Runtime failures are terminal: no partially
//! applied tick may be presented as successful audio or resumed silently.
use crate::{
    AudioError, Bus, Mixer, OfflineBackend, PlayOptions, PlaybackState, Result, Spatial, VoiceId,
};
use glam::DQuat;
use incant_assets::{AssetStore, CookedAudio, RuntimeAssetData};
use incant_core::RuntimeSnapshot;
use incant_doc::{AudioBus, AudioSource, Project};
use kira::{AudioManagerSettings, backend::Backend};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Debug,
    sync::Arc,
};

pub struct SceneAudio<B: Backend> {
    mixer: Mixer<B>,
    layout: BTreeMap<String, Option<String>>,
    sources: BTreeMap<String, Source>,
    failed: bool,
}
struct Source {
    declaration: AudioSource,
    fingerprint: String,
    voice: Option<VoiceId>,
}
struct Prepared {
    id: String,
    declaration: AudioSource,
    clip: Arc<CookedAudio>,
    options: PlayOptions,
}
fn buses(project: &Project) -> Result<Vec<Bus>> {
    project.validate()?;
    project
        .scenes
        .values()
        .flat_map(|s| s.entities.values())
        .filter_map(|e| {
            e.components.get("AudioBus").map(|value| {
                let b: AudioBus = serde_json::from_value(value.clone())?;
                Ok(Bus {
                    id: e.id.clone(),
                    parent: b.parent,
                    gain_db: b.gain_db,
                })
            })
        })
        .collect()
}
fn layout(buses: &[Bus]) -> BTreeMap<String, Option<String>> {
    buses
        .iter()
        .map(|b| (b.id.clone(), b.parent.clone()))
        .collect()
}
fn position(snapshot: &RuntimeSnapshot, id: &str) -> Result<[f32; 3]> {
    let e = snapshot.entities.get(id).ok_or(AudioError::Invalid(
        "audio transform is absent from runtime",
    ))?;
    let p = [
        e.world_transform[3][0] as f32,
        e.world_transform[3][1] as f32,
        e.world_transform[3][2] as f32,
    ];
    crate::config::position(p)?;
    Ok(p)
}
fn rotation(snapshot: &RuntimeSnapshot, id: &str) -> Result<[f32; 4]> {
    // Compose rotations directly. Nonuniform ancestor scale affects position,
    // but must not shear a listener's orthonormal orientation.
    let mut next = Some(id);
    let mut seen = BTreeSet::new();
    let mut rotation = DQuat::IDENTITY;
    while let Some(id) = next {
        if !seen.insert(id) {
            return Err(AudioError::Invalid("runtime audio parent cycle"));
        }
        let e = snapshot
            .entities
            .get(id)
            .ok_or(AudioError::Invalid("audio parent is absent from runtime"))?;
        let local = DQuat::from_array(e.rotation);
        if !local.is_finite() || local.length_squared() < 1e-12 {
            return Err(AudioError::Invalid("audio rotation is invalid"));
        }
        rotation = local.normalize() * rotation;
        next = e.parent.as_deref();
    }
    Ok(rotation.normalize().as_quat().to_array())
}
impl<B: Backend> SceneAudio<B>
where
    B::Error: Debug,
{
    pub fn new(settings: AudioManagerSettings<B>, project: &Project) -> Result<Self> {
        let buses = buses(project)?;
        Ok(Self {
            mixer: Mixer::new(settings, &buses)?,
            layout: layout(&buses),
            sources: BTreeMap::new(),
            failed: false,
        })
    }
    pub fn sync(
        &mut self,
        project: &Project,
        snapshot: &RuntimeSnapshot,
        assets: &AssetStore,
    ) -> Result<()> {
        if self.failed {
            return Err(AudioError::FailedSession);
        }
        let result = self.sync_inner(project, snapshot, assets);
        self.failed = result.is_err();
        result
    }
    fn stop_if_active(&mut self, id: VoiceId) -> Result<()> {
        if self
            .mixer
            .state(id)
            .is_ok_and(|s| s != PlaybackState::Stopped)
        {
            self.mixer.stop(id)?;
        }
        Ok(())
    }
    fn check_errors(&mut self) -> Result<()> {
        let errors = self.mixer.stream_errors();
        if errors.is_empty() {
            Ok(())
        } else {
            Err(AudioError::Stream(format!("{errors:?}")))
        }
    }
    fn sync_inner(
        &mut self,
        project: &Project,
        snapshot: &RuntimeSnapshot,
        assets: &AssetStore,
    ) -> Result<()> {
        self.check_errors()?;
        let buses = buses(project)?;
        if layout(&buses) != self.layout {
            return Err(AudioError::Unsupported(
                "changing bus topology requires restarting playback",
            ));
        }
        let mut listener = None;
        let mut prepared = Vec::new();
        // Validate the complete declaration and resolve every retained asset
        // before issuing commands to the running mixer.
        for e in project.scenes.values().flat_map(|s| s.entities.values()) {
            if e.components.contains_key("AudioListener") {
                listener = Some((position(snapshot, &e.id)?, rotation(snapshot, &e.id)?));
            }
            let Some(value) = e.components.get("AudioSource") else {
                continue;
            };
            let declaration: AudioSource = serde_json::from_value(value.clone())?;
            let asset = assets
                .get(&declaration.clip)
                .ok_or_else(|| AudioError::MissingAsset(declaration.clip.clone()))?;
            let RuntimeAssetData::Audio(clip) = asset.data() else {
                return Err(AudioError::MissingAsset(declaration.clip));
            };
            if asset.info().fingerprint != project.assets[&declaration.clip].sha256 {
                return Err(AudioError::MissingAsset(declaration.clip));
            }
            let spatial = declaration
                .spatial
                .as_ref()
                .map(|s| {
                    Ok::<_, AudioError>(Spatial {
                        position: position(snapshot, &e.id)?,
                        min_distance: s.min_distance,
                        max_distance: s.max_distance,
                    })
                })
                .transpose()?;
            let options = PlayOptions {
                bus: declaration.bus.clone(),
                gain_db: declaration.gain_db,
                pan: declaration.pan,
                rate: declaration.rate,
                start_seconds: declaration.start_seconds,
                looping: declaration.looping,
                streaming: declaration.streaming,
                spatial,
            };
            options.validate(clip.metadata.frames as f64 / f64::from(clip.metadata.sample_rate))?;
            prepared.push(Prepared {
                id: e.id.clone(),
                declaration,
                clip: Arc::clone(clip),
                options,
            });
        }
        if let Some((p, r)) = listener {
            self.mixer.set_listener(p, r)?;
        }
        for bus in buses {
            self.mixer.set_bus_gain(Some(&bus.id), bus.gain_db)?;
        }
        let ids: BTreeSet<_> = prepared.iter().map(|p| p.id.as_str()).collect();
        let removed: Vec<_> = self
            .sources
            .keys()
            .filter(|id| !ids.contains(id.as_str()))
            .cloned()
            .collect();
        for id in removed {
            if let Some(voice) = self.sources.remove(&id).unwrap().voice {
                self.stop_if_active(voice)?;
            }
        }
        for p in prepared {
            let previous = self.sources.remove(&p.id);
            let mut voice = previous.as_ref().and_then(|s| s.voice);
            let restart = previous.as_ref().is_some_and(|s| {
                let mut prior = s.declaration.clone();
                prior.playing = p.declaration.playing;
                prior.gain_db = p.declaration.gain_db;
                prior.pan = p.declaration.pan;
                prior.rate = p.declaration.rate;
                prior != p.declaration || s.fingerprint != p.clip.metadata.fingerprint
            });
            if restart && let Some(id) = voice.take() {
                self.stop_if_active(id)?;
            }
            let stopped = voice.is_none_or(|v| {
                self.mixer
                    .state(v)
                    .map_or(true, |s| s == PlaybackState::Stopped)
            });
            let starts = p.declaration.playing
                && (previous.as_ref().is_none_or(|s| !s.declaration.playing) || restart);
            if stopped && starts {
                voice = Some(self.mixer.play(Arc::clone(&p.clip), p.options.clone())?);
            }
            if let Some(id) = voice.filter(|v| {
                self.mixer
                    .state(*v)
                    .is_ok_and(|s| s != PlaybackState::Stopped)
            }) {
                self.mixer
                    .configure_voice(id, p.options.gain_db, p.options.pan, p.options.rate)?;
                self.mixer.set_paused(id, !p.declaration.playing)?;
                if let Some(s) = p.options.spatial {
                    self.mixer.set_position(id, s.position)?;
                }
            }
            self.sources.insert(
                p.id,
                Source {
                    declaration: p.declaration,
                    fingerprint: p.clip.metadata.fingerprint.clone(),
                    voice,
                },
            );
        }
        self.check_errors()
    }
}
impl SceneAudio<OfflineBackend> {
    pub fn offline(sample_rate: u32, project: &Project) -> Result<Self> {
        let buses = buses(project)?;
        Ok(Self {
            mixer: Mixer::offline(sample_rate, &buses)?,
            layout: layout(&buses),
            sources: BTreeMap::new(),
            failed: false,
        })
    }
    pub fn render(&mut self, output: &mut [f32]) -> Result<()> {
        if self.failed {
            return Err(AudioError::FailedSession);
        }
        let result = self.mixer.render(output).and_then(|_| self.check_errors());
        self.failed = result.is_err();
        result
    }
}

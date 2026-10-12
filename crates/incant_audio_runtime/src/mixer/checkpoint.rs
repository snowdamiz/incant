//! Exact offline transport boundaries. No raw Kira arenas or device handles are
//! serialized. Native voice IDs are monotonic birth order; Kira's pinned arena
//! iterates newest first, so reconstruction uses that same insertion sequence.
use super::*;
use kira::sound::static_sound::Checkpoint as StaticCheckpoint;

// Shared play construction accepts this type; only checkpoint-enabled owners
// construct its variants.
#[cfg_attr(not(any(feature = "scene", test)), allow(dead_code))]
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum TransportCheckpoint {
    Static(StaticCheckpoint),
    Stream(crate::offline_stream::Checkpoint),
}
#[cfg(any(feature = "scene", test))]
impl TransportCheckpoint {
    fn stopped(self) -> bool {
        match self {
            Self::Static(s) => s.as_bytes()[12] == 2,
            Self::Stream(s) => s.state == 2,
        }
    }
}
#[cfg(any(feature = "scene", test))]
#[derive(Clone)]
pub(crate) struct VoiceCheckpoint {
    pub id: VoiceId,
    pub clip: Arc<CookedAudio>,
    pub options: PlayOptions,
    pub transport: TransportCheckpoint,
    pub retained: bool,
}
#[cfg(any(feature = "scene", test))]
#[derive(Clone)]
pub(crate) struct MixerCheckpoint {
    pub startup: bool,
    pub suspended: bool,
    pub master_paused: bool,
    pub pause_pending: bool,
    pub resume_pending: bool,
    pub sample_rate: u32,
    pub rendered_frames: u64,
    pub next: VoiceId,
    pub buses: Vec<Bus>,
    pub listener: ([f32; 3], [f32; 4]),
    pub main_gain: f32,
    pub voices: Vec<VoiceCheckpoint>,
}
#[cfg(any(feature = "scene", test))]
impl<B: Backend> Mixer<B>
where
    B::Error: Debug,
{
    pub(crate) fn checkpoint(&self) -> Result<MixerCheckpoint> {
        if !self.synchronous_streams {
            return Err(AudioError::Unsupported("device audio checkpoint"));
        }
        if (!self.settled && !self.startup) || !self.errors.is_empty() {
            return Err(AudioError::Invalid(
                "audio checkpoint requires an accepted render or initial startup boundary",
            ));
        }
        let master_paused = match self.master.state() {
            kira::track::TrackPlaybackState::Playing => false,
            kira::track::TrackPlaybackState::Paused => true,
            _ => return Err(AudioError::Invalid("unsettled audio master")),
        };
        let mut voices = Vec::with_capacity(self.voices.len());
        for (&id, v) in &self.voices {
            let (transport, retained) = match &v.sound {
                VoiceSound::OfflineStatic(h) => (
                    TransportCheckpoint::Static(h.checkpoint().map_err(resource)?),
                    h.retained(),
                ),
                VoiceSound::Offline(h) => {
                    (TransportCheckpoint::Stream(h.checkpoint()), h.retained())
                }
                VoiceSound::Removed(state) => (*state, false),
                _ => return Err(AudioError::Unsupported("non-offline voice")),
            };
            voices.push(VoiceCheckpoint {
                id,
                clip: Arc::clone(&v.clip),
                options: v.options.clone(),
                transport,
                retained,
            });
        }
        Ok(MixerCheckpoint {
            startup: self.startup,
            suspended: self.suspended,
            master_paused,
            pause_pending: self.pause_pending,
            resume_pending: self.resume_pending,
            sample_rate: self.sample_rate,
            rendered_frames: self.rendered_frames,
            next: self.next,
            buses: self.definitions.clone(),
            listener: self.listener_pose,
            main_gain: self.main_gain,
            voices,
        })
    }
}
#[cfg(any(feature = "scene", test))]
impl Mixer<crate::OfflineBackend> {
    #[cfg(test)]
    pub(crate) fn restore(state: &MixerCheckpoint) -> Result<Self> {
        Self::restore_with_samples(state, BTreeMap::new())
    }
    pub(crate) fn restore_with_samples(
        state: &MixerCheckpoint,
        mut static_clips: BTreeMap<String, Arc<[Frame]>>,
    ) -> Result<Self> {
        if !(8000..=192000).contains(&state.sample_rate)
            || state.voices.len() > MAX_VOICES
            || state.next == 0
            || (state.startup && (state.rendered_frames != 0 || state.master_paused))
        {
            return Err(AudioError::Invalid("audio checkpoint dimensions"));
        }
        config::gain(state.main_gain)?;
        config::position(state.listener.0)?;
        if state.listener.1.iter().any(|v| !v.is_finite())
            || (state.listener.1.iter().map(|v| v * v).sum::<f32>() - 1.).abs() > 1e-4
        {
            return Err(AudioError::Invalid("audio checkpoint listener"));
        }
        let mut previous = 0;
        let mut total = static_clips.values().try_fold(0usize, |n, frames| {
            n.checked_add(frames.len())
                .filter(|&n| n <= MAX_STATIC_FRAMES)
                .ok_or(AudioError::Capacity)
        })?;
        // Validate every transport against retained cooked PCM before creating
        // the mixer. This is bounded startup work, never the callback path.
        for v in &state.voices {
            if v.id <= previous || v.id >= state.next || (!v.retained && !v.transport.stopped()) {
                return Err(AudioError::Invalid(
                    "audio checkpoint voice identity/lifecycle",
                ));
            }
            previous = v.id;
            v.options
                .validate(v.clip.metadata.frames as f64 / f64::from(v.clip.metadata.sample_rate))?;
            match v.transport {
                TransportCheckpoint::Stream(s) if v.options.streaming => s.validate(&v.clip)?,
                TransportCheckpoint::Static(s) if !v.options.streaming => {
                    let key = &v.clip.metadata.content_sha256;
                    let frames = if let Some(frames) = static_clips.get(key) {
                        Arc::clone(frames)
                    } else {
                        let n = usize::try_from(v.clip.metadata.frames)
                            .map_err(|_| AudioError::Capacity)?;
                        total = total
                            .checked_add(n)
                            .filter(|&n| n <= MAX_STATIC_FRAMES)
                            .ok_or(AudioError::Capacity)?;
                        let frames: Arc<[Frame]> = v
                            .clip
                            .read_all(n)?
                            .into_iter()
                            .map(|f| Frame::new(f[0], f[1]))
                            .collect::<Vec<_>>()
                            .into();
                        static_clips.insert(key.clone(), Arc::clone(&frames));
                        frames
                    };
                    let mut data = StaticSoundData {
                        sample_rate: v.clip.metadata.sample_rate,
                        frames,
                        settings: StaticSoundSettings::default(),
                        slice: None,
                    };
                    if v.options.looping {
                        data = data.loop_region(..);
                    }
                    StaticCheckpoint::from_bytes(s.as_bytes(), &data).map_err(resource)?;
                }
                _ => return Err(AudioError::Invalid("audio checkpoint transport kind")),
            }
        }
        let mut candidate = Self::new_at(
            AudioManagerSettings {
                backend_settings: state.sample_rate,
                ..Default::default()
            },
            &state.buses,
            if state.startup {
                ([0.; 3], [0., 0., 0., 1.])
            } else {
                state.listener
            },
            Some(state.main_gain),
            state.master_paused,
        )?;
        if state.startup {
            candidate.set_listener(state.listener.0, state.listener.1)?;
        }
        candidate.synchronous_streams = true;
        candidate.sample_rate = state.sample_rate;
        candidate.samples = static_clips;
        for v in &state.voices {
            candidate.next = v.id;
            candidate.play_inner(
                Arc::clone(&v.clip),
                v.options.clone(),
                false,
                Some((v.transport, v.retained)),
            )?;
        }
        candidate.next = state.next;
        candidate.rendered_frames = state.rendered_frames;
        // Publish queued track/sound objects without processing even one sample.
        // Retained stopped sounds remain until the next ordinary processing
        // boundary; already removed sounds have handle-only tombstones.
        if !state.startup {
            candidate
                .manager
                .backend_mut()
                .render(&mut [])
                .map_err(resource)?;
        }
        // Kira has independent last-value pause and resume queues, consumed in
        // that order. Preserve both rather than inferring them from one flag.
        if state.pause_pending {
            candidate.master.pause(immediate());
        }
        if state.resume_pending {
            candidate.master.resume(immediate());
        }
        candidate.startup = state.startup;
        candidate.settled = !state.startup;
        candidate.suspended = state.suspended;
        candidate.pause_pending = state.pause_pending;
        candidate.resume_pending = state.resume_pending;
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests;

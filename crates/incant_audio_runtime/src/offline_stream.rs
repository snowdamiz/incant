//! Bounded synchronous decoding for the offline backend only. File reads and
//! allocations here must never run on a device callback. Kira still owns bus
//! routing, mixing and spatialization; its public Hermite interpolator handles
//! fractional source positions without depending on a decoder thread's timing.
use crate::PlayOptions;
use incant_assets_runtime::{AUDIO_CHUNK_FRAMES, CookedAudio};
use kira::{
    Frame, Panning,
    info::Info,
    interpolate_frame,
    sound::{PlaybackState, Sound, SoundData},
};
use std::{
    collections::VecDeque,
    convert::Infallible,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering},
    },
};

const PLAYING: u8 = 0;
const PAUSED: u8 = 1;
const STOPPED: u8 = 2;
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Checkpoint {
    pub position: f64,
    pub state: u8,
    pub parameters: (f32, f32, f64),
}
#[cfg(any(feature = "scene", test))]
impl Checkpoint {
    pub fn validate(self, clip: &CookedAudio) -> crate::Result<()> {
        // A final sample may advance at most 96 source frames (192k / 8k * 4).
        // Loop normalization occurs before sampling, not after the increment.
        if !self.position.is_finite()
            || self.position < 0.0
            || self.position > clip.metadata.frames as f64 + 96.0
            || self.state > STOPPED
        {
            return Err(crate::AudioError::Invalid("stream cursor/state"));
        }
        PlayOptions {
            gain_db: self.parameters.0,
            pan: self.parameters.1,
            rate: self.parameters.2,
            ..Default::default()
        }
        .validate(0.)
    }
}
struct Shared {
    position: AtomicU64,
    retained: AtomicBool,
    state: AtomicU8,
    error: Mutex<Option<String>>,
    parameters: Mutex<(f32, f32, f64)>,
}
pub(crate) struct Handle(Arc<Shared>);
impl Handle {
    #[cfg(any(feature = "scene", test))]
    pub fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            position: f64::from_bits(self.0.position.load(Ordering::Relaxed)),
            state: self.0.state.load(Ordering::Relaxed),
            parameters: *self.0.parameters.lock().unwrap_or_else(|e| e.into_inner()),
        }
    }
    #[cfg(any(feature = "scene", test))]
    pub fn retained(&self) -> bool {
        self.0.retained.load(Ordering::Relaxed)
    }
    pub fn state(&self) -> PlaybackState {
        match self.0.state.load(Ordering::Relaxed) {
            PLAYING => PlaybackState::Playing,
            PAUSED => PlaybackState::Paused,
            _ => PlaybackState::Stopped,
        }
    }
    pub fn stop(&self) {
        self.0.state.store(STOPPED, Ordering::Relaxed);
    }
    pub fn pause(&self, paused: bool) {
        // Stopped sounds cannot be resurrected after the mixer has removed them.
        let _ = self
            .0
            .state
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |s| {
                (s != STOPPED).then_some(if paused { PAUSED } else { PLAYING })
            });
    }
    pub fn pop_error(&self) -> Option<String> {
        self.0
            .error
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .take()
    }
    pub fn configure(&self, gain: f32, pan: f32, rate: f64) {
        *self.0.parameters.lock().unwrap_or_else(|e| e.into_inner()) = (gain, pan, rate);
    }
}
pub(crate) struct Data {
    pub clip: Arc<CookedAudio>,
    pub options: PlayOptions,
    pub resume: Option<Checkpoint>,
}
impl SoundData for Data {
    type Error = Infallible;
    type Handle = Handle;
    fn into_sound(self) -> Result<(Box<dyn Sound>, Handle), Infallible> {
        let checkpoint = self.resume.unwrap_or(Checkpoint {
            position: self.options.start_seconds * f64::from(self.clip.metadata.sample_rate),
            state: PLAYING,
            parameters: (self.options.gain_db, self.options.pan, self.options.rate),
        });
        let shared = Arc::new(Shared {
            position: AtomicU64::new(checkpoint.position.to_bits()),
            retained: AtomicBool::new(true),
            state: AtomicU8::new(checkpoint.state),
            error: Mutex::new(None),
            parameters: Mutex::new(checkpoint.parameters),
        });
        let position = checkpoint.position;
        let gain = kira::Decibels(self.options.gain_db).as_amplitude();
        Ok((
            Box::new(OfflineSound {
                clip: self.clip,
                options: self.options,
                position,
                gain,
                chunks: VecDeque::new(),
                shared: Arc::clone(&shared),
            }),
            Handle(shared),
        ))
    }
}
struct OfflineSound {
    clip: Arc<CookedAudio>,
    options: PlayOptions,
    position: f64,
    gain: f32,
    // Four neighboring samples span at most two chunks (including a loop seam).
    chunks: VecDeque<(usize, Vec<[f32; 2]>)>,
    shared: Arc<Shared>,
}
impl OfflineSound {
    fn frame(&mut self, mut index: i64) -> Result<Frame, incant_assets_runtime::AssetError> {
        let length = self.clip.metadata.frames as i64;
        if self.options.looping {
            index = index.rem_euclid(length);
        }
        if index < 0 || index >= length {
            return Ok(Frame::ZERO);
        }
        let chunk = index as usize / AUDIO_CHUNK_FRAMES;
        if !self.chunks.iter().any(|(id, _)| *id == chunk) {
            if self.chunks.len() == 2 {
                self.chunks.pop_front();
            }
            self.chunks.push_back((chunk, self.clip.read_chunk(chunk)?));
        }
        let sample = self.chunks.iter().find(|(id, _)| *id == chunk).unwrap().1
            [index as usize % AUDIO_CHUNK_FRAMES];
        Ok(Frame::new(sample[0], sample[1]))
    }
    fn next(&mut self, dt: f64) -> Result<Frame, incant_assets_runtime::AssetError> {
        let length = self.clip.metadata.frames as f64;
        if self.options.looping {
            self.position = self.position.rem_euclid(length);
        }
        if self.position >= length {
            self.shared.state.store(STOPPED, Ordering::Relaxed);
            return Ok(Frame::ZERO);
        }
        let index = self.position.floor() as i64;
        let frame = interpolate_frame(
            self.frame(index - 1)?,
            self.frame(index)?,
            self.frame(index + 1)?,
            self.frame(index + 2)?,
            self.position.fract() as f32,
        );
        self.position += dt * f64::from(self.clip.metadata.sample_rate) * self.options.rate;
        Ok(frame.panned(Panning(self.options.pan)) * self.gain)
    }
}
impl Sound for OfflineSound {
    fn process(&mut self, out: &mut [Frame], dt: f64, _: &Info) {
        let (gain, pan, rate) = *self
            .shared
            .parameters
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.gain = kira::Decibels(gain).as_amplitude();
        self.options.pan = pan;
        self.options.rate = rate;
        out.fill(Frame::ZERO);
        for frame in out {
            if self.shared.state.load(Ordering::Relaxed) != PLAYING {
                break;
            }
            match self.next(dt) {
                Ok(sample) => *frame = sample,
                Err(error) => {
                    *self.shared.error.lock().unwrap_or_else(|e| e.into_inner()) =
                        Some(error.to_string());
                    self.shared.state.store(STOPPED, Ordering::Relaxed);
                    break;
                }
            }
        }
        self.shared
            .position
            .store(self.position.to_bits(), Ordering::Relaxed);
    }
    fn finished(&self) -> bool {
        let finished = self.shared.state.load(Ordering::Relaxed) == STOPPED;
        if finished {
            self.shared.retained.store(false, Ordering::Relaxed);
        }
        finished
    }
}

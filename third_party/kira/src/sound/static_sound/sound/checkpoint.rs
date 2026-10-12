//! Optional non-real-time adapter. The original StaticSound performs every
//! sample calculation; this wrapper only copies settled boundary state.
use super::{resampler::Resampler, StaticSound};
use crate::{
    info::Info,
    playback_state_manager::PlaybackStateManager,
    sound::{
        static_sound::{StaticSoundData, StaticSoundHandle},
        EndPosition, PlaybackPosition, PlaybackState, Sound, SoundData,
    },
    Decibels, Frame, Panning, Parameter, PlaybackRate, StartTime, Tween, Value,
};
use std::{
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

/// Fixed-size encoding for the supported forward, whole-clip static transport.
/// Clip bytes are supplied by the caller's checked asset binding, never embedded.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Checkpoint([u8; 144]);
/// Unsupported/unsettled data refuses without changing a sound or renderer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CheckpointError {
    /// Malformed data, bounds or immutable PCM mismatch.
    Invalid,
    /// The sound uses a transport capability outside this adapter.
    Unsupported,
    /// A command or tween has not reached an accepted processing boundary.
    Pending,
}
impl fmt::Display for CheckpointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "static checkpoint: {self:?}")
    }
}
impl std::error::Error for CheckpointError {}
type Result<T> = std::result::Result<T, CheckpointError>;
fn fixed<T: crate::Tweenable>(value: Value<T>) -> Result<T> {
    match value {
        Value::Fixed(value) => Ok(value),
        _ => Err(CheckpointError::Unsupported),
    }
}
fn f32_at(b: &[u8], p: usize) -> f32 {
    f32::from_le_bytes(b[p..p + 4].try_into().unwrap())
}
fn f64_at(b: &[u8], p: usize) -> f64 {
    f64::from_le_bytes(b[p..p + 8].try_into().unwrap())
}
fn u64_at(b: &[u8], p: usize) -> u64 {
    u64::from_le_bytes(b[p..p + 8].try_into().unwrap())
}
fn parameters(gain: f32, pan: f32, rate: f64) -> Result<()> {
    if !gain.is_finite()
        || !(-80.0..=24.0).contains(&gain)
        || !pan.is_finite()
        || !(-1.0..=1.0).contains(&pan)
        || !rate.is_finite()
        || !(0.25..=4.0).contains(&rate)
    {
        return Err(CheckpointError::Invalid);
    }
    Ok(())
}
fn position_samples(position: PlaybackPosition, rate: u32, length: usize) -> Result<usize> {
    match position {
        PlaybackPosition::Seconds(v)
            if !v.is_finite() || v < 0.0 || v > length as f64 / f64::from(rate) =>
        {
            Err(CheckpointError::Invalid)
        }
        PlaybackPosition::Samples(v) if v > length => Err(CheckpointError::Invalid),
        value => Ok(value.into_samples(rate)),
    }
}
fn validate_data(data: &StaticSoundData) -> Result<()> {
    let s = data.settings;
    if data.frames.is_empty()
        || data.frames.len() > 8 * 1024 * 1024
        || data.sample_rate == 0
        || data.sample_rate > 192000
        || data.slice.is_some()
        || s.reverse
        || s.start_time != StartTime::Immediate
        || s.fade_in_tween.is_some()
    {
        return Err(CheckpointError::Unsupported);
    }
    parameters(
        fixed(s.volume)?.0,
        fixed(s.panning)?.0,
        fixed(s.playback_rate)?.0,
    )?;
    match s.start_position {
        PlaybackPosition::Seconds(v)
            if !v.is_finite()
                || v < 0.0
                || v > data.frames.len() as f64 / data.sample_rate as f64 =>
        {
            return Err(CheckpointError::Invalid);
        }
        PlaybackPosition::Samples(v) if v > data.frames.len() => {
            return Err(CheckpointError::Invalid);
        }
        _ => {}
    }
    if let Some(region) = s.loop_region {
        let end = match region.end {
            EndPosition::EndOfAudio => data.frames.len(),
            EndPosition::Custom(p) => position_samples(p, data.sample_rate, data.frames.len())?,
        };
        if position_samples(region.start, data.sample_rate, data.frames.len())? != 0
            || end != data.frames.len()
        {
            return Err(CheckpointError::Unsupported);
        }
    }
    if data
        .frames
        .iter()
        .any(|f| !f.left.is_finite() || !f.right.is_finite())
    {
        return Err(CheckpointError::Invalid);
    }
    Ok(())
}
impl Checkpoint {
    /// Decode and validate against the actual immutable PCM data. This does not
    /// construct a sound, allocate sample storage, or run any sample calculation.
    pub fn from_bytes(bytes: &[u8], data: &StaticSoundData) -> Result<Self> {
        validate_data(data)?;
        let b: [u8; 144] = bytes.try_into().map_err(|_| CheckpointError::Invalid)?;
        if &b[..8] != b"KIRAST01"
            || u32::from_le_bytes(b[8..12].try_into().unwrap()) != data.sample_rate
            || b[12] > 2
            || b[13] > 1
            || b[14] > 1
            || b[15] != 0
            || u64_at(&b, 16) != data.frames.len() as u64
            || u64_at(&b, 24) > data.frames.len() as u64 + 1
            || !f64_at(&b, 32).is_finite()
            || !(0.0..1.0).contains(&f64_at(&b, 32))
            || ![0.0, -60.0].contains(&f32_at(&b, 56))
            || u32::from_le_bytes(b[60..64].try_into().unwrap()) > 4
        {
            return Err(CheckpointError::Invalid);
        }
        parameters(f32_at(&b, 40), f32_at(&b, 44), f64_at(&b, 48))?;
        let looping = data.settings.loop_region.is_some();
        if b[14] != u8::from(looping)
            || u64_at(&b, 64) != 0
            || u64_at(&b, 72) != if looping { data.frames.len() as u64 } else { 0 }
            || (b[13] == 1 && u64_at(&b, 24) >= data.frames.len() as u64)
        {
            return Err(CheckpointError::Invalid);
        }
        for i in 0..4 {
            let p = 80 + i * 16;
            let index = u64_at(&b, p);
            if index > data.frames.len() as u64 + 1 {
                return Err(CheckpointError::Invalid);
            }
            let frame = Frame::new(f32_at(&b, p + 8), f32_at(&b, p + 12));
            // Startup/drained slots contain zero. Other samples must be exact
            // immutable source PCM; untrusted data cannot inject a new waveform.
            if frame != Frame::ZERO && data.frames.get(index as usize) != Some(&frame) {
                return Err(CheckpointError::Invalid);
            }
        }
        Ok(Self(b))
    }
    /// Little-endian, fixed-size data; no Rust layout or executable state.
    pub fn as_bytes(&self) -> &[u8; 144] {
        &self.0
    }
    fn capture(s: &StaticSound) -> Result<Self> {
        let (state, fade) = s
            .playback_state_manager
            .settled_checkpoint()
            .ok_or(CheckpointError::Pending)?;
        let gain = s
            .volume
            .fixed_checkpoint_value()
            .ok_or(CheckpointError::Pending)?
            .0;
        let pan = s
            .panning
            .fixed_checkpoint_value()
            .ok_or(CheckpointError::Pending)?
            .0;
        let rate = s
            .playback_rate
            .fixed_checkpoint_value()
            .ok_or(CheckpointError::Pending)?
            .0;
        let (frames, remaining) = s.resampler.checkpoint();
        let mut b = [0; 144];
        b[..8].copy_from_slice(b"KIRAST01");
        b[8..12].copy_from_slice(&s.sample_rate.to_le_bytes());
        b[12] = match state {
            PlaybackState::Playing => 0,
            PlaybackState::Paused => 1,
            PlaybackState::Stopped => 2,
            _ => unreachable!(),
        };
        b[13] = u8::from(s.transport.playing);
        b[14] = u8::from(s.transport.loop_region.is_some());
        for (p, n) in [
            (16, s.frames.len()),
            (24, s.transport.position),
            (64, s.transport.loop_region.map_or(0, |v| v.0)),
            (72, s.transport.loop_region.map_or(0, |v| v.1)),
        ] {
            b[p..p + 8].copy_from_slice(&(n as u64).to_le_bytes());
        }
        b[32..40].copy_from_slice(&s.fractional_position.to_le_bytes());
        b[40..44].copy_from_slice(&gain.to_le_bytes());
        b[44..48].copy_from_slice(&pan.to_le_bytes());
        b[48..56].copy_from_slice(&rate.to_le_bytes());
        b[56..60].copy_from_slice(&fade.0.to_le_bytes());
        b[60..64].copy_from_slice(&(remaining as u32).to_le_bytes());
        for (i, (frame, index)) in frames.into_iter().enumerate() {
            let p = 80 + i * 16;
            b[p..p + 8].copy_from_slice(&(index as u64).to_le_bytes());
            b[p + 8..p + 12].copy_from_slice(&frame.left.to_le_bytes());
            b[p + 12..p + 16].copy_from_slice(&frame.right.to_le_bytes());
        }
        Ok(Self(b))
    }
    fn apply(self, s: &mut StaticSound) {
        let b = self.0;
        s.transport.position = u64_at(&b, 24) as usize;
        s.transport.playing = b[13] != 0;
        s.transport.loop_region =
            (b[14] != 0).then(|| (u64_at(&b, 64) as usize, u64_at(&b, 72) as usize));
        s.fractional_position = f64_at(&b, 32);
        s.volume = Parameter::new(Value::Fixed(Decibels(f32_at(&b, 40))), Decibels::IDENTITY);
        s.panning = Parameter::new(Value::Fixed(Panning(f32_at(&b, 44))), Panning::CENTER);
        s.playback_rate = Parameter::new(
            Value::Fixed(PlaybackRate(f64_at(&b, 48))),
            PlaybackRate(1.0),
        );
        s.playback_state_manager = PlaybackStateManager::restore_settled(
            match b[12] {
                0 => PlaybackState::Playing,
                1 => PlaybackState::Paused,
                _ => PlaybackState::Stopped,
            },
            Decibels(f32_at(&b, 56)),
        )
        .unwrap();
        s.resampler = Resampler::from_checkpoint(
            std::array::from_fn(|i| {
                let p = 80 + i * 16;
                (
                    Frame::new(f32_at(&b, p + 8), f32_at(&b, p + 12)),
                    u64_at(&b, p) as usize,
                )
            }),
            u32::from_le_bytes(b[60..64].try_into().unwrap()) as usize,
        );
        s.update_shared_playback_state();
    }
}
/// Opt-in offline wrapper. It retains original Kira sample and command math;
/// its mutex publication is intentionally not a real-time device callback API.
pub struct CheckpointData {
    data: StaticSoundData,
    resume: Option<Checkpoint>,
}
impl CheckpointData {
    /// Admit the bounded forward whole-clip subset used by the native mixer.
    pub fn new(data: StaticSoundData) -> Result<Self> {
        validate_data(&data)?;
        Ok(Self { data, resume: None })
    }
    /// Validate the snapshot against this exact PCM before constructing a voice.
    pub fn resume(data: StaticSoundData, state: Checkpoint) -> Result<Self> {
        let state = Checkpoint::from_bytes(state.as_bytes(), &data)?;
        Ok(Self {
            data,
            resume: Some(state),
        })
    }
}
/// Offline command handle. Commands invalidate capture until a processing
/// boundary consumes them; a stale published cursor is never returned as current.
pub struct CheckpointHandle {
    inner: StaticSoundHandle,
    shared: Arc<Mutex<Result<Checkpoint>>>,
    retained: Arc<AtomicBool>,
}
impl CheckpointHandle {
    /// Current settled state copied after the last processed block.
    pub fn checkpoint(&self) -> Result<Checkpoint> {
        *self.shared.lock().map_err(|_| CheckpointError::Pending)?
    }
    /// The same playback state as the wrapped original handle.
    pub fn state(&self) -> PlaybackState {
        self.inner.state()
    }
    /// Whether the renderer still retains this voice in its sound arena.
    /// Querying a finished voice at removal records the actual lifecycle boundary.
    pub fn retained(&self) -> bool {
        self.retained.load(Ordering::Relaxed)
    }
    fn dirty(&self) {
        *self.shared.lock().unwrap_or_else(|e| e.into_inner()) = Err(CheckpointError::Pending);
    }
    /// Immediate owned-source gain/pan/rate changes, as in the native mixer.
    pub fn configure(&mut self, gain: f32, pan: f32, rate: f64) -> Result<()> {
        parameters(gain, pan, rate)?;
        if self.state() == PlaybackState::Stopped {
            return Ok(());
        }
        self.dirty();
        let t = Tween {
            duration: Duration::ZERO,
            ..Tween::default()
        };
        self.inner.set_volume(gain, t);
        self.inner.set_panning(pan, t);
        self.inner.set_playback_rate(rate, t);
        Ok(())
    }
    /// Queue the existing immediate pause/resume command.
    pub fn pause(&mut self, paused: bool) {
        if self.state() == PlaybackState::Stopped {
            return;
        }
        self.dirty();
        let t = Tween {
            duration: Duration::ZERO,
            ..Tween::default()
        };
        if paused {
            self.inner.pause(t);
        } else {
            self.inner.resume(t);
        }
    }
    /// Queue the existing immediate stop command.
    pub fn stop(&mut self) {
        if self.state() == PlaybackState::Stopped {
            return;
        }
        self.dirty();
        self.inner.stop(Tween {
            duration: Duration::ZERO,
            ..Tween::default()
        });
    }
}
struct CheckpointSound {
    inner: StaticSound,
    shared: Arc<Mutex<Result<Checkpoint>>>,
    retained: Arc<AtomicBool>,
}
impl Sound for CheckpointSound {
    fn on_start_processing(&mut self) {
        self.inner.on_start_processing();
    }
    fn process(&mut self, out: &mut [Frame], dt: f64, info: &Info) {
        self.inner.process(out, dt, info);
        *self.shared.lock().unwrap_or_else(|e| e.into_inner()) = Checkpoint::capture(&self.inner);
    }
    fn finished(&self) -> bool {
        let finished = self.inner.finished();
        if finished {
            self.retained.store(false, Ordering::Relaxed);
        }
        finished
    }
}
impl SoundData for CheckpointData {
    type Error = CheckpointError;
    type Handle = CheckpointHandle;
    fn into_sound(self) -> Result<(Box<dyn Sound>, Self::Handle)> {
        let (mut inner, handle) = self.data.split();
        if let Some(state) = self.resume {
            state.apply(&mut inner);
        }
        let shared = Arc::new(Mutex::new(Checkpoint::capture(&inner)));
        let retained = Arc::new(AtomicBool::new(true));
        Ok((
            Box::new(CheckpointSound {
                inner,
                shared: Arc::clone(&shared),
                retained: Arc::clone(&retained),
            }),
            CheckpointHandle {
                inner: handle,
                shared,
                retained,
            },
        ))
    }
}

#[cfg(test)]
mod tests;

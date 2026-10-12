//! Explicit bounded little-endian audio data. No Kira layout/arena deserializer.
use super::*;
use sha2::{Digest, Sha256};
const HEADER: usize = 48;
// Header/body scalars <=176, catalog rows92, owner rows33, retired IDs8,
// voice rows <=240 (including the complete 144-byte static history).
pub const MAX_AUDIO_CHECKPOINT_BYTES: usize =
    176 + MAX_SCENE_CLIPS * 92 + crate::MAX_VOICES * (33 + 8 + 240);
const MAGIC: &[u8; 8] = b"INCAUD01";
fn invalid() -> AudioError {
    AudioError::Invalid("audio checkpoint encoding")
}
fn hash(bytes: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(&bytes[..16]);
    h.update(&bytes[HEADER..]);
    h.finalize().into()
}
fn id(out: &mut Vec<u8>, v: StableId) {
    out.extend(v.0);
}
fn option(out: &mut Vec<u8>, v: Option<StableId>) {
    out.push(u8::from(v.is_some()));
    if let Some(v) = v {
        id(out, v);
    }
}
fn f32w(out: &mut Vec<u8>, v: f32) {
    out.extend(v.to_le_bytes());
}
fn f64w(out: &mut Vec<u8>, v: f64) {
    out.extend(v.to_le_bytes());
}
pub(super) fn encode(state: &State) -> Result<Vec<u8>> {
    let mut out = vec![0; HEADER];
    out[..8].copy_from_slice(MAGIC);
    // Preserve v1 bytes for sources that have never received a transport
    // request. Version 2 binds each saved cursor to its consumed request.
    let version = if state.sources.iter().any(|s| s.transport_epoch != 0) {
        2u32
    } else {
        1u32
    };
    out[8..12].copy_from_slice(&version.to_le_bytes());
    out.extend(state.origin);
    id(&mut out, state.scene);
    out.extend(state.tick.to_le_bytes());
    out.extend(state.sample_rate.to_le_bytes());
    out.extend(state.frames.to_le_bytes());
    out.extend(state.next.to_le_bytes());
    for v in state.listener.0.into_iter().chain(state.listener.1) {
        f32w(&mut out, v);
    }
    f32w(&mut out, state.main_gain);
    for n in [
        state.catalog.len(),
        state.sources.len(),
        state.retired.len(),
        state.voices.len(),
    ] {
        out.extend(u16::try_from(n).map_err(|_| invalid())?.to_le_bytes());
    }
    out.push(
        u8::from(state.startup)
            | (u8::from(state.suspended) << 1)
            | (u8::from(state.master_paused) << 2)
            | (u8::from(state.pause_pending) << 3)
            | (u8::from(state.resume_pending) << 4),
    );
    for c in &state.catalog {
        id(&mut out, c.id);
        out.extend(c.fingerprint);
        out.extend(c.content);
        out.extend(c.rate.to_le_bytes());
        out.extend(c.frames.to_le_bytes());
    }
    for s in &state.sources {
        id(&mut out, s.id);
        if version >= 2 {
            out.extend(s.transport_epoch.to_le_bytes());
        }
        out.extend(s.voice.unwrap_or(0).to_le_bytes());
        out.push(u8::from(s.pending));
    }
    for &v in &state.retired {
        out.extend(v.to_le_bytes());
    }
    for v in &state.voices {
        out.extend(v.id.to_le_bytes());
        id(&mut out, v.clip);
        option(&mut out, v.bus);
        f32w(&mut out, v.options.gain_db);
        f32w(&mut out, v.options.pan);
        f64w(&mut out, v.options.rate);
        f64w(&mut out, v.options.start_seconds);
        out.push(u8::from(v.options.looping));
        out.push(u8::from(v.retained));
        out.push(u8::from(v.options.spatial.is_some()));
        if let Some(s) = v.options.spatial {
            for value in s
                .position
                .into_iter()
                .chain([s.min_distance, s.max_distance])
            {
                f32w(&mut out, value);
            }
        }
        match v.state {
            Transport::Static(b) => {
                out.push(0);
                out.extend(b);
            }
            Transport::Stream(s) => {
                out.push(1);
                f64w(&mut out, s.position);
                out.push(s.state);
                f32w(&mut out, s.parameters.0);
                f32w(&mut out, s.parameters.1);
                f64w(&mut out, s.parameters.2);
            }
        }
    }
    if out.len() > MAX_AUDIO_CHECKPOINT_BYTES {
        return Err(AudioError::Capacity);
    }
    let length = out.len() as u32;
    out[12..16].copy_from_slice(&length.to_le_bytes());
    let h = hash(&out);
    out[16..48].copy_from_slice(&h);
    // Exercise the same structural admission for internally produced records.
    decode(&out)?;
    Ok(out)
}
struct Reader<'a>(&'a [u8]);
impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        if n > self.0.len() {
            return Err(invalid());
        }
        let (a, b) = self.0.split_at(n);
        self.0 = b;
        Ok(a)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn bool(&mut self) -> Result<bool> {
        match self.byte()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(invalid()),
        }
    }
    fn id(&mut self) -> Result<StableId> {
        Ok(StableId(self.take(16)?.try_into().unwrap()))
    }
    fn optional(&mut self) -> Result<Option<StableId>> {
        if self.bool()? {
            Ok(Some(self.id()?))
        } else {
            Ok(None)
        }
    }
    fn u16(&mut self) -> Result<usize> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()) as usize)
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32> {
        let v = f32::from_bits(self.u32()?);
        if v.is_finite() { Ok(v) } else { Err(invalid()) }
    }
    fn f64(&mut self) -> Result<f64> {
        let v = f64::from_bits(self.u64()?);
        if v.is_finite() { Ok(v) } else { Err(invalid()) }
    }
}
pub(super) fn decode(bytes: &[u8]) -> Result<State> {
    if !(HEADER..=MAX_AUDIO_CHECKPOINT_BYTES).contains(&bytes.len())
        || &bytes[..8] != MAGIC
        || !(1..=2).contains(&u32::from_le_bytes(bytes[8..12].try_into().unwrap()))
        || u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize != bytes.len()
        || bytes[16..48] != hash(bytes)
    {
        return Err(invalid());
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    let owner_bytes = if version >= 2 { 33 } else { 25 };
    let mut r = Reader(&bytes[HEADER..]);
    let origin = r.take(32)?.try_into().unwrap();
    let scene = r.id()?;
    let tick = r.u64()?;
    let sample_rate = r.u32()?;
    let frames = r.u64()?;
    let next = r.u64()?;
    let listener = (
        [r.f32()?, r.f32()?, r.f32()?],
        [r.f32()?, r.f32()?, r.f32()?, r.f32()?],
    );
    let main_gain = r.f32()?;
    crate::config::position(listener.0)?;
    crate::config::gain(main_gain)?;
    if !(8000..=192000).contains(&sample_rate)
        || next == 0
        || (listener.1.iter().map(|v| v * v).sum::<f32>() - 1.).abs() > 1e-4
    {
        return Err(invalid());
    }
    let counts = [r.u16()?, r.u16()?, r.u16()?, r.u16()?];
    let flags = r.byte()?;
    if flags & !31 != 0 || (flags & 1 != 0 && (frames != 0 || flags & 4 != 0)) {
        return Err(invalid());
    }
    if counts[0] > MAX_SCENE_CLIPS
        || counts[1..].iter().any(|&v| v > crate::MAX_VOICES)
        || counts[0] * 92 + counts[1] * owner_bytes + counts[2] * 8 + counts[3] * 80 > r.0.len()
    {
        return Err(invalid());
    }
    let mut catalog = Vec::with_capacity(counts[0]);
    for _ in 0..counts[0] {
        let c = Catalog {
            id: r.id()?,
            fingerprint: r.take(32)?.try_into().unwrap(),
            content: r.take(32)?.try_into().unwrap(),
            rate: r.u32()?,
            frames: r.u64()?,
        };
        if !(8000..=192000).contains(&c.rate)
            || c.frames == 0
            || c.frames > incant_assets_runtime::MAX_AUDIO_PCM_BYTES as u64 / 8
            || catalog.last().is_some_and(|old: &Catalog| old.id >= c.id)
        {
            return Err(invalid());
        }
        catalog.push(c);
    }
    let mut sources = Vec::with_capacity(counts[1]);
    for _ in 0..counts[1] {
        let id = r.id()?;
        let transport_epoch = if version >= 2 { r.u64()? } else { 0 };
        let voice = r.u64()?;
        let pending = r.bool()?;
        if voice >= next || sources.last().is_some_and(|old: &SavedSource| old.id >= id) {
            return Err(invalid());
        }
        sources.push(SavedSource {
            id,
            transport_epoch,
            voice: (voice != 0).then_some(voice),
            pending,
        });
    }
    if version >= 2 && !sources.iter().any(|s| s.transport_epoch != 0) {
        return Err(invalid());
    }
    let mut retired = Vec::with_capacity(counts[2]);
    for _ in 0..counts[2] {
        let id = r.u64()?;
        if id == 0 || id >= next || retired.contains(&id) {
            return Err(invalid());
        }
        retired.push(id);
    }
    let mut voices = Vec::with_capacity(counts[3]);
    let mut static_clips = BTreeSet::new();
    let mut static_frames = 0u64;
    for _ in 0..counts[3] {
        let id = r.u64()?;
        let clip = r.id()?;
        let bus = r.optional()?;
        let gain_db = r.f32()?;
        let pan = r.f32()?;
        let rate = r.f64()?;
        let start_seconds = r.f64()?;
        let looping = r.bool()?;
        let retained = r.bool()?;
        let spatial = if r.bool()? {
            Some(crate::Spatial {
                position: [r.f32()?, r.f32()?, r.f32()?],
                min_distance: r.f32()?,
                max_distance: r.f32()?,
            })
        } else {
            None
        };
        let c = catalog.iter().find(|c| c.id == clip).ok_or_else(invalid)?;
        let state = match r.byte()? {
            0 => {
                let b: [u8; 144] = r.take(144)?.try_into().unwrap();
                // Independent envelope/numeric checks happen before actual PCM
                // binding. The Kira adapter then checks every history sample.
                let f = |p| f64::from_le_bytes(b[p..p + 8].try_into().unwrap());
                let n = |p| u64::from_le_bytes(b[p..p + 8].try_into().unwrap());
                let small = |p| f32::from_le_bytes(b[p..p + 4].try_into().unwrap());
                if &b[..8] != b"KIRAST01"
                    || u32::from_le_bytes(b[8..12].try_into().unwrap()) != c.rate
                    || b[12] > 2
                    || b[13] > 1
                    || b[14] != u8::from(looping)
                    || b[15] != 0
                    || n(16) != c.frames
                    || n(24) > c.frames + 1
                    || !f(32).is_finite()
                    || !(0.0..1.0).contains(&f(32))
                    || n(64) != 0
                    || n(72) != if looping { c.frames } else { 0 }
                    || (!retained && b[12] != 2)
                    || u32::from_le_bytes(b[60..64].try_into().unwrap()) > 4
                    || ![0., -60.].contains(&small(56))
                {
                    return Err(invalid());
                }
                PlayOptions {
                    gain_db: small(40),
                    pan: small(44),
                    rate: f(48),
                    ..Default::default()
                }
                .validate(0.)?;
                for i in 0..4 {
                    let p = 80 + i * 16;
                    if n(p) > c.frames + 1
                        || !small(p + 8).is_finite()
                        || !small(p + 12).is_finite()
                    {
                        return Err(invalid());
                    }
                }
                if static_clips.insert(clip) {
                    static_frames = static_frames
                        .checked_add(c.frames)
                        .ok_or(AudioError::Capacity)?;
                    if static_frames > crate::mixer::MAX_STATIC_FRAMES as u64 {
                        return Err(AudioError::Capacity);
                    }
                }
                Transport::Static(b)
            }
            1 => {
                let s = crate::offline_stream::Checkpoint {
                    position: r.f64()?,
                    state: r.byte()?,
                    parameters: (r.f32()?, r.f32()?, r.f64()?),
                };
                if s.position < 0.
                    || s.position > c.frames as f64 + 96.
                    || s.state > 2
                    || (!retained && s.state != 2)
                {
                    return Err(invalid());
                }
                PlayOptions {
                    gain_db: s.parameters.0,
                    pan: s.parameters.1,
                    rate: s.parameters.2,
                    ..Default::default()
                }
                .validate(0.)?;
                Transport::Stream(s)
            }
            _ => return Err(invalid()),
        };
        let options = PlayOptions {
            bus: None,
            gain_db,
            pan,
            rate,
            start_seconds,
            looping,
            streaming: matches!(state, Transport::Stream(_)),
            spatial,
        };
        options.validate(c.frames as f64 / f64::from(c.rate))?;
        let (playing, parameters) = match state {
            Transport::Static(b) => (
                b[12] != 2,
                (
                    f32::from_le_bytes(b[40..44].try_into().unwrap()),
                    f32::from_le_bytes(b[44..48].try_into().unwrap()),
                    f64::from_le_bytes(b[48..56].try_into().unwrap()),
                ),
            ),
            Transport::Stream(s) => (s.state != 2, s.parameters),
        };
        if playing && parameters != (gain_db, pan, rate) {
            return Err(invalid());
        }
        if id == 0 || id >= next || voices.last().is_some_and(|v: &SavedVoice| v.id >= id) {
            return Err(invalid());
        }
        voices.push(SavedVoice {
            id,
            clip,
            bus,
            options,
            retained,
            state,
        });
    }
    if !r.0.is_empty() {
        return Err(invalid());
    }
    Ok(State {
        origin,
        scene,
        tick,
        sample_rate,
        frames,
        next,
        listener,
        main_gain,
        startup: flags & 1 != 0,
        suspended: flags & 2 != 0,
        master_paused: flags & 4 != 0,
        pause_pending: flags & 8 != 0,
        resume_pending: flags & 16 != 0,
        catalog,
        sources,
        retired,
        voices,
    })
}

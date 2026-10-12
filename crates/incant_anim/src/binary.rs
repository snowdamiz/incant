//! Root-only binary payloads nested in a checksum-checked cooked scene. This
//! format checks bounds and canonical framing; it is not authentication.
use crate::{
    MAX_ROOT_MOTION_CATALOG_BYTES, MAX_ROOT_MOTION_CLIPS, MAX_ROOT_MOTION_KEYS, RootMotionCatalog,
    RootMotionClip, RootMotionDelta, RootMotionError, RootMotionTrack,
    track::{Interpolation, Kind},
};

const MAGIC: &[u8; 8] = b"INCROOT\0";
const VERSION: u32 = 1;
const HEADER: usize = 136;

#[derive(Debug, thiserror::Error)]
pub enum RootMotionBinaryError {
    #[error("root-motion catalog header")]
    Header,
    #[error("unsupported root-motion catalog version {0}")]
    Version(u32),
    #[error("truncated root-motion catalog")]
    Truncated,
    #[error("noncanonical root-motion catalog framing")]
    Framing,
    #[error("root-motion catalog binary capacity")]
    Capacity,
    #[error(transparent)]
    Value(#[from] RootMotionError),
}
type Result<T> = std::result::Result<T, RootMotionBinaryError>;

impl RootMotionCatalog {
    pub fn encoded_len(&self) -> Result<usize> {
        let mut n = HEADER;
        for clip in self.clips() {
            n = add(n, 1, 8)?;
            for track in [clip.translation(), clip.rotation()].into_iter().flatten() {
                n = add(n, 1, 8)?;
                n = add(n, track.times().len(), 20)?;
            }
        }
        Ok(n)
    }
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let length = self.encoded_len()?;
        let mut out = Vec::with_capacity(length);
        out.extend(MAGIC);
        out.extend(VERSION.to_le_bytes());
        out.extend(self.fingerprint());
        out.extend(self.root().to_le_bytes());
        out.extend([0; 2]);
        for n in self
            .rest_translation()
            .into_iter()
            .chain(self.rest_rotation())
        {
            out.extend(n.to_le_bytes());
        }
        let reference = self.model_reference();
        for n in reference.translation.into_iter().chain(reference.rotation) {
            out.extend(n.to_le_bytes());
        }
        out.extend((self.clips().len() as u32).to_le_bytes());
        for clip in self.clips() {
            out.extend(clip.duration().to_le_bytes());
            out.push(
                u8::from(clip.translation().is_some()) | (u8::from(clip.rotation().is_some()) << 1),
            );
            out.extend([0; 3]);
            for track in [clip.translation(), clip.rotation()].into_iter().flatten() {
                out.push(match track.interpolation() {
                    Interpolation::Linear => 0,
                    Interpolation::Step => 1,
                });
                out.extend([0; 3]);
                out.extend((track.times().len() as u32).to_le_bytes());
                for (&t, value) in track.times().iter().zip(track.values()) {
                    out.extend(t.to_le_bytes());
                    for n in value {
                        out.extend(n.to_le_bytes());
                    }
                }
            }
        }
        debug_assert_eq!(out.len(), length);
        Ok(out)
    }
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_ROOT_MOTION_CATALOG_BYTES {
            return Err(RootMotionBinaryError::Capacity);
        }
        if bytes.len() < HEADER || &bytes[..8] != MAGIC {
            return Err(RootMotionBinaryError::Header);
        }
        let mut r = Reader { bytes: &bytes[8..] };
        let version = r.u32()?;
        if version != VERSION {
            return Err(RootMotionBinaryError::Version(version));
        }
        let fingerprint = r.take()?;
        let root = u16::from_le_bytes(r.take()?);
        r.zero::<2>()?;
        let translation = [r.f32()?, r.f32()?, r.f32()?];
        let rotation = [r.f32()?, r.f32()?, r.f32()?, r.f32()?];
        let reference = RootMotionDelta {
            translation: [r.f64()?, r.f64()?, r.f64()?],
            rotation: [r.f64()?, r.f64()?, r.f64()?, r.f64()?],
        };
        let count = r.u32()? as usize;
        if count > MAX_ROOT_MOTION_CLIPS || count > r.bytes.len() / 8 {
            return Err(RootMotionBinaryError::Capacity);
        }
        let mut clips = Vec::with_capacity(count);
        let mut keys = 0usize;
        for _ in 0..count {
            let duration = r.f32()?;
            let flags = r.take::<1>()?[0];
            if flags & !3 != 0 {
                return Err(RootMotionBinaryError::Framing);
            }
            r.zero::<3>()?;
            let translation = if flags & 1 == 0 {
                None
            } else {
                Some(r.track(Kind::Vector, &mut keys)?)
            };
            let rotation = if flags & 2 == 0 {
                None
            } else {
                Some(r.track(Kind::Rotation, &mut keys)?)
            };
            clips.push(RootMotionClip::new(duration, translation, rotation)?);
        }
        if !r.bytes.is_empty() {
            return Err(RootMotionBinaryError::Framing);
        }
        Ok(Self::new(
            fingerprint,
            root,
            translation,
            rotation,
            reference,
            clips,
        )?)
    }
}

fn add(total: usize, count: usize, stride: usize) -> Result<usize> {
    count
        .checked_mul(stride)
        .and_then(|n| total.checked_add(n))
        .filter(|n| *n <= MAX_ROOT_MOTION_CATALOG_BYTES)
        .ok_or(RootMotionBinaryError::Capacity)
}
struct Reader<'a> {
    bytes: &'a [u8],
}
impl Reader<'_> {
    fn take<const N: usize>(&mut self) -> Result<[u8; N]> {
        let (head, tail) = self
            .bytes
            .split_at_checked(N)
            .ok_or(RootMotionBinaryError::Truncated)?;
        self.bytes = tail;
        Ok(head.try_into().expect("fixed width"))
    }
    fn zero<const N: usize>(&mut self) -> Result<()> {
        if self.take::<N>()? != [0; N] {
            return Err(RootMotionBinaryError::Framing);
        }
        Ok(())
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.take()?))
    }
    fn f32(&mut self) -> Result<f32> {
        Ok(f32::from_le_bytes(self.take()?))
    }
    fn f64(&mut self) -> Result<f64> {
        Ok(f64::from_le_bytes(self.take()?))
    }
    fn track(&mut self, kind: Kind, keys: &mut usize) -> Result<RootMotionTrack> {
        let interpolation = match self.take::<1>()?[0] {
            0 => Interpolation::Linear,
            1 => Interpolation::Step,
            _ => return Err(RootMotionBinaryError::Framing),
        };
        self.zero::<3>()?;
        let count = self.u32()? as usize;
        *keys = keys
            .checked_add(count)
            .filter(|n| *n <= MAX_ROOT_MOTION_KEYS)
            .ok_or(RootMotionBinaryError::Capacity)?;
        if count == 0 || count > self.bytes.len() / 20 {
            return Err(RootMotionBinaryError::Capacity);
        }
        let mut times = Vec::with_capacity(count);
        let mut values = Vec::with_capacity(count);
        for _ in 0..count {
            times.push(self.f32()?);
            values.push([self.f32()?, self.f32()?, self.f32()?, self.f32()?]);
        }
        Ok(RootMotionTrack::new(times, values, interpolation, kind)?)
    }
}

//! Borrowed committed skeletal playback and copied, transactional controls.
use crate::{EntityId, Error, Result};
use incant_wasm_abi as abi;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationPlayback {
    pub entity: EntityId,
    pub clip: Option<u16>,
    pub seconds: f64,
    pub speed: f64,
    pub playing: bool,
    pub looping: bool,
    pub completed: bool,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationControl {
    Select { clip: Option<u16>, seconds: f64 },
    Seek(f64),
    Playing(bool),
    Looping(bool),
    Speed(f64),
}
impl AnimationControl {
    pub(crate) fn encode(self) -> Result<[u8; abi::AnimationCommand::SIZE]> {
        let mut row = abi::AnimationCommand {
            kind: 0,
            clip: 0,
            seconds: 0.,
            value: 0.,
        };
        match self {
            Self::Select { clip, seconds } => {
                if clip.is_some_and(|c| c as usize >= abi::ANIMATION_MAX_CLIPS)
                    || !seconds.is_finite()
                    || seconds < 0.
                    || (clip.is_none() && seconds != 0.)
                {
                    return Err(Error::Malformed);
                }
                row.clip = clip.map_or(u32::MAX, u32::from);
                row.seconds = seconds;
            }
            Self::Seek(seconds) => {
                if !seconds.is_finite() || seconds < 0. {
                    return Err(Error::Malformed);
                }
                row.kind = 1;
                row.seconds = seconds;
            }
            Self::Playing(value) | Self::Looping(value) => {
                row.kind = if matches!(self, Self::Playing(_)) {
                    2
                } else {
                    3
                };
                row.value = f64::from(u8::from(value));
            }
            Self::Speed(speed) => {
                if !speed.is_finite() || speed.abs() > abi::ANIMATION_MAX_SPEED as f64 {
                    return Err(Error::Malformed);
                }
                row.kind = 4;
                row.value = speed;
            }
        }
        Ok(row.encode())
    }
}
#[derive(Clone, Copy)]
pub struct AnimationSnapshot<'a> {
    rows: &'a [u8],
}
impl<'a> AnimationSnapshot<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        let header = abi::AnimationHeader::decode(bytes).ok_or(Error::Malformed)?;
        if &header.magic != abi::ANIMATION_MAGIC
            || header.stride as usize != abi::AnimationRow::SIZE
            || header.count as usize > abi::ANIMATION_MAX_ROWS
        {
            return Err(Error::Malformed);
        }
        let length = (header.count as usize)
            .checked_mul(abi::AnimationRow::SIZE)
            .and_then(|n| n.checked_add(abi::AnimationHeader::SIZE))
            .ok_or(Error::Bounds)?;
        if bytes.len() != length {
            return Err(Error::Malformed);
        }
        let rows = &bytes[abi::AnimationHeader::SIZE..];
        for bytes in rows.as_chunks::<{ abi::AnimationRow::SIZE }>().0 {
            let row = abi::AnimationRow::decode(bytes).ok_or(Error::Malformed)?;
            if !row.is_finite()
                || row.seconds < 0.
                || row.speed.abs() > abi::ANIMATION_MAX_SPEED as f64
                || row.flags & !7 != 0
                || (row.flags & 4 != 0 && row.flags & 3 != 0)
                || if row.clip == u32::MAX {
                    row.seconds != 0. || row.flags & 4 != 0
                } else {
                    row.clip as usize >= abi::ANIMATION_MAX_CLIPS
                }
            {
                return Err(Error::Malformed);
            }
        }
        Ok(Self { rows })
    }
    pub fn len(&self) -> usize {
        self.rows.len() / abi::AnimationRow::SIZE
    }
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
    pub fn iter(&self) -> impl Iterator<Item = AnimationPlayback> + 'a {
        self.rows
            .as_chunks::<{ abi::AnimationRow::SIZE }>()
            .0
            .iter()
            .map(|bytes| {
                let row = abi::AnimationRow::decode(bytes).expect("validated animation row");
                AnimationPlayback {
                    entity: EntityId(row.id),
                    clip: if row.clip == u32::MAX {
                        None
                    } else {
                        Some(row.clip as u16)
                    },
                    seconds: row.seconds,
                    speed: row.speed,
                    playing: row.flags & 1 != 0,
                    looping: row.flags & 2 != 0,
                    completed: row.flags & 4 != 0,
                }
            })
    }
}

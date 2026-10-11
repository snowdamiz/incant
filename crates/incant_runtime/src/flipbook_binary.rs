use crate::binary::Reader;
use crate::{NativeFlipbook, NativeSpriteRegion, SceneError};
use std::sync::Arc;
pub(crate) fn encoded_size(f: &NativeFlipbook) -> usize {
    29 + f.frames.len() * 17
}
pub(crate) fn write(bytes: &mut Vec<u8>, id: crate::StableId, f: &NativeFlipbook) {
    bytes.extend(id.0);
    bytes.extend(f.rate.numerator.to_le_bytes());
    bytes.extend(f.rate.denominator.to_le_bytes());
    bytes.push(u8::from(f.looping) | u8::from(f.playing) << 1);
    bytes.extend((f.frames.len() as u32).to_le_bytes());
    for frame in f.frames.iter() {
        match frame {
            NativeSpriteRegion::Pixels(r) => {
                bytes.push(0);
                for v in r {
                    bytes.extend(v.to_le_bytes());
                }
            }
            NativeSpriteRegion::Atlas { frame } => {
                bytes.push(1);
                bytes.extend(frame.0);
            }
        }
    }
}
pub(crate) fn read(
    r: &mut Reader<'_>,
    budget: &mut usize,
) -> Result<Arc<NativeFlipbook>, SceneError> {
    let rate = crate::FrameRate {
        numerator: r.u32()?,
        denominator: r.u32()?,
    };
    let flags = r.byte()?;
    if flags > 3 {
        return Err(SceneError::Invalid("flipbook flags"));
    }
    let n = r.u32()? as usize;
    if n == 0
        || n > crate::MAX_FLIPBOOK_FRAMES
        || n > r.remaining_len() / 17
        || n > crate::MAX_SCENE_FLIPBOOK_FRAMES.saturating_sub(*budget)
    {
        return Err(SceneError::Invalid("flipbook frame count"));
    }
    *budget += n;
    let mut frames = Vec::with_capacity(n);
    for _ in 0..n {
        frames.push(match r.byte()? {
            0 => NativeSpriteRegion::Pixels([r.u32()?, r.u32()?, r.u32()?, r.u32()?]),
            1 => NativeSpriteRegion::Atlas {
                frame: crate::StableId(r.array()?),
            },
            _ => return Err(SceneError::Invalid("flipbook region tag")),
        });
    }
    Ok(Arc::new(NativeFlipbook {
        frames: frames.into(),
        rate,
        looping: flags & 1 != 0,
        playing: flags & 2 != 0,
    }))
}

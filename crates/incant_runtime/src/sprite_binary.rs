use crate::binary::Reader;
use crate::{
    NativeSpriteRegion, NativeSpriteRenderer, SceneError, SpriteMode, SpriteSampling, StableId,
};

pub(crate) fn encoded_size(s: &NativeSpriteRenderer) -> usize {
    106 + if matches!(s.mode, SpriteMode::NineSlice { .. }) {
        24
    } else {
        0
    }
}
pub(crate) fn write(bytes: &mut Vec<u8>, s: &NativeSpriteRenderer) {
    bytes.extend(s.texture.0);
    match s.region {
        NativeSpriteRegion::Pixels(region) => {
            bytes.push(0);
            for v in region {
                bytes.extend(v.to_le_bytes());
            }
        }
        NativeSpriteRegion::Atlas { frame } => {
            bytes.push(1);
            bytes.extend(frame.0);
        }
    }
    for v in s.size.into_iter().chain(s.pivot).chain(s.tint) {
        bytes.extend(v.to_le_bytes());
    }
    bytes.push(u8::from(s.flip_x) | u8::from(s.flip_y) << 1);
    bytes.extend(s.layer.to_le_bytes());
    bytes.extend(s.order.to_le_bytes());
    bytes.push(match s.sampling {
        SpriteSampling::Nearest => 0,
        SpriteSampling::Linear => 1,
    });
    match s.mode {
        SpriteMode::Simple => bytes.push(0),
        SpriteMode::NineSlice {
            border,
            pixels_per_unit,
        } => {
            bytes.push(1);
            for v in border {
                bytes.extend(v.to_le_bytes());
            }
            bytes.extend(pixels_per_unit.to_le_bytes());
        }
    }
}
pub(crate) fn read(r: &mut Reader<'_>) -> Result<NativeSpriteRenderer, SceneError> {
    let texture = StableId(r.array()?);
    let region = match r.byte()? {
        0 => NativeSpriteRegion::Pixels([r.u32()?, r.u32()?, r.u32()?, r.u32()?]),
        1 => NativeSpriteRegion::Atlas {
            frame: StableId(r.array()?),
        },
        _ => return Err(SceneError::Invalid("sprite region tag")),
    };
    let size = r.floats()?;
    let pivot = r.floats()?;
    let tint = r.floats()?;
    let flags = r.byte()?;
    if flags > 3 {
        return Err(SceneError::Invalid("sprite flip flags"));
    }
    let layer = i16::from_le_bytes(r.array()?);
    let order = i32::from_le_bytes(r.array()?);
    let sampling = match r.byte()? {
        0 => SpriteSampling::Nearest,
        1 => SpriteSampling::Linear,
        _ => return Err(SceneError::Invalid("sprite sampling tag")),
    };
    let mode = match r.byte()? {
        0 => SpriteMode::Simple,
        1 => SpriteMode::NineSlice {
            border: [r.u32()?, r.u32()?, r.u32()?, r.u32()?],
            pixels_per_unit: r.floats::<1>()?[0],
        },
        _ => return Err(SceneError::Invalid("sprite mode tag")),
    };
    Ok(NativeSpriteRenderer {
        texture,
        region,
        size,
        pivot,
        tint,
        flip_x: flags & 1 != 0,
        flip_y: flags & 2 != 0,
        layer,
        order,
        sampling,
        mode,
    })
}

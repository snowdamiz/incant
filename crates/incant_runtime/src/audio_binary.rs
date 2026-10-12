use crate::binary::Reader;
use crate::{CookedAudio, NativeAudioBus, NativeAudioSource, SceneError, StableId};
use incant_types::AudioSpatial;

pub(crate) fn encoded_size(node: &CookedAudio) -> usize {
    17 + node
        .bus
        .as_ref()
        .map_or(0, |b| 5 + usize::from(b.parent.is_some()) * 16)
        + node.source.as_ref().map_or(0, |s| {
            50 + usize::from(s.bus.is_some()) * 16 + usize::from(s.spatial.is_some()) * 8
        })
}
fn id(bytes: &mut Vec<u8>, value: Option<StableId>) {
    bytes.push(u8::from(value.is_some()));
    if let Some(value) = value {
        bytes.extend(value.0);
    }
}
pub(crate) fn write(bytes: &mut Vec<u8>, node: &CookedAudio, version: u32) {
    bytes.extend(node.entity.0);
    bytes.push(
        u8::from(node.bus.is_some())
            | (u8::from(node.source.is_some()) << 1)
            | (u8::from(node.listener) << 2),
    );
    if let Some(bus) = &node.bus {
        id(bytes, bus.parent);
        bytes.extend(bus.gain_db.to_le_bytes());
    }
    if let Some(source) = &node.source {
        if version >= 19 {
            bytes.extend(source.transport_epoch.to_le_bytes());
        }
        bytes.extend(source.clip.0);
        id(bytes, source.bus);
        bytes.extend(source.gain_db.to_le_bytes());
        bytes.extend(source.pan.to_le_bytes());
        bytes.extend(source.rate.to_le_bytes());
        bytes.extend(source.start_seconds.to_le_bytes());
        bytes.push(
            u8::from(source.playing)
                | (u8::from(source.looping) << 1)
                | (u8::from(source.streaming) << 2)
                | (u8::from(source.spatial.is_some()) << 3),
        );
        if let Some(spatial) = &source.spatial {
            bytes.extend(spatial.min_distance.to_le_bytes());
            bytes.extend(spatial.max_distance.to_le_bytes());
        }
    }
}
fn optional_id(reader: &mut Reader<'_>) -> Result<Option<StableId>, SceneError> {
    match reader.byte()? {
        0 => Ok(None),
        1 => Ok(Some(StableId(reader.array()?))),
        _ => Err(SceneError::Invalid("audio optional ID flag")),
    }
}
fn float(reader: &mut Reader<'_>) -> Result<f32, SceneError> {
    Ok(f32::from_le_bytes(reader.array()?))
}
pub(crate) fn read(reader: &mut Reader<'_>, version: u32) -> Result<CookedAudio, SceneError> {
    let entity = StableId(reader.array()?);
    let mask = reader.byte()?;
    if mask == 0 || mask > 7 {
        return Err(SceneError::Invalid("audio component mask"));
    }
    let bus = if mask & 1 != 0 {
        Some(NativeAudioBus {
            parent: optional_id(reader)?,
            gain_db: float(reader)?,
        })
    } else {
        None
    };
    let source = if mask & 2 != 0 {
        let transport_epoch = if version >= 19 {
            u64::from_le_bytes(reader.array()?)
        } else {
            0
        };
        let clip = StableId(reader.array()?);
        let bus = optional_id(reader)?;
        let gain_db = float(reader)?;
        let pan = float(reader)?;
        let [rate, start_seconds] = reader.floats()?;
        let flags = reader.byte()?;
        if flags > 15 {
            return Err(SceneError::Invalid("audio source flags"));
        }
        let spatial = if flags & 8 != 0 {
            Some(AudioSpatial {
                min_distance: float(reader)?,
                max_distance: float(reader)?,
            })
        } else {
            None
        };
        Some(NativeAudioSource {
            transport_epoch,
            clip,
            bus,
            gain_db,
            pan,
            rate,
            start_seconds,
            playing: flags & 1 != 0,
            looping: flags & 2 != 0,
            streaming: flags & 4 != 0,
            spatial,
        })
    } else {
        None
    };
    Ok(CookedAudio {
        entity,
        bus,
        source,
        listener: mask & 4 != 0,
    })
}

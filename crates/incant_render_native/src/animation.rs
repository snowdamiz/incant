//! Cold binding checks. Motion carries only the committed numeric sample.
use crate::{NativeRenderError, RuntimeAsset, StableId};
use incant_assets_runtime::RuntimeAssetData;
use incant_runtime::NativeAnimation;

pub(crate) fn validate(
    entity: StableId,
    source: &RuntimeAsset,
    definition: Option<&NativeAnimation>,
) -> Result<(), NativeRenderError> {
    let Some(definition) = definition else {
        return Ok(()); // Animated models without playback use their authored rest pose.
    };
    let invalid = |reason| NativeRenderError::Animation { entity, reason };
    let fingerprint = source.info().fingerprint.as_bytes();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    if fingerprint.len() != 64
        || definition
            .mesh_fingerprint
            .iter()
            .enumerate()
            .any(|(i, b)| {
                fingerprint[2 * i] != HEX[usize::from(b >> 4)]
                    || fingerprint[2 * i + 1] != HEX[usize::from(b & 15)]
            })
    {
        return Err(invalid("model fingerprint differs from native clip table"));
    }
    let RuntimeAssetData::Model(model) = source.data() else {
        return Err(invalid("expected a model"));
    };
    let data = model
        .animation
        .as_ref()
        .ok_or_else(|| invalid("model is static"))?;
    if data.clips.len() != definition.clip_durations.len()
        || data
            .clips
            .iter()
            .zip(definition.clip_durations.iter())
            .any(|(clip, expected)| {
                let actual = clip.channels.iter().fold(0.0f32, |duration, channel| {
                    duration.max(*channel.times.last().expect("validated nonempty keys"))
                });
                actual.to_bits() != expected.to_bits()
            })
    {
        return Err(invalid("clip duration table differs from cooked model"));
    }
    Ok(())
}

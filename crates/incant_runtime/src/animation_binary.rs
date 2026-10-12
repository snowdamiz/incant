use crate::{AnimationSample, NativeAnimation, SceneError, StableId, binary::Reader};
use incant_types::Animation;
use std::sync::Arc;

// Entity + fingerprint + count + clip + flags + time + speed, before durations.
pub(crate) const MIN_BYTES: usize = 71;
pub(crate) fn encoded_size(animation: &NativeAnimation) -> usize {
    MIN_BYTES + animation.clip_durations.len() * 4
}
pub(crate) fn write(bytes: &mut Vec<u8>, id: StableId, animation: &NativeAnimation) {
    bytes.extend(id.0);
    bytes.extend(animation.mesh_fingerprint);
    bytes.extend((animation.clip_durations.len() as u32).to_le_bytes());
    bytes.extend(animation.initial.clip.unwrap_or(u16::MAX).to_le_bytes());
    bytes.push(u8::from(animation.initial.playing) | u8::from(animation.initial.looping) << 1);
    bytes.extend(animation.initial.seconds.to_le_bytes());
    bytes.extend(animation.initial.speed.to_le_bytes());
    for d in animation.clip_durations.iter() {
        bytes.extend(d.to_le_bytes());
    }
}
pub(crate) fn read(
    r: &mut Reader<'_>,
    budget: &mut usize,
) -> Result<Arc<NativeAnimation>, SceneError> {
    let mesh_fingerprint = r.array()?;
    let n = r.u32()? as usize;
    let clip = u16::from_le_bytes(r.array()?);
    let flags = r.byte()?;
    let [seconds, speed] = r.floats()?;
    if flags > 3
        || n > crate::MAX_ANIMATION_CLIPS
        || n > r.remaining_len() / 4
        || n > crate::MAX_SCENE_ANIMATION_CLIPS.saturating_sub(*budget)
    {
        return Err(SceneError::Invalid("animation record"));
    }
    *budget += n;
    let mut durations = Vec::with_capacity(n);
    for _ in 0..n {
        durations.push(f32::from_le_bytes(r.array()?));
    }
    let result = NativeAnimation {
        graph: None,
        mesh_fingerprint,
        clip_durations: durations.into(),
        initial: Animation {
            clip: (clip != u16::MAX).then_some(clip),
            seconds,
            speed,
            playing: flags & 1 != 0,
            looping: flags & 2 != 0,
        },
    };
    result.validate()?;
    Ok(Arc::new(result))
}

pub(crate) fn sample(clip: u16, seconds: f64) -> AnimationSample {
    AnimationSample {
        clip: (clip != u16::MAX).then_some(clip),
        seconds,
    }
}

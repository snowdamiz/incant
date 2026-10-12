//! Forty-byte v10 extension; absent rows are entirely canonical zero bytes.
use super::SaveError;
use crate::animation_graph::fade::AnimationGraphFade;
pub(super) fn write(bytes: &mut Vec<u8>, fade: Option<AnimationGraphFade>) {
    if let Some(fade) = fade {
        bytes.extend([1, fade.state, u8::from(fade.playing), 0, 0, 0, 0, 0]);
        for n in [fade.seconds, fade.speed, fade.elapsed, fade.duration] {
            bytes.extend(n.to_le_bytes());
        }
    } else {
        bytes.resize(bytes.len() + 40, 0);
    }
}
pub(super) fn read(bytes: &[u8]) -> Result<Option<AnimationGraphFade>, SaveError> {
    if bytes.len() != 40 {
        return Err(SaveError::Invalid("graph fade size"));
    }
    if bytes[0] == 0 {
        if bytes.iter().any(|b| *b != 0) {
            return Err(SaveError::Invalid("graph absent fade"));
        }
        return Ok(None);
    }
    if bytes[0] != 1 || bytes[2] > 1 || bytes[3..8] != [0; 5] {
        return Err(SaveError::Invalid("graph fade flags"));
    }
    let number = |start: usize| f64::from_le_bytes(bytes[start..start + 8].try_into().unwrap());
    Ok(Some(AnimationGraphFade {
        state: bytes[1],
        playing: bytes[2] != 0,
        seconds: number(8),
        speed: number(16),
        elapsed: number(24),
        duration: number(32),
    }))
}

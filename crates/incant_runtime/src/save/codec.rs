//! Versioned framing with preallocation checks. All numeric encodings are
//! explicit little endian; no Rust layout, serde or executable deserialization.
use super::SaveError;
use crate::StableId;
use incant_2d::flipbook::FlipbookCheckpoint;
use sha2::{Digest, Sha256};
const MAGIC: &[u8; 8] = b"INCANTSV";
const HEADER: usize = 64;
const EXTENSION: usize = 40;
pub(super) const STATE_BYTES: usize = 28;
pub(super) const ANIMATION_BYTES: usize = 36;
pub const MAX_SAVE_BYTES: usize = HEADER
    + EXTENSION
    + 4
    + crate::binary::MAX_SCENE_BYTES
    + crate::MAX_SPRITE_ENTITIES * STATE_BYTES
    + crate::MAX_MESH_ENTITIES * ANIMATION_BYTES;

pub(super) fn encode(
    tick: u64,
    origin: [u8; 32],
    scene: &[u8],
    states: &[(StableId, FlipbookCheckpoint)],
    animations: &[(StableId, crate::AnimationState)],
) -> Result<Vec<u8>, SaveError> {
    if scene.len() > crate::binary::MAX_SCENE_BYTES
        || states.len() > crate::MAX_SPRITE_ENTITIES
        || animations.len() > crate::MAX_MESH_ENTITIES
    {
        return Err(SaveError::Invalid("save capacity"));
    }
    let version = if animations.is_empty() { 3u32 } else { 4 };
    let extension = EXTENSION + usize::from(version == 4) * 4;
    let length =
        extension + scene.len() + states.len() * STATE_BYTES + animations.len() * ANIMATION_BYTES;
    let mut bytes = Vec::with_capacity(HEADER + length);
    bytes.extend(MAGIC);
    bytes.extend(version.to_le_bytes());
    bytes.extend(0u32.to_le_bytes());
    bytes.extend(tick.to_le_bytes());
    bytes.extend((length as u64).to_le_bytes());
    bytes.resize(HEADER, 0);
    bytes.extend(origin);
    bytes.extend((scene.len() as u32).to_le_bytes());
    bytes.extend((states.len() as u32).to_le_bytes());
    if version == 4 {
        bytes.extend((animations.len() as u32).to_le_bytes());
    }
    bytes.extend(scene);
    for (id, value) in states {
        bytes.extend(id.0);
        bytes.extend(value.frame.to_le_bytes());
        bytes.push(u8::from(value.playing));
        bytes.push(u8::from(value.completed));
        bytes.extend(value.remainder.to_le_bytes());
    }
    for (id, state) in animations {
        bytes.extend(id.0);
        bytes.extend(state.sample.clip.unwrap_or(u16::MAX).to_le_bytes());
        bytes.push(
            u8::from(state.playing) | u8::from(state.looping) << 1 | u8::from(state.completed) << 2,
        );
        bytes.push(0);
        bytes.extend(state.sample.seconds.to_le_bytes());
        bytes.extend(state.speed.to_le_bytes());
    }
    let digest = checksum(&bytes);
    bytes[32..HEADER].copy_from_slice(&digest);
    Ok(bytes)
}
pub(super) struct Decoded<'a> {
    pub version: u32,
    pub tick: u64,
    pub origin: Option<[u8; 32]>,
    pub scene: &'a [u8],
    pub states: &'a [u8],
    pub animations: &'a [u8],
}
pub(super) fn is_bound(bytes: &[u8]) -> bool {
    matches!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 2..=4)
}
pub(super) fn decode(bytes: &[u8]) -> Result<Decoded<'_>, SaveError> {
    if !(HEADER..=MAX_SAVE_BYTES).contains(&bytes.len()) {
        return Err(SaveError::Invalid("file size"));
    }
    if &bytes[..8] != MAGIC {
        return Err(SaveError::Invalid("magic"));
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if !(1..=4).contains(&version) {
        return Err(SaveError::Version(version));
    }
    if bytes[12..16] != [0; 4] {
        return Err(SaveError::Invalid("unsupported flags"));
    }
    let tick = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
    if u64::from_le_bytes(bytes[24..32].try_into().unwrap()) != (bytes.len() - HEADER) as u64 {
        return Err(SaveError::Invalid("payload size"));
    }
    if checksum(bytes) != bytes[32..HEADER] {
        return Err(SaveError::Integrity);
    }
    if version == 1 {
        return Ok(Decoded {
            version,
            tick,
            origin: None,
            scene: &bytes[HEADER..],
            states: &[],
            animations: &[],
        });
    }
    let extension = EXTENSION + usize::from(version == 4) * 4;
    if bytes.len() < HEADER + extension {
        return Err(SaveError::Invalid("bound save header"));
    }
    let origin = bytes[64..96].try_into().unwrap();
    let scene_len = u32::from_le_bytes(bytes[96..100].try_into().unwrap()) as usize;
    let count = u32::from_le_bytes(bytes[100..104].try_into().unwrap()) as usize;
    let animation_count = if version == 4 {
        u32::from_le_bytes(bytes[104..108].try_into().unwrap()) as usize
    } else {
        0
    };
    if animation_count > crate::MAX_MESH_ENTITIES
        || (version == 4 && animation_count == 0)
        || scene_len > crate::binary::MAX_SCENE_BYTES
        || count > crate::MAX_SPRITE_ENTITIES
        || extension + scene_len + count * STATE_BYTES + animation_count * ANIMATION_BYTES
            != bytes.len() - HEADER
    {
        return Err(SaveError::Invalid("bound section lengths"));
    }
    let (scene, rest) = bytes[HEADER + extension..].split_at(scene_len);
    let (states, animations) = rest.split_at(count * STATE_BYTES);
    let mut previous = None;
    for row in states.as_chunks::<STATE_BYTES>().0 {
        let (id, _) = state(row)?;
        if previous.is_some_and(|p| p >= id) {
            return Err(SaveError::Invalid("flipbook canonical order"));
        }
        previous = Some(id);
    }
    let mut previous = None;
    for row in animations.as_chunks::<ANIMATION_BYTES>().0 {
        let (id, _) = animation(row)?;
        if previous.is_some_and(|p| p >= id) {
            return Err(SaveError::Invalid("animation canonical order"));
        }
        previous = Some(id);
    }
    Ok(Decoded {
        version,
        tick,
        origin: Some(origin),
        scene,
        states,
        animations,
    })
}
pub(super) fn state(row: &[u8; STATE_BYTES]) -> Result<(StableId, FlipbookCheckpoint), SaveError> {
    if row[18] > 1 || row[19] > 1 {
        return Err(SaveError::Invalid("flipbook boolean"));
    }
    Ok((
        StableId(row[..16].try_into().unwrap()),
        FlipbookCheckpoint {
            frame: u16::from_le_bytes(row[16..18].try_into().unwrap()),
            playing: row[18] != 0,
            completed: row[19] != 0,
            remainder: u64::from_le_bytes(row[20..28].try_into().unwrap()),
        },
    ))
}
pub(super) fn animation(
    row: &[u8; ANIMATION_BYTES],
) -> Result<(StableId, crate::AnimationState), SaveError> {
    if row[18] > 7 || row[19] != 0 {
        return Err(SaveError::Invalid("animation flags"));
    }
    Ok((
        StableId(row[..16].try_into().unwrap()),
        crate::AnimationState {
            sample: crate::animation_binary::sample(
                u16::from_le_bytes(row[16..18].try_into().unwrap()),
                f64::from_le_bytes(row[20..28].try_into().unwrap()),
            ),
            speed: f64::from_le_bytes(row[28..36].try_into().unwrap()),
            playing: row[18] & 1 != 0,
            looping: row[18] & 2 != 0,
            completed: row[18] & 4 != 0,
        },
    ))
}
fn checksum(bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(&bytes[..32]);
    hash.update(&bytes[HEADER..]);
    hash.finalize().into()
}

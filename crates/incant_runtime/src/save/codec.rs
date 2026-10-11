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
pub const MAX_SAVE_BYTES: usize =
    HEADER + EXTENSION + crate::binary::MAX_SCENE_BYTES + crate::MAX_SPRITE_ENTITIES * STATE_BYTES;

pub(super) fn encode(
    tick: u64,
    origin: [u8; 32],
    scene: &[u8],
    states: &[(StableId, FlipbookCheckpoint)],
) -> Result<Vec<u8>, SaveError> {
    if scene.len() > crate::binary::MAX_SCENE_BYTES || states.len() > crate::MAX_SPRITE_ENTITIES {
        return Err(SaveError::Invalid("save capacity"));
    }
    let length = EXTENSION + scene.len() + states.len() * STATE_BYTES;
    let mut bytes = Vec::with_capacity(HEADER + length);
    bytes.extend(MAGIC);
    bytes.extend(2u32.to_le_bytes());
    bytes.extend(0u32.to_le_bytes());
    bytes.extend(tick.to_le_bytes());
    bytes.extend((length as u64).to_le_bytes());
    bytes.resize(HEADER, 0);
    bytes.extend(origin);
    bytes.extend((scene.len() as u32).to_le_bytes());
    bytes.extend((states.len() as u32).to_le_bytes());
    bytes.extend(scene);
    for (id, value) in states {
        bytes.extend(id.0);
        bytes.extend(value.frame.to_le_bytes());
        bytes.push(u8::from(value.playing));
        bytes.push(u8::from(value.completed));
        bytes.extend(value.remainder.to_le_bytes());
    }
    let digest = checksum(&bytes);
    bytes[32..HEADER].copy_from_slice(&digest);
    Ok(bytes)
}
pub(super) struct Decoded<'a> {
    pub tick: u64,
    pub origin: Option<[u8; 32]>,
    pub scene: &'a [u8],
    pub states: &'a [u8],
}
pub(super) fn is_v2(bytes: &[u8]) -> bool {
    bytes[8..12] == 2u32.to_le_bytes()
}
pub(super) fn decode(bytes: &[u8]) -> Result<Decoded<'_>, SaveError> {
    if !(HEADER..=MAX_SAVE_BYTES).contains(&bytes.len()) {
        return Err(SaveError::Invalid("file size"));
    }
    if &bytes[..8] != MAGIC {
        return Err(SaveError::Invalid("magic"));
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if !(1..=2).contains(&version) {
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
            tick,
            origin: None,
            scene: &bytes[HEADER..],
            states: &[],
        });
    }
    if bytes.len() < HEADER + EXTENSION {
        return Err(SaveError::Invalid("v2 header"));
    }
    let origin = bytes[64..96].try_into().unwrap();
    let scene_len = u32::from_le_bytes(bytes[96..100].try_into().unwrap()) as usize;
    let count = u32::from_le_bytes(bytes[100..104].try_into().unwrap()) as usize;
    if scene_len > crate::binary::MAX_SCENE_BYTES
        || count > crate::MAX_SPRITE_ENTITIES
        || EXTENSION + scene_len + count * STATE_BYTES != bytes.len() - HEADER
    {
        return Err(SaveError::Invalid("v2 section lengths"));
    }
    let (scene, states) = bytes[104..].split_at(scene_len);
    let mut previous = None;
    for row in states.as_chunks::<STATE_BYTES>().0 {
        let (id, _) = state(row)?;
        if previous.is_some_and(|p| p >= id) {
            return Err(SaveError::Invalid("flipbook canonical order"));
        }
        previous = Some(id);
    }
    Ok(Decoded {
        tick,
        origin: Some(origin),
        scene,
        states,
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
fn checksum(bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(&bytes[..32]);
    hash.update(&bytes[HEADER..]);
    hash.finalize().into()
}

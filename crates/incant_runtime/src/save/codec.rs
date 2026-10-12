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
pub(super) const GRAPH_BYTES: usize = 168;
const TRANSITION_ANIMATION_BYTES: usize = 72;
pub(super) fn animation_stride(version: u32) -> usize {
    if version >= 5 {
        TRANSITION_ANIMATION_BYTES
    } else {
        ANIMATION_BYTES
    }
}
pub const MAX_SAVE_BYTES: usize = HEADER
    + EXTENSION
    + 8
    + crate::MAX_ANIMATION_GRAPH_INSTANCES * GRAPH_BYTES
    + crate::binary::MAX_SCENE_BYTES
    + crate::MAX_SPRITE_ENTITIES * STATE_BYTES
    + crate::MAX_MESH_ENTITIES * TRANSITION_ANIMATION_BYTES;

#[allow(clippy::too_many_arguments)] // Explicit versioned section inputs.
pub(super) fn encode(
    tick: u64,
    origin: [u8; 32],
    scene: &[u8],
    states: &[(StableId, FlipbookCheckpoint)],
    animations: &[(StableId, crate::AnimationState)],
    graphs: &[(StableId, crate::AnimationGraphState)],
    ui_used: bool,
    audio_used: bool,
) -> Result<Vec<u8>, SaveError> {
    if scene.len() > crate::binary::MAX_SCENE_BYTES
        || states.len() > crate::MAX_SPRITE_ENTITIES
        || animations.len() > crate::MAX_MESH_ENTITIES
        || graphs.len() > crate::MAX_ANIMATION_GRAPH_INSTANCES
    {
        return Err(SaveError::Invalid("save capacity"));
    }
    let version = if audio_used {
        8u32
    } else if ui_used {
        7u32
    } else if !graphs.is_empty() {
        6u32
    } else if animations.is_empty() {
        3u32
    } else if animations
        .iter()
        .any(|(_, state)| state.transition.is_some())
    {
        5
    } else {
        4
    };
    let extension = EXTENSION + usize::from(version >= 4) * 4 + usize::from(version >= 6) * 4;
    let length = extension
        + scene.len()
        + states.len() * STATE_BYTES
        + animations.len() * animation_stride(version)
        + graphs.len() * GRAPH_BYTES;
    let mut bytes = Vec::with_capacity(HEADER + length);
    bytes.extend(MAGIC);
    bytes.extend(version.to_le_bytes());
    bytes.extend((u32::from(ui_used) | (u32::from(audio_used) << 1)).to_le_bytes());
    bytes.extend(tick.to_le_bytes());
    bytes.extend((length as u64).to_le_bytes());
    bytes.resize(HEADER, 0);
    bytes.extend(origin);
    bytes.extend((scene.len() as u32).to_le_bytes());
    bytes.extend((states.len() as u32).to_le_bytes());
    if version >= 4 {
        bytes.extend((animations.len() as u32).to_le_bytes());
    }
    if version >= 6 {
        bytes.extend((graphs.len() as u32).to_le_bytes());
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
        if version >= 5 {
            if let Some(transition) = state.transition {
                bytes.extend(transition.source.clip.unwrap_or(u16::MAX).to_le_bytes());
                bytes.push(u8::from(transition.playing) | u8::from(transition.looping) << 1 | 4);
                bytes.push(0);
                bytes.extend(transition.source.seconds.to_le_bytes());
                bytes.extend(transition.speed.to_le_bytes());
                bytes.extend(transition.elapsed.to_le_bytes());
                bytes.extend(transition.duration.to_le_bytes());
            } else {
                bytes.resize(
                    bytes.len() + TRANSITION_ANIMATION_BYTES - ANIMATION_BYTES,
                    0,
                );
            }
        }
    }
    for (id, graph) in graphs {
        bytes.extend(id.0);
        bytes.push(graph.current);
        bytes.push(u8::from(graph.paused));
        bytes.extend([0; 6]);
        bytes.extend(graph.seconds.to_le_bytes());
        bytes.extend(graph.elapsed.to_le_bytes());
        for value in graph.parameters {
            bytes.extend(value.to_le_bytes());
        }
    }
    let digest = checksum(&bytes);
    bytes[32..HEADER].copy_from_slice(&digest);
    Ok(bytes)
}
pub(super) struct Decoded<'a> {
    pub version: u32,
    pub tick: u64,
    pub ui_used: bool,
    pub audio_used: bool,
    pub origin: Option<[u8; 32]>,
    pub scene: &'a [u8],
    pub states: &'a [u8],
    pub animations: &'a [u8],
    pub graphs: &'a [u8],
}
pub(super) fn is_bound(bytes: &[u8]) -> bool {
    matches!(u32::from_le_bytes(bytes[8..12].try_into().unwrap()), 2..=8)
}
pub(super) fn decode(bytes: &[u8]) -> Result<Decoded<'_>, SaveError> {
    if !(HEADER..=MAX_SAVE_BYTES).contains(&bytes.len()) {
        return Err(SaveError::Invalid("file size"));
    }
    if &bytes[..8] != MAGIC {
        return Err(SaveError::Invalid("magic"));
    }
    let version = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
    if !(1..=8).contains(&version) {
        return Err(SaveError::Version(version));
    }
    let flags = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
    if (version < 8 && flags != u32::from(version == 7))
        || (version == 8 && !matches!(flags, 2 | 3))
    {
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
            ui_used: false,
            audio_used: false,
            origin: None,
            scene: &bytes[HEADER..],
            states: &[],
            animations: &[],
            graphs: &[],
        });
    }
    let extension = EXTENSION + usize::from(version >= 4) * 4 + usize::from(version >= 6) * 4;
    if bytes.len() < HEADER + extension {
        return Err(SaveError::Invalid("bound save header"));
    }
    let origin = bytes[64..96].try_into().unwrap();
    let scene_len = u32::from_le_bytes(bytes[96..100].try_into().unwrap()) as usize;
    let count = u32::from_le_bytes(bytes[100..104].try_into().unwrap()) as usize;
    let animation_count = if version >= 4 {
        u32::from_le_bytes(bytes[104..108].try_into().unwrap()) as usize
    } else {
        0
    };
    let graph_count = if version >= 6 {
        u32::from_le_bytes(bytes[108..112].try_into().unwrap()) as usize
    } else {
        0
    };
    if graph_count > crate::MAX_ANIMATION_GRAPH_INSTANCES
        || (version == 6 && graph_count == 0)
        || animation_count > crate::MAX_MESH_ENTITIES
        || ((4..=6).contains(&version) && animation_count == 0)
        || scene_len > crate::binary::MAX_SCENE_BYTES
        || count > crate::MAX_SPRITE_ENTITIES
        || extension
            + scene_len
            + count * STATE_BYTES
            + animation_count * animation_stride(version)
            + graph_count * GRAPH_BYTES
            != bytes.len() - HEADER
    {
        return Err(SaveError::Invalid("bound section lengths"));
    }
    let (scene, rest) = bytes[HEADER + extension..].split_at(scene_len);
    let (states, rest) = rest.split_at(count * STATE_BYTES);
    let (animations, graphs) = rest.split_at(animation_count * animation_stride(version));
    let mut previous = None;
    for row in states.as_chunks::<STATE_BYTES>().0 {
        let (id, _) = state(row)?;
        if previous.is_some_and(|p| p >= id) {
            return Err(SaveError::Invalid("flipbook canonical order"));
        }
        previous = Some(id);
    }
    let mut previous = None;
    let mut has_transition = false;
    for row in animations.chunks_exact(animation_stride(version)) {
        let (id, state) = animation(row)?;
        has_transition |= state.transition.is_some();
        if previous.is_some_and(|p| p >= id) {
            return Err(SaveError::Invalid("animation canonical order"));
        }
        previous = Some(id);
    }
    if version == 5 && !has_transition {
        return Err(SaveError::Invalid("animation transition membership"));
    }
    let mut previous = None;
    for row in graphs.as_chunks::<GRAPH_BYTES>().0 {
        let (id, _) = graph(row)?;
        if previous.is_some_and(|p| p >= id) {
            return Err(SaveError::Invalid("animation graph canonical order"));
        }
        previous = Some(id);
    }
    Ok(Decoded {
        version,
        tick,
        ui_used: flags & 1 != 0,
        audio_used: flags & 2 != 0,
        origin: Some(origin),
        scene,
        states,
        animations,
        graphs,
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
pub(super) fn animation(row: &[u8]) -> Result<(StableId, crate::AnimationState), SaveError> {
    if !matches!(row.len(), ANIMATION_BYTES | TRANSITION_ANIMATION_BYTES) {
        return Err(SaveError::Invalid("animation row size"));
    }
    if row[18] > 7 || row[19] != 0 {
        return Err(SaveError::Invalid("animation flags"));
    }
    let transition = if row.len() == TRANSITION_ANIMATION_BYTES && row[38] & 4 != 0 {
        if row[38] > 7 || row[39] != 0 {
            return Err(SaveError::Invalid("animation transition flags"));
        }
        Some(crate::AnimationTransition {
            source: crate::animation_binary::sample(
                u16::from_le_bytes(row[36..38].try_into().unwrap()),
                f64::from_le_bytes(row[40..48].try_into().unwrap()),
            ),
            playing: row[38] & 1 != 0,
            looping: row[38] & 2 != 0,
            speed: f64::from_le_bytes(row[48..56].try_into().unwrap()),
            elapsed: f64::from_le_bytes(row[56..64].try_into().unwrap()),
            duration: f64::from_le_bytes(row[64..72].try_into().unwrap()),
        })
    } else {
        if row[ANIMATION_BYTES..].iter().any(|byte| *byte != 0) {
            return Err(SaveError::Invalid("animation absent transition"));
        }
        None
    };
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
            transition,
        },
    ))
}
pub(super) fn graph(
    row: &[u8; GRAPH_BYTES],
) -> Result<(StableId, crate::AnimationGraphState), SaveError> {
    if row[17] > 1 || row[18..24] != [0; 6] {
        return Err(SaveError::Invalid("animation graph reserved flags"));
    }
    let mut parameters = [0.0; incant_types::MAX_ANIMATION_GRAPH_PARAMETERS];
    for (value, bytes) in parameters.iter_mut().zip(row[40..].as_chunks::<8>().0) {
        *value = f64::from_le_bytes(*bytes);
    }
    Ok((
        StableId(row[..16].try_into().unwrap()),
        crate::AnimationGraphState {
            current: row[16],
            paused: row[17] != 0,
            seconds: f64::from_le_bytes(row[24..32].try_into().unwrap()),
            elapsed: f64::from_le_bytes(row[32..40].try_into().unwrap()),
            parameters,
        },
    ))
}
fn checksum(bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(&bytes[..32]);
    hash.update(&bytes[HEADER..]);
    hash.finalize().into()
}

//! Wasm state occupies new v7/v8 records; bytecode v1-v6 remains byte-identical.
use super::*;
use incant_wasm_runtime::persistent::PersistentSnapshot;

fn format(persisted: bool) -> Result<Format> {
    if !persisted {
        return Ok(Format::Ordinary);
    }
    #[cfg(feature = "native-checkpoints")]
    {
        Ok(Format::Persisted)
    }
    #[cfg(not(feature = "native-checkpoints"))]
    {
        Err(Error::Unsupported("persisted native checkpoint feature"))
    }
}
pub(in crate::checkpoint) fn wasm_sections(
    bytes: &[u8],
    program: [u8; 32],
    persisted: bool,
) -> Result<Sections<'_>> {
    let format = format(persisted)?;
    if !(HEADER..=format.limit()).contains(&bytes.len()) || &bytes[..8] != MAGIC {
        return Err(Error::Invalid("framing"));
    }
    if bytes[8..12] != (if persisted { 8u32 } else { 7u32 }).to_le_bytes() {
        return Err(Error::Invalid("Wasm checkpoint version"));
    }
    if bytes[12..16] != 1u32.to_le_bytes() {
        return Err(Error::Invalid("ordinary-load input policy"));
    }
    if bytes[16..48] != program {
        return Err(Error::ProgramMismatch);
    }
    if bytes[96..128] != digest(bytes, HEADER) {
        return Err(Error::Integrity);
    }
    let mut lens = [0usize; 4];
    let mut total = HEADER;
    for (i, len) in lens.iter_mut().enumerate() {
        *len = u32::from_le_bytes(bytes[80 + 4 * i..84 + 4 * i].try_into().unwrap()) as usize;
        total = total
            .checked_add(*len)
            .ok_or(Error::Invalid("length overflow"))?;
    }
    if total != bytes.len()
        || lens[0] > format.world_limit()
        || lens[1] > PersistentSnapshot::MAX_ENCODED_BYTES
        || lens[1] < 73
        || lens[2] > MAX_UI_BYTES
        || lens[3] > MAX_AUDIO_BYTES
    {
        return Err(Error::Invalid("Wasm section lengths"));
    }
    let (world, rest) = bytes[HEADER..].split_at(lens[0]);
    let (state, rest) = rest.split_at(lens[1]);
    let (ui, audio) = rest.split_at(lens[2]);
    Ok(Sections {
        origin: bytes[48..80].try_into().unwrap(),
        world,
        wasm: Some(state),
        active: &[],
        store: &[],
        schedule: &[],
        coroutines: &[],
        ui: (!ui.is_empty()).then_some(ui),
        audio: (!audio.is_empty()).then_some(audio),
    })
}
#[allow(clippy::too_many_arguments)] // Independent bounded checkpoint sections.
pub(in crate::checkpoint) fn wasm_encode(
    program: [u8; 32],
    origin: [u8; 32],
    world: &[u8],
    state: &[u8],
    ui: Option<&[u8]>,
    audio: Option<&[u8]>,
    persisted: bool,
) -> Result<Vec<u8>> {
    let format = format(persisted)?;
    if world.len() > format.world_limit()
        || !(73..=PersistentSnapshot::MAX_ENCODED_BYTES).contains(&state.len())
        || ui.is_some_and(|b| b.is_empty() || b.len() > MAX_UI_BYTES)
        || audio.is_some_and(|b| b.is_empty() || b.len() > MAX_AUDIO_BYTES)
    {
        return Err(Error::Invalid("Wasm section capacity"));
    }
    let parts = [
        world,
        state,
        ui.unwrap_or_default(),
        audio.unwrap_or_default(),
    ];
    let total = parts
        .iter()
        .try_fold(HEADER, |n, part| n.checked_add(part.len()))
        .filter(|n| *n <= format.limit())
        .ok_or(Error::Invalid("checkpoint capacity"))?;
    let mut output = Vec::with_capacity(total);
    output.extend(MAGIC);
    output.extend((if persisted { 8u32 } else { 7u32 }).to_le_bytes());
    output.extend(1u32.to_le_bytes());
    output.extend(program);
    output.extend(origin);
    for part in parts {
        output.extend((part.len() as u32).to_le_bytes());
    }
    output.extend([0u8; 32]);
    for part in parts {
        output.extend(part);
    }
    let digest = digest(&output, HEADER);
    output[96..128].copy_from_slice(&digest);
    Ok(output)
}

//! Data-only cooperative routines: immutable declarations and copied byte frames.
use crate::{
    COROUTINE_CATALOG_MAGIC, COROUTINE_FRAME_BYTES, COROUTINE_MAGIC, COROUTINE_MAX_ROUTINES,
    CoroutineHeader, timers,
};
pub const SECTION: &str = "incant_coroutines_v1";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Malformed;
type Result<T> = core::result::Result<T, Malformed>;

/// Each frame has a fixed explicit layout, identified independently of code.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Routine {
    name: [u8; 64],
    len: u32,
    bytes: u32,
    schema: [u8; 32],
}
impl Routine {
    pub const SIZE: usize = 104;
    pub const fn new(name: &str, bytes: u32, schema: [u8; 32]) -> Option<Self> {
        if name.is_empty()
            || name.len() > 64
            || bytes == 0
            || bytes as usize > COROUTINE_FRAME_BYTES
        {
            return None;
        }
        let mut encoded = [0; 64];
        let raw = name.as_bytes();
        let mut i = 0;
        while i < raw.len() {
            let b = raw[i];
            if !((b >= b'a' && b <= b'z')
                || (b >= b'A' && b <= b'Z')
                || (b >= b'0' && b <= b'9')
                || b == b'_'
                || b == b'.'
                || b == b':'
                || b == b'-')
            {
                return None;
            }
            encoded[i] = b;
            i += 1;
        }
        i = 0;
        let mut nonzero = false;
        while i < 32 {
            nonzero |= schema[i] != 0;
            i += 1;
        }
        if !nonzero {
            return None;
        }
        Some(Self {
            name: encoded,
            len: name.len() as u32,
            bytes,
            schema,
        })
    }
    pub fn name(&self) -> &str {
        core::str::from_utf8(&self.name[..self.len as usize]).unwrap()
    }
    pub const fn bytes(&self) -> usize {
        self.bytes as usize
    }
    pub const fn schema(&self) -> [u8; 32] {
        self.schema
    }
    pub const fn encode(&self) -> [u8; Self::SIZE] {
        let mut out = [0; Self::SIZE];
        let mut i = 0;
        while i < 64 {
            out[i] = self.name[i];
            i += 1;
        }
        i = 0;
        while i < 4 {
            out[64 + i] = self.len.to_le_bytes()[i];
            out[68 + i] = self.bytes.to_le_bytes()[i];
            i += 1;
        }
        i = 0;
        while i < 32 {
            out[72 + i] = self.schema[i];
            i += 1;
        }
        out
    }
    pub fn decode(raw: &[u8]) -> Result<Self> {
        if raw.len() != Self::SIZE {
            return Err(Malformed);
        }
        let name = raw[..64].try_into().map_err(|_| Malformed)?;
        let len = u32::from_le_bytes(raw[64..68].try_into().unwrap());
        let name = timers::id(name, len).map_err(|_| Malformed)?;
        Self::new(
            name,
            u32::from_le_bytes(raw[68..72].try_into().unwrap()),
            raw[72..].try_into().unwrap(),
        )
        .ok_or(Malformed)
    }
}

pub const fn encode_catalog<const N: usize>(routines: &[Routine]) -> [u8; N] {
    assert!(!routines.is_empty() && routines.len() <= COROUTINE_MAX_ROUTINES);
    assert!(N == 16 + routines.len() * Routine::SIZE);
    let mut out = [0; N];
    let mut i = 0;
    while i < 8 {
        out[i] = COROUTINE_CATALOG_MAGIC[i];
        i += 1;
    }
    out[8] = 1;
    let count = (routines.len() as u32).to_le_bytes();
    i = 0;
    while i < 4 {
        out[12 + i] = count[i];
        i += 1;
    }
    i = 0;
    while i < routines.len() {
        let record = routines[i].encode();
        let mut j = 0;
        while j < record.len() {
            out[16 + i * Routine::SIZE + j] = record[j];
            j += 1;
        }
        i += 1;
    }
    out
}

#[derive(Clone, Copy)]
pub struct Catalog<'a>(&'a [u8]);
impl<'a> Catalog<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() < 16
            || bytes[..8] != *COROUTINE_CATALOG_MAGIC
            || bytes[8..12] != 1u32.to_le_bytes()
        {
            return Err(Malformed);
        }
        let count = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
        if !(1..=COROUTINE_MAX_ROUTINES).contains(&count)
            || bytes.len() != 16 + count * Routine::SIZE
        {
            return Err(Malformed);
        }
        let catalog = Self(&bytes[16..]);
        for (i, raw) in catalog
            .0
            .as_chunks::<{ Routine::SIZE }>()
            .0
            .iter()
            .enumerate()
        {
            let routine = Routine::decode(raw)?;
            for prior in catalog.routines().take(i) {
                if prior.name() == routine.name() {
                    return Err(Malformed);
                }
            }
        }
        Ok(catalog)
    }
    pub fn routines(self) -> impl ExactSizeIterator<Item = Routine> + 'a {
        self.0
            .as_chunks::<{ Routine::SIZE }>()
            .0
            .iter()
            .map(|raw| Routine::decode(raw).unwrap())
    }
}

pub fn catalog_in_module(bytes: &[u8]) -> core::result::Result<Option<Catalog<'_>>, &'static str> {
    crate::persistent::custom_section(bytes, SECTION)?
        .map(|data| Catalog::parse(data).map_err(|_| "invalid coroutine catalog"))
        .transpose()
}

/// Validate exact framing and reserved fields before any state allocation.
pub fn frame(bytes: &[u8]) -> Result<(CoroutineHeader, &[u8])> {
    let h = CoroutineHeader::decode(bytes).ok_or(Malformed)?;
    if &h.magic != COROUTINE_MAGIC
        || h.flags > 1
        || (h.flags == 0 && h.entity != [0; 16])
        || h.state_len as usize > COROUTINE_FRAME_BYTES
        || bytes.len() != CoroutineHeader::SIZE + h.state_len as usize
    {
        return Err(Malformed);
    }
    timers::id(&h.id, h.id_len).map_err(|_| Malformed)?;
    match h.kind {
        1 if h.delay == 0
            && h.routine_len == 0
            && h.routine == [0; 64]
            && h.flags == 0
            && h.state_len == 0 => {}
        0 | 2 | 3 | 4 => {
            timers::id(&h.routine, h.routine_len).map_err(|_| Malformed)?;
            if (h.kind == 3 && h.delay == 0)
                || (h.kind != 3 && h.delay != 0)
                || (h.kind == 2 && h.state_len != 0)
                || (h.kind != 2 && h.state_len == 0)
            {
                return Err(Malformed);
            }
        }
        _ => return Err(Malformed),
    }
    Ok((h, &bytes[CoroutineHeader::SIZE..]))
}

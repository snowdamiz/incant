//! Fixed opt-in declaration. No native pointer or executable metadata is admitted.
pub const SECTION: &str = "incant_state_v1";
pub const MAX_BYTES: usize = 65_536;
pub const MAX_CALLS: usize = 256;
pub const MAX_TRANSFERS: usize = 1_048_576;
const MAGIC: &[u8; 8] = b"INCWST01";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Declaration {
    bytes: u32,
    schema: [u8; 32],
}
impl Declaration {
    pub const SIZE: usize = 56;
    pub const fn new(bytes: u32, schema: [u8; 32]) -> Option<Self> {
        if bytes == 0 || bytes as usize > MAX_BYTES {
            return None;
        }
        let mut index = 0;
        let mut nonzero = false;
        while index < schema.len() {
            nonzero |= schema[index] != 0;
            index += 1;
        }
        if nonzero {
            Some(Self { bytes, schema })
        } else {
            None
        }
    }
    pub const fn len(self) -> usize {
        self.bytes as usize
    }
    pub const fn is_empty(self) -> bool {
        false
    }
    pub const fn schema(self) -> [u8; 32] {
        self.schema
    }
    pub const fn encode(self) -> [u8; Self::SIZE] {
        let mut output = [0; Self::SIZE];
        let mut index = 0;
        while index < 8 {
            output[index] = MAGIC[index];
            index += 1;
        }
        output[8] = 1;
        let count = self.bytes.to_le_bytes();
        index = 0;
        while index < 4 {
            output[12 + index] = count[index];
            index += 1;
        }
        index = 0;
        while index < 32 {
            output[16 + index] = self.schema[index];
            index += 1;
        }
        output
    }
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        if bytes.len() != Self::SIZE
            || &bytes[..8] != MAGIC
            || bytes[8..12] != 1u32.to_le_bytes()
            || bytes[48..] != [0; 8]
        {
            return None;
        }
        Self::new(
            u32::from_le_bytes(bytes[12..16].try_into().ok()?),
            bytes[16..48].try_into().ok()?,
        )
    }
}

/// Read only the bounded opt-in custom section. Full Wasm validation remains the
/// caller's responsibility. No allocation or guest execution is performed.
pub fn declaration_in_module(bytes: &[u8]) -> Result<Option<Declaration>, &'static str> {
    custom_section(bytes, SECTION)?
        .map(|data| Declaration::decode(data).ok_or("invalid persistent declaration"))
        .transpose()
}

pub(crate) fn custom_section<'a>(
    bytes: &'a [u8],
    wanted: &str,
) -> Result<Option<&'a [u8]>, &'static str> {
    if bytes.len() > 1024 * 1024 || bytes.get(..8) != Some(b"\0asm\x01\0\0\0") {
        return Err("persistent declaration requires core Wasm <= 1 MiB");
    }
    fn unsigned(bytes: &mut &[u8]) -> Result<usize, &'static str> {
        let mut value = 0u32;
        for i in 0..5 {
            let (&byte, tail) = bytes.split_first().ok_or("truncated Wasm length")?;
            *bytes = tail;
            if i == 4 && byte > 15 {
                return Err("Wasm length overflow");
            }
            value |= u32::from(byte & 127) << (7 * i);
            if byte & 128 == 0 {
                return Ok(value as usize);
            }
        }
        Err("Wasm length overflow")
    }
    let mut remaining = &bytes[8..];
    let mut declaration = None;
    while let Some((&id, tail)) = remaining.split_first() {
        remaining = tail;
        let len = unsigned(&mut remaining)?;
        if len > remaining.len() {
            return Err("truncated Wasm section");
        }
        let (mut section, tail) = remaining.split_at(len);
        remaining = tail;
        if id == 0 {
            let len = unsigned(&mut section)?;
            if len > section.len() {
                return Err("truncated custom section name");
            }
            let (name, data) = section.split_at(len);
            if name == wanted.as_bytes() {
                if declaration.is_some() {
                    return Err("duplicate persistent declaration");
                }
                declaration = Some(data);
            }
        }
    }
    Ok(declaration)
}

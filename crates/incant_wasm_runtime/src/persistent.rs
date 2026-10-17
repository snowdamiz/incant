//! Host-owned committed bytes, copied only by explicit guest calls or checkpoints.
use crate::BoundaryError as Error;
pub use incant_wasm_abi::persistent::{Declaration, declaration_in_module};
use incant_wasm_abi::persistent::{MAX_CALLS, MAX_TRANSFERS};

/// Identity supplied by validated development input or trusted generated linkage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersistentProgram {
    pub declaration: Declaration,
    pub fingerprint: [u8; 32],
}

/// Data only. No active epoch, native handle or executable pointer is captured.
/// The watermark prevents reuse of old epochs stored in the explicit guest data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PersistentSnapshot {
    declaration: Declaration,
    bytes: Box<[u8]>,
    tick: u64,
    watermark: i64,
}
impl PersistentSnapshot {
    pub const MAX_ENCODED_BYTES: usize =
        Declaration::SIZE + 16 + incant_wasm_abi::persistent::MAX_BYTES;
    pub fn declaration(&self) -> Declaration {
        self.declaration
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }
    pub fn watermark(&self) -> i64 {
        self.watermark
    }
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Declaration::SIZE + 16 + self.bytes.len());
        bytes.extend_from_slice(&self.declaration.encode());
        bytes.extend_from_slice(&self.tick.to_le_bytes());
        bytes.extend_from_slice(&self.watermark.to_le_bytes());
        bytes.extend_from_slice(&self.bytes);
        bytes
    }
    pub fn decode(bytes: &[u8], declaration: Declaration) -> Result<Self, Error> {
        const HEADER: usize = Declaration::SIZE + 16;
        if bytes.len() != HEADER + declaration.len()
            || Declaration::decode(&bytes[..Declaration::SIZE]) != Some(declaration)
        {
            return Err(Error::Abi("persistent state framing/declaration"));
        }
        let tick = u64::from_le_bytes(
            bytes[Declaration::SIZE..Declaration::SIZE + 8]
                .try_into()
                .unwrap(),
        );
        let watermark =
            i64::from_le_bytes(bytes[Declaration::SIZE + 8..HEADER].try_into().unwrap());
        if tick > incant_runtime::logging::MAX_LOG_TICK
            || !(0..i64::MAX).contains(&watermark)
            || (watermark as u64) < tick
        {
            return Err(Error::Abi("persistent state clock/watermark"));
        }
        Ok(Self {
            declaration,
            bytes: bytes[HEADER..].into(),
            tick,
            watermark,
        })
    }
}

pub(crate) struct PersistentState {
    declaration: Declaration,
    committed: Box<[u8]>,
    staging: Box<[u8]>,
    dirty: bool,
    calls: usize,
    transferred: usize,
}
impl PersistentState {
    pub(crate) fn new(declaration: Declaration) -> Self {
        Self {
            declaration,
            committed: vec![0; declaration.len()].into_boxed_slice(),
            staging: vec![0; declaration.len()].into_boxed_slice(),
            dirty: false,
            calls: 0,
            transferred: 0,
        }
    }
    pub(crate) fn declaration(&self) -> Declaration {
        self.declaration
    }
    pub(crate) fn begin(&mut self) {
        self.abort();
        self.calls = 0;
        self.transferred = 0;
    }
    pub(crate) fn abort(&mut self) {
        self.dirty = false;
    }
    pub(crate) fn commit(&mut self) {
        if self.dirty {
            std::mem::swap(&mut self.committed, &mut self.staging);
        }
        self.dirty = false;
    }
    fn range(&mut self, offset: i32, len: usize) -> Result<std::ops::Range<usize>, Error> {
        self.calls = self
            .calls
            .checked_add(1)
            .filter(|n| *n <= MAX_CALLS)
            .ok_or(Error::Limit("persistent state calls"))?;
        self.transferred = self
            .transferred
            .checked_add(len)
            .filter(|n| *n <= MAX_TRANSFERS)
            .ok_or(Error::Limit("persistent state transfers"))?;
        let start = usize::try_from(offset).map_err(|_| Error::Abi("persistent state offset"))?;
        let end = start
            .checked_add(len)
            .filter(|end| *end <= self.declaration.len())
            .ok_or(Error::Abi("persistent state range"))?;
        Ok(start..end)
    }
    pub(crate) fn read(&mut self, offset: i32, output: &mut [u8]) -> Result<(), Error> {
        let range = self.range(offset, output.len())?;
        let source = if self.dirty {
            &self.staging
        } else {
            &self.committed
        };
        output.copy_from_slice(&source[range]);
        Ok(())
    }
    pub(crate) fn write(&mut self, offset: i32, input: &[u8]) -> Result<(), Error> {
        let range = self.range(offset, input.len())?;
        if !self.dirty {
            self.staging.copy_from_slice(&self.committed);
            self.dirty = true;
        }
        self.staging[range].copy_from_slice(input);
        Ok(())
    }
    pub(crate) fn snapshot(&self, tick: u64, watermark: i64) -> Result<PersistentSnapshot, Error> {
        if self.dirty || watermark == i64::MAX {
            return Err(Error::Abi("persistent state not quiescent"));
        }
        Ok(PersistentSnapshot {
            declaration: self.declaration,
            bytes: self.committed.clone(),
            tick,
            watermark,
        })
    }
    pub(crate) fn restore(&mut self, snapshot: &PersistentSnapshot) -> Result<(), Error> {
        if self.declaration != snapshot.declaration {
            return Err(Error::Abi("persistent state declaration mismatch"));
        }
        self.committed.copy_from_slice(&snapshot.bytes);
        self.begin();
        Ok(())
    }
}

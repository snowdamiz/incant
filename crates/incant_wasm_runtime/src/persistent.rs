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
    timers: Box<[u8]>,
    coroutines: Option<Box<[u8]>>,
}
impl PersistentSnapshot {
    pub const MAX_ENCODED_BYTES: usize = 8
        + Declaration::SIZE
        + 16
        + incant_wasm_abi::persistent::MAX_BYTES
        + incant_wasm_abi::TimerHeader::SIZE
        + incant_wasm_abi::TIMER_MAX_TIMERS * incant_wasm_abi::TimerStateRow::SIZE
        + 4
        + crate::coroutines::checkpoint::MAX_BYTES;
    pub(crate) fn timer_state(&self) -> Result<crate::timers::State, Error> {
        crate::timers::State::restore(&self.timers, self.tick)
    }
    pub(crate) fn coroutine_bytes(&self) -> Option<&[u8]> {
        self.coroutines.as_deref()
    }
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
        let mut bytes =
            Vec::with_capacity(8 + Declaration::SIZE + 16 + self.bytes.len() + self.timers.len());
        bytes.extend_from_slice(if self.coroutines.is_some() {
            b"INCWPS03"
        } else {
            b"INCWPS02"
        });
        bytes.extend_from_slice(&self.declaration.encode());
        bytes.extend_from_slice(&self.tick.to_le_bytes());
        bytes.extend_from_slice(&self.watermark.to_le_bytes());
        bytes.extend_from_slice(&self.bytes);
        if self.coroutines.is_some() {
            bytes.extend_from_slice(&(self.timers.len() as u32).to_le_bytes());
        }
        bytes.extend_from_slice(&self.timers);
        if let Some(tasks) = &self.coroutines {
            bytes.extend_from_slice(tasks);
        }
        bytes
    }
    pub fn decode(bytes: &[u8], declaration: Declaration) -> Result<Self, Error> {
        const HEADER: usize = Declaration::SIZE + 16;
        let (bytes, version) = if let Some(rest) = bytes.strip_prefix(b"INCWPS03") {
            (rest, 3)
        } else if let Some(rest) = bytes.strip_prefix(b"INCWPS02") {
            (rest, 2)
        } else {
            (bytes, 1)
        };
        let payload_end = HEADER + declaration.len();
        if (if version >= 2 {
            bytes.len() < payload_end + incant_wasm_abi::TimerHeader::SIZE
                || bytes.len() > Self::MAX_ENCODED_BYTES - 8
        } else {
            bytes.len() != payload_end
        }) || Declaration::decode(&bytes[..Declaration::SIZE]) != Some(declaration)
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
        let (timers, coroutines) = if version == 3 {
            let tail = &bytes[payload_end..];
            let len = u32::from_le_bytes(
                tail.get(..4)
                    .ok_or(Error::Abi("coroutine snapshot framing"))?
                    .try_into()
                    .unwrap(),
            ) as usize;
            let timer = tail
                .get(
                    4..4usize
                        .checked_add(len)
                        .ok_or(Error::Abi("timer length overflow"))?,
                )
                .ok_or(Error::Abi("timer framing"))?;
            crate::timers::State::restore(timer, tick)?;
            let tasks = &tail[4 + len..];
            crate::coroutines::checkpoint::validate(tasks, tick)?;
            (timer.to_vec(), Some(tasks.into()))
        } else if version == 2 {
            let value = &bytes[payload_end..];
            crate::timers::State::restore(value, tick)?;
            (value.to_vec(), None)
        } else {
            (crate::timers::State::default().checkpoint(tick), None)
        };
        Ok(Self {
            declaration,
            bytes: bytes[HEADER..payload_end].into(),
            timers: timers.into(),
            coroutines,
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
    pub(crate) fn snapshot(
        &self,
        tick: u64,
        watermark: i64,
        timers: Vec<u8>,
        coroutines: Option<Vec<u8>>,
    ) -> Result<PersistentSnapshot, Error> {
        if self.dirty || watermark == i64::MAX {
            return Err(Error::Abi("persistent state not quiescent"));
        }
        Ok(PersistentSnapshot {
            declaration: self.declaration,
            bytes: self.committed.clone(),
            tick,
            watermark,
            timers: timers.into(),
            coroutines: coroutines.map(Vec::into_boxed_slice),
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

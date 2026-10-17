//! Bounded module-local simulation schedule, committed with the ECS transaction.
use crate::BoundaryError as Error;
use incant_wasm_abi::{self as abi, timers::MAX_TICK};
use std::collections::BTreeMap;

#[derive(Clone)]
struct Timer {
    due: u64,
    interval: u32,
}
#[derive(Clone, Default)]
struct Schedule {
    elapsed: f64,
    timers: BTreeMap<String, Timer>,
}
#[derive(Default)]
pub(crate) struct State {
    committed: Schedule,
    staged: Option<Schedule>,
    events: Vec<abi::TimerEvent>,
    tick: u64,
    calls: usize,
    pub rejected: bool,
}
fn deadline(tick: u64, delay: u32) -> Result<u64, Error> {
    tick.checked_add(u64::from(delay))
        .filter(|n| delay > 0 && *n <= MAX_TICK)
        .ok_or(Error::Abi("timer deadline"))
}
impl State {
    pub fn begin(&mut self, tick: u64, dt: f64) -> Result<(), Error> {
        self.abort();
        self.rejected = false;
        self.calls = 0;
        let elapsed = self.committed.elapsed + dt;
        if tick == 0 || tick > MAX_TICK || !dt.is_finite() || dt <= 0. || !elapsed.is_finite() {
            return Err(Error::Abi("timer clock"));
        }
        let mut next = self.committed.clone();
        next.elapsed = elapsed;
        for (id, timer) in &mut next.timers {
            if timer.due < tick {
                return Err(Error::Abi("missed timer deadline"));
            }
            if timer.due == tick {
                let (id, id_len) =
                    abi::timers::encode_id(id).map_err(|_| Error::Abi("timer id"))?;
                self.events.push(abi::TimerEvent {
                    id,
                    id_len,
                    reserved: 0,
                    scheduled_tick: tick,
                });
                if timer.interval != 0 {
                    timer.due = deadline(tick, timer.interval)?;
                }
            }
        }
        next.timers.retain(|_, timer| timer.due > tick);
        self.tick = tick;
        self.staged = Some(next);
        Ok(())
    }
    pub fn abort(&mut self) {
        self.staged = None;
        self.events.clear();
    }
    pub fn commit(&mut self) {
        self.committed = self.staged.take().expect("active timer transaction");
        self.events.clear();
    }
    pub fn replacement(&mut self) -> Self {
        Self {
            committed: std::mem::take(&mut self.committed),
            ..Self::default()
        }
    }
    pub fn snapshot_len(&self) -> usize {
        abi::TimerHeader::SIZE + self.events.len() * abi::TimerEvent::SIZE
    }
    pub fn snapshot(&self, output: &mut [u8]) -> Result<(), Error> {
        let state = self
            .staged
            .as_ref()
            .ok_or(Error::Abi("inactive timer transaction"))?;
        output[..abi::TimerHeader::SIZE].copy_from_slice(
            &abi::TimerHeader {
                magic: *abi::TIMER_MAGIC,
                tick: self.tick,
                elapsed_seconds: state.elapsed,
                count: self.events.len() as u32,
                stride: abi::TimerEvent::SIZE as u32,
            }
            .encode(),
        );
        for (event, to) in self.events.iter().zip(
            output[abi::TimerHeader::SIZE..]
                .as_chunks_mut::<{ abi::TimerEvent::SIZE }>()
                .0,
        ) {
            to.copy_from_slice(&event.encode());
        }
        Ok(())
    }
    pub fn command(&mut self, row: abi::TimerCommand) -> Result<(), Error> {
        self.calls += 1;
        if self.calls > abi::TIMER_MAX_ACTIONS {
            return Err(Error::Limit("timer action quota"));
        }
        abi::timers::validate_command(&row).map_err(|_| Error::Abi("timer command"))?;
        let id = abi::timers::id(&row.id, row.id_len).map_err(|_| Error::Abi("timer id"))?;
        let next = self
            .staged
            .as_mut()
            .ok_or(Error::Abi("inactive timer transaction"))?;
        if row.kind == 1 {
            next.timers.remove(id);
        } else {
            let due = deadline(self.tick, row.delay)?;
            if next.timers.len() == abi::TIMER_MAX_TIMERS && !next.timers.contains_key(id) {
                return Err(Error::Limit("pending timer quota"));
            }
            next.timers.insert(
                id.to_owned(),
                Timer {
                    due,
                    interval: row.interval,
                },
            );
        }
        Ok(())
    }
    pub fn checkpoint(&self, tick: u64) -> Vec<u8> {
        let mut bytes = abi::TimerHeader {
            magic: *abi::TIMER_STATE_MAGIC,
            tick,
            elapsed_seconds: self.committed.elapsed,
            count: self.committed.timers.len() as u32,
            stride: abi::TimerStateRow::SIZE as u32,
        }
        .encode()
        .to_vec();
        for (id, timer) in &self.committed.timers {
            let (id, id_len) = abi::timers::encode_id(id).expect("validated timer id");
            bytes.extend(
                abi::TimerStateRow {
                    id,
                    id_len,
                    interval: timer.interval,
                    due: timer.due,
                }
                .encode(),
            );
        }
        bytes
    }
    pub fn restore(bytes: &[u8], tick: u64) -> Result<Self, Error> {
        let invalid = || Error::Abi("timer checkpoint");
        let h = abi::TimerHeader::decode(bytes).ok_or_else(invalid)?;
        if &h.magic != abi::TIMER_STATE_MAGIC
            || h.tick != tick
            || tick > MAX_TICK
            || !h.elapsed_seconds.is_finite()
            || h.elapsed_seconds < 0.
            || h.elapsed_seconds.to_bits() == (-0.0f64).to_bits()
            || h.count as usize > abi::TIMER_MAX_TIMERS
            || h.stride as usize != abi::TimerStateRow::SIZE
            || bytes.len() != abi::TimerHeader::SIZE + h.count as usize * abi::TimerStateRow::SIZE
            || (tick == 0 && (h.elapsed_seconds != 0. || h.count != 0))
        {
            return Err(invalid());
        }
        let mut timers = BTreeMap::new();
        let mut previous = None;
        for raw in bytes[abi::TimerHeader::SIZE..]
            .as_chunks::<{ abi::TimerStateRow::SIZE }>()
            .0
        {
            let row = abi::TimerStateRow::decode(raw).ok_or_else(invalid)?;
            let id = abi::timers::id(&row.id, row.id_len).map_err(|_| invalid())?;
            if row.due <= tick || row.due > MAX_TICK || previous.is_some_and(|p| p >= row.id) {
                return Err(invalid());
            }
            previous = Some(row.id);
            timers.insert(
                id.to_owned(),
                Timer {
                    due: row.due,
                    interval: row.interval,
                },
            );
        }
        Ok(Self {
            committed: Schedule {
                elapsed: h.elapsed_seconds,
                timers,
            },
            ..Self::default()
        })
    }
}

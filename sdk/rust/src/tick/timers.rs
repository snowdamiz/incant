use super::Tick;
use crate::{Error, Result, raw};
use incant_wasm_abi as abi;

impl Tick<'_> {
    /// Accepted-update clock and sorted due events. The first callback is tick1;
    /// neither wall-clock time nor a timer command advances this clock.
    pub fn timers<'a>(&mut self, scratch: &'a mut [u8]) -> Result<abi::timers::Snapshot<'a>> {
        self.ready()?;
        // SAFETY: scalar call under this callback's active authority.
        let len = unsafe { raw::incant_timers_v1_snapshot_len(self.epoch) };
        let len = self.snapshot_length(len, scratch.len(), abi::TIMER_MAX_BYTES)?;
        // SAFETY: initialized exclusive scratch is checked and copied synchronously.
        let copied = unsafe {
            raw::incant_timers_v1_read_snapshot(
                self.epoch,
                scratch.as_mut_ptr() as usize as i32,
                len as i32,
            )
        };
        self.check(if copied == len as i32 {
            Ok(())
        } else {
            Err(Error::Malformed)
        })?;
        self.check(abi::timers::Snapshot::parse(&scratch[..len]).map_err(|_| Error::Malformed))
    }
    /// Replace this module's timer. Positive fixed tick counts; None is one-shot.
    /// Events already published to this callback remain unchanged.
    pub fn set_timer(
        &mut self,
        id: &str,
        delay_ticks: u32,
        interval_ticks: Option<u32>,
    ) -> Result<()> {
        self.ready()?;
        self.check(if interval_ticks == Some(0) {
            Err(Error::Bounds)
        } else {
            Ok(())
        })?;
        self.timer_command(id, 0, delay_ticks, interval_ticks.unwrap_or(0))
    }
    /// Cancelling an absent ID is valid and consumes one of256 commands/tick.
    pub fn cancel_timer(&mut self, id: &str) -> Result<()> {
        self.timer_command(id, 1, 0, 0)
    }
    fn timer_command(&mut self, id: &str, kind: u32, delay: u32, interval: u32) -> Result<()> {
        self.ready()?;
        let (id, id_len) = self.check(abi::timers::encode_id(id).map_err(|_| Error::Bounds))?;
        let row = abi::TimerCommand {
            id,
            id_len,
            kind,
            delay,
            interval,
        };
        self.check(abi::timers::validate_command(&row).map_err(|_| Error::Bounds))?;
        let bytes = row.encode();
        // SAFETY: initialized fixed frame is copied within the synchronous call.
        let result = unsafe {
            raw::incant_timers_v1_command(
                self.epoch,
                bytes.as_ptr() as usize as i32,
                bytes.len() as i32,
            )
        };
        self.check(if result == 0 {
            Ok(())
        } else {
            Err(Error::Failed)
        })
    }
}
